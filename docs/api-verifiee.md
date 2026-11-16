# API Horizon vérifiées

Toutes lues dans le code source de **Nightly 2026-11-13 · `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`**
(`crates/horizon-client/src/game/native_hooks.rs`, `NATIVE_HOOKS_API = 415`), la version installée.
Rien ici n'est supposé : si une fonction n'est pas dans ce tableau, elle n'est pas encore vérifiée.

## Hook points (méthodes de `Game`, injectées en `HEAD`)

| Hook | Signature | Usage prévu |
|---|---|---|
| `native_frame` | `(&mut self, elapsed: Duration)` | temps de frame → FPS Statistics |
| `native_tick` | `(&mut self)` | sauvegarde différée, mémoire 1×/s (tester `native_window()`) |
| `native_hud` | `(&mut self, batch: &mut HudBatch, corners: &mut Corners)` | dessin des widgets |
| `native_key` | `(&mut self, key: &str, pressed: bool)` | clics (`key.mouse.left/right`), touches |
| `native_command` | `(…, command: &str, ran: &mut bool)` | `/essentials …` |
| `native_mods` | `(&mut self, mods: &mut Vec<NativeMod>)` | ligne dans Options > Mods |
| `native_settings` | `(&mut self, mod_id: &str, buttons: &mut Vec<String>)` | page de réglages (libellés) |
| `native_settings_click` | `(&mut self, mod_id: &str, button: usize)` | clic sur un bouton |
| `native_screen_render_post` | `(&self, batch: &mut HudBatch, screen: &NativeScreenView<'_>)` — API 78 | dessin de l'éditeur |
| `native_screen_mouse_button_pressed_pre` | `(&self, screen, button: i32, modifiers: u32, canceled: &mut bool)` — API 80 | éditeur |
| `native_screen_mouse_dragged_pre` (v1) | `(&self, screen, button, delta: [f32;2], modifiers, canceled)` | éditeur (la v2 est « pending », à éviter) |
| `native_screen_mouse_scrolled_pre` (v1) | `(&self, screen, scroll: [f64;2], modifiers, canceled)` | éditeur (taille) |

## Helpers

| Helper | Retour / remarque |
|---|---|
| `native_on(id)` | `bool` — mod activé dans Options > Mods |
| `native_window()`, `native_playing()` | `bool` |
| `native_player()` | `Option<NativePlayer>` |
| `native_frame_rate()` | `(u32, u32, u32)` : FPS, frame la plus lente, 1 % low (5 s) |
| `native_clicks()` | `(u32, u32)` : clics attaque / utilisation dernière seconde |
| `native_mapping(name)` | `Option<(bool, String)>` : tenu ?, touche affichée |
| `native_mod_key(id, name, default)` | `String` |
| `native_text_width(text)` | `f32` |
| `native_fill(batch, [x,y,w,h], argb)` | alpha respecté |
| `native_text(batch, text, [x,y], rgb, shadow)` | **texte toujours opaque** (alpha forcé à 0xFF) |
| `native_cursor()` | `[f32; 2]` en pixels GUI — API 20 |
| `native_mod_screen_open(id)` / `_close(id)` / `native_mod_screen()` | écran propre au mod — API 111 |
| `native_setting(id, key)` | **lit un fichier à chaque appel** (`fs::read_to_string`) → à mettre en cache |
| `native_set_setting(id, key, value)` | écrit un fichier |
| `native_data_dir(id)` | `horizon-mods/.horizon-mod-data/<id>/` |
| `native_chat(text)` | message local |

## Hors API stable (code interne public, à isoler dans un seul helper)

| Élément | Où | Usage |
|---|---|---|
| `HudBatch::scale_since(start, cx, cy, sx, sy)` + `batch.vertices.len()` | `horizon-client-render/src/hud.rs` | taille des widgets |
| `batch.width`, `batch.height` | idem (champs publics, cités par la doc des hooks) | positions relatives |
| `horizon_threads::process_memory::usage()` → `Option<[u64; 3]>` (privée, résidente, RAM) | `horizon-threads/src/process_memory.rs` (Windows) ; `resident_bytes()` sous Linux | Memory Display |

## Absents (vérifié)

- Aucun helper pour lire/écrire les options intégrées « PvP & HUD » (`horizon-extras.txt`).
- Aucun helper armure / effets / ping côté client (piste : `native_client_packet`, API 408, lecture seule).
- Aucun choix de police.

## Dépendances disponibles dans le crate du jeu

`serde` 1.0.229 (derive) et `serde_json` 1.0.151 : dépendances directes de `horizon-client`.
