// Horizon Essentials: widget placement on screen (crate::game::essentials_draw).
// Pure calculations, in GUI pixels: used by the HUD and later by the HUD Editor.

use crate::game::essentials_config::Layout;
use crate::game::essentials_types::Anchor;

/// The margin left at the screen edge by a widget anchored outside the corners.
pub const MARGIN: f32 = 2.0;

/// The position of a widget of size `size` on a `screen`, except for automatic
/// corners (those go through Horizon's `Corners`):
/// - automatic: attached to `anchor` (center, middle of an edge…);
/// - free: `x` and `y` are the share of the free space, from 0 to 1.
pub fn position(layout: &Layout, size: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    let room = [(screen[0] - size[0]).max(0.0), (screen[1] - size[1]).max(0.0)];
    if layout.auto {
        anchored(layout.anchor, room)
    } else {
        [(layout.x.clamp(0.0, 1.0) * room[0]).round(), (layout.y.clamp(0.0, 1.0) * room[1]).round()]
    }
}

fn anchored(anchor: Anchor, room: [f32; 2]) -> [f32; 2] {
    let (fx, fy) = anchor.factors();
    let axis = |f: f32, room: f32| {
        // At the edge: the margin; in the middle: centered.
        let inner = (room - 2.0 * MARGIN).max(0.0);
        (MARGIN.min(room) + f * inner).round()
    };
    [axis(fx, room[0]), axis(fy, room[1])]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(anchor: Anchor, auto: bool, x: f32, y: f32) -> Layout {
        Layout { anchor, auto, x, y, scale: 100 }
    }

    #[test]
    fn automatic_anchoring() {
        let screen = [480.0, 270.0];
        let size = [100.0, 12.0];
        assert_eq!(position(&layout(Anchor::Center, true, 0.0, 0.0), size, screen), [190.0, 129.0]);
        assert_eq!(position(&layout(Anchor::TopCenter, true, 0.0, 0.0), size, screen), [190.0, 2.0]);
        assert_eq!(position(&layout(Anchor::BottomRight, true, 0.0, 0.0), size, screen), [378.0, 256.0]);
    }

    #[test]
    fn free_placement_in_fractions() {
        let size = [100.0, 12.0];
        let l = layout(Anchor::TopLeft, false, 0.5, 1.0);
        assert_eq!(position(&l, size, [480.0, 270.0]), [190.0, 258.0]);
        // The window shrinks: the widget stays on screen.
        assert_eq!(position(&l, size, [240.0, 135.0]), [70.0, 123.0]);
    }

    #[test]
    fn a_widget_larger_than_the_screen_stays_top_left() {
        let l = layout(Anchor::Center, false, 1.0, 1.0);
        assert_eq!(position(&l, [600.0, 300.0], [480.0, 270.0]), [0.0, 0.0]);
        let l = layout(Anchor::Center, true, 0.0, 0.0);
        assert_eq!(position(&l, [600.0, 300.0], [480.0, 270.0]), [0.0, 0.0]);
    }
}
