// The HUD (hidden by F1): the Core decides what to draw and where; this helper only
// translates that into game calls. No disk reads and no allocations once the
// widget shapes are built.
fn essentials_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    use crate::game::{essentials_events as ev, essentials_types as t};
    use horizon_client::extras::Corner;
    if !game.native_on(t::MOD_ID) {
        return;
    }
    let game = &*game;
    let screen = [batch.width, batch.height];
    let corner = |i: usize| match i {
        0 => Corner::TopLeft,
        1 => Corner::TopRight,
        2 => Corner::BottomLeft,
        _ => Corner::BottomRight,
    };
    let mut place = |i: usize, w: f32, h: f32| {
        // F3 takes the top corners, as it does for Horizon's own boxes.
        if !game.native_corner_shown(corner(i)) {
            return None;
        }
        let (x, y) = corners.place(corner(i), w, h);
        Some([x, y])
    };
    let mut paint = |shapes: &t::Shapes, [ox, oy]: [f32; 2]| {
        for op in shapes.ops() {
            match *op {
                t::Op::Fill { rect: [x, y, w, h], argb } => game.native_fill(batch, [ox + x, oy + y, w, h], argb),
                t::Op::Text { slot, at: [x, y], rgb, shadow } => {
                    game.native_text(batch, shapes.text_of(slot), [ox + x, oy + y], rgb, shadow)
                }
            }
        }
    };
    ev::hud(screen, &|text| game.native_text_width(text), &mut place, &mut paint);
}
