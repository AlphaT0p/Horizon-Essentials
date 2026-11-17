// Horizon Essentials : types partagés par le Core, l'interface et les modules.
// Ce fichier devient le module crate::game::essentials_types du jeu, et un module
// du banc de test (harness/lib.rs). Pas de `//!` ni d'attribut interne : le format
// des bundles horizon-mixins les refuse.

use crate::game::essentials_config::ResolvedStyle;
use crate::game::essentials_registry::ModuleId;

/// L'id du mod : sa ligne dans Options > Mods et son dossier de données
/// (horizon-mods/.horizon-mod-data/horizon_essentials/).
pub const MOD_ID: &str = "horizon_essentials";
pub const NAME: &str = "Horizon Essentials";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "Profils, widgets et réglages PvP en complément de Horizon.";

/// Le titre de la ligne dans Options > Mods.
pub fn title() -> String {
    format!("{NAME} {VERSION}")
}

// ── Placement ───────────────────────────────────────────────────────────────

/// Le point de l'écran auquel un widget est accroché.
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

    /// Le nom dans les fichiers de configuration.
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

    /// Les coins, dans l'ordre de `horizon_client::extras::Corner`
    /// (TopLeft, TopRight, BottomLeft, BottomRight) : en placement automatique,
    /// le widget s'empile dans ce coin avec les boîtes de Horizon et des autres mods.
    pub fn corner_index(self) -> Option<usize> {
        match self {
            Anchor::TopLeft => Some(0),
            Anchor::TopRight => Some(1),
            Anchor::BottomLeft => Some(2),
            Anchor::BottomRight => Some(3),
            _ => None,
        }
    }

    /// La part de la place libre à gauche et au-dessus du widget (0, ½ ou 1).
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

// ── Actions : la seule façon pour l'interface de changer la configuration ──

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Toggle(ModuleId),
    Set(ModuleId, bool),
    /// Remet les réglages par défaut d'un module (activation, placement, style).
    ResetModule(ModuleId),
    /// Remet toute la configuration par défaut.
    ResetAll,
    /// Relit la configuration sur le disque.
    Reload,
}

// ── Rendu : une liste de formes réutilisée d'une frame à l'autre ───────────

/// Une forme, en pixels GUI, relative au coin haut gauche du widget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Fill { rect: [f32; 4], argb: u32 },
    Text { slot: usize, at: [f32; 2], rgb: u32, shadow: bool },
}

/// Ce qu'un widget dessine. `clear` garde la mémoire : une fois les premières
/// frames passées, reconstruire la liste n'alloue plus rien (les textes sont
/// recopiés dans des `String` déjà allouées).
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

    /// La taille du widget, pour le placer.
    pub fn size(&self) -> [f32; 2] {
        self.size
    }

    pub fn set_size(&mut self, size: [f32; 2]) {
        self.size = size;
    }
}

// ── L'interface commune des modules ─────────────────────────────────────────

/// Ce que le Core sait au moment d'un tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickCtx {
    pub tick: u64,
    pub in_world: bool,
}

/// Ce qu'un widget reçoit pour se dessiner.
pub struct WidgetCtx<'a> {
    /// La largeur d'un texte en pixels GUI (`native_text_width` dans le jeu).
    pub measure: &'a dyn Fn(&str) -> f32,
    pub style: ResolvedStyle,
    pub profile: &'a str,
    pub active_modules: usize,
    pub total_modules: usize,
}

/// Un module d'Essentials. Toutes les méthodes ont un corps vide par défaut :
/// un module n'écrit que celles dont il a besoin.
pub trait Module {
    /// Une frame vient d'être dessinée.
    fn on_frame(&mut self, _elapsed_ms: f32) {}
    /// Un tick du jeu (20 par seconde).
    fn on_tick(&mut self, _ctx: &TickCtx) {}
    /// Une touche ou un bouton de souris, par son nom vanilla (`key.mouse.left`).
    fn on_key(&mut self, _key: &str, _down: bool) {}
    /// Le module vient d'être activé ou désactivé : l'occasion de remettre son état à zéro.
    fn on_enabled(&mut self, _on: bool) {}
    /// Son widget, s'il en a un : le module le reconstruit seulement si ce qu'il
    /// affiche a changé, et rend sinon la liste déjà prête.
    fn widget(&mut self, _ctx: &WidgetCtx<'_>) -> Option<&Shapes> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_titre_contient_la_version() {
        assert_eq!(title(), "Horizon Essentials 0.1.0");
    }

    #[test]
    fn l_id_respecte_le_format_horizon() {
        // a-z, 0-9 et _ uniquement (docs/mod-manifest).
        assert!(MOD_ID.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
    }

    #[test]
    fn chaque_ancre_se_relit_par_son_nom() {
        for a in Anchor::ALL {
            assert_eq!(Anchor::from_key(a.key()), Some(a));
            assert_eq!(serde_json::to_string(&a).unwrap(), format!("\"{}\"", a.key()));
        }
        assert_eq!(Anchor::from_key("nulle_part"), None);
    }

    #[test]
    fn les_coins_suivent_l_ordre_de_horizon() {
        assert_eq!(Anchor::TopLeft.corner_index(), Some(0));
        assert_eq!(Anchor::TopRight.corner_index(), Some(1));
        assert_eq!(Anchor::BottomLeft.corner_index(), Some(2));
        assert_eq!(Anchor::BottomRight.corner_index(), Some(3));
        assert_eq!(Anchor::Center.corner_index(), None);
    }

    #[test]
    fn shapes_reutilise_ses_textes() {
        let mut s = Shapes::default();
        s.text("un", [0.0, 0.0], 0xFFFFFF, true);
        s.fill([0.0, 0.0, 1.0, 1.0], 0x7800_0000);
        let capacity = s.texts[0].capacity();
        s.clear();
        s.text("deux", [1.0, 2.0], 0, false);
        assert_eq!(s.texts.len(), 1, "le texte reprend l'emplacement existant");
        assert!(s.texts[0].capacity() >= capacity);
        assert_eq!(s.ops(), &[Op::Text { slot: 0, at: [1.0, 2.0], rgb: 0, shadow: false }]);
        assert_eq!(s.text_of(0), "deux");
        assert_eq!(s.text_of(9), "");
    }
}
