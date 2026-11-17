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
    let room = [
        (screen[0] - size[0]).max(0.0),
        (screen[1] - size[1]).max(0.0),
    ];
    if layout.auto {
        anchored(layout.anchor, room)
    } else {
        [
            (layout.x.clamp(0.0, 1.0) * room[0]).round(),
            (layout.y.clamp(0.0, 1.0) * room[1]).round(),
        ]
    }
}

/// `layout` moved to the pixel position `at` (a HUD Editor drag): free placement, `x` and `y`
/// the share of the free space, so the widget keeps its place relative to the edges when the
/// window is resized. Anchor and scale are kept.
pub fn moved_to(layout: &Layout, at: [f32; 2], size: [f32; 2], screen: [f32; 2]) -> Layout {
    let share = |at: f32, size: f32, screen: f32| {
        let room = screen - size;
        if room > 0.0 {
            (at / room).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    Layout {
        auto: false,
        x: share(at[0], size[0], screen[0]),
        y: share(at[1], size[1], screen[1]),
        ..*layout
    }
}

/// `at` (a widget's top left) magnetised, on each axis, to the closest of: the screen's edges,
/// its centre, or a line of a `grid`-pixel grid, when one is within `reach` pixels; then kept
/// on screen. A `grid` of 0 disables the grid lines.
pub fn snap(at: [f32; 2], size: [f32; 2], screen: [f32; 2], grid: f32, reach: f32) -> [f32; 2] {
    let axis = |at: f32, size: f32, screen: f32| {
        let room = (screen - size).max(0.0);
        let line = if grid > 0.0 {
            (at / grid).round() * grid
        } else {
            f32::NAN
        };
        let best = [0.0, room / 2.0, room, line]
            .into_iter()
            .filter(|c| c.is_finite() && (c - at).abs() <= reach)
            .min_by(|a, b| (a - at).abs().total_cmp(&(b - at).abs()));
        best.unwrap_or(at).clamp(0.0, room).round()
    };
    [
        axis(at[0], size[0], screen[0]),
        axis(at[1], size[1], screen[1]),
    ]
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
        Layout {
            anchor,
            auto,
            x,
            y,
            scale: 100,
        }
    }

    #[test]
    fn automatic_anchoring() {
        let screen = [480.0, 270.0];
        let size = [100.0, 12.0];
        assert_eq!(
            position(&layout(Anchor::Center, true, 0.0, 0.0), size, screen),
            [190.0, 129.0]
        );
        assert_eq!(
            position(&layout(Anchor::TopCenter, true, 0.0, 0.0), size, screen),
            [190.0, 2.0]
        );
        assert_eq!(
            position(&layout(Anchor::BottomRight, true, 0.0, 0.0), size, screen),
            [378.0, 256.0]
        );
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

    #[test]
    fn a_dragged_widget_lands_where_it_was_dropped() {
        let screen = [480.0, 270.0];
        let size = [100.0, 12.0];
        let start = layout(Anchor::TopRight, true, 0.0, 0.0);
        let moved = moved_to(&start, [190.0, 129.0], size, screen);
        assert!(!moved.auto, "a drag switches to free placement");
        assert_eq!(moved.anchor, Anchor::TopRight, "the anchor is kept");
        assert_eq!(position(&moved, size, screen), [190.0, 129.0]);
        // Dropped past the edge: kept on screen.
        let out = moved_to(&start, [900.0, -50.0], size, screen);
        assert_eq!(position(&out, size, screen), [380.0, 0.0]);
        // A widget wider than the screen has no free space: top left.
        let wide = moved_to(&start, [10.0, 10.0], [600.0, 300.0], screen);
        assert_eq!((wide.x, wide.y), (0.0, 0.0));
    }

    #[test]
    fn snapping_to_edges_centre_and_grid() {
        let screen = [480.0, 270.0];
        let size = [100.0, 12.0];
        // Close to the left edge and the vertical centre (129).
        assert_eq!(snap([3.0, 126.0], size, screen, 0.0, 4.0), [0.0, 129.0]);
        // Close to the right edge (380) and the bottom (258).
        assert_eq!(snap([377.0, 255.0], size, screen, 0.0, 4.0), [380.0, 258.0]);
        // Far from everything: unchanged; with a 10 px grid: the closest line.
        assert_eq!(snap([57.0, 64.0], size, screen, 0.0, 4.0), [57.0, 64.0]);
        assert_eq!(snap([57.0, 64.0], size, screen, 10.0, 4.0), [60.0, 60.0]);
        // The closest candidate wins: 127 is nearer the centre (129) than the 125 line.
        assert_eq!(snap([70.0, 127.0], size, screen, 25.0, 4.0)[1], 129.0);
        // Off screen: kept on it.
        assert_eq!(snap([-30.0, 400.0], size, screen, 0.0, 4.0), [0.0, 258.0]);
    }
}
