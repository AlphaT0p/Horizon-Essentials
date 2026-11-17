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
    #[path = "../../mixins/horizon_essentials/core_events.rs"]
    pub mod essentials_events;
    #[path = "../../mixins/horizon_essentials/ui_draw.rs"]
    pub mod essentials_draw;
    #[path = "../../mixins/horizon_essentials/mod_status.rs"]
    pub mod essentials_status;
}

/// Le nombre d'allocations faites par le thread courant depuis son début.
#[cfg(test)]
pub fn allocations() -> u64 {
    counting::ALLOCATIONS.with(|c| c.get())
}

// Les tests de performance comptent les allocations du thread courant : un
// allocateur qui délègue à celui du système et incrémente un compteur par thread
// (les autres threads, dont celui d'E/S, ne sont pas comptés).
#[cfg(test)]
mod counting {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        pub static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    }

    pub struct Counting;

    // SAFETY : délègue chaque appel à `System` tel quel ; le compteur est un
    // `Cell` local au thread, initialisé sans allocation.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
            unsafe { System.realloc(ptr, layout, size) }
        }
    }

    #[global_allocator]
    static ALLOCATOR: Counting = Counting;
}
