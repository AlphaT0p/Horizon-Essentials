// Horizon Essentials : la configuration (crate::game::essentials_config).
//
// Résilience : la lecture ne fait jamais planter. Un fichier illisible donne la
// configuration par défaut ; un champ absent, d'un mauvais type ou hors limites
// prend sa valeur par défaut sans toucher aux autres ; un module inconnu (ajouté
// par une version plus récente) est gardé tel quel ; un module absent (ajouté par
// cette version) reçoit ses réglages par défaut. Chaque correction est signalée
// par un avertissement, que le Core affiche une fois dans le chat.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::game::essentials_registry::{COUNT, Descriptor, MODULES, ModuleId};
use crate::game::essentials_types::{Action, Anchor};

/// La version du format. Une version plus récente se lit quand même, au mieux.
pub const CONFIG_VERSION: u32 = 1;
pub const SCALE_MIN: u16 = 50;
pub const SCALE_MAX: u16 = 200;
/// L'opacité du fond par défaut : 47 % donne l'alpha 0x78, celui des boîtes de Horizon.
pub const DEFAULT_BACKGROUND: u8 = 47;
pub const DEFAULT_TEXT: &str = "FFFFFF";

/// Où et à quelle taille le widget s'affiche (le HUD Editor écrit ici).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Layout {
    pub anchor: Anchor,
    /// Vrai : empilé automatiquement dans le coin de `anchor` avec les autres boîtes.
    /// Faux : placé librement, à `x` et `y`.
    pub auto: bool,
    /// La part de la place libre à gauche (0 à 1) : le widget reste à l'écran
    /// quand la fenêtre change de taille.
    pub x: f32,
    /// La part de la place libre au-dessus (0 à 1).
    pub y: f32,
    /// En pourcents, de 50 à 200.
    pub scale: u16,
}

/// Les couleurs du widget.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Style {
    /// Opacité du fond, de 0 à 100 %.
    pub background: u8,
    /// Couleur du texte, `RRGGBB`.
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
    /// Par clé de module. Une `BTreeMap` garde un ordre stable dans le fichier.
    pub modules: BTreeMap<String, ModuleConfig>,
}

/// Le style prêt pour le rendu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedStyle {
    pub background: u32,
    pub text: u32,
    pub shadow: bool,
}

/// La configuration prête pour le rendu : des tableaux indexés par `ModuleId`,
/// recalculés seulement quand la configuration change, pour qu'une frame ne
/// cherche jamais une clé dans une table.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub on: [bool; COUNT],
    pub layout: [Layout; COUNT],
    pub style: [ResolvedStyle; COUNT],
}

impl Resolved {
    pub fn active_count(&self) -> usize {
        self.on.iter().filter(|on| **on).count()
    }
}

impl Default for Style {
    fn default() -> Self {
        Style { background: DEFAULT_BACKGROUND, text: DEFAULT_TEXT.to_owned(), shadow: true }
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
        Layout { anchor: d.default_anchor, auto: true, x: 0.0, y: 0.0, scale: 100 }
    }
}

impl ModuleConfig {
    pub fn default_for(d: &Descriptor) -> ModuleConfig {
        ModuleConfig { on: d.default_on, layout: Layout::default_for(d), style: Style::default() }
    }

    /// Pour un module que ce registre ne connaît pas.
    fn default_unknown() -> ModuleConfig {
        ModuleConfig {
            on: false,
            layout: Layout { anchor: Anchor::TopLeft, auto: true, x: 0.0, y: 0.0, scale: 100 },
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
        Config { version: CONFIG_VERSION, modules }
    }
}

impl Config {
    /// La configuration d'un module du registre (toujours présente après `parse`
    /// ou `default`, sinon ses réglages par défaut).
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
        let first = self.module(ModuleId::all().next().expect("au moins un module"));
        let mut r = Resolved {
            on: [false; COUNT],
            layout: [first.layout; COUNT],
            style: [first.style.resolve(); COUNT],
        };
        for id in ModuleId::all() {
            let m = self.module(id);
            r.on[id.index()] = m.on;
            r.layout[id.index()] = m.layout;
            r.style[id.index()] = m.style.resolve();
        }
        r
    }

    /// Applique une action. Vrai si la configuration a changé.
    /// `Reload` ne change rien ici : c'est le stockage qui relit le disque.
    pub fn apply(&mut self, action: Action) -> bool {
        let before = self.clone();
        match action {
            Action::Toggle(id) => {
                let m = self.module_mut(id);
                m.on = !m.on;
            }
            Action::Set(id, on) => self.module_mut(id).on = on,
            Action::ResetModule(id) => *self.module_mut(id) = ModuleConfig::default_for(id.descriptor()),
            Action::ResetAll => *self = Config::default(),
            Action::Reload => {}
        }
        *self != before
    }

