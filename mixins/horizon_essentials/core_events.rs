// Horizon Essentials : la répartition des événements (crate::game::essentials_events).
//
// Les helpers h_*.rs appellent ces fonctions et rien d'autre du Core. Elles ne
// connaissent pas le jeu : ce qu'il faut du jeu leur est passé en paramètre
// (le dossier de données, une mesure de texte, de quoi placer et peindre).

use std::path::PathBuf;

use crate::game::essentials_config::Resolved;
use crate::game::essentials_draw as draw;
use crate::game::essentials_registry::{COUNT, ModuleId};
use crate::game::essentials_state::{self as state};
use crate::game::essentials_types::{Action, Shapes, TickCtx, WidgetCtx};

/// Une frame (sur la fenêtre). À la première, demande la lecture de la configuration
/// au thread d'E/S ; ensuite, relit ses réponses sans attendre. Aucun accès disque ici.
pub fn frame(dir: impl FnOnce() -> PathBuf, elapsed_ms: f32) {
    state::with(dir, |s| {
        s.advance(elapsed_ms);
        if !s.loaded {
            return;
        }
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
        if s.loaded {
            let ctx = TickCtx { tick: s.tick, in_world };
            for id in ModuleId::all() {
                if s.resolved.on[id.index()] {
                    s.modules.get_mut(id).on_tick(&ctx);
                }
            }
        }
        if in_world { s.next_message() } else { None }
    })
    .flatten()
}

/// Une touche ou un bouton de souris, par son nom vanilla.
pub fn key(dir: impl FnOnce() -> PathBuf, key: &str, down: bool) {
    state::with(dir, |s| {
        if !s.loaded {
            return;
        }
        for id in ModuleId::all() {
            if s.resolved.on[id.index()] {
                s.modules.get_mut(id).on_key(key, down);
            }
        }
    });
}

/// Fermeture du jeu : le dernier changement est écrit avant la sortie (le thread
/// d'E/S a au plus `SHUTDOWN_TIMEOUT` pour finir). Ne démarre rien si le Core n'a
/// jamais tourné.
pub fn shutdown() {
    state::with_existing(|s| s.shutdown());
}

/// Applique une action venue de l'interface ou d'une commande.
pub fn apply(dir: impl FnOnce() -> PathBuf, action: Action) -> bool {
    state::with(dir, |s| s.apply(action)).unwrap_or(false)
}

/// Ce que l'interface peut lire du Core, en lecture seule : la configuration
/// résolue et le profil actif. La page de réglages et la commande lisent toutes
/// deux cet instantané : elles montrent toujours le même état.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot<'a> {
    pub resolved: &'a Resolved,
    pub profile: &'a str,
}

