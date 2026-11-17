// Horizon Essentials : l'état global unique (crate::game::essentials_state).
//
// Un seul `State` par thread : celui de la fenêtre. En solo, le serveur intégré
// tourne sur un autre thread et aurait le sien ; les helpers n'appellent le Core
// que depuis la fenêtre (`native_window()`).
//
// Disque : le thread du jeu n'y touche jamais. Le chargement et les sauvegardes
// passent par le thread « Essentials I/O » (essentials_store::Io). Si ce thread ne
// peut pas démarrer, le Core travaille sans lui (comme avant) plutôt que de
// perdre les réglages.
//
// Chargement asynchrone : tant que la configuration n'est pas lue, `loaded` est
// faux, le HUD ne dessine rien et les modules ne reçoivent rien. Un rechargement
// garde la configuration actuelle affichée jusqu'à l'arrivée de la nouvelle.
//
// Résilience : l'accès passe par `with`, qui refuse un accès imbriqué (un hook
// déclenché pendant un autre) au lieu de paniquer, ce qui arrêterait le jeu.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::game::essentials_config::{Config, Resolved};
use crate::game::essentials_registry::{ModuleId, Modules};
use crate::game::essentials_store::{self as store, Io, Loaded, Reply, Request, SaveTimer};
use crate::game::essentials_types::Action;

/// Au plus autant de messages en attente pour le chat (les plus anciens partent).
const MAX_MESSAGES: usize = 16;
/// À la fermeture, le temps laissé au thread d'E/S pour finir d'écrire.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

pub struct State {
    pub dir: PathBuf,
    pub profile: String,
    pub config: Config,
    /// La configuration prête pour le rendu, recalculée à chaque changement.
    pub resolved: Resolved,
    /// Faux tant que la configuration n'a pas été lue une première fois.
    pub loaded: bool,
    /// Augmente à chaque changement de configuration : les widgets et l'interface
    /// comparent ce nombre pour savoir s'il faut reconstruire leurs formes.
    pub revision: u64,
    pub timer: SaveTimer,
    /// Les messages à montrer dans le chat, dès que le joueur est dans un monde.
    pub messages: VecDeque<String>,
    /// L'état de chaque module.
    pub modules: Modules,
    /// Les ticks vus depuis le chargement.
    pub tick: u64,
    io: Option<Io>,
    /// La dernière sauvegarde envoyée et la dernière confirmée par le thread d'E/S.
    save_sent: u64,
    save_done: u64,
    reloading: bool,
}

impl State {
    fn empty(dir: &Path) -> State {
        let config = Config::default();
        State {
            dir: dir.to_owned(),
            profile: store::DEFAULT_PROFILE.to_owned(),
            resolved: config.resolve(),
            config,
            loaded: false,
            revision: 0,
            timer: SaveTimer::default(),
            messages: VecDeque::new(),
            modules: Modules::default(),
            tick: 0,
            io: None,
            save_sent: 0,
            save_done: 0,
            reloading: false,
        }
    }

    /// Démarre : demande la lecture au thread d'E/S et rend aussitôt la main.
    pub fn start(dir: &Path) -> State {
        let mut s = State::empty(dir);
        match Io::spawn() {
            Some(io) if io.send(Request::Load { dir: dir.to_owned() }).is_ok() => s.io = Some(io),
            _ => {
                // Pas de thread d'E/S : lecture directe, une seule fois.
                s.finish_load(store::load(dir));
            }
        }
        s
    }

    /// Lecture directe, sans thread (tests).
    pub fn load(dir: &Path) -> State {
        let mut s = State::empty(dir);
        s.finish_load(store::load(dir));
        s
    }

    fn finish_load(&mut self, loaded: Loaded) {
        self.profile = loaded.profile;
        self.config = loaded.config;
        for w in loaded.warnings {
            self.say(w);
        }
        self.loaded = true;
        self.changed();
        if std::mem::take(&mut self.reloading) {
            self.say("configuration rechargée");
        }
    }

    /// Un message pour le chat, préfixé du nom du mod.
    pub fn say(&mut self, text: impl Into<String>) {
        if self.messages.len() == MAX_MESSAGES {
            self.messages.pop_front();
        }
        self.messages.push_back(format!("[Essentials] {}", text.into()));
    }

    /// Une réponse du thread d'E/S est attendue (lecture ou écriture en cours).
    fn awaiting(&self) -> bool {
        !self.loaded || self.reloading || self.save_done < self.save_sent
    }

    /// Relit les réponses du thread d'E/S, sans attendre. Quand rien n'est attendu
    /// (presque toujours), le canal n'est même pas consulté.
    pub fn poll(&mut self) {
        if !self.awaiting() {
            return;
        }
        while let Some(reply) = self.io.as_ref().and_then(Io::try_recv) {
            self.handle(reply);
        }
    }

    fn handle(&mut self, reply: Reply) {
        match reply {
            Reply::Loaded(loaded) => self.finish_load(loaded),
            Reply::Saved { seq, error } => {
                self.save_done = self.save_done.max(seq);
                if let Some(e) = error {
                    self.say(format!("sauvegarde impossible ({e}) : réglages gardés en mémoire"));
                }
            }
            Reply::Stopped => {}
        }
    }

