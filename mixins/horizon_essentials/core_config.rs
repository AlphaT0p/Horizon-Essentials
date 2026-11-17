// Horizon Essentials: the configuration (crate::game::essentials_config).
//
// Resilience: reading never crashes. An unreadable file gives the default
// configuration; a missing field, a field of the wrong type or out of range
// gets its default value without touching the others; an unknown module (added
// by a newer version) is kept as is; a missing module (added by this version)
// gets its default settings. Every correction is reported by a warning, which
// the Core shows once in the chat.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::game::essentials_registry::{COUNT, Descriptor, MODULES, ModuleId};
use crate::game::essentials_types::{Action, Anchor};

/// The format version. A newer version is still read, as well as possible.
pub const CONFIG_VERSION: u32 = 1;
pub const SCALE_MIN: u16 = 50;
pub const SCALE_MAX: u16 = 200;
/// The default background opacity: 47 % gives alpha 0x78, the one of Horizon's boxes.
pub const DEFAULT_BACKGROUND: u8 = 47;
pub const DEFAULT_TEXT: &str = "FFFFFF";

/// Where and at what size the widget is shown (the HUD Editor writes here).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Layout {
    pub anchor: Anchor,
    /// True: stacked automatically in the `anchor` corner with the other boxes.
    /// False: placed freely, at `x` and `y`.
    pub auto: bool,
    /// The share of the free space on the left (0 to 1): the widget stays on
    /// screen when the window is resized.
    pub x: f32,
    /// The share of the free space above (0 to 1).
    pub y: f32,
    /// In percent, from 50 to 200.
    pub scale: u16,
}

/// The widget colors.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Style {
    /// Background opacity, from 0 to 100 %.
    pub background: u8,
    /// Text color, `RRGGBB`.
    pub text: String,
    pub shadow: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ModuleConfig {
    pub on: bool,
    pub layout: Layout,
    pub style: Style,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Config {
    pub version: u32,
    /// By module key. A `BTreeMap` keeps a stable order in the file.
    pub modules: BTreeMap<String, ModuleConfig>,
}

/// The style, ready for rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedStyle {
    pub background: u32,
    pub text: u32,
    pub shadow: bool,
}

/// The configuration, ready for rendering: arrays indexed by `ModuleId`,
/// recomputed only when the configuration changes, so that a frame never
/// looks up a key in a map.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub on: [bool; COUNT],
    pub layout: [Layout; COUNT],
    pub style: [ResolvedStyle; COUNT],
    /// The number of active modules, counted once per change.
    active: usize,
}

impl Resolved {
    pub fn active_count(&self) -> usize {
        self.active
    }
}

impl Default for Style {
    fn default() -> Self {
        Style {
            background: DEFAULT_BACKGROUND,
            text: DEFAULT_TEXT.to_owned(),
            shadow: true,
        }
    }
}

impl Style {
    pub fn resolve(&self) -> ResolvedStyle {
        let alpha = (u32::from(self.background.min(100)) * 255 + 50) / 100;
        ResolvedStyle {
            background: alpha << 24,
            text: parse_rgb(&self.text).unwrap_or(0xFF_FFFF),
            shadow: self.shadow,
        }
    }
}

impl Layout {
    pub fn default_for(d: &Descriptor) -> Layout {
        Layout {
            anchor: d.default_anchor,
            auto: true,
            x: 0.0,
            y: 0.0,
            scale: 100,
        }
    }

