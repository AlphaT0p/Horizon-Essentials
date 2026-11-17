// Horizon Essentials : la page de réglages dans Options > Mods (crate::game::essentials_settings).
//
// Horizon dessine la page : une liste de boutons dont le mod donne les libellés,
// redemandés après chaque clic. Cette page ne change rien elle-même : elle
// lit l'instantané du Core (`essentials_events::view`) et traduit un clic en
// `Action`, que le Core applique. Les modules viennent du registre : un module
// ajouté au registre a son bouton sans rien changer ici.
// « Tout réinitialiser » demande un second clic de confirmation.

use std::cell::Cell;
use std::time::{Duration, Instant};

use crate::game::essentials_events::Snapshot;
use crate::game::essentials_registry::{MODULES, ModuleId};
use crate::game::essentials_types::Action;

/// Le libellé unique de la page tant que la configuration est en cours de chargement.
pub const LOADING: &str = "Chargement de la configuration…";

/// Un bouton de la page, dans l'ordre d'affichage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Module(ModuleId),
    Reload,
    ResetAll,
}

/// Le délai pour confirmer « Tout réinitialiser » (le même que pour la commande).
pub const RESET_DELAY: Duration = Duration::from_secs(15);

thread_local! {
    // Le moment du premier clic sur « Tout réinitialiser », pas encore confirmé.
    static RESET_ARMED: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// La confirmation en attente, si elle n'a pas expiré.
fn armed(now: Instant) -> bool {
    RESET_ARMED.get().is_some_and(|at| now.saturating_duration_since(at) <= RESET_DELAY)
}

/// Le bouton numéro `index`.
pub fn button(index: usize) -> Option<Button> {
    let n = MODULES.len();
    match index {
        i if i < n => Some(Button::Module(MODULES[i].id)),
        i if i == n => Some(Button::Reload),
        i if i == n + 1 => Some(Button::ResetAll),
        _ => None,
    }
}

/// Les libellés de la page : un seul, « Chargement… », tant que la configuration n'est pas lue.
pub fn labels(snapshot: Option<Snapshot<'_>>, out: &mut Vec<String>) {
    labels_at(snapshot, out, Instant::now());
}

/// `labels` à l'instant `now` (les tests avancent le temps sans attendre).
pub fn labels_at(snapshot: Option<Snapshot<'_>>, out: &mut Vec<String>, now: Instant) {
    let Some(snapshot) = snapshot else {
        out.push(LOADING.to_owned());
        return;
    };
    for d in MODULES {
        let state = if snapshot.resolved.on[d.id.index()] { "ACTIVÉ" } else { "DÉSACTIVÉ" };
        out.push(format!("{} : {state}", d.name));
    }
    out.push("Recharger la configuration".to_owned());
    out.push(if armed(now) {
        "Confirmer (15 s) : TOUS les réglages par défaut ?".to_owned()
    } else {
        "Tout réinitialiser".to_owned()
    });
}

/// L'action d'un clic, s'il y en a une. Pendant le chargement, aucun clic ne fait rien.
pub fn click(index: usize, loaded: bool) -> Option<Action> {
    click_at(index, loaded, Instant::now())
}

/// `click` à l'instant `now`.
pub fn click_at(index: usize, loaded: bool, now: Instant) -> Option<Action> {
    if !loaded {
        return None;
    }
    let button = button(index)?;
    let confirmed = armed(now);
    RESET_ARMED.set(None);
    match button {
        Button::Module(id) => Some(Action::Toggle(id)),
        Button::Reload => Some(Action::Reload),
        Button::ResetAll if confirmed => Some(Action::ResetAll),
        Button::ResetAll => {
            RESET_ARMED.set(Some(now));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::essentials_config::Config;

    fn labels_of(config: &Config) -> Vec<String> {
        let resolved = config.resolve();
        let mut out = Vec::new();
        labels(Some(Snapshot { resolved: &resolved, profile: "default" }), &mut out);
        out
    }

    #[test]
    fn un_bouton_par_module_du_registre_puis_les_actions_generales() {
        let out = labels_of(&Config::default());
        assert_eq!(out.len(), MODULES.len() + 2);
        for (i, d) in MODULES.iter().enumerate() {
            assert!(out[i].starts_with(d.name), "le bouton {i} vient du registre");
        }
        assert_eq!(out[0], "Témoin Essentials : ACTIVÉ");
        assert_eq!(click(0, true), Some(Action::Toggle(ModuleId::Status)));
        assert_eq!(click(MODULES.len(), true), Some(Action::Reload));
        assert_eq!(click(99, true), None);
    }

    #[test]
    fn pendant_le_chargement_la_page_attend() {
        let mut out = Vec::new();
        labels(None, &mut out);
        assert_eq!(out, [LOADING]);
        assert_eq!(click(0, false), None, "aucune modification avant la fin du chargement");
    }

    #[test]
    fn tout_reinitialiser_demande_confirmation() {
        let reset = MODULES.len() + 1;
        click(0, true); // part d'un état sans confirmation en suspens
        assert_eq!(click(reset, true), None, "premier clic : on demande confirmation");
        assert_eq!(labels_of(&Config::default())[reset], "Confirmer (15 s) : TOUS les réglages par défaut ?");
        assert_eq!(click(reset, true), Some(Action::ResetAll));
        assert_eq!(click(reset, true), None, "la confirmation ne sert qu'une fois");
        assert_eq!(click(0, true), Some(Action::Toggle(ModuleId::Status)), "un autre clic annule");
        assert_eq!(click(reset, true), None);
    }

    #[test]
    fn la_confirmation_expire_apres_15_secondes() {
        let reset = MODULES.len() + 1;
        let t0 = Instant::now();
        click_at(0, true, t0);
        assert_eq!(click_at(reset, true, t0), None);
        let later = t0 + Duration::from_secs(16);
        let resolved = Config::default().resolve();
        let mut out = Vec::new();
        labels_at(Some(Snapshot { resolved: &resolved, profile: "default" }), &mut out, later);
        assert_eq!(out[reset], "Tout réinitialiser", "le libellé redevient normal après le délai");
        assert_eq!(click_at(reset, true, later), None, "un clic tardif redemande confirmation");
        assert_eq!(click_at(reset, true, later + Duration::from_secs(1)), Some(Action::ResetAll));
    }
}
