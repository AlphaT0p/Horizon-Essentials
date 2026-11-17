# Verified Horizon APIs

All read in the source code of **Nightly 2026-11-13 · `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`**
(`crates/horizon-client/src/game/native_hooks.rs`, `NATIVE_HOOKS_API = 415`), the installed version.
Nothing here is assumed: if a function is not in these tables, it has not been verified yet.

## Hook points (`Game` methods, injected at `HEAD`)

| Hook | Signature | Planned use |
|---|---|---|
| `native_frame` | `(&mut self, elapsed: Duration)` | frame time → FPS Statistics |
| `native_tick` | `(&mut self)` | deferred saving, memory once per second (check `native_window()`) |
| `native_hud` | `(&mut self, batch: &mut HudBatch, corners: &mut Corners)` | widget drawing |
| `native_key` | `(&mut self, key: &str, pressed: bool)` | clicks (`key.mouse.left/right`), keys |
| `native_command` | `(…, command: &str, ran: &mut bool)` | `/essentials …` |
| `native_mods` | `(&mut self, mods: &mut Vec<NativeMod>)` | entry in Options > Mods |
| `native_settings` | `(&mut self, mod_id: &str, buttons: &mut Vec<String>)` | settings page (labels) |
| `native_settings_click` | `(&mut self, mod_id: &str, button: usize)` | click on a button |
| `native_screen_render_post` | `(&self, batch: &mut HudBatch, screen: &NativeScreenView<'_>)` — API 78 | editor drawing |
| `native_screen_mouse_button_pressed_pre` | `(&self, screen, button: i32, modifiers: u32, canceled: &mut bool)` — API 80 | editor |
| `native_screen_mouse_dragged_pre` (v1) | `(&self, screen, button, delta: [f32;2], modifiers, canceled)` | editor (v2 is "pending", avoid it) |
| `native_screen_mouse_scrolled_pre` (v1) | `(&self, screen, scroll: [f64;2], modifiers, canceled)` | editor (size) |
| `native_screen_key_pressed_pre` | `(&self, screen, key: &str, repeat: bool, modifiers: u32, canceled: &mut bool)` | escape menu |
| `native_client_stopping` | `(&self)` — "Once before this client's final save/exit. A mod can flush and join its worker here." Called by `save_for_exit` (`game/world_session.rs`), once, window only | save on exit |

`NativeScreenView` (`horizon-client/src/mod_api.rs`): `id` (stable: `paused` for the escape menu,
`inventory`, `options`, `title`…), `menu`, `title`, `rect`, `slots`, `mouse`, `partial_tick`.
`native_mod_screen_open` only opens a mod's screen from the game with no screen open (`mod_screen_opens`):
impossible from the escape menu, hence the panel drawn on that menu.

## Helpers

| Helper | Return / note |
|---|---|
| `native_on(id)` | `bool` — mod enabled in Options > Mods |
| `native_window()`, `native_playing()` | `bool` |
| `native_player()` | `Option<NativePlayer>` |
| `native_frame_rate()` | `(u32, u32, u32)`: FPS, slowest frame, 1 % low (5 s) |
| `native_clicks()` | `(u32, u32)`: attack / use clicks in the last second |
| `native_mapping(name)` | `Option<(bool, String)>`: held?, displayed key |
| `native_mod_key(id, name, default)` | `String` |
| `native_text_width(text)` | `f32` |
| `native_fill(batch, [x,y,w,h], argb)` | alpha respected |
| `native_text(batch, text, [x,y], rgb, shadow)` | **text always opaque** (alpha forced to 0xFF) |
| `native_cursor()` | `[f32; 2]` in GUI pixels — API 20 |
| `native_mod_screen_open(id)` / `_close(id)` / `native_mod_screen()` | the mod's own screen — API 111 |
| `native_setting(id, key)` | **reads a file on every call** (`fs::read_to_string`) → must be cached |
| `native_set_setting(id, key, value)` | writes a file |
| `native_data_dir(id)` | `horizon-mods/.horizon-mod-data/<id>/` |
| `native_chat(text)` | local message |

## Outside the stable API (public internal code, to be isolated in a single helper)

| Item | Where | Use |
|---|---|---|
| `HudBatch::scale_since(start, cx, cy, sx, sy)` + `batch.vertices.len()` | `horizon-client-render/src/hud.rs` | widget size |
| `batch.width`, `batch.height` | same (public fields, mentioned by the hooks' docs) | relative positions |
| `horizon_threads::process_memory::usage()` → `Option<[u64; 3]>` (private, resident, RAM) | `horizon-threads/src/process_memory.rs` (Windows); `resident_bytes()` on Linux | Memory Display |

## Missing (verified)

- No helper to read or write the built-in "PvP & HUD" options (`horizon-extras.txt`).
- No client-side armor / effects / ping helper (lead: `native_client_packet`, API 408, read-only).
- No font choice.

## Dependencies available in the game crate

`serde` 1.0.229 (derive) and `serde_json` 1.0.151: direct dependencies of `horizon-client`.
