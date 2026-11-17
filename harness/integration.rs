//! Integration tests: the settings page, the /essentials command and the Core
//! together, as the `h_settings`, `h_click` and `h_command` helpers connect them
//! in the game (those helpers call the game and are not compiled here: the
//! functions below reproduce exactly the same sequence).

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::game::essentials_command as command;
use crate::game::essentials_events as ev;
use crate::game::essentials_registry::{MODULES, ModuleId};
use crate::game::essentials_settings as page;
use crate::game::essentials_state as state;
use crate::game::essentials_store as store;

const WAIT: Duration = Duration::from_secs(5);

struct Dir(PathBuf);
impl Dir {
    fn new(name: &str) -> Dir {
        let d = std::env::temp_dir().join(format!("horizon-essentials-it-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Dir(d)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Starts the Core on `dir` and waits for loading to finish.
fn start(dir: &Path) {
    ev::frame(|| dir.to_path_buf(), 16.0);
    assert_eq!(state::with_existing(|s| s.wait_idle(WAIT)), Some(true));
}

/// h_settings: the page labels.
fn page_labels(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    ev::view(|| dir.to_path_buf(), |snapshot| page::labels(snapshot, &mut out));
    out
}

/// h_click: a click on the page.
fn page_click(dir: &Path, button: usize) {
    let loaded = ev::view(|| dir.to_path_buf(), |s| s.is_some()).unwrap_or(false);
    if let Some(action) = page::click(button, loaded) {
        ev::apply(|| dir.to_path_buf(), action);
    }
}

/// h_command: /essentials <args>, and the lines written in the chat.
fn run(dir: &Path, args: &str) -> Vec<String> {
    let reply = ev::view(|| dir.to_path_buf(), |snapshot| command::run(args, snapshot)).unwrap();
    let mut lines = reply.lines;
    if let Some(action) = reply.action {
        let changed = ev::apply(|| dir.to_path_buf(), action);
        let line = ev::view(|| dir.to_path_buf(), |s| {
            s.map(|s| command::outcome(action, changed, s.resolved)).unwrap_or_default()
        })
        .unwrap();
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

fn status_on(dir: &Path) -> bool {
    ev::view(|| dir.to_path_buf(), |s| s.unwrap().resolved.on[ModuleId::Status.index()]).unwrap()
}

/// Lets the save delay pass and waits for the I/O thread.
fn settle() {
    for _ in 0..1100 {
        ev::frame(PathBuf::new, 1.0);
    }
    assert_eq!(state::with_existing(|s| s.wait_idle(WAIT)), Some(true));
}

#[test]
fn the_page_and_the_command_show_the_same_state() {
    let d = Dir::new("same-state");
    start(&d.0);
    assert_eq!(page_labels(&d.0)[0], "Essentials Status: ON");
    assert_eq!(run(&d.0, "list"), ["status: ON (Essentials Status)"]);

    // A command changes the state: the page shows it.
    assert_eq!(run(&d.0, "off status"), ["Essentials Status: disabled"]);
    assert_eq!(page_labels(&d.0)[0], "Essentials Status: OFF");

    // A click on the page changes the state: the command shows it.
    page_click(&d.0, 0);
    assert_eq!(run(&d.0, "list"), ["status: ON (Essentials Status)"]);
    assert!(status_on(&d.0));
}

#[test]
fn one_button_per_registry_module() {
    let d = Dir::new("registry");
    start(&d.0);
    let labels = page_labels(&d.0);
    assert_eq!(labels.len(), MODULES.len() + 2, "one button per module, then reload and reset");
    assert_eq!(run(&d.0, "list").len(), MODULES.len());
}

#[test]
fn several_successive_actions_are_all_saved() {
    let d = Dir::new("burst");
    start(&d.0);
    // Clicks and commands in a row without waiting, while the I/O thread writes.
    for i in 0..25 {
        page_click(&d.0, 0);
        run(&d.0, if i % 2 == 0 { "toggle status" } else { "on status" });
        state::with_existing(|s| s.flush());
    }
    run(&d.0, "off status");
    let expected = state::with_existing(|s| s.config.clone()).unwrap();
    settle();
    let on_disk = store::load(&d.0);
    assert_eq!(on_disk.config, expected, "the disk has the very last change");
    assert!(!on_disk.config.module(ModuleId::Status).on);
}

#[test]
fn full_reset_goes_through_confirmation() {
    let d = Dir::new("reset");
    start(&d.0);
    run(&d.0, "off status");
    let ask = run(&d.0, "reset");
    assert!(ask[0].contains("ALL Horizon Essentials settings"));
    assert!(!status_on(&d.0), "nothing is reset before confirmation");
    assert_eq!(run(&d.0, "reset confirm"), ["All settings reset to defaults"]);
    assert!(status_on(&d.0));
    settle();
    assert!(store::load(&d.0).config.module(ModuleId::Status).on, "and it is saved");
}

#[test]
fn reset_from_the_page_takes_two_clicks() {
    let d = Dir::new("reset-page");
    start(&d.0);
    run(&d.0, "off status");
    let reset = MODULES.len() + 1;
    page_click(&d.0, reset);
    assert!(!status_on(&d.0), "first click: nothing changes");
    assert!(page_labels(&d.0)[reset].starts_with("Confirm (15 s)"));
    page_click(&d.0, reset);
    assert!(status_on(&d.0), "second click: everything is reset to defaults");
}

#[test]
fn reload_reads_the_configuration_from_disk() {
    let d = Dir::new("reload");
    start(&d.0);
    let mut other = crate::game::essentials_config::Config::default();
    other.apply(crate::game::essentials_types::Action::Set(ModuleId::Status, false));
    store::save(&d.0, store::DEFAULT_PROFILE, &other).unwrap();
    run(&d.0, "reload");
    assert_eq!(state::with_existing(|s| s.wait_idle(WAIT)), Some(true));
    assert!(!status_on(&d.0));
    assert_eq!(page_labels(&d.0)[0], "Essentials Status: OFF");
}

#[test]
fn an_unknown_module_changes_nothing() {
    let d = Dir::new("unknown");
    start(&d.0);
    let before = state::with_existing(|s| s.config.clone()).unwrap();
    assert_eq!(run(&d.0, "off radar"), ["Unknown module: radar (modules: status)"]);
    assert_eq!(run(&d.0, "reset radar"), ["Unknown module: radar (modules: status)"]);
    assert_eq!(state::with_existing(|s| s.config.clone()).unwrap(), before);
}
