// Horizon Essentials: the settings page in Options > Mods (crate::game::essentials_settings).
//
// Horizon draws the page: a list of buttons whose labels the mod provides,
// requested again after every click. This page changes nothing itself: it
// reads the Core snapshot (`essentials_events::view`) and turns a click into
// an `Action`, which the Core applies. Modules come from the registry: a module
// added to the registry gets its button without changing anything here.
// "Reset everything" asks for a second click to confirm.

use std::cell::Cell;
use std::time::{Duration, Instant};

use crate::game::essentials_events::Snapshot;
use crate::game::essentials_registry::{MODULES, ModuleId};
use crate::game::essentials_types::Action;

/// The page's only label while the configuration is loading.
pub const LOADING: &str = "Loading the configuration…";

/// A button of the page, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Module(ModuleId),
    Reload,
    ResetAll,
}

/// The delay to confirm "Reset everything" (the same as for the command).
pub const RESET_DELAY: Duration = Duration::from_secs(15);

thread_local! {
    // The time of the first click on "Reset everything", not yet confirmed.
    static RESET_ARMED: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// The pending confirmation, if it has not expired.
fn armed(now: Instant) -> bool {
    RESET_ARMED.get().is_some_and(|at| now.saturating_duration_since(at) <= RESET_DELAY)
}

/// The button number `index`.
pub fn button(index: usize) -> Option<Button> {
    let n = MODULES.len();
    match index {
        i if i < n => Some(Button::Module(MODULES[i].id)),
        i if i == n => Some(Button::Reload),
        i if i == n + 1 => Some(Button::ResetAll),
        _ => None,
    }
}

/// The page labels: only one, "Loading…", until the configuration has been read.
pub fn labels(snapshot: Option<Snapshot<'_>>, out: &mut Vec<String>) {
    labels_at(snapshot, out, Instant::now());
}

/// `labels` at time `now` (tests move time forward without waiting).
pub fn labels_at(snapshot: Option<Snapshot<'_>>, out: &mut Vec<String>, now: Instant) {
    let Some(snapshot) = snapshot else {
        out.push(LOADING.to_owned());
        return;
    };
    for d in MODULES {
        let state = if snapshot.resolved.on[d.id.index()] { "ON" } else { "OFF" };
        out.push(format!("{}: {state}", d.name));
    }
    out.push("Reload configuration".to_owned());
    out.push(if armed(now) {
        "Confirm (15 s): reset ALL settings to defaults?".to_owned()
    } else {
        "Reset everything".to_owned()
    });
}

/// The action of a click, if any. While loading, no click does anything.
pub fn click(index: usize, loaded: bool) -> Option<Action> {
    click_at(index, loaded, Instant::now())
}

/// `click` at time `now`.
pub fn click_at(index: usize, loaded: bool, now: Instant) -> Option<Action> {
    if !loaded {
        return None;
    }
    let button = button(index)?;
    let confirmed = armed(now);
    RESET_ARMED.set(None);
    match button {
        Button::Module(id) => Some(Action::Toggle(id)),
        Button::Reload => Some(Action::Reload),
        Button::ResetAll if confirmed => Some(Action::ResetAll),
        Button::ResetAll => {
            RESET_ARMED.set(Some(now));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::essentials_config::Config;

    fn labels_of(config: &Config) -> Vec<String> {
        let resolved = config.resolve();
        let mut out = Vec::new();
        labels(Some(Snapshot { resolved: &resolved, profile: "default" }), &mut out);
        out
    }

    #[test]
    fn one_button_per_registry_module_then_the_general_actions() {
        let out = labels_of(&Config::default());
        assert_eq!(out.len(), MODULES.len() + 2);
        for (i, d) in MODULES.iter().enumerate() {
            assert!(out[i].starts_with(d.name), "button {i} comes from the registry");
        }
        assert_eq!(out[0], "Essentials Status: ON");
        assert_eq!(click(0, true), Some(Action::Toggle(ModuleId::Status)));
        assert_eq!(click(MODULES.len(), true), Some(Action::Reload));
        assert_eq!(click(99, true), None);
    }

    #[test]
    fn the_page_waits_while_loading() {
        let mut out = Vec::new();
        labels(None, &mut out);
        assert_eq!(out, [LOADING]);
        assert_eq!(click(0, false), None, "no change before loading ends");
    }

    #[test]
    fn reset_everything_asks_for_confirmation() {
        let reset = MODULES.len() + 1;
        click(0, true); // starts from a state with no pending confirmation
        assert_eq!(click(reset, true), None, "first click: confirmation is requested");
        assert_eq!(labels_of(&Config::default())[reset], "Confirm (15 s): reset ALL settings to defaults?");
        assert_eq!(click(reset, true), Some(Action::ResetAll));
        assert_eq!(click(reset, true), None, "the confirmation works only once");
        assert_eq!(click(0, true), Some(Action::Toggle(ModuleId::Status)), "another click cancels");
        assert_eq!(click(reset, true), None);
    }

    #[test]
    fn the_confirmation_expires_after_15_seconds() {
        let reset = MODULES.len() + 1;
        let t0 = Instant::now();
        click_at(0, true, t0);
        assert_eq!(click_at(reset, true, t0), None);
        let later = t0 + Duration::from_secs(16);
        let resolved = Config::default().resolve();
        let mut out = Vec::new();
        labels_at(Some(Snapshot { resolved: &resolved, profile: "default" }), &mut out, later);
        assert_eq!(out[reset], "Reset everything", "the label goes back to normal after the delay");
        assert_eq!(click_at(reset, true, later), None, "a late click asks for confirmation again");
        assert_eq!(click_at(reset, true, later + Duration::from_secs(1)), Some(Action::ResetAll));
    }
}
