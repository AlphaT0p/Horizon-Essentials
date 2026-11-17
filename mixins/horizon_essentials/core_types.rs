// Horizon Essentials : types partagés par le Core, l'interface et les modules.
// Ce fichier devient le module crate::game::essentials_types du jeu, et un module
// du banc de test (harness/lib.rs). Pas de `//!` ni d'attribut interne : le format
// des bundles horizon-mixins les refuse.

/// L'id du mod : sa ligne dans Options > Mods et son dossier de données
/// (horizon-mods/.horizon-mod-data/horizon_essentials/).
pub const MOD_ID: &str = "horizon_essentials";
pub const NAME: &str = "Horizon Essentials";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "Profils, widgets et réglages PvP en complément de Horizon.";

/// Le titre de la ligne dans Options > Mods.
pub fn title() -> String {
    format!("{NAME} {VERSION}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn le_titre_contient_la_version() {
        assert_eq!(super::title(), "Horizon Essentials 0.1.0");
    }

    #[test]
    fn l_id_respecte_le_format_horizon() {
        // a-z, 0-9 et _ uniquement (docs/mod-manifest).
        assert!(super::MOD_ID.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
    }
}
