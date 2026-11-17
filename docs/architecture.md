# Architecture

## Principles

1. **A single bundle** `mixins/horizon_essentials/`, mod id `horizon_essentials`, several `[[module]]`s
   (visibility `pub`, so no "dead code" warning for what the game does not call yet).
   Mixin bundles have no dependency system: a Core in a separate bundle would break the build
   whenever it was missing.
2. **Modules = plain Rust** (no game types); **helpers = glue** of 5 to 40 lines: read the game,
   call the Core, paint. Modules are tested with `cargo test`, without Horizon.
3. **One helper per hook**, going through `essentials_events` (controlled order, one injection per hook).
4. **The UI never changes anything itself**: it produces `Action`s, which only the Core applies.
5. **The game thread never touches the disk**: loading and saving go through the
   "Essentials I/O" thread.
6. **Zero allocations per frame** in steady state; a disabled module is never called.
7. Prefixed files rather than subfolders (the bundle format forbids `mod` and `include!`):
   `core_` (Core), `ui_` (user interface), `mod_` (modules), `profiles_` (profiles), `h_` (helpers).

## Files

| File | Module in the game | Role |
|---|---|---|
| `core_types.rs` | `crate::game::essentials_types` | constants, anchors, `Action`, `Shapes` (reused shapes), `Module` trait |
| `core_registry.rs` | `essentials_registry` | `ModuleId`, `MODULES` table, `Modules` + `get_mut` (the only way into a module) |
| `core_config.rs` | `essentials_config` | versioned model, resilient field-by-field parsing, `Resolved` for rendering |
| `core_store.rs` | `essentials_store` | files (`active`, `profiles/<name>.json`), atomic writes, `.bak`, I/O thread |
| `core_state.rs` | `essentials_state` | single state per thread, asynchronous loading, deferred saving, shutdown |
| `core_events.rs` | `essentials_events` | frame / tick / key / apply / hud / shutdown: the helpers' only entry point |
| `ui_draw.rs` | `essentials_draw` | automatic placement (anchors) or free placement (fractions of the free space) |
| `mod_status.rs` | `essentials_status` | status module |
| `h_list.rs` | hook `native_mods` | entry in Options > Mods |
| `h_frame.rs` | hook `native_frame` | elapsed time, replies from the I/O thread |
| `h_tick.rs` | hook `native_tick` | modules, Core messages in the chat |
| `h_key.rs` | hook `native_key` | keys and mouse buttons |
| `h_hud.rs` | hook `native_hud` | widget drawing (corners through Horizon's `Corners`) |
| `h_stopping.rs` | hook `native_client_stopping` | last change written on exit |

## Adding a module

1. Write `mod_<name>.rs` (implements `Module`), add it to `horizon-mixin.toml` and to `harness/lib.rs`.
2. In `core_registry.rs` only: a `ModuleId` variant, a `MODULES` row, a `Modules` field and a
   `get_mut` branch.

The configuration (default settings), enabling, the HUD and the UI pick it up on their own.

## Flow

```
native_frame ─► events::frame ─► state: I/O replies, deferred save ─► active modules
native_tick  ─► events::tick  ─► active modules; one Core message to the chat
native_key   ─► events::key   ─► active modules
native_hud   ─► events::hud   ─► active widget: cached or rebuilt shapes ─► helper paints
UI           ─► Action ─► events::apply ─► config changed ─► Resolved recomputed, saved after 1 s
shutdown     ─► events::shutdown ─► last change sent, waits for the I/O thread (2 s max)
```

## Configuration

Stored in the instance folder (never in a version folder, so it survives rebuilds and updates):

```
horizon-mods/.horizon-mod-data/horizon_essentials/
  active                     name of the active profile
  profiles/<name>.json       one profile
  profiles/<name>.json.bak   copy of a file that was invalid
```

```json
{ "version": 1,
  "modules": { "status": {
      "on": true,
      "layout": { "anchor": "top_right", "auto": true, "x": 0.0, "y": 0.0, "scale": 100 },
      "style":  { "background": 47, "text": "FFFFFF", "shadow": true } } } }
```

- Resilient parsing: unreadable file → default values; invalid field → only that field gets its
  default value; unknown module → kept; missing module → added; a warning in the chat.
- Atomic write (temporary file, then rename), 1 s after the last change, by the I/O thread;
  a request that arrives during a write is never lost.

## Development order

Step 1 (Core): 1 structure ✅, 2 types/registry/config ✅, 3 storage/state ✅, 4 events/rendering/status ✅,
I/O optimization (branch `perf/io-thread`), 5 module management (Options > Mods, `/essentials`).
Then: escape menu, HUD Editor, profiles, PvP modules.
