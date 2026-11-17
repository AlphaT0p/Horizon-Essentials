// Horizon Essentials: event dispatch (crate::game::essentials_events).
//
// The h_*.rs helpers call these functions and nothing else of the Core. They do
// not know the game: what they need from it is passed as parameters
// (the data folder, a text measurement, a way to place and paint).

use std::path::PathBuf;

use crate::game::essentials_config::Resolved;
use crate::game::essentials_draw as draw;
use crate::game::essentials_registry::{COUNT, ModuleId};
use crate::game::essentials_state::{self as state};
use crate::game::essentials_types::{Action, Shapes, TickCtx, WidgetCtx};

/// A frame (on the window). On the first one, asks the I/O thread to read the
/// configuration; after that, picks up its replies without waiting. No disk access here.
pub fn frame(dir: impl FnOnce() -> PathBuf, elapsed_ms: f32) {
    state::with(dir, |s| {
        s.advance(elapsed_ms);
        if !s.loaded {
            return;
        }
        for id in ModuleId::all() {
            if s.resolved.on[id.index()] {
                s.modules.get_mut(id).on_frame(elapsed_ms);
            }
        }
    });
}

/// A tick. Returns the next message to write in the chat, when the player is
/// in a world (at most one per tick).
pub fn tick(dir: impl FnOnce() -> PathBuf, in_world: bool) -> Option<String> {
    state::with(dir, |s| {
        s.tick += 1;
        if s.loaded {
            let ctx = TickCtx {
                tick: s.tick,
                in_world,
            };
            for id in ModuleId::all() {
                if s.resolved.on[id.index()] {
                    s.modules.get_mut(id).on_tick(&ctx);
                }
            }
        }
        if in_world { s.next_message() } else { None }
    })
    .flatten()
}

/// A key or a mouse button, by its vanilla name.
pub fn key(dir: impl FnOnce() -> PathBuf, key: &str, down: bool) {
    state::with(dir, |s| {
        if !s.loaded {
            return;
        }
        for id in ModuleId::all() {
            if s.resolved.on[id.index()] {
                s.modules.get_mut(id).on_key(key, down);
            }
        }
    });
}

/// Game shutdown: the last change is written before exiting (the I/O thread has
/// at most `SHUTDOWN_TIMEOUT` to finish). Starts nothing if the Core never ran.
pub fn shutdown() {
    state::with_existing(|s| s.shutdown());
}

/// Applies an action coming from the UI or from a command.
pub fn apply(dir: impl FnOnce() -> PathBuf, action: Action) -> bool {
    state::with(dir, |s| s.apply(action)).unwrap_or(false)
}

/// What the UI can read from the Core, read-only: the resolved configuration
/// and the active profile. The settings page and the command both read this
/// snapshot, so they always show the same state.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot<'a> {
    pub resolved: &'a Resolved,
    pub profile: &'a str,
}