    /// Within the limits the file accepts: `x` and `y` from 0 to 1 (0 when not a number),
    /// `scale` from `SCALE_MIN` to `SCALE_MAX`. What a UI sends is never saved out of range.
    pub fn sanitized(self) -> Layout {
        let share = |v: f32| {
            if v.is_finite() {
                v.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        Layout {
            x: share(self.x),
            y: share(self.y),
            scale: self.scale.clamp(SCALE_MIN, SCALE_MAX),
            ..self
        }
    }
}

impl ModuleConfig {
    pub fn default_for(d: &Descriptor) -> ModuleConfig {
        ModuleConfig {
            on: d.default_on,
            layout: Layout::default_for(d),
            style: Style::default(),
        }
    }

    /// For a module this registry does not know.
    fn default_unknown() -> ModuleConfig {
        ModuleConfig {
            on: false,
            layout: Layout {
                anchor: Anchor::TopLeft,
                auto: true,
                x: 0.0,
                y: 0.0,
                scale: 100,
            },
            style: Style::default(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let modules = MODULES
            .iter()
            .map(|d| (d.key.to_owned(), ModuleConfig::default_for(d)))
            .collect();
        Config {
            version: CONFIG_VERSION,
            modules,
        }
    }
}

impl Config {
    /// The configuration of a registry module (always present after `parse`
    /// or `default`, otherwise its default settings).
    pub fn module(&self, id: ModuleId) -> ModuleConfig {
        self.modules
            .get(id.key())
            .cloned()
            .unwrap_or_else(|| ModuleConfig::default_for(id.descriptor()))
    }

    fn module_mut(&mut self, id: ModuleId) -> &mut ModuleConfig {
        self.modules
            .entry(id.key().to_owned())
            .or_insert_with(|| ModuleConfig::default_for(id.descriptor()))
    }

    pub fn resolve(&self) -> Resolved {
        let first = self.module(ModuleId::all().next().expect("at least one module"));
        let mut r = Resolved {
            on: [false; COUNT],
            layout: [first.layout; COUNT],
            style: [first.style.resolve(); COUNT],
            active: 0,
        };
        for id in ModuleId::all() {
            let m = self.module(id);
            r.on[id.index()] = m.on;
            r.layout[id.index()] = m.layout;
            r.style[id.index()] = m.style.resolve();
        }
        r.active = r.on.iter().filter(|on| **on).count();
        r
    }

    /// Applies an action. True if the configuration changed.
    /// `Reload` and the profile actions change nothing here: the state reads, writes or
    /// deletes the profile files.
    pub fn apply(&mut self, action: Action) -> bool {
        let before = self.clone();
        match action {
            Action::Toggle(id) => {
                let m = self.module_mut(id);
                m.on = !m.on;
            }
            Action::Set(id, on) => self.module_mut(id).on = on,
            Action::SetLayout(id, layout) => self.module_mut(id).layout = layout.sanitized(),
            Action::ResetModule(id) => {
                *self.module_mut(id) = ModuleConfig::default_for(id.descriptor())
            }
            Action::ResetAll => *self = Config::default(),
            Action::Reload
            | Action::SwitchProfile(_)
            | Action::CopyProfile(_)
            | Action::DeleteProfile(_) => {}
        }
        *self != before
    }

    pub fn to_json(&self) -> String {
        // Serializing these types cannot fail (only strings, numbers and booleans).
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Reads a configuration file. Never panics: always returns a complete and
    /// valid configuration, and the list of what was corrected.
    pub fn parse(text: &str) -> (Config, Vec<String>) {
        let mut warnings = Vec::new();
        let root = match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(root)) => root,
            Ok(_) => {
                warnings
                    .push("invalid configuration (not a JSON object): default values".to_owned());
                return (Config::default(), warnings);
            }
            Err(e) => {
                warnings.push(format!("unreadable configuration ({e}): default values"));
                return (Config::default(), warnings);
            }
        };
        match root.get("version").and_then(Value::as_u64) {
            Some(v) if v > u64::from(CONFIG_VERSION) => warnings.push(format!(
                "configuration from a newer version ({v}): read as well as possible"
            )),
            Some(_) => {}
            None => warnings.push("configuration without a version: read as version 1".to_owned()),
        }
        let mut config = Config {
            version: CONFIG_VERSION,
            modules: BTreeMap::new(),
        };
        match root.get("modules") {
            Some(Value::Object(modules)) => {
                for (key, value) in modules {
                    let default = match ModuleId::from_key(key) {
                        Some(id) => ModuleConfig::default_for(id.descriptor()),
                        None => ModuleConfig::default_unknown(),
                    };
                    let module = read_module(key, value, default, &mut warnings);
                    config.modules.insert(key.clone(), module);
                }
            }
            Some(_) => warnings.push("\"modules\" is not an object: default settings".to_owned()),
            None => {}
        }
        for d in MODULES {
            config
                .modules
                .entry(d.key.to_owned())
                .or_insert_with(|| ModuleConfig::default_for(d));
        }
        (config, warnings)
    }
}

/// `RRGGBB` (or `#RRGGBB`) as an integer, or None.
pub fn parse_rgb(text: &str) -> Option<u32> {
    let hex = text.strip_prefix('#').unwrap_or(text);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

fn read_module(
    key: &str,
    value: &Value,
    default: ModuleConfig,
    warnings: &mut Vec<String>,
) -> ModuleConfig {
    let Value::Object(fields) = value else {
        warnings.push(format!("{key}: invalid settings, default values"));
        return default;
    };
    let mut fix = |what: &str| warnings.push(format!("invalid {key}.{what}: default value"));
    let on = read_bool(fields.get("on"), default.on, || fix("on"));
    let layout = match fields.get("layout") {
        None => default.layout,
        Some(Value::Object(l)) => {
            let d = default.layout;
            let anchor = match l.get("anchor") {
                None => d.anchor,
                Some(v) => v.as_str().and_then(Anchor::from_key).unwrap_or_else(|| {
                    fix("layout.anchor");
                    d.anchor
                }),
            };
            Layout {
                anchor,
                auto: read_bool(l.get("auto"), d.auto, || fix("layout.auto")),
                x: read_fraction(l.get("x"), d.x, || fix("layout.x")),
                y: read_fraction(l.get("y"), d.y, || fix("layout.y")),
                scale: match l.get("scale") {
                    None => d.scale,
                    Some(v) => match v.as_u64() {
                        Some(s) if (u64::from(SCALE_MIN)..=u64::from(SCALE_MAX)).contains(&s) => {
                            s as u16
                        }
                        _ => {
                            fix("layout.scale");
                            d.scale
                        }
                    },
                },
            }
        }
        Some(_) => {
            fix("layout");
            default.layout
        }
    };
    let style = match fields.get("style") {
        None => default.style.clone(),
        Some(Value::Object(s)) => {
            let d = &default.style;
            Style {
                background: match s.get("background") {
                    None => d.background,
                    Some(v) => match v.as_u64() {
                        Some(b) if b <= 100 => b as u8,
                        _ => {
                            fix("style.background");
                            d.background
                        }
                    },
                },
                text: match s.get("text") {
                    None => d.text.clone(),
                    Some(v) => match v.as_str().and_then(parse_rgb) {
                        Some(rgb) => format!("{rgb:06X}"),
                        None => {
                            fix("style.text");
                            d.text.clone()
                        }
                    },
                },
                shadow: read_bool(s.get("shadow"), d.shadow, || fix("style.shadow")),
            }
        }
        Some(_) => {
            fix("style");
            default.style.clone()
        }
    };
    ModuleConfig { on, layout, style }
}

fn read_bool(value: Option<&Value>, default: bool, mut invalid: impl FnMut()) -> bool {
    match value {
        None => default,
        Some(v) => v.as_bool().unwrap_or_else(|| {
            invalid();
            default
        }),
    }
}

fn read_fraction(value: Option<&Value>, default: f32, mut invalid: impl FnMut()) -> f32 {
    match value {
        None => default,
        Some(v) => match v.as_f64() {
            Some(f) if f.is_finite() && (0.0..=1.0).contains(&f) => f as f32,
            _ => {
                invalid();
                default
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_configuration_covers_the_registry() {
        let c = Config::default();
        assert_eq!(c.version, CONFIG_VERSION);
        for d in MODULES {
            assert_eq!(c.modules[d.key], ModuleConfig::default_for(d));
        }
    }

    #[test]
    fn json_round_trip() {
        let mut c = Config::default();
        c.apply(Action::Toggle(ModuleId::Status));
        let (back, warnings) = Config::parse(&c.to_json());
        assert_eq!(back, c);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn an_unreadable_file_gives_default_values() {
        for text in ["", "{", "[1, 2]", "\"text\"", "null", "\u{0}\u{1}binary"] {
            let (c, warnings) = Config::parse(text);
            assert_eq!(c, Config::default(), "for {text:?}");
            assert_eq!(warnings.len(), 1, "for {text:?}");
        }
    }

    #[test]
    fn an_invalid_field_only_affects_itself() {
        let text = r#"{ "version": 1, "modules": { "status": {
            "on": "yes",
            "layout": { "anchor": "bottom_left", "auto": false, "x": 2.5, "y": 0.25, "scale": 900 },
            "style": { "background": 80, "text": "zzz", "shadow": false } } } }"#;
        let (c, warnings) = Config::parse(text);
        let m = &c.modules["status"];
        let d = ModuleConfig::default_for(ModuleId::Status.descriptor());
        assert_eq!(m.on, d.on, "\"yes\" is not a boolean");
        assert_eq!(m.layout.anchor, Anchor::BottomLeft);
        assert!(!m.layout.auto);
        assert_eq!(m.layout.x, d.layout.x, "2.5 is outside 0..1");
        assert_eq!(m.layout.y, 0.25);
        assert_eq!(m.layout.scale, d.layout.scale, "900 % is out of range");
        assert_eq!(m.style.background, 80);
        assert_eq!(m.style.text, DEFAULT_TEXT);
        assert!(!m.style.shadow);
        assert_eq!(warnings.len(), 4, "{warnings:?}");
    }

    #[test]
    fn an_unknown_module_is_kept_and_a_missing_module_is_added() {
        let text = r#"{ "version": 1, "modules": { "future": { "on": true } } }"#;
        let (c, _) = Config::parse(text);
        assert!(
            c.modules["future"].on,
            "the module from a newer version survives"
        );
        assert_eq!(
            c.modules["status"],
            ModuleConfig::default_for(ModuleId::Status.descriptor())
        );
        let (again, _) = Config::parse(&c.to_json());
        assert_eq!(again, c);
    }

    #[test]
    fn a_newer_version_is_read_as_well_as_possible() {
        let (c, warnings) =
            Config::parse(r#"{ "version": 7, "modules": { "status": { "on": false } } }"#);
        assert!(!c.modules["status"].on);
        assert_eq!(warnings.len(), 1);
        assert_eq!(
            c.version, CONFIG_VERSION,
            "rewritten in this version's format"
        );
    }

    #[test]
    fn actions() {
        let mut c = Config::default();
        let id = ModuleId::Status;
        let on = c.module(id).on;
        assert!(c.apply(Action::Toggle(id)));
        assert_eq!(c.module(id).on, !on);
        assert!(
            !c.apply(Action::Set(id, !on)),
            "already in that state: nothing changes"
        );
        assert!(c.apply(Action::ResetModule(id)));
        assert_eq!(c.module(id).on, on);
        c.apply(Action::Toggle(id));
        assert!(c.apply(Action::ResetAll));
        assert_eq!(c, Config::default());
        assert!(!c.apply(Action::Reload));
    }

    #[test]
    fn resolved_style() {
        let s = Style::default().resolve();
        assert_eq!(
            s.background, 0x7800_0000,
            "47 % = the alpha of Horizon's boxes"
        );
        assert_eq!(s.text, 0xFF_FFFF);
        assert_eq!(parse_rgb("#55ff55"), Some(0x55FF55));
        assert_eq!(parse_rgb("12345"), None);
        let r = Config::default().resolve();
        assert_eq!(
            r.active_count(),
            MODULES.iter().filter(|d| d.default_on).count()
        );
    }

    #[test]
    fn set_layout_keeps_the_layout_in_range() {
        let mut c = Config::default();
        let id = ModuleId::Status;
        let moved = Layout {
            anchor: Anchor::Center,
            auto: false,
            x: 0.25,
            y: f32::NAN,
            scale: 900,
        };
        assert!(c.apply(Action::SetLayout(id, moved)));
        let l = c.module(id).layout;
        assert_eq!((l.anchor, l.auto, l.x), (Anchor::Center, false, 0.25));
        assert_eq!(l.y, 0.0, "not a number: 0");
        assert_eq!(l.scale, SCALE_MAX);
        assert!(
            !c.apply(Action::SetLayout(id, l)),
            "the same layout again: no change"
        );
        let (back, warnings) = Config::parse(&c.to_json());
        assert_eq!(back, c, "a moved widget survives a save");
        assert!(warnings.is_empty(), "{warnings:?}");
    }
}