    /// Applique une action. Vrai si quelque chose a changé (ou va changer : rechargement).
    pub fn apply(&mut self, action: Action) -> bool {
        if !self.loaded {
            self.say("configuration en cours de chargement, réessaie dans un instant");
            return false;
        }
        if action == Action::Reload {
            self.reload();
            return true;
        }
        if !self.config.apply(action) {
            return false;
        }
        self.changed();
        self.timer.mark();
        true
    }

    /// Relit le disque, après avoir envoyé un changement en attente (le thread d'E/S
    /// traite les demandes dans l'ordre : l'écriture passe avant la lecture). La
    /// configuration actuelle reste affichée jusqu'à l'arrivée de la nouvelle.
    pub fn reload(&mut self) {
        self.flush();
        self.reloading = true;
        let sent = self.io.as_ref().is_some_and(|io| io.send(Request::Load { dir: self.dir.clone() }).is_ok());
        if !sent {
            self.io = None;
            let loaded = store::load(&self.dir);
            self.finish_load(loaded);
        }
    }

    fn changed(&mut self) {
        let before = self.resolved.on;
        self.resolved = self.config.resolve();
        self.revision += 1;
        // Prévient chaque module dont l'activation a changé.
        for id in ModuleId::all() {
            let on = self.resolved.on[id.index()];
            if on != before[id.index()] {
                self.modules.get_mut(id).on_enabled(on);
            }
        }
    }

    /// Le temps passe : relit les réponses du thread d'E/S, et envoie la sauvegarde
    /// quand la configuration n'a plus bougé depuis une seconde.
    pub fn advance(&mut self, elapsed_ms: f32) {
        self.poll();
        if self.loaded && self.timer.advance(elapsed_ms) {
            self.flush();
        }
    }

    /// Envoie maintenant au thread d'E/S le changement en attente. Le JSON est
    /// préparé ici (quelques µs), l'écriture se fait sur le thread d'E/S.
    pub fn flush(&mut self) {
        if !self.timer.pending() {
            return;
        }
        self.timer.done();
        let json = self.config.to_json();
        self.save_sent += 1;
        let request = Request::Save {
            dir: self.dir.clone(),
            profile: self.profile.clone(),
            json,
            seq: self.save_sent,
        };
        let refused = match &self.io {
            Some(io) => io.send(request).err(),
            None => Some(request),
        };
        if let Some(Request::Save { json, seq, .. }) = refused {
            // Plus de thread d'E/S : écriture directe plutôt que perte des réglages.
            self.io = None;
            if let Err(e) = store::save_json(&self.dir, &self.profile, &json) {
                self.say(format!("sauvegarde impossible ({e}) : réglages gardés en mémoire"));
            }
            self.save_done = seq;
        }
    }

    /// Fermeture du jeu : envoie le dernier changement et attend que le thread d'E/S
    /// ait tout écrit (au plus `SHUTDOWN_TIMEOUT`).
    pub fn shutdown(&mut self) {
        self.flush();
        if let Some(io) = self.io.take() {
            for reply in io.finish(SHUTDOWN_TIMEOUT) {
                self.handle(reply);
            }
        }
    }

    /// Vrai quand rien n'attend le disque : configuration lue (et relue si un
    /// rechargement est en cours) et sauvegardes confirmées.
    pub fn idle(&self) -> bool {
        self.loaded && !self.reloading && !self.timer.pending() && self.save_done >= self.save_sent
    }

    /// Attend que rien n'attende le disque (tests et outils de mesure ; jamais en jeu).
    pub fn wait_idle(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while !self.idle() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            match self.io.as_ref().and_then(|io| io.recv_timeout(left)) {
                Some(reply) => self.handle(reply),
                None => return self.idle(),
            }
        }
        true
    }

    /// Le prochain message pour le chat.
    pub fn next_message(&mut self) -> Option<String> {
        self.messages.pop_front()
    }
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Donne l'état à `f`, en le démarrant la première fois (lecture demandée au thread
/// d'E/S depuis le dossier que `dir` indique). None si l'état est déjà emprunté
/// (accès imbriqué) : l'appelant ne fait rien plutôt que de faire planter le jeu.
pub fn with<R>(dir: impl FnOnce() -> PathBuf, f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        let state = slot.get_or_insert_with(|| State::start(&dir()));
        Some(f(state))
    })
}

/// L'état s'il existe déjà, chargé ou non (jamais créé ici).
pub fn with_existing<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        slot.as_mut().map(f)
    })
}

