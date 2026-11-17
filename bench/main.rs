//! Horizon Essentials measurement protocol (see docs/performance.md).
//!
//! Compiles the bundle modules **outside test mode**, as in the game (without the
//! test-only counters), with an allocator that counts the current thread's
//! allocations, then runs protocol P1 to P4.
//!
//!     cargo run --release --manifest-path bench/Cargo.toml

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

struct Counting;

// SAFETY: forwards every call to `System` unchanged; the counter is a thread-local
// `Cell`, initialized without allocating.
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

/// The allocations made by the current thread since it started.
pub fn allocs() -> u64 {
    ALLOCS.with(|c| c.get())
}

// The bundle modules, at the same paths as in the game. What the protocol does
// not use is not an error here.
#[allow(dead_code)]
pub mod game {
    #[path = "../../mixins/horizon_essentials/core_config.rs"]
    pub mod essentials_config;
    #[path = "../../mixins/horizon_essentials/ui_draw.rs"]
    pub mod essentials_draw;
    #[path = "../../mixins/horizon_essentials/core_events.rs"]
    pub mod essentials_events;
    #[path = "../../mixins/horizon_essentials/core_registry.rs"]
    pub mod essentials_registry;
    #[path = "../../mixins/horizon_essentials/core_state.rs"]
    pub mod essentials_state;
    #[path = "../../mixins/horizon_essentials/mod_status.rs"]
    pub mod essentials_status;
    #[path = "../../mixins/horizon_essentials/core_store.rs"]
    pub mod essentials_store;
    #[path = "../../mixins/horizon_essentials/core_types.rs"]
    pub mod essentials_types;
}

mod protocole;

fn main() {
    protocole::run();
}
