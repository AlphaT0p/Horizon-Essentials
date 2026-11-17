//! Banc de test de Horizon Essentials.
//!
//! Le jeu ajoute chaque fichier du bundle comme module `crate::game::essentials_*`.
//! Ce crate reproduit les mêmes chemins pour que `cargo test` compile et teste les
//! modules sans Horizon. Les helpers (`h_*.rs`) ne sont pas compilés ici : ils
//! appellent le jeu, et sont vérifiés par `native.py check`.
//!
//! Chaque module ajouté à `horizon-mixin.toml` doit l'être ici aussi, avec le même nom.

pub mod game {
    #[path = "../../mixins/horizon_essentials/core_types.rs"]
    pub mod essentials_types;
    #[path = "../../mixins/horizon_essentials/core_registry.rs"]
    pub mod essentials_registry;
    #[path = "../../mixins/horizon_essentials/core_config.rs"]
    pub mod essentials_config;
    #[path = "../../mixins/horizon_essentials/core_store.rs"]
    pub mod essentials_store;
    #[path = "../../mixins/horizon_essentials/core_state.rs"]
    pub mod essentials_state;
}