    pub fn to_json(&self) -> String {
        // La sérialisation de ces types ne peut pas échouer (que des chaînes, nombres et booléens).
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Lit un fichier de configuration. Ne panique jamais : renvoie toujours une
    /// configuration complète et valide, et la liste de ce qui a été corrigé.
    pub fn parse(text: &str) -> (Config, Vec<String>) {
        let mut warnings = Vec::new();
        let root = match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(root)) => root,
            Ok(_) => {
                warnings.push("configuration invalide (pas un objet JSON) : valeurs par défaut".to_owned());
                return (Config::default(), warnings);
            }
            Err(e) => {
                warnings.push(format!("configuration illisible ({e}) : valeurs par défaut"));
                return (Config::default(), warnings);
            }
        };
        match root.get("version").and_then(Value::as_u64) {
            Some(v) if v > u64::from(CONFIG_VERSION) => warnings.push(format!(
                "configuration d'une version plus récente ({v}) : lue au mieux"
            )),
            Some(_) => {}
            None => warnings.push("configuration sans version : lue comme la version 1".to_owned()),
        }
        let mut config = Config { version: CONFIG_VERSION, modules: BTreeMap::new() };
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
            Some(_) => warnings.push("« modules » n'est pas un objet : réglages par défaut".to_owned()),
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

/// `RRGGBB` (ou `#RRGGBB`) en entier, ou None.
pub fn parse_rgb(text: &str) -> Option<u32> {
    let hex = text.strip_prefix('#').unwrap_or(text);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

fn read_module(key: &str, value: &Value, default: ModuleConfig, warnings: &mut Vec<String>) -> ModuleConfig {
    let Value::Object(fields) = value else {
        warnings.push(format!("{key} : réglages invalides, valeurs par défaut"));
        return default;
    };
    let mut fix = |what: &str| warnings.push(format!("{key}.{what} invalide : valeur par défaut"));
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
                        Some(s) if (u64::from(SCALE_MIN)..=u64::from(SCALE_MAX)).contains(&s) => s as u16,
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
    fn la_configuration_par_defaut_couvre_le_registre() {
        let c = Config::default();
        assert_eq!(c.version, CONFIG_VERSION);
        for d in MODULES {
            assert_eq!(c.modules[d.key], ModuleConfig::default_for(d));
        }
    }

    #[test]
    fn aller_retour_json() {
        let mut c = Config::default();
        c.apply(Action::Toggle(ModuleId::Status));
        let (back, warnings) = Config::parse(&c.to_json());
        assert_eq!(back, c);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn un_fichier_illisible_donne_les_valeurs_par_defaut() {
        for text in ["", "{", "[1, 2]", "\"texte\"", "null", "\u{0}\u{1}binaire"] {
            let (c, warnings) = Config::parse(text);
            assert_eq!(c, Config::default(), "pour {text:?}");
            assert_eq!(warnings.len(), 1, "pour {text:?}");
        }
    }

    #[test]
    fn un_champ_invalide_ne_touche_que_lui() {
        let text = r#"{ "version": 1, "modules": { "status": {
            "on": "oui",
            "layout": { "anchor": "bottom_left", "auto": false, "x": 2.5, "y": 0.25, "scale": 900 },
            "style": { "background": 80, "text": "zzz", "shadow": false } } } }"#;
        let (c, warnings) = Config::parse(text);
        let m = &c.modules["status"];
        let d = ModuleConfig::default_for(ModuleId::Status.descriptor());
        assert_eq!(m.on, d.on, "« oui » n'est pas un booléen");
        assert_eq!(m.layout.anchor, Anchor::BottomLeft);
        assert!(!m.layout.auto);
        assert_eq!(m.layout.x, d.layout.x, "2.5 est hors de 0..1");
        assert_eq!(m.layout.y, 0.25);
        assert_eq!(m.layout.scale, d.layout.scale, "900 % est hors limites");
        assert_eq!(m.style.background, 80);
        assert_eq!(m.style.text, DEFAULT_TEXT);
        assert!(!m.style.shadow);
        assert_eq!(warnings.len(), 4, "{warnings:?}");
    }

    #[test]
    fn un_module_inconnu_est_garde_et_un_module_absent_est_ajoute() {
        let text = r#"{ "version": 1, "modules": { "futur": { "on": true } } }"#;
        let (c, _) = Config::parse(text);
        assert!(c.modules["futur"].on, "le module d'une version plus récente survit");
        assert_eq!(c.modules["status"], ModuleConfig::default_for(ModuleId::Status.descriptor()));
        let (again, _) = Config::parse(&c.to_json());
        assert_eq!(again, c);
    }

    #[test]
    fn une_version_plus_recente_se_lit_au_mieux() {
        let (c, warnings) = Config::parse(r#"{ "version": 7, "modules": { "status": { "on": false } } }"#);
        assert!(!c.modules["status"].on);
        assert_eq!(warnings.len(), 1);
        assert_eq!(c.version, CONFIG_VERSION, "réécrite au format de cette version");
    }

    #[test]
    fn les_actions() {
        let mut c = Config::default();
        let id = ModuleId::Status;
        let on = c.module(id).on;
        assert!(c.apply(Action::Toggle(id)));
        assert_eq!(c.module(id).on, !on);
        assert!(!c.apply(Action::Set(id, !on)), "déjà dans cet état : rien ne change");
        assert!(c.apply(Action::ResetModule(id)));
        assert_eq!(c.module(id).on, on);
        c.apply(Action::Toggle(id));
        assert!(c.apply(Action::ResetAll));
        assert_eq!(c, Config::default());
        assert!(!c.apply(Action::Reload));
    }

    #[test]
    fn le_style_resolu() {
        let s = Style::default().resolve();
        assert_eq!(s.background, 0x7800_0000, "47 % = l'alpha des boîtes de Horizon");
        assert_eq!(s.text, 0xFF_FFFF);
        assert_eq!(parse_rgb("#55ff55"), Some(0x55FF55));
        assert_eq!(parse_rgb("12345"), None);
        let r = Config::default().resolve();
        assert_eq!(r.active_count(), MODULES.iter().filter(|d| d.default_on).count());
    }
}
