//! Tests d'intégration : la page de réglages, la commande /essentials et le Core
//! ensemble, comme les helpers `h_settings`, `h_click` et `h_command` les relient
//! dans le jeu (ces helpers appellent le jeu et ne sont pas compilés ici : les
//! fonctions ci-dessous refont exactement leur enchaînement).

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::game::essentials_command as command;
use crate::game::essentials_events as ev;
use crate::game::essentials_registry::{MODULES, ModuleId};
use crate::game::essentials_settings as page;
use crate::game::essentials_state as state;
use crate::game::essentials_store as store;

const WAIT: Duration = Duration::from_secs(5);

struct Dir(PathBuf);
impl Dir {
    fn new(name: &str) -> Dir {
        let d = std::env::temp_dir().join(format!("horizon-essentials-it-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Dir(d)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Démarre le Core sur `dir` et attend la fin du chargement.
fn start(dir: &Path) {
    ev::frame(|| dir.to_path_buf(), 16.0);
    assert_eq!(state::with_existing(|s| s.wait_idle(WAIT)), Some(true));
}

/// h_settings : les libellés de la page.
fn page_labels(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    ev::view(|| dir.to_path_buf(), |snapshot| page::labels(snapshot, &mut out));
    out
}

/// h_click : un clic sur la page.
fn page_click(dir: &Path, button: usize) {
    let loaded = ev::view(|| dir.to_path_buf(), |s| s.is_some()).unwrap_or(false);
    if let Some(action) = page::click(button, loaded) {
        ev::apply(|| dir.to_path_buf(), action);
    }
}

/// h_command : /essentials <args>, et les lignes écrites dans le chat.
fn run(dir: &Path, args: &str) -> Vec<String> {
    let reply = ev::view(|| dir.to_path_buf(), |snapshot| command::run(args, snapshot)).unwrap();
    let mut lines = reply.lines;
    if let Some(action) = reply.action {
        let changed = ev::apply(|| dir.to_path_buf(), action);
        let line = ev::view(|| dir.to_path_buf(), |s| {
            s.map(|s| command::outcome(action, changed, s.resolved)).unwrap_or_default()
        })
        .unwrap();
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

fn status_on(dir: &Path) -> bool {
    ev::view(|| dir.to_path_buf(), |s| s.unwrap().resolved.on[ModuleId::Status.index()]).unwrap()
}

/// Laisse passer le délai de sauvegarde et attend le thread d'E/S.
fn settle() {
    for _ in 0..1100 {
        ev::frame(PathBuf::new, 1.0);
    }
    assert_eq!(state::with_existing(|s| s.wait_idle(WAIT)), Some(true));
}

#[test]
fn la_page_et_la_commande_montrent_le_meme_etat() {
    let d = Dir::new("meme-etat");
    start(&d.0);
    assert_eq!(page_labels(&d.0)[0], "Témoin Essentials : ACTIVÉ");
    assert_eq!(run(&d.0, "list"), ["status : ACTIVÉ (Témoin Essentials)"]);

    // Une commande change l'état : la page le montre.
    assert_eq!(run(&d.0, "off status"), ["Témoin Essentials : désactivé"]);
    assert_eq!(page_labels(&d.0)[0], "Témoin Essentials : DÉSACTIVÉ");

    // Un clic sur la page change l'état : la commande le montre.
    page_click(&d.0, 0);
    assert_eq!(run(&d.0, "list"), ["status : ACTIVÉ (Témoin Essentials)"]);
    assert!(status_on(&d.0));
}

#[test]
fn une_page_par_module_du_registre() {
    let d = Dir::new("registre");
    start(&d.0);
    let labels = page_labels(&d.0);
    assert_eq!(labels.len(), MODULES.len() + 2, "un bouton par module, puis recharger et réinitialiser");
    assert_eq!(run(&d.0, "list").len(), MODULES.len());
}

#[test]
fn plusieurs_actions_successives_sont_toutes_sauvegardees() {
    let d = Dir::new("rafale");
    start(&d.0);
    // Clics et commandes enchaînés sans attendre, pendant que le thread d'E/S écrit.
    for i in 0..25 {
        page_click(&d.0, 0);
        run(&d.0, if i % 2 == 0 { "toggle status" } else { "on status" });
        state::with_existing(|s| s.flush());
    }
    run(&d.0, "off status");
    let expected = state::with_existing(|s| s.config.clone()).unwrap();
    settle();
    let on_disk = store::load(&d.0);
    assert_eq!(on_disk.config, expected, "le disque a la toute dernière modification");
    assert!(!on_disk.config.module(ModuleId::Status).on);
}

#[test]
fn la_reinitialisation_globale_passe_par_la_confirmation() {
    let d = Dir::new("reset");
    start(&d.0);
    run(&d.0, "off status");
    let ask = run(&d.0, "reset");
    assert!(ask[0].contains("TOUS les réglages"));
    assert!(!status_on(&d.0), "rien n'est réinitialisé avant la confirmation");
    assert_eq!(run(&d.0, "reset confirm"), ["Tous les réglages remis par défaut"]);
    assert!(status_on(&d.0));
    settle();
    assert!(store::load(&d.0).config.module(ModuleId::Status).on, "et c'est sauvegardé");
}

#[test]
fn la_reinitialisation_depuis_la_page_demande_deux_clics() {
    let d = Dir::new("reset-page");
    start(&d.0);
    run(&d.0, "off status");
    let reset = MODULES.len() + 1;
    page_click(&d.0, reset);
    assert!(!status_on(&d.0), "premier clic : rien ne change");
    assert!(page_labels(&d.0)[reset].starts_with("Confirmer (15 s)"));
    page_click(&d.0, reset);
    assert!(status_on(&d.0), "second clic : tout est remis par défaut");
}

#[test]
fn le_rechargement_relit_la_configuration_du_disque() {
    let d = Dir::new("reload");
    start(&d.0);
    let mut other = crate::game::essentials_config::Config::default();
    other.apply(crate::game::essentials_types::Action::Set(ModuleId::Status, false));
    store::save(&d.0, store::DEFAULT_PROFILE, &other).unwrap();
    run(&d.0, "reload");
    assert_eq!(state::with_existing(|s| s.wait_idle(WAIT)), Some(true));
    assert!(!status_on(&d.0));
    assert_eq!(page_labels(&d.0)[0], "Témoin Essentials : DÉSACTIVÉ");
}

#[test]
fn un_module_inconnu_ne_change_rien() {
    let d = Dir::new("inconnu");
    start(&d.0);
    let before = state::with_existing(|s| s.config.clone()).unwrap();
    assert_eq!(run(&d.0, "off radar"), ["Module inconnu : radar (modules : status)"]);
    assert_eq!(run(&d.0, "reset radar"), ["Module inconnu : radar (modules : status)"]);
    assert_eq!(state::with_existing(|s| s.config.clone()).unwrap(), before);
}
