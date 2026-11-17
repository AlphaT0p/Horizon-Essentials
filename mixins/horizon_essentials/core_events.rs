// Horizon Essentials : la répartition des événements (crate::game::essentials_events).
//
// Les helpers h_*.rs appellent ces fonctions et rien d'autre du Core. Elles ne
// connaissent pas le jeu : ce qu'il faut du jeu leur est passé en paramètre
// (le dossier de données, une mesure de texte, de quoi placer et peindre).

use std::path::PathBuf;

use crate::game::essentials_draw as draw;
use crate::game::essentials_registry::{COUNT, ModuleId};
use crate::game::essentials_state::{self as state};
use crate::game::essentials_types::{Action, Shapes, TickCtx, WidgetCtx};

/// Une frame (sur la fenêtre). Charge la configuration à la première.
pub fn frame(dir: impl FnOnce() -> PathBuf, elapsed_ms: f32) {
    state::with(dir, |s| {
        s.advance(elapsed_ms);
        for id in ModuleId::all() {
            if s.resolved.on[id.index()] {
                s.modules.get_mut(id).on_frame(elapsed_ms);
            }
        }
    });
}

/// Un tick. Rend le prochain message à écrire dans le chat, quand le joueur est
/// dans un monde (au plus un par tick).
pub fn tick(dir: impl FnOnce() -> PathBuf, in_world: bool) -> Option<String> {
    state::with(dir, |s| {
        s.tick += 1;
        let ctx = TickCtx { tick: s.tick, in_world };
        for id in ModuleId::all() {
            if s.resolved.on[id.index()] {
                s.modules.get_mut(id).on_tick(&ctx);
            }
        }
        if in_world { s.next_message() } else { None }
    })
    .flatten()
}

/// Une touche ou un bouton de souris, par son nom vanilla.
pub fn key(dir: impl FnOnce() -> PathBuf, key: &str, down: bool) {
    state::with(dir, |s| {
        for id in ModuleId::all() {
            if s.resolved.on[id.index()] {
                s.modules.get_mut(id).on_key(key, down);
            }
        }
    });
}

/// Applique une action venue de l'interface ou d'une commande.
pub fn apply(dir: impl FnOnce() -> PathBuf, action: Action) -> bool {
    state::with(dir, |s| s.apply(action)).unwrap_or(false)
}

/// Dessine les widgets actifs.
/// - `screen` : la taille de l'écran en pixels GUI ;
/// - `measure` : la largeur d'un texte ;
/// - `place_corner(coin, w, h)` : la place dans un coin (0 haut gauche, 1 haut droit,
///   2 bas gauche, 3 bas droit), ou None si le coin est pris (écran F3) ;
/// - `paint(formes, position)` : peint les formes.
///
/// Ne lit jamais le disque : si la configuration n'est pas encore chargée (avant
/// la première frame), rien n'est dessiné.
pub fn hud(
    screen: [f32; 2],
    measure: &dyn Fn(&str) -> f32,
    place_corner: &mut dyn FnMut(usize, f32, f32) -> Option<[f32; 2]>,
    paint: &mut dyn FnMut(&Shapes, [f32; 2]),
) {
    state::with_loaded(|s| {
        let state::State { modules, resolved, profile, .. } = s;
        let active = resolved.active_count();
        for id in ModuleId::all() {
            let i = id.index();
            if !resolved.on[i] || !id.descriptor().has_widget {
                continue;
            }
            let ctx = WidgetCtx {
                measure,
                style: resolved.style[i],
                profile,
                active_modules: active,
                total_modules: COUNT,
            };
            let Some(shapes) = modules.get_mut(id).widget(&ctx) else {
                continue;
            };
            let size = shapes.size();
            let layout = resolved.layout[i];
            let at = match (layout.auto, layout.anchor.corner_index()) {
                (true, Some(corner)) => match place_corner(corner, size[0], size[1]) {
                    Some(at) => at,
                    None => continue,
                },
                _ => draw::position(&layout, size, screen),
            };
            paint(shapes, at);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::essentials_store::tests::TempDir;

    #[test]
    fn le_hud_suit_l_activation_et_les_coins() {
        let d = TempDir::new("evenements-hud");
        let dir = d.0.clone();
        frame(|| dir.clone(), 16.0);
        let measure = |s: &str| s.len() as f32 * 6.0;
        let mut corners = Vec::new();
        let mut painted = Vec::new();
        let draw_once = |corners: &mut Vec<usize>, painted: &mut Vec<String>| {
            hud(
                [480.0, 270.0],
                &measure,
                &mut |c, _, _| {
                    corners.push(c);
                    Some([300.0, 2.0])
                },
                &mut |shapes, at| painted.push(format!("{} @ {at:?}", shapes.text_of(0))),
            );
        };
        draw_once(&mut corners, &mut painted);
        assert_eq!(corners, [1], "le témoin s'empile en haut à droite");
        assert_eq!(painted, ["Essentials 0.1.0 - default - 1/1 @ [300.0, 2.0]"]);

        assert!(apply(|| dir.clone(), Action::Toggle(ModuleId::Status)));
        draw_once(&mut corners, &mut painted);
        assert_eq!(painted.len(), 1, "désactivé : plus rien n'est dessiné");
    }

    #[test]
    fn un_coin_pris_cache_le_widget() {
        let d = TempDir::new("evenements-coin");
        let dir = d.0.clone();
        frame(|| dir.clone(), 16.0);
        let mut painted = 0;
        hud([480.0, 270.0], &|_| 10.0, &mut |_, _, _| None, &mut |_, _| painted += 1);
        assert_eq!(painted, 0);
    }

    #[test]
    fn avant_la_premiere_frame_rien_n_est_dessine_ni_lu() {
        let mut painted = 0;
        hud([480.0, 270.0], &|_| 10.0, &mut |_, _, _| Some([0.0, 0.0]), &mut |_, _| painted += 1);
        assert_eq!(painted, 0);
    }

    #[test]
    fn les_messages_attendent_le_monde() {
        let d = TempDir::new("evenements-messages");
        std::fs::write(d.0.join("active"), "Interdit").unwrap();
        let dir = d.0.clone();
        assert_eq!(tick(|| dir.clone(), false), None, "pas encore dans un monde");
        let m = tick(|| dir.clone(), true).expect("un avertissement");
        assert!(m.contains("profil actif invalide"));
        assert_eq!(tick(|| dir.clone(), true), None);
    }
}
