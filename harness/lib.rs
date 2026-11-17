//! Banc de test de Horizon Essentials.
//!
//! Le jeu ajoute chaque fichier du bundle comme module `crate::game::essentials_*`.
//! Ce crate reproduit les mêmes chemins pour que `cargo test` compile et teste les
//! modules sans Horizon. Les helpers (`h_*.rs`) ne sont pas compilés ici : ils
//! appellent le jeu, et sont vérifiés par `native.py check`.

pub mod game {
    #[path = "../../mixins/horizon_essentials/core_types.rs"]
    pub mod essentials_types;
}
