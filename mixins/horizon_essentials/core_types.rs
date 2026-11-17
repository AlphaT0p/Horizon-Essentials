// Horizon Essentials: types shared by the Core, the UI and the modules.
// This file becomes the game's crate::game::essentials_types module, and a module
// of the test harness (harness/lib.rs). No `//!` and no inner attributes: the
// horizon-mixins bundle format rejects them.

use crate::game::essentials_config::ResolvedStyle;
use crate::game::essentials_registry::ModuleId;

/// The mod id: its entry in Options > Mods and its data folder
/// (horizon-mods/.horizon-mod-data/horizon_essentials/).
pub const MOD_ID: &str = "horizon_essentials";
pub const NAME: &str = "Horizon Essentials";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "Profiles, widgets and PvP settings that complement Horizon.";

/// The title of the entry in Options > Mods.
pub fn title() -> String {
    format!("{NAME} {VERSION}")
}

// ── Placement ───────────────────────────────────────────────────────────────

/// The point of the screen a widget is attached to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Anchor {
    pub const ALL: [Anchor; 9] = [
        Anchor::TopLeft,
        Anchor::TopCenter,
        Anchor::TopRight,
        Anchor::CenterLeft,
        Anchor::Center,
        Anchor::CenterRight,
        Anchor::BottomLeft,
        Anchor::BottomCenter,
        Anchor::BottomRight,
    ];

    /// The name in configuration files.
    pub fn key(self) -> &'static str {
        match self {
            Anchor::TopLeft => "top_left",
            Anchor::TopCenter => "top_center",
            Anchor::TopRight => "top_right",
            Anchor::CenterLeft => "center_left",
            Anchor::Center => "center",
            Anchor::CenterRight => "center_right",
            Anchor::BottomLeft => "bottom_left",
            Anchor::BottomCenter => "bottom_center",
            Anchor::BottomRight => "bottom_right",
        }
    }

    pub fn from_key(key: &str) -> Option<Anchor> {
        Anchor::ALL.into_iter().find(|a| a.key() == key)
    }

    /// The corners, in the order of `horizon_client::extras::Corner`
    /// (TopLeft, TopRight, BottomLeft, BottomRight): with automatic placement,
    /// the widget stacks in that corner with the boxes of Horizon and other mods.
    pub fn corner_index(self) -> Option<usize> {
        match self {
            Anchor::TopLeft => Some(0),
            Anchor::TopRight => Some(1),
            Anchor::BottomLeft => Some(2),
            Anchor::BottomRight => Some(3),
            _ => None,
        }
    }

    /// The share of the free space to the left of and above the widget (0, ½ or 1).
    pub fn factors(self) -> (f32, f32) {
        match self {
            Anchor::TopLeft => (0.0, 0.0),
            Anchor::TopCenter => (0.5, 0.0),
            Anchor::TopRight => (1.0, 0.0),
            Anchor::CenterLeft => (0.0, 0.5),
            Anchor::Center => (0.5, 0.5),
            Anchor::CenterRight => (1.0, 0.5),
            Anchor::BottomLeft => (0.0, 1.0),
            Anchor::BottomCenter => (0.5, 1.0),
            Anchor::BottomRight => (1.0, 1.0),
        }
    }
}

// ── Actions: the only way for the UI to change the configuration ─────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Toggle(ModuleId),
    Set(ModuleId, bool),
    /// Restores a module's default settings (enabled state, placement, style).
    ResetModule(ModuleId),
    /// Restores the whole default configuration.
    ResetAll,
    /// Reads the configuration from disk again.
    Reload,
}

// ── Rendering: a list of shapes reused from one frame to the next ──────────

/// A shape, in GUI pixels, relative to the widget's top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Fill { rect: [f32; 4], argb: u32 },
    Text { slot: usize, at: [f32; 2], rgb: u32, shadow: bool },
}

/// What a widget draws. `clear` keeps the memory: after the first frames,
/// rebuilding the list no longer allocates (texts are copied into `String`s
/// that are already allocated).
#[derive(Debug, Default)]
pub struct Shapes {
    ops: Vec<Op>,
    texts: Vec<String>,
    used: usize,
    size: [f32; 2],
}

