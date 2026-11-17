// Horizon Essentials : placement des widgets à l'écran (crate::game::essentials_draw).
// Des calculs purs, en pixels GUI : le HUD et plus tard le HUD Editor s'en servent.

use crate::game::essentials_config::Layout;
use crate::game::essentials_types::Anchor;

/// La marge laissée au bord de l'écran par un widget accroché sans coin.
pub const MARGIN: f32 = 2.0;

/// La place d'un widget de taille `size` sur un écran `screen`, hors coins
/// automatiques (ceux-là passent par les `Corners` de Horizon) :
/// - automatique : accroché à `anchor` (centre, milieu d'un bord…) ;
/// - libre : `x` et `y` sont la part de la place libre, de 0 à 1.
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
        // Au bord : la marge ; au milieu : centré.
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
    fn accroche_automatique() {
        let screen = [480.0, 270.0];
        let size = [100.0, 12.0];
        assert_eq!(position(&layout(Anchor::Center, true, 0.0, 0.0), size, screen), [190.0, 129.0]);
        assert_eq!(position(&layout(Anchor::TopCenter, true, 0.0, 0.0), size, screen), [190.0, 2.0]);
        assert_eq!(position(&layout(Anchor::BottomRight, true, 0.0, 0.0), size, screen), [378.0, 256.0]);
    }

    #[test]
    fn placement_libre_en_fractions() {
        let size = [100.0, 12.0];
        let l = layout(Anchor::TopLeft, false, 0.5, 1.0);
        assert_eq!(position(&l, size, [480.0, 270.0]), [190.0, 258.0]);
        // La fenêtre rétrécit : le widget reste dans l'écran.
        assert_eq!(position(&l, size, [240.0, 135.0]), [70.0, 123.0]);
    }

    #[test]
    fn un_widget_plus_grand_que_l_ecran_reste_en_haut_a_gauche() {
        let l = layout(Anchor::Center, false, 1.0, 1.0);
        assert_eq!(position(&l, [600.0, 300.0], [480.0, 270.0]), [0.0, 0.0]);
        let l = layout(Anchor::Center, true, 0.0, 0.0);
        assert_eq!(position(&l, [600.0, 300.0], [480.0, 270.0]), [0.0, 0.0]);
    }
}
