//! Horizon Essentials test harness.
//!
//! The game adds each bundle file as a `crate::game::essentials_*` module.
//! This crate reproduces the same paths so that `cargo test` compiles and tests
//! the modules without Horizon. The helpers (`h_*.rs`) are not compiled here: they
//! call the game, and are checked by `native.py check`.
//!
//! Every module added to `horizon-mixin.toml` must be added here too, with the same name.

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

/// The number of allocations made by the current thread since it started.
#[cfg(test)]
pub fn allocations() -> u64 {
    counting::ALLOCATIONS.with(|c| c.get())
}

// The performance tests count the current thread's allocations: an allocator
// that forwards to the system one and increments a per-thread counter
// (other threads, including the I/O thread, are not counted).
#[cfg(test)]
mod counting {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        pub static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    }

    pub struct Counting;

    // SAFETY: forwards every call to `System` unchanged; the counter is a
    // thread-local `Cell`, initialized without allocating.
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
