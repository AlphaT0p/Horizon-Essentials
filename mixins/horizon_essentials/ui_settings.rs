// Horizon Essentials : la page de réglages dans Options > Mods (crate::game::essentials_settings).
//
// Horizon dessine la page : une liste de boutons dont le mod donne les libellés,
// redemandés après chaque clic. Cette page ne change rien elle-même : elle
// traduit un clic en `Action`, que le Core applique.
// « Tout réinitialiser » demande un second clic de confirmation.

use std::cell::Cell;

use crate::game::essentials_config::Resolved;
use crate::game::essentials_registry::{MODULES, ModuleId};
use crate::game::essentials_types::Action;

/// Un bouton de la page, dans l'ordre d'affichage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Module(ModuleId),
    Reload,
    ResetAll,
}

thread_local! {
    // Vrai après un premier clic sur « Tout réinitialiser ».
    static RESET_ARMED: Cell<bool> = const { Cell::new(false) };
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

/// Les libellés de la page.
pub fn labels(resolved: &Resolved, out: &mut Vec<String>) {
    for d in MODULES {
        let state = if resolved.on[d.id.index()] { "ACTIVÉ" } else { "DÉSACTIVÉ" };
        out.push(format!("{} : {state}", d.name));
    }
    out.push("Recharger la configuration".to_owned());
    out.push(if RESET_ARMED.get() {
        "Confirmer : tout réinitialiser ?".to_owned()
    } else {
        "Tout réinitialiser".to_owned()
    });
}

/// L'action d'un clic, s'il y en a une.
pub fn click(index: usize) -> Option<Action> {
    let button = button(index)?;
    let armed = RESET_ARMED.replace(false);
    match button {
        Button::Module(id) => Some(Action::Toggle(id)),
        Button::Reload => Some(Action::Reload),
        Button::ResetAll if armed => Some(Action::ResetAll),
        Button::ResetAll => {
            RESET_ARMED.set(true);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::essentials_config::Config;

    #[test]
    fn un_bouton_par_module_puis_les_actions_generales() {
        let mut out = Vec::new();
        labels(&Config::default().resolve(), &mut out);
        assert_eq!(out.len(), MODULES.len() + 2);
        assert_eq!(out[0], "Témoin Essentials : ACTIVÉ");
        assert_eq!(click(0), Some(Action::Toggle(ModuleId::Status)));
        assert_eq!(click(MODULES.len()), Some(Action::Reload));
        assert_eq!(click(99), None);
    }

    #[test]
    fn tout_reinitialiser_demande_confirmation() {
        let reset = MODULES.len() + 1;
        click(0); // part d'un état sans confirmation en suspens
        assert_eq!(click(reset), None, "premier clic : on demande confirmation");
        let mut out = Vec::new();
        labels(&Config::default().resolve(), &mut out);
        assert_eq!(out[reset], "Confirmer : tout réinitialiser ?");
        assert_eq!(click(reset), Some(Action::ResetAll));
        assert_eq!(click(reset), None, "la confirmation ne sert qu'une fois");
        assert_eq!(click(0), Some(Action::Toggle(ModuleId::Status)), "un autre clic annule");
        assert_eq!(click(reset), None);
    }
}
