# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
cargo build               # compile
cargo run                 # run the GUI app
cargo test                # all tests (unit + integration)
cargo test <name>         # run a single test by name substring
cargo fmt                 # format code
cargo clippy --all-targets -- -D warnings  # lint (CI-enforced, no warnings allowed)
cargo audit               # dependency vulnerability scan
cargo deny check          # license / source policy check
```

## Architecture

The crate is split into a **library** (`src/lib.rs`) and a **binary** (`src/main.rs`).

- `game_of_life` (lib) — exposes only `sim::` so integration tests in `tests/` can reach the simulation without pulling in GPU/window dependencies.
- `src/sim/mod.rs` — `Grid` (toroidal, flat `Vec<bool>`, Conway rules via `tick()`). **Must never import `winit`, `wgpu`, or `egui`** — enforced by a CI grep check.
- `src/sim/session.rs` — `GameSession` wraps a `Grid` and drives the game loop: advances one tick at a time, detects cycles via a `HashSet<u64>` of grid hashes, enforces a step cap, and tracks `population_history`. `GameResult` is the return value of `advance()`.
- `src/render/mod.rs` — `Renderer` (wgpu surface + egui_wgpu backend) and `draw_grid()` (egui `Painter`-based grid drawing). Returns `(grid_origin, cell_size)` so mouse clicks can be mapped to cells.
- `src/app.rs` — `App` / `AppInner` implements `winit::ApplicationHandler`. Owns the grid, phase state, and session. `AppPhase` is `Editing | Running | Ended(EndState)`. The game loop is driven from `about_to_wait` using `Instant`-based pacing. UI mutations from the egui closure are collected into local variables and applied after the closure to avoid borrow conflicts.
- `src/input.rs` — maps raw `KeyCode` to `AppAction` (Quit, ToggleFullscreen, ExitFullscreen).

**Key invariant:** `screen_to_cell()` in `app.rs` is a free function (not a method) so it can be unit-tested without any window or GPU state.

**Score formula:** `(unique_states_seen + 1)^2` — more distinct states before cycle/cap = higher score.

## CI pipeline

Lint → Build → Test → Audit (each step depends on the previous). The `sim purity check` in the Lint job greps `src/sim/` for `winit::`, `wgpu::`, and `egui::` imports and fails if found.