/// Donne à `f` l'instantané du Core, ou None tant que la configuration est en
/// cours de chargement (l'interface affiche alors « Chargement… » et ne propose
/// aucune modification). Le résultat est None si l'état est déjà emprunté.
pub fn view<R>(dir: impl FnOnce() -> PathBuf, f: impl FnOnce(Option<Snapshot<'_>>) -> R) -> Option<R> {
    state::with(dir, |s| {
        let snapshot = s.loaded.then_some(Snapshot { resolved: &s.resolved, profile: &s.profile });
        f(snapshot)
    })
}

/// Dessine les widgets actifs.
/// - `screen` : la taille de l'écran en pixels GUI ;
/// - `measure` : la largeur d'un texte ;
/// - `place_corner(coin, w, h)` : la place dans un coin (0 haut gauche, 1 haut droit,
///   2 bas gauche, 3 bas droit), ou None si le coin est pris (écran F3) ;
/// - `paint(formes, position)` : peint les formes.
///
/// Ne lit jamais le disque : tant que la configuration n'est pas chargée, rien
/// n'est dessiné (pas de widget qui apparaît puis disparaît).
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
    use std::time::Duration;

    const WAIT: Duration = Duration::from_secs(5);

    /// Première frame, puis attente de la fin du chargement asynchrone.
    fn started(dir: &std::path::Path) {
        frame(|| dir.to_path_buf(), 16.0);
        assert_eq!(state::with_existing(|s| s.wait_idle(WAIT)), Some(true));
    }

    fn draw(painted: &mut u64) {
        hud(
            [480.0, 270.0],
            &|s: &str| s.len() as f32 * 6.0,
            &mut |_, _, _| Some([300.0, 2.0]),
            &mut |shapes, at| *painted += shapes.ops().len() as u64 + at[0] as u64,
        );
    }

    #[test]
    fn le_hud_suit_l_activation_et_les_coins() {
        let d = TempDir::new("evenements-hud");
        let dir = d.0.clone();
        started(&dir);
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
        started(&dir);
        let mut painted = 0;
        hud([480.0, 270.0], &|_| 10.0, &mut |_, _, _| None, &mut |_, _| painted += 1);
        assert_eq!(painted, 0);
    }

    #[test]
    fn rien_n_est_dessine_avant_la_fin_du_chargement() {
        let mut painted = 0;
        draw(&mut painted);
        assert_eq!(painted, 0, "avant la première frame");
        let d = TempDir::new("evenements-attente");
        let dir = d.0.clone();
        frame(|| dir.clone(), 16.0);
        let loaded = state::with_existing(|s| s.loaded).unwrap();
        draw(&mut painted);
        assert_eq!(painted > 0, loaded, "le HUD n'affiche rien tant que la configuration n'est pas lue");
        state::with_existing(|s| s.wait_idle(WAIT));
        draw(&mut painted);
        assert!(painted > 0, "puis le témoin apparaît");
    }

    #[test]
    fn les_messages_attendent_le_monde() {
        let d = TempDir::new("evenements-messages");
        std::fs::write(d.0.join("active"), "Interdit").unwrap();
        let dir = d.0.clone();
        started(&dir);
        assert_eq!(tick(|| dir.clone(), false), None, "pas encore dans un monde");
        let m = tick(|| dir.clone(), true).expect("un avertissement");
        assert!(m.contains("profil actif invalide"));
        assert_eq!(tick(|| dir.clone(), true), None);
    }

    #[test]
    fn view_ne_donne_rien_pendant_le_chargement() {
        let d = TempDir::new("evenements-view");
        let dir = d.0.clone();
        // Premier accès : la lecture vient d'être demandée au thread d'E/S.
        let first = view(|| dir.clone(), |snap| snap.is_some()).unwrap();
        let loaded = state::with_existing(|s| s.loaded).unwrap();
        assert_eq!(first, loaded, "pas d'instantané tant que la configuration n'est pas lue");
        state::with_existing(|s| s.wait_idle(WAIT));
        let (on, profile) = view(|| dir.clone(), |snap| {
            let snap = snap.expect("chargée");
            (snap.resolved.on[ModuleId::Status.index()], snap.profile.to_owned())
        })
        .unwrap();
        assert!(on);
        assert_eq!(profile, "default");
    }

    // ── Non-régression des performances ─────────────────────────────────────

    #[test]
    fn zero_allocation_par_frame() {
        let d = TempDir::new("evenements-alloc");
        let dir = d.0.clone();
        started(&dir);
        let mut painted = 0;
        for _ in 0..10 {
            frame(|| dir.clone(), 1.0);
            draw(&mut painted);
        }
        let before = crate::allocations();
        for _ in 0..10_000 {
            frame(|| dir.clone(), 1.0);
            draw(&mut painted);
            let _ = tick(|| dir.clone(), true);
            key(|| dir.clone(), "key.mouse.left", true);
        }
        assert_eq!(crate::allocations() - before, 0, "une frame, un tick ou une touche a alloué");
        assert!(painted > 0);
    }

    #[test]
    fn un_module_desactive_n_est_jamais_appele() {
        use crate::game::essentials_registry::calls;
        let d = TempDir::new("evenements-desactive");
        let dir = d.0.clone();
        started(&dir);
        assert!(apply(|| dir.clone(), Action::Set(ModuleId::Status, false)));
        let before = calls(ModuleId::Status);
        let mut painted = 0;
        for _ in 0..1000 {
            frame(|| dir.clone(), 1.0);
            draw(&mut painted);
            let _ = tick(|| dir.clone(), true);
            key(|| dir.clone(), "key.keyboard.w", true);
        }
        assert_eq!(calls(ModuleId::Status), before, "le module désactivé a été appelé");
        assert_eq!(painted, 0);
        assert!(apply(|| dir.clone(), Action::Set(ModuleId::Status, true)));
        draw(&mut painted);
        assert!(calls(ModuleId::Status) > before, "réactivé, il est de nouveau appelé");
    }

    #[test]
    fn aucune_operation_disque_sur_le_thread_du_jeu() {
        use crate::game::essentials_store::DISK_OPS;
        let d = TempDir::new("evenements-disque");
        let dir = d.0.clone();
        let before = DISK_OPS.with(|c| c.get());
        started(&dir);
        let mut painted = 0;
        // Un changement puis 2 s de frames : la sauvegarde a lieu pendant la boucle.
        assert!(apply(|| dir.clone(), Action::Toggle(ModuleId::Status)));
        for _ in 0..2000 {
            frame(|| dir.clone(), 1.0);
            draw(&mut painted);
        }
        assert_eq!(state::with_existing(|s| s.wait_idle(WAIT)), Some(true));
        assert_eq!(DISK_OPS.with(|c| c.get()), before, "le thread du jeu a touché le disque");
        let file = crate::game::essentials_store::profile_path(&dir, "default");
        assert!(file.exists(), "la sauvegarde a bien été faite, par le thread d'E/S");
    }
}
