# Resuming development

State on 11/17/2026. Development paused in the middle of **commit 5** (module management):
code finished and tested automatically, **no in-game test done yet**.

## Branches

| Branch | Role |
|---|---|
| `main` (= `origin/main` on GitHub) | Core 1 to 4, I/O optimization, docs and bench, all in English — everything that is pushed |
| `perf/io-thread` | Kept (already in `main` through a fast-forward) |
| **`feat/gestion-modules`** | **Commit 5, local, not pushed** — resume here |

Stash kept (do not delete without agreement): `stash@{0}` "commit 5 en attente : page de réglages et
/essentials" — the 5 original files, also present (identical) in commit `18449b3`.

## Commit 5 commits (`main..feat/gestion-modules`)

| Commit | Content |
|---|---|
| `18449b3` (1/n) | Restore the 5 files set aside (`git stash apply`, byte-for-byte identical) |
| `45207c7` (2/n) | `essentials_events::view`: read-only snapshot of the Core, None while loading |
| `e5fa772` (3/n) | Options > Mods page and `/essentials` wired to the Core; "Loading the configuration…"; `ui_settings` / `ui_command` modules and `native_settings`, `native_settings_click`, `native_command` hooks declared |
| `d82c898` (4/n) | `/essentials reset` → warning, then `/essentials reset confirm` within 15 s; `reset <module>`; "Confirm (15 s)" on the page |
| `dc8567c` (5/n) | `harness/integration.rs`: page ↔ command ↔ Core tests with the real I/O thread |
| translation | Code comments, in-game text, tests and docs translated to English (no behavior change) |
| `9039667` | `cargo fmt` (formatting only) |
| module management | `/essentials` = help; `enable` / `disable <module>` (`on` / `off` kept); `list` with descriptions; clear errors (missing module, too many arguments); `native_commands` helper so the chat knows `/essentials`; tests for registry, restart persistence, disabled module not run, incorrect commands |

## Passed tests (automatic only)

- `cargo fmt --check`: ok.
- `cargo test`: **75/75**, three identical runs.
- `cargo clippy --all-targets -- -D warnings`: ok.
- `native.py check` on Horizon 9f42064: `"check": "ok"`.
- Benchmark (`cargo run --release --manifest-path bench/Cargo.toml`): frame + HUD 57 ns and 0 allocations,
  worst frame during a save 21 µs — unchanged since the optimization.

## Installed build

- **Essentials Dev** instance → `native-9f42064-ad4e79711d7f` (commit `dc8567c`, mod `horizon_essentials 0.1.0`),
  built on 11/17 in 15 min 2 s, performance profile. **This build predates the translation and the
  module-management changes**: a new build is needed before the in-game tests.
- Main **Horizon** instance: unchanged, `native-9f42064-b8ab2b688772`.

Before playing: **fully close the launcher and reopen it** (a window opened before the build starts
the old binary). The **first launch** of a new build takes about 1 minute (Horizon's data cache is
rebuilt), later ones about 7 s.

## In-game tests still to do (commit 5)

In Essentials Dev, in a world:

1. Options > Mods > Horizon Essentials > Settings: the page shows ("Essentials Status: ON",
   "Reload configuration", "Reset everything").
2. Click the status button: OFF, the box at the top right disappears; click again: it comes back.
3. `/essentials` and `/essentials help`: help; `/essentials status`; `/essentials list`: each module with its state and description. Typing `/essentials` is not shown in red and Tab completes it.
4. `/essentials disable status`, `enable status`, `toggle status`: the box follows at once, and so does the page.
5. `/essentials disable radar`: "Unknown module"; `/essentials enable`: "Missing module name"; `/essentials enable status now`: "Too many arguments"; `/essentials dance`: "Unknown command" and the help.
6. `/essentials reset`: warning; `/essentials reset confirm` within 15 s: everything back to defaults;
   after 15 s: refused. Same with the page's two clicks.
7. `/essentials reload` after editing `profiles/default.json` by hand.
8. Close the game right after a change, start it again: the change is kept (save on exit).
9. Rebuild (new build), start again: the configuration is kept.
10. F1 and F3: the box disappears.
11. Comparable FPS with the mod enabled and disabled (Options > Mods).

Configuration file: `%APPDATA%\horizon\instances\essentials-dev\horizon-mods\.horizon-mod-data\horizon_essentials\`.

## Unfinished work

- **In-game validation of commit 5** (list above), then any fixes.
- **Push** of `feat/gestion-modules`: only after in-game validation and explicit agreement.
- **"Essentials Center" UI**: proposal presented (module list with categories, screen opened by a key
  in game, compact panel on the escape menu), **awaiting validation, nothing coded**.
- Next (validated): first modules CPS Counter, Keystrokes, Zoom, FPS Counter — first checking what
  Horizon's official Zoom and FPS Counter mods already do, so nothing is duplicated.
- Deliberately not started: Horizon Graphics, Horizon Visuals.

## Environment

- Project: `Desktop\Horizon-Essentials`; Ubuntu (WSL) with the link `~/horizon-mods/native/horizon-essentials`
  to this folder; commands in [environnement.md](environnement.md).
- Check a `native.py check` by its `"check": "ok"` line (the exit code shown by some scripts can be
  misleading).
- Temporary worktrees (`he-commit4`, `he-docs`) may exist in the session's temporary folder: they are no
  longer used; if they disappear with that folder, `git worktree prune` cleans up the reference.