/// Gives `f` the Core snapshot, or None while the configuration is loading
/// (the UI then shows "Loading…" and offers no change). The result is None if
/// the state is already borrowed.
pub fn view<R>(
    dir: impl FnOnce() -> PathBuf,
    f: impl FnOnce(Option<Snapshot<'_>>) -> R,
) -> Option<R> {
    state::with(dir, |s| {
        let snapshot = s.loaded.then_some(Snapshot {
            resolved: &s.resolved,
            profile: &s.profile,
        });
        f(snapshot)
    })
}

/// Draws the active widgets.
/// - `screen`: the screen size in GUI pixels;
/// - `measure`: the width of a text;
/// - `place_corner(corner, w, h)`: the position in a corner (0 top left, 1 top right,
///   2 bottom left, 3 bottom right), or None if the corner is taken (F3 screen);
/// - `paint(shapes, position)`: paints the shapes.
///
/// Never reads the disk: until the configuration is loaded, nothing is drawn
/// (no widget that appears and then disappears).
pub fn hud(
    screen: [f32; 2],
    measure: &dyn Fn(&str) -> f32,
    place_corner: &mut dyn FnMut(usize, f32, f32) -> Option<[f32; 2]>,
    paint: &mut dyn FnMut(&Shapes, [f32; 2]),
) {
    state::with_loaded(|s| {
        let state::State {
            modules,
            resolved,
            profile,
            ..
        } = s;
        let active = resolved.active_count();
        for id in ModuleId::all() {
            let i = id.index();
            if !resolved.on[i] || !id.descriptor().has_widget {
                continue;
            }
            let ctx = WidgetCtx {
                measure,
                style: resolved.style[i],
                profile,
                active_modules: active,
                total_modules: COUNT,
            };
            let Some(shapes) = modules.get_mut(id).widget(&ctx) else {
                continue;
            };
            let size = shapes.size();
            let layout = resolved.layout[i];
            let at = match (layout.auto, layout.anchor.corner_index()) {
                (true, Some(corner)) => match place_corner(corner, size[0], size[1]) {
                    Some(at) => at,
                    None => continue,
                },
                _ => draw::position(&layout, size, screen),
            };
            paint(shapes, at);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::essentials_store::tests::TempDir;
    use std::time::Duration;

    const WAIT: Duration = Duration::from_secs(5);

    /// First frame, then wait for the asynchronous loading to finish.
    fn started(dir: &std::path::Path) {
        frame(|| dir.to_path_buf(), 16.0);
        assert_eq!(state::with_existing(|s| s.wait_idle(WAIT)), Some(true));
    }

    fn draw(painted: &mut u64) {
        hud(
            [480.0, 270.0],
            &|s: &str| s.len() as f32 * 6.0,
            &mut |_, _, _| Some([300.0, 2.0]),
            &mut |shapes, at| *painted += shapes.ops().len() as u64 + at[0] as u64,
        );
    }

    #[test]
    fn the_hud_follows_enabling_and_corners() {
        let d = TempDir::new("events-hud");
        let dir = d.0.clone();
        started(&dir);
        let measure = |s: &str| s.len() as f32 * 6.0;
        let mut corners = Vec::new();
        let mut painted = Vec::new();
        let draw_once = |corners: &mut Vec<usize>, painted: &mut Vec<String>| {
            hud(
                [480.0, 270.0],
                &measure,
                &mut |c, _, _| {
                    corners.push(c);
                    Some([300.0, 2.0])
                },
                &mut |shapes, at| painted.push(format!("{} @ {at:?}", shapes.text_of(0))),
            );
        };
        draw_once(&mut corners, &mut painted);
        assert_eq!(corners, [1], "the status box stacks at the top right");
        assert_eq!(painted, ["Essentials 0.1.0 - default - 1/1 @ [300.0, 2.0]"]);

        assert!(apply(|| dir.clone(), Action::Toggle(ModuleId::Status)));
        draw_once(&mut corners, &mut painted);
        assert_eq!(painted.len(), 1, "disabled: nothing is drawn any more");
    }

    #[test]
    fn a_taken_corner_hides_the_widget() {
        let d = TempDir::new("events-corner");
        let dir = d.0.clone();
        started(&dir);
        let mut painted = 0;
        hud(
            [480.0, 270.0],
            &|_| 10.0,
            &mut |_, _, _| None,
            &mut |_, _| painted += 1,
        );
        assert_eq!(painted, 0);
    }

    #[test]
    fn nothing_is_drawn_before_loading_ends() {
        let mut painted = 0;
        draw(&mut painted);
        assert_eq!(painted, 0, "before the first frame");
        let d = TempDir::new("events-waiting");
        let dir = d.0.clone();
        frame(|| dir.clone(), 16.0);
        let loaded = state::with_existing(|s| s.loaded).unwrap();
        draw(&mut painted);
        assert_eq!(
            painted > 0,
            loaded,
            "the HUD shows nothing until the configuration has been read"
        );
        state::with_existing(|s| s.wait_idle(WAIT));
        draw(&mut painted);
        assert!(painted > 0, "then the status box appears");
    }

    #[test]
    fn messages_wait_for_the_world() {
        let d = TempDir::new("events-messages");
        std::fs::write(d.0.join("active"), "Forbidden").unwrap();
        let dir = d.0.clone();
        started(&dir);
        assert_eq!(tick(|| dir.clone(), false), None, "not in a world yet");
        let m = tick(|| dir.clone(), true).expect("a warning");
        assert!(m.contains("invalid active profile"));
        assert_eq!(tick(|| dir.clone(), true), None);
    }

    #[test]
    fn view_gives_nothing_while_loading() {
        let d = TempDir::new("events-view");
        let dir = d.0.clone();
        // First access: the read has just been requested from the I/O thread.
        let first = view(|| dir.clone(), |snap| snap.is_some()).unwrap();
        let loaded = state::with_existing(|s| s.loaded).unwrap();
        assert_eq!(
            first, loaded,
            "no snapshot until the configuration has been read"
        );
        state::with_existing(|s| s.wait_idle(WAIT));
        let (on, profile) = view(
            || dir.clone(),
            |snap| {
                let snap = snap.expect("loaded");
                (
                    snap.resolved.on[ModuleId::Status.index()],
                    snap.profile.to_owned(),
                )
            },
        )
        .unwrap();
        assert!(on);
        assert_eq!(profile, "default");
    }

    // ── Performance regression tests ────────────────────────────────────────

    #[test]
    fn zero_allocations_per_frame() {
        let d = TempDir::new("events-alloc");
        let dir = d.0.clone();
        started(&dir);
        let mut painted = 0;
        for _ in 0..10 {
            frame(|| dir.clone(), 1.0);
            draw(&mut painted);
        }
        let before = crate::allocations();
        for _ in 0..10_000 {
            frame(|| dir.clone(), 1.0);
            draw(&mut painted);
            let _ = tick(|| dir.clone(), true);
            key(|| dir.clone(), "key.mouse.left", true);
        }
        assert_eq!(
            crate::allocations() - before,
            0,
            "a frame, a tick or a key press allocated"
        );
        assert!(painted > 0);
    }

    #[test]
    fn a_disabled_module_is_never_called() {
        use crate::game::essentials_registry::calls;
        let d = TempDir::new("events-disabled");
        let dir = d.0.clone();
        started(&dir);
        assert!(apply(|| dir.clone(), Action::Set(ModuleId::Status, false)));
        let before = calls(ModuleId::Status);
        let mut painted = 0;
        for _ in 0..1000 {
            frame(|| dir.clone(), 1.0);
            draw(&mut painted);
            let _ = tick(|| dir.clone(), true);
            key(|| dir.clone(), "key.keyboard.w", true);
        }
        assert_eq!(
            calls(ModuleId::Status),
            before,
            "the disabled module was called"
        );
        assert_eq!(painted, 0);
        assert!(apply(|| dir.clone(), Action::Set(ModuleId::Status, true)));
        draw(&mut painted);
        assert!(
            calls(ModuleId::Status) > before,
            "re-enabled, it is called again"
        );
    }

    #[test]
    fn no_disk_operation_on_the_game_thread() {
        use crate::game::essentials_store::DISK_OPS;
        let d = TempDir::new("events-disk");
        let dir = d.0.clone();
        let before = DISK_OPS.with(|c| c.get());
        started(&dir);
        let mut painted = 0;
        // A change, then 2 s of frames: the save happens during the loop.
        assert!(apply(|| dir.clone(), Action::Toggle(ModuleId::Status)));
        for _ in 0..2000 {
            frame(|| dir.clone(), 1.0);
            draw(&mut painted);
        }
        assert_eq!(state::with_existing(|s| s.wait_idle(WAIT)), Some(true));
        assert_eq!(
            DISK_OPS.with(|c| c.get()),
            before,
            "the game thread touched the disk"
        );
        let file = crate::game::essentials_store::profile_path(&dir, "default");
        assert!(file.exists(), "the save was done, by the I/O thread");
    }
}
