# Performance

## Rules

- No disk reads or writes on the game (render) thread: the "Essentials I/O" thread handles them.
- Zero allocations per frame, tick or key press in steady state.
- A disabled module is never called.
- Nothing is optimized without a before/after measurement using the same protocol.

The first three rules are checked by every `cargo test` run (`core_events.rs`, tests
`zero_allocation_par_frame`, `un_module_desactive_n_est_jamais_appele`,
`aucune_operation_disque_sur_le_thread_du_jeu`).

## Protocol (`bench/`)

`cargo run --release --manifest-path bench/Cargo.toml` compiles the modules **outside test mode**
(as in the game), with an allocator that counts the current thread's allocations:

| | Measurement |
|---|---|
| P1 | frame + HUD in steady state, 2,000,000 frames: ns and allocations per frame, status module enabled then disabled |
| P2 | tick: ns and allocations per tick |
| P3 | config change, then frames until the file is written: duration of the worst frame (20 runs) |
| P4 | first frame of a fresh thread with an existing config file (20 runs) |

The "game" is simulated (dummy text measurement and painting): only our own code is measured.
The actual drawing is done by Horizon (`native_fill`, `native_text`), as for its official mods.

## Results (Windows 11, i5-11400H)

Commit 4 (`1e4d857`) versus the `perf/io-thread` branch, 3 alternating passes, medians:

| Measurement | Before | After |
|---|---|---|
| P1 frame + HUD, status module enabled | 53-57 ns, 0 alloc | 51-58 ns, 0 alloc |
| P1 frame + HUD, status module disabled | 43-50 ns, 0 alloc | 44-46 ns, 0 alloc |
| P2 tick | 21-22 ns, 0 alloc | 21-23 ns, 0 alloc |
| P3 worst frame during a save | **1,484-1,581 µs** | **18-23 µs** |
| P4 first frame (loading) | **339-385 µs** | **40-43 µs** |

At 1,250 FPS a frame lasts 800 µs: our code takes less than 0.01 % of it.

## Measured startup (Nov 17, Essentials Dev instance, same machine)

Command `horizon-launcher play --instance essentials-dev`, from running the command to the
"first frame after" line of the log (main menu); memory read 2 s after "rest-ready";
clean exit through the window.

| Build | Launch | Play → process | Play → menu | Memory (working set / private) |
|---|---|---|---|---|
| Regular Nightly | 2nd | 5.5 s | **7.4 s** | 552 / 965 MB |
| Native without Essentials (official fps-overlay) | 2nd | 6.0 s | 13.5 s ¹ | 916 / 1,144 MB |
| Essentials commit 4 | 2nd | 5.5 s | **7.4 s** | 895 / 1,129 MB |
| Essentials `perf/io-thread` | 1st (cache rebuilt) | 18.1 s | 60.2 s | 963 / 2,295 MB |
| Essentials `perf/io-thread` | 2nd / 3rd | 5.9 / 5.7 s | 14.2 ¹ / **7.7 s** | 963-1,026 / 1,410-1,657 MB |

¹ A 5.7-6.2 s wait in Horizon's "first frame after" phase, seen with and without Essentials.

- On a warm start, a build with Essentials starts like regular Horizon (7.4-7.7 s, including ~5.5 s of
  preparation by the launcher before the game starts).
- The first launch of a new build (≈ 60 s) and switching from one version to another in the same
  instance (≈ 18 s, the launcher copies the game files again) are down to the launcher.
- A native build uses ~350 MB more than a Nightly, **with or without Essentials**: this comes from the
  launcher's native compilation (`fast` profile, 147 MB binary versus 107 MB), not from our code.

## Game startup

Measured in the game logs (`logs/latest.log`) and the launcher reports (`benchmarks/`).

| Launch | "core-ready" | Resources loaded | Play → first frame |
|---|---|---|---|
| Regular Nightly, later launches | 0.7 s | 0.9 s | 5.0-8.0 s |
| Native build, **1st** launch (with or without Essentials) | 28-34 s | 55-60 s | 35-40 s |
| Native build, 2nd launch (official fps-overlay) | 0.7 s | 1.2 s | 5.1 s |

Cause, in Horizon's code (`horizon-core/src/prepared_files.rs`): every native build creates
a new version folder (a 1.3 GB copy). The copied `horizon-data.bin` cache contains the path of the
old folder, so it is rejected; the first launch then reads ~22,000 files one by one (each scanned
by Windows Defender) and rewrites the cache. Later launches are as fast as regular Horizon.
Our code plays no part in this: it does nothing before the first frame, then asks the I/O thread
to read its configuration (~40 µs).
