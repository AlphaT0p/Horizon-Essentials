//! Protocole de mesure de Horizon Essentials (voir docs/performance.md).
//!
//! Compile les modules du bundle **hors mode test**, comme dans le jeu (sans les
//! compteurs réservés aux tests), avec un allocateur qui compte les allocations
//! du thread courant, puis exécute le protocole P1 à P4.
//!
//!     cargo run --release --manifest-path bench/Cargo.toml

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

struct Counting;

// SAFETY : délègue chaque appel à `System` tel quel ; le compteur est un `Cell`
// local au thread, initialisé sans allocation.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Les allocations faites par le thread courant depuis son début.
pub fn allocs() -> u64 {
    ALLOCS.with(|c| c.get())
}

// Les modules du bundle, aux mêmes chemins que dans le jeu. Ce qui ne sert pas au
// protocole n'est pas une erreur ici.
#[allow(dead_code)]
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

mod protocole;

fn main() {
    protocole::run();
}
