// Horizon Essentials : l'état global unique (crate::game::essentials_state).
//
// Un seul `State` par thread : celui de la fenêtre. En solo, le serveur intégré
// tourne sur un autre thread et aurait le sien ; les helpers n'appellent le Core
// que depuis la fenêtre (`native_window()`).
//
// Résilience : l'accès passe par `with`, qui refuse un accès imbriqué (un hook
// déclenché pendant un autre) au lieu de paniquer, ce qui arrêterait le jeu.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use crate::game::essentials_config::{Config, Resolved};
use crate::game::essentials_store::{self as store, SaveTimer};
use crate::game::essentials_types::Action;

/// Au plus autant de messages en attente pour le chat (les plus anciens partent).
const MAX_MESSAGES: usize = 16;

pub struct State {
    pub dir: PathBuf,
    pub profile: String,
    pub config: Config,
    /// La configuration prête pour le rendu, recalculée à chaque changement.
    pub resolved: Resolved,
    /// Augmente à chaque changement de configuration : les widgets et l'interface
    /// comparent ce nombre pour savoir s'il faut reconstruire leurs formes.
    pub revision: u64,
    pub timer: SaveTimer,
    /// Les messages à montrer dans le chat, dès que le joueur est dans un monde.
    pub messages: VecDeque<String>,
}

impl State {
    pub fn load(dir: &Path) -> State {
        let loaded = store::load(dir);
        let mut state = State {
            dir: dir.to_owned(),
            profile: loaded.profile,
            resolved: loaded.config.resolve(),
            config: loaded.config,
            revision: 0,
            timer: SaveTimer::default(),
            messages: VecDeque::new(),
        };
        for w in loaded.warnings {
            state.say(w);
        }
        state
    }

    /// Un message pour le chat, préfixé du nom du mod.
    pub fn say(&mut self, text: impl Into<String>) {
        if self.messages.len() == MAX_MESSAGES {
            self.messages.pop_front();
        }
        self.messages.push_back(format!("[Essentials] {}", text.into()));
    }

    /// Applique une action. Vrai si quelque chose a changé.
    pub fn apply(&mut self, action: Action) -> bool {
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

    /// Relit le disque, en sauvegardant d'abord un changement en attente.
    pub fn reload(&mut self) {
        self.flush();
        let loaded = store::load(&self.dir);
        self.profile = loaded.profile;
        self.config = loaded.config;
        for w in loaded.warnings {
            self.say(w);
        }
        self.changed();
        self.say("configuration rechargée");
    }

    fn changed(&mut self) {
        self.resolved = self.config.resolve();
        self.revision += 1;
    }

    /// Le temps passe : sauvegarde quand la configuration n'a plus bougé depuis une seconde.
    pub fn advance(&mut self, elapsed_ms: f32) {
        if self.timer.advance(elapsed_ms) {
            self.flush();
        }
    }

    /// Sauvegarde maintenant ce qui est en attente.
    pub fn flush(&mut self) {
        if !self.timer.pending() {
            return;
        }
        self.timer.done();
        if let Err(e) = store::save(&self.dir, &self.profile, &self.config) {
            self.say(format!("sauvegarde impossible ({e}) : réglages gardés jusqu'à la fermeture"));
        }
    }

    /// Le prochain message pour le chat.
    pub fn next_message(&mut self) -> Option<String> {
        self.messages.pop_front()
    }
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Donne l'état à `f`, en le chargeant la première fois depuis le dossier que
/// `dir` indique. None si l'état est déjà emprunté (accès imbriqué) : l'appelant
/// ne fait rien plutôt que de faire planter le jeu.
pub fn with<R>(dir: impl FnOnce() -> PathBuf, f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        let state = slot.get_or_insert_with(|| State::load(&dir()));
        Some(f(state))
    })
}

/// L'état s'il est déjà chargé (sans jamais lire le disque) : pour le rendu.
pub fn with_loaded<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        slot.as_mut().map(f)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::essentials_registry::ModuleId;
    use crate::game::essentials_store::tests::TempDir;
    use crate::game::essentials_store::{DEFAULT_PROFILE, profile_path};

    #[test]
    fn une_action_est_sauvegardee_apres_une_seconde_calme() {
        let d = TempDir::new("etat-sauvegarde");
        let mut s = State::load(&d.0);
        let on = s.resolved.on[ModuleId::Status.index()];
        assert!(s.apply(Action::Toggle(ModuleId::Status)));
        assert_eq!(s.revision, 1);
        assert_eq!(s.resolved.on[ModuleId::Status.index()], !on, "Resolved suit la configuration");
        s.advance(400.0);
        assert!(!profile_path(&d.0, DEFAULT_PROFILE).exists(), "pas encore");
        s.advance(700.0);
        let saved = store::load(&d.0);
        assert_eq!(saved.config.module(ModuleId::Status).on, !on);
    }

    #[test]
    fn une_action_sans_effet_ne_sauvegarde_rien() {
        let d = TempDir::new("etat-sans-effet");
        let mut s = State::load(&d.0);
        let on = s.resolved.on[ModuleId::Status.index()];
        assert!(!s.apply(Action::Set(ModuleId::Status, on)));
        assert_eq!(s.revision, 0);
        s.advance(5000.0);
        assert!(!profile_path(&d.0, DEFAULT_PROFILE).exists());
    }

    #[test]
    fn recharger_relit_le_disque() {
        let d = TempDir::new("etat-recharger");
        let mut s = State::load(&d.0);
        let mut other = Config::default();
        other.apply(Action::Toggle(ModuleId::Status));
        store::save(&d.0, DEFAULT_PROFILE, &other).unwrap();
        assert!(s.apply(Action::Reload));
        assert_eq!(s.config, other);
        assert_eq!(s.next_message().as_deref(), Some("[Essentials] configuration rechargée"));
    }

    #[test]
    fn les_avertissements_vont_dans_le_chat() {
        let d = TempDir::new("etat-avertissements");
        std::fs::write(d.0.join("active"), "Interdit").unwrap();
        let mut s = State::load(&d.0);
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
        assert!(with_loaded(|s| s.profile.clone()).is_some(), "chargé une fois, gardé ensuite");
    }
}
