# Development environment (Windows 11)

Set up and tested on 2026-11-16.

| Tool | Version | Role |
|---|---|---|
| Rust (Windows) | 1.100.0, `stable-x86_64-pc-windows-gnu` | `cargo test` of the modules. GNU: no Visual Studio needed |
| WSL Ubuntu 24.04 | Python 3.12, rsync 3.2.7, git 2.43, Rust 1.100.0 | `native.py` (checking against Horizon's source) |
| `~/horizon-mods` (in Ubuntu) | clone of `horizon-rust/horizon-mods` | official tools `native/tools/native.py` |
| Horizon Launcher | 0.2.69 (`%LOCALAPPDATA%\Programs\Horizon Launcher\horizon-launcher.exe`, which is also the CLI) | builds the game with the mod |
| Launcher's Rust | 1.99.1 windows-gnu (`horizon-launcher rust install`) | used by `native-build` |
| Installed Horizon | Nightly 2026-11-13 · `9f42064` (`NATIVE_HOOKS_API = 415`) | target |
| Test instance | `essentials-dev` ("Essentials Dev") | receives the builds; the main "Horizon" instance is never touched |

## Development loop

1. `cargo test` and `cargo clippy --all-targets -- -D warnings` (seconds).
2. `native.py check` in Ubuntu (1 to 3 min): the project is linked there with a symbolic link
   `~/horizon-mods/native/horizon-essentials -> /mnt/c/Users/<you>/Desktop/Horizon-Essentials`.
3. The launcher's `native-build` into the `essentials-dev` instance (about 14 min: 4 min applying
   the mixins, 5 min compiling, 4.5 min installing a 1.3 GB copy).
4. After a build, **fully close the launcher window** and reopen it: a window opened before the
   build keeps the old version list and starts the old binary.
5. The **first** launch of a new build takes about 1 minute (data cache rebuilt, see
   [performance.md](performance.md)); later launches about 5 s.

## Pitfalls encountered

- Paths that are too long (> 260 characters) break `cargo` on Windows: work in a folder with a short path.
- `native.py` expects a folder name under `native/`; compiling an external mod requires placing (or linking) it in `~/horizon-mods/native/`.
- Check the result of `native.py check` by its `"check": "ok"` line: the exit code shown by some wrapper scripts can be misleading.

## Commands

```bash
# Module tests (Windows, at the project root)
cargo test

# Check against Horizon (Ubuntu)
wsl -d Ubuntu-24.04 -u root -- bash -lc ". ~/.cargo/env; cd ~/horizon-mods && python3 native/tools/native.py check <mod> --horizon 9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0"

# Build the game with the mod, in the test instance (Windows)
"%LOCALAPPDATA%\Programs\Horizon Launcher\horizon-launcher.exe" native-build <mixins\... folder> --instance essentials-dev --trust-native-code
```

## Step 0 results (official `fps-overlay` mod, unmodified)

| Test | Result | Duration |
|---|---|---|
| `cargo test` Windows | ✅ 1/1 | 1.5 s |
| `cargo test` Ubuntu | ✅ 1/1 | 0.5 s |
| `native.py check` on 9f42064 | ✅ `"check": "ok"` | 5 min 12 s (first run) |
| Launcher `native-build`, performance profile, `essentials-dev` instance | ✅ `native-9f42064-33ffd7ce9a19 built and selected for its instance` | 24 min 48 s (first build) |
| In-game launch and counter display | ⏳ not tested yet (Play to be run on "Essentials Dev") | |

After a build, the launcher selects the built instance for Play: switch back to the main instance with
`horizon-launcher instance select horizon`.
