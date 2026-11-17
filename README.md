# Horizon Essentials

A companion extension for [Horizon](https://playhorizon.dev) (Minecraft: Java Edition 26.3 rewritten in Rust):
a native mod (`.hnmod`) that adds what Horizon does not have yet, without redoing what it already has
(crosshair, Hit Color, armor, effects, ping, HUD editor… are already built into Horizon).

Target: **Horizon Nightly 2026-11-13 · `9f42064`** (hook points `NATIVE_HOOKS_API = 415`).

## Planned features

| Feature | What it adds to Horizon | Status |
|---|---|---|
| Essentials Core | Module registry, resilient and persistent configuration, I/O off the render thread | ✅ |
| Status module | A small box that proves the Core is running | ✅ |
| Module management | Options > Mods page, `/essentials` command | in progress |
| Escape menu | Side panel drawn on the pause menu | planned |
| HUD Customizer | Position, size and colors of every widget, drag-and-drop editor | planned |
| HUD Profiles | PvP, Survival, Minimal, Custom | planned |
| FPS Statistics, CPS History, Advanced Keystrokes, Memory Display | PvP modules | planned |

## Repository layout

```
mixins/horizon_essentials/   the mod (a single horizon-mixins bundle)
  horizon-mixin.toml           manifest: modules and hooks
  core_*.rs                    Essentials Core (types, registry, config, storage, state, events)
  ui_*.rs                      user interface (placement; later the menu and the HUD Editor)
  mod_*.rs                     modules (one file per module)
  h_*.rs                       helpers: the glue with Horizon's hook points, one per hook
harness/lib.rs               test harness: `cargo test` without compiling Horizon
bench/                       performance measurement protocol
compatibility.json           tested Horizon and horizon-mixins commits
docs/                        architecture, verified APIs, environment, performance
```

## Build and test

```bash
cargo test                                               # logic (Windows or Linux), a few seconds
cargo clippy --all-targets -- -D warnings
cargo run --release --manifest-path bench/Cargo.toml     # performance measurements
```

Checking against Horizon's source and building the game: see [docs/environnement.md](docs/environnement.md).

## Documentation

- [Architecture](docs/architecture.md)
- [Verified Horizon APIs](docs/api-verifiee.md)
- [Development environment](docs/environnement.md)
- [Performance](docs/performance.md)

Personal and learning project, not affiliated with Horizon, Mojang or Microsoft. MIT License.