impl Shapes {
    pub fn clear(&mut self) {
        self.ops.clear();
        self.used = 0;
        self.size = [0.0, 0.0];
    }

    pub fn fill(&mut self, rect: [f32; 4], argb: u32) {
        self.ops.push(Op::Fill { rect, argb });
    }

    pub fn text(&mut self, text: &str, at: [f32; 2], rgb: u32, shadow: bool) {
        let slot = self.used;
        if slot < self.texts.len() {
            let s = &mut self.texts[slot];
            s.clear();
            s.push_str(text);
        } else {
            self.texts.push(text.to_owned());
        }
        self.used += 1;
        self.ops.push(Op::Text { slot, at, rgb, shadow });
    }

    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    pub fn text_of(&self, slot: usize) -> &str {
        self.texts.get(slot).map_or("", String::as_str)
    }

    /// The widget size, used to place it.
    pub fn size(&self) -> [f32; 2] {
        self.size
    }

    pub fn set_size(&mut self, size: [f32; 2]) {
        self.size = size;
    }
}

// ── The common module interface ─────────────────────────────────────────────

/// What the Core knows at the time of a tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickCtx {
    pub tick: u64,
    pub in_world: bool,
}

/// What a widget receives to draw itself.
pub struct WidgetCtx<'a> {
    /// The width of a text in GUI pixels (`native_text_width` in the game).
    pub measure: &'a dyn Fn(&str) -> f32,
    pub style: ResolvedStyle,
    pub profile: &'a str,
    pub active_modules: usize,
    pub total_modules: usize,
}

/// An Essentials module. Every method has an empty default body:
/// a module only writes the ones it needs.
pub trait Module {
    /// A frame has just been drawn.
    fn on_frame(&mut self, _elapsed_ms: f32) {}
    /// A game tick (20 per second).
    fn on_tick(&mut self, _ctx: &TickCtx) {}
    /// A key or a mouse button, by its vanilla name (`key.mouse.left`).
    fn on_key(&mut self, _key: &str, _down: bool) {}
    /// The module has just been enabled or disabled: a chance to reset its state.
    fn on_enabled(&mut self, _on: bool) {}
    /// Its widget, if it has one: the module rebuilds it only when what it shows
    /// has changed, and otherwise returns the list that is already built.
    fn widget(&mut self, _ctx: &WidgetCtx<'_>) -> Option<&Shapes> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_contains_the_version() {
        assert_eq!(title(), "Horizon Essentials 0.1.0");
    }

    #[test]
    fn the_id_follows_the_horizon_format() {
        // a-z, 0-9 and _ only (docs/mod-manifest).
        assert!(MOD_ID.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
    }

    #[test]
    fn every_anchor_is_read_back_by_its_name() {
        for a in Anchor::ALL {
            assert_eq!(Anchor::from_key(a.key()), Some(a));
            assert_eq!(serde_json::to_string(&a).unwrap(), format!("\"{}\"", a.key()));
        }
        assert_eq!(Anchor::from_key("nowhere"), None);
    }

    #[test]
    fn corners_follow_horizon_order() {
        assert_eq!(Anchor::TopLeft.corner_index(), Some(0));
        assert_eq!(Anchor::TopRight.corner_index(), Some(1));
        assert_eq!(Anchor::BottomLeft.corner_index(), Some(2));
        assert_eq!(Anchor::BottomRight.corner_index(), Some(3));
        assert_eq!(Anchor::Center.corner_index(), None);
    }

    #[test]
    fn shapes_reuses_its_texts() {
        let mut s = Shapes::default();
        s.text("one", [0.0, 0.0], 0xFFFFFF, true);
        s.fill([0.0, 0.0, 1.0, 1.0], 0x7800_0000);
        let capacity = s.texts[0].capacity();
        s.clear();
        s.text("two", [1.0, 2.0], 0, false);
        assert_eq!(s.texts.len(), 1, "the text reuses the existing slot");
        assert!(s.texts[0].capacity() >= capacity);
        assert_eq!(s.ops(), &[Op::Text { slot: 0, at: [1.0, 2.0], rgb: 0, shadow: false }]);
        assert_eq!(s.text_of(0), "two");
        assert_eq!(s.text_of(9), "");
    }
}