/// L'état s'il est chargé : pour le rendu, qui ne déclenche jamais rien.
pub fn with_loaded<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        slot.as_mut().filter(|s| s.loaded).map(f)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::essentials_store::tests::TempDir;
    use crate::game::essentials_store::{DEFAULT_PROFILE, profile_path};

    const WAIT: Duration = Duration::from_secs(5);

    fn started(d: &TempDir) -> State {
        let mut s = State::start(&d.0);
        assert!(s.wait_idle(WAIT), "chargement asynchrone");
        s
    }

    #[test]
    fn le_chargement_est_asynchrone_et_le_hud_attend() {
        let d = TempDir::new("etat-async");
        let mut s = State::start(&d.0);
        // Juste après le démarrage, rien n'est forcément lu : on ne dessine rien,
        // et une action est refusée proprement.
        if !s.loaded {
            assert!(!s.apply(Action::Toggle(ModuleId::Status)));
            assert!(s.next_message().unwrap().contains("en cours de chargement"));
        }
        assert!(s.wait_idle(WAIT));
        assert!(s.loaded);
    }

    #[test]
    fn une_action_est_sauvegardee_apres_une_seconde_calme() {
        let d = TempDir::new("etat-sauvegarde");
        let mut s = started(&d);
        let on = s.resolved.on[ModuleId::Status.index()];
        assert!(s.apply(Action::Toggle(ModuleId::Status)));
        assert_eq!(s.resolved.on[ModuleId::Status.index()], !on, "Resolved suit la configuration");
        s.advance(400.0);
        assert!(!profile_path(&d.0, DEFAULT_PROFILE).exists(), "pas encore");
        s.advance(700.0);
        assert!(s.wait_idle(WAIT));
        let saved = store::load(&d.0);
        assert_eq!(saved.config.module(ModuleId::Status).on, !on);
    }

    #[test]
    fn une_sauvegarde_pendant_une_autre_n_est_pas_perdue() {
        let d = TempDir::new("etat-rafale");
        let mut s = started(&d);
        // 50 changements envoyés sans attendre : le dernier état doit arriver sur disque.
        for _ in 0..50 {
            s.apply(Action::Toggle(ModuleId::Status));
            s.flush();
        }
        let expected = s.config.clone();
        assert!(s.wait_idle(WAIT));
        assert_eq!(store::load(&d.0).config, expected);
    }

    #[test]
    fn la_fermeture_ecrit_le_dernier_changement() {
        let d = TempDir::new("etat-fermeture");
        let mut s = started(&d);
        s.apply(Action::Toggle(ModuleId::Status));
        // Fermeture tout de suite, sans attendre la seconde de calme.
        s.shutdown();
        assert_eq!(store::load(&d.0).config, s.config);
    }

    #[test]
    fn une_action_sans_effet_ne_sauvegarde_rien() {
        let d = TempDir::new("etat-sans-effet");
        let mut s = started(&d);
        let on = s.resolved.on[ModuleId::Status.index()];
        let revision = s.revision;
        assert!(!s.apply(Action::Set(ModuleId::Status, on)));
        assert_eq!(s.revision, revision);
        s.advance(5000.0);
        assert!(s.wait_idle(WAIT));
        assert!(!profile_path(&d.0, DEFAULT_PROFILE).exists());
    }

    #[test]
    fn recharger_relit_le_disque_sans_vider_l_affichage() {
        let d = TempDir::new("etat-recharger");
        let mut s = started(&d);
        let mut other = Config::default();
        other.apply(Action::Toggle(ModuleId::Status));
        store::save(&d.0, DEFAULT_PROFILE, &other).unwrap();
        assert!(s.apply(Action::Reload));
        assert!(s.loaded, "l'ancienne configuration reste affichée pendant la relecture");
        assert!(s.wait_idle(WAIT));
        assert_eq!(s.config, other);
        assert_eq!(s.next_message().as_deref(), Some("[Essentials] configuration rechargée"));
    }

    #[test]
    fn une_erreur_d_ecriture_devient_un_message() {
        let d = TempDir::new("etat-erreur");
        let mut s = started(&d);
        // Un fichier à la place du dossier « profiles » : l'écriture échoue.
        std::fs::write(d.0.join("profiles"), "pas un dossier").unwrap();
        s.apply(Action::Toggle(ModuleId::Status));
        s.flush();
        assert!(s.wait_idle(WAIT));
        assert!(s.next_message().unwrap().contains("sauvegarde impossible"));
    }

    #[test]
    fn les_avertissements_vont_dans_le_chat() {
        let d = TempDir::new("etat-avertissements");
        std::fs::write(d.0.join("active"), "Interdit").unwrap();
        let mut s = started(&d);
        assert!(s.next_message().unwrap().starts_with("[Essentials] profil actif invalide"));
        assert_eq!(s.next_message(), None);
        for i in 0..40 {
            s.say(format!("{i}"));
        }
        assert_eq!(s.messages.len(), MAX_MESSAGES, "la file est bornée");
    }

    #[test]
    fn un_acces_imbrique_est_refuse_sans_paniquer() {
        let d = TempDir::new("etat-imbrique");
        let dir = d.0.clone();
        let outer = with(
            || dir.clone(),
            |_| with_loaded(|_| ()).is_none() && with(|| d.0.clone(), |_| ()).is_none(),
        );
        assert_eq!(outer, Some(true));
        assert!(with_existing(|s| s.wait_idle(WAIT)).unwrap());
        assert!(with_loaded(|s| s.profile.clone()).is_some(), "chargé une fois, gardé ensuite");
    }
}
