// Horizon Essentials: the status module (crate::game::essentials_status).
// A small "Essentials 0.1.0 - default - 1/1" box: it proves that the Core loads
// the configuration, applies enabling, placement and style, and that rendering
// rebuilds nothing as long as nothing changes.

use std::fmt::Write as _;

use crate::game::essentials_config::ResolvedStyle;
use crate::game::essentials_types::{Module, Shapes, VERSION, WidgetCtx};

/// The height of a box line, like Horizon's boxes.
pub const LINE_HEIGHT: f32 = 12.0;

#[derive(Debug, Default)]
pub struct Status {
    shapes: Shapes,
    line: String,
    // What `shapes` was built from: rebuilt only when it changes.
    built: Option<(usize, usize, ResolvedStyle)>,
    built_profile: String,
    /// How many times the shapes were rebuilt (for tests).
    pub rebuilds: u64,
}

impl Module for Status {
    fn on_enabled(&mut self, _on: bool) {
        self.built = None;
    }

    fn widget(&mut self, ctx: &WidgetCtx<'_>) -> Option<&Shapes> {
        let key = (ctx.active_modules, ctx.total_modules, ctx.style);
        if self.built != Some(key) || self.built_profile != ctx.profile {
            self.line.clear();
            let _ = write!(
                self.line,
                "Essentials {VERSION} - {} - {}/{}",
                ctx.profile, ctx.active_modules, ctx.total_modules
            );
            let w = (ctx.measure)(&self.line) + 4.0;
            self.shapes.clear();
            if ctx.style.background >> 24 != 0 {
                self.shapes.fill([0.0, 0.0, w, LINE_HEIGHT], ctx.style.background);
            }
            self.shapes.text(&self.line, [2.0, 2.0], ctx.style.text, ctx.style.shadow);
            self.shapes.set_size([w, LINE_HEIGHT]);
            self.built = Some(key);
            self.built_profile.clear();
            self.built_profile.push_str(ctx.profile);
            self.rebuilds += 1;
        }
        Some(&self.shapes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::essentials_config::Style;
    use crate::game::essentials_types::Op;

    fn ctx<'a>(measure: &'a dyn Fn(&str) -> f32, profile: &'a str, style: ResolvedStyle) -> WidgetCtx<'a> {
        WidgetCtx { measure, style, profile, active_modules: 1, total_modules: 1 }
    }

    #[test]
    fn the_box_is_rebuilt_only_when_it_changes() {
        let measure = |s: &str| s.len() as f32 * 6.0;
        let style = Style::default().resolve();
        let mut m = Status::default();
        let shapes = m.widget(&ctx(&measure, "default", style)).unwrap();
        assert_eq!(shapes.text_of(0), "Essentials 0.1.0 - default - 1/1");
        assert_eq!(shapes.size(), [6.0 * 32.0 + 4.0, LINE_HEIGHT]);
        assert!(matches!(shapes.ops()[0], Op::Fill { argb: 0x7800_0000, .. }));
        for _ in 0..100 {
            m.widget(&ctx(&measure, "default", style));
        }
        assert_eq!(m.rebuilds, 1, "100 identical frames: no rebuild");
        m.widget(&ctx(&measure, "pvp", style));
        assert_eq!(m.rebuilds, 2);
        m.on_enabled(true);
        m.widget(&ctx(&measure, "pvp", style));
        assert_eq!(m.rebuilds, 3);
    }

    #[test]
    fn no_background_no_rectangle() {
        let measure = |_: &str| 10.0;
        let style = Style { background: 0, ..Style::default() }.resolve();
        let mut m = Status::default();
        let shapes = m.widget(&ctx(&measure, "default", style)).unwrap();
        assert_eq!(shapes.ops().len(), 1);
        assert!(matches!(shapes.ops()[0], Op::Text { .. }));
    }
}
