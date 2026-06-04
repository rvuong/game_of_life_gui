# Initial Bootstrap Plan — Game of Life

> Status: DRAFT — bootstrap/skeleton specification
> Author: Lead Developer
> Date: 2026-05-06
> Scope: Technology selection, architecture, testing strategy, portability roadmap. **No game rules implementation in this document.**

---

## 1. Overview

This document is the bootstrap specification for a GUI implementation of Conway's Game of Life. The deliverable described here is the **project skeleton and architectural foundation**, not the game logic itself. The game rules will be specified in a follow-up document.

The product target is a desktop GUI application that:

- Opens in its own window and can be closed cleanly.
- Supports both windowed and fullscreen modes (toggleable at runtime).
- Accepts mouse and keyboard input.
- Runs on Linux (Ubuntu) as the primary first-class platform.
- Is engineered from day one to extend cleanly to Windows, macOS, smartphones (iOS / Android), and the web (WebAssembly).

The plan optimizes for two often-competing axes: **portability across heterogeneous platforms** and **a clean separation between simulation logic and presentation**. Every architectural decision below ladders up to those two axes.

---

## 2. Goals & Non-Goals

### 2.1 Goals (in scope of bootstrap)

- Select a language, UI framework, and build tooling.
- Define a layered architecture with strict boundaries between domain (simulation), application (orchestration), and presentation (rendering, input).
- Define the module / package structure of the repository.
- Define a test strategy covering unit, integration, snapshot/visual, and CI.
- Lay out a phased roadmap that delivers a closeable, fullscreen-capable, mouse+keyboard-driven empty window first, then layers in the simulation and the additional platforms.
- Assess WebAssembly feasibility for the chosen stack.

### 2.2 Non-Goals (deferred)

- Implementing Conway's rules, neighborhood iteration, or any cellular automaton variants.
- Defining the user-facing UX (menus, HUD, settings panel design) beyond input modality.
- Persistence of patterns (RLE, plaintext, .lif) — design hooks only.
- Networking, multiplayer, or shared-canvas features.
- Localization / i18n beyond reserving an extensibility seam.
- Accessibility features beyond reserving an extensibility seam (keyboard parity already ensures a baseline).
- Marketing, packaging signing certificates, app store submissions.

---

## 3. Technology Stack

### 3.1 Language Decision: Rust

**Recommendation: Rust.**

Both Go and Rust are credible options for a cross-platform, native GUI game. The decisive factors are (a) the strength of the native game/graphics ecosystem, (b) WebAssembly viability, and (c) mobile portability. On all three, Rust is materially ahead of Go for this specific use case.

#### Comparison

| Criterion | Go | Rust |
|---|---|---|
| Native desktop GUI / game frameworks | Limited; Ebiten is the strongest option; Fyne and Gio exist but are GUI-toolkit-shaped, not game-shaped. | Strong: `macroquad`, `ggez`, `bevy`, `winit`+`wgpu`, `iced`, `egui`. Multiple production-quality choices. |
| WebAssembly story | Two paths: official `GOOS=js GOARCH=wasm` (large binaries, GC overhead) or TinyGo (smaller but with stdlib limitations). Mature but heavy. | First-class: `wasm32-unknown-unknown` target, `wasm-bindgen`, `wgpu` works on WebGPU/WebGL2. Smaller binaries, no GC. |
| Mobile (iOS / Android) | Possible via `gomobile`, but the toolchain is fragile and ecosystem support for GUI framebinding is thin. | Mature: `cargo-ndk` for Android, `cargo-mobile2`, `winit` supports both. Bevy and macroquad ship Android/iOS examples. |
| Rendering performance | GC pauses observable in tight game loops at high cell counts. | No GC; deterministic frame budget; SIMD-friendly. |
| Memory safety | Yes (GC). | Yes (compile-time). |
| Ergonomics | Higher; very fast onboarding. | Lower; borrow checker has a learning curve. |
| Build & cross-compile | Excellent and trivial. | Excellent via `cargo` + `rustup` targets; slightly more setup per target. |
| Compile times | Very fast. | Slow on cold builds, acceptable incrementally. |

#### Why Rust, specifically

1. **Game of Life is rendering-bound at scale.** A GC pause in the middle of a 144 Hz redraw on a million-cell grid is exactly the kind of stutter Rust avoids by construction.
2. **WebAssembly is a stated future target.** Rust's WASM toolchain is the reference implementation for the entire ecosystem; Go's WASM output is significantly larger and bound to either GC overhead (stdlib) or stdlib gaps (TinyGo).
3. **Mobile is a stated future target.** Rust's mobile path (winit + wgpu, or Bevy) is in active production use; Go's mobile GUI path is essentially Ebiten-or-nothing and has known friction.
4. **`wgpu` is the pivotal piece.** A single rendering abstraction that targets Vulkan (Linux), Metal (macOS/iOS), DirectX 12 (Windows), OpenGL ES (Android), and WebGPU/WebGL2 (web) eliminates per-platform rendering rewrites. Go has no equivalent.

#### Dissenting view (kept for record)

If the team has zero Rust experience and a hard delivery deadline, **Go + Ebiten** is a defensible alternative for desktop + web; mobile would be deferred or sacrificed. The rest of this document assumes the Rust path; an appendix could be added later to document the Go variant if the team chooses it.

### 3.2 UI / Game Framework: `winit` + `wgpu` + `egui`

Three layers, each chosen so it can be replaced independently:

- **`winit`** — windowing, event loop, input. Cross-platform (Linux/macOS/Windows/iOS/Android/Web). It owns the window, the fullscreen toggle, and the raw keyboard/mouse events.
- **`wgpu`** — GPU abstraction. Renders the cell grid via a compute or fragment shader, or via instanced quads for small grids. Targets Vulkan / Metal / DX12 / GL / WebGPU / WebGL2 from one codebase.
- **`egui`** — immediate-mode UI for HUD, menus, settings, debug overlay. Integrates cleanly with winit + wgpu via `egui-winit` and `egui-wgpu`. Optional but recommended; the game can ship without it for Phase 1.

#### Why not Bevy?

Bevy is the most popular Rust game engine and would work, but:

- It is an opinionated ECS framework. For Game of Life, the domain is naturally a 2D grid with a single tick function — ECS adds ceremony without payoff.
- It pulls in a large dependency surface, increasing WASM bundle size and compile times.
- It couples the project to Bevy's release cadence and breaking changes.

`winit + wgpu + egui` keeps the engine assembly explicit and the WASM bundle lean. If the project later wants Bevy's scheduler, plugin system, or asset pipeline, the migration is local to the presentation layer because the simulation core has no engine dependency (see §4).

#### Why not `macroquad` or `ggez`?

Both are simpler and would deliver a working desktop prototype faster. They are weaker on:

- Mobile target maturity (especially iOS).
- WebGPU support (they are GL-first).
- Decoupling: they tend to want to own the main loop and the rendering primitives, which makes the simulation/presentation split harder to enforce.

They remain valid for a pivot if `wgpu` proves too heavy for the team.

### 3.3 Build Tooling

- **`cargo`** — primary build tool. Native Rust. Single crate for Phase 1–3; the crate splits into a workspace only when a second binary is concretely needed (Phase 4 WASM entry point).
- **`cargo nextest`** — faster test runner with better isolation than `cargo test`.
- **`cargo clippy`** — linter, run in CI as a gate.
- **`cargo fmt`** — formatter, run in CI as a gate.
- **`cargo deny`** — license / advisory / duplicate-dependency auditing in CI.
- **`cargo audit`** — security advisory check in CI.
- **`wasm-pack`** or **`trunk`** — WASM build and dev server (Phase 4). `trunk` is preferred for an end-to-end web bundle (HTML + WASM + assets).
- **`cargo-ndk`** + **`cargo-mobile2`** — Android / iOS toolchains (Phase 5).

### 3.4 Toolchain Pinning

- Rust edition: **2021** (stable, widely supported; revisit if a 2024-edition feature is concretely needed).
- MSRV (minimum supported Rust version): pin to the latest stable at project start; document in `rust-toolchain.toml`. Re-evaluate quarterly.
- Use `Cargo.lock` committed to the repo (this is an application, not a library).

---

## 4. Software Architecture

### 4.1 Architectural Style

**Layered modules with a pure simulation core.**

The invariant is the same as in any hexagonal design — the simulation must never know about the renderer, the window, or the platform — but it is enforced with Rust's **module visibility** rather than crate boundaries. A crate boundary costs ceremony and forces API surface decisions before the design is stable. Modules cost nothing until a second binary concretely needs to share the code, at which point the split is mechanical: the module folder becomes a library crate, module paths become crate paths, nothing else changes.

The four-crate workspace described in the original plan is the correct Phase 4 end state; it is not the correct Phase 1 start state.

**The one rule that must never be broken:** `src/sim/` must contain zero references to `winit`, `wgpu`, `egui`, or any I/O crate. This is enforced by a CI step (`grep -rn "winit::\|wgpu::\|egui::" src/sim/` must return empty).

### 4.2 Single-Crate Module Layout

One `Cargo.toml`, one crate, four modules with a clean one-way dependency direction.

```
game-of-life/
├── Cargo.toml
├── src/
│   ├── lib.rs          — declares modules; sim is pub so integration tests reach it
│   ├── main.rs         — fn main(): creates EventLoop, runs App
│   ├── app.rs          — App struct, ApplicationHandler impl, state machine
│   ├── input.rs        — AppAction enum, winit keycode → AppAction mapping
│   ├── render/
│   │   └── mod.rs      — Renderer: wgpu surface + egui_wgpu integration
│   └── sim/
│       └── mod.rs      — Grid (Phase 1: placeholder); tick() added in Phase 2
├── tests/
│   └── sim_tests.rs    — integration tests for sim (no GPU, no window)
└── docs/
```

#### `sim/` — Domain (pure, no I/O)

- No `winit`, no `wgpu`, no `egui`, no filesystem, no `log` beyond the `log` facade.
- Exposes a `Grid` type (Phase 1: placeholder struct). Phase 2 adds cell storage and `tick()`.
- This module and its descendants are the only code that will migrate unchanged to WASM and mobile.

#### `app.rs` — Orchestration

- Owns the `App` struct and implements winit's `ApplicationHandler` trait.
- Manages run mode (paused / running / stepping), speed, and the camera/viewport.
- Translates `WindowEvent`s into `AppAction`s (via `input.rs`), then acts on them.
- Owns the `Arc<Window>`, drives the render loop.

#### `render/` — Presentation

- Owns the wgpu surface, device, queue, and egui renderer.
- `Renderer::render()` takes a slice of tessellated egui primitives and a texture delta; knows nothing about `sim`.
- Phase 2 adds a grid draw pass between the clear and the egui overlay.

#### `input.rs` — Input mapping

- `AppAction` enum: `Quit`, `ToggleFullscreen`, and (Phase 2) `TogglePause`, `Step`, `Reset`, `ToggleCellAt`, etc.
- A pure function `AppAction::from_key(KeyCode) -> Option<AppAction>`.
- No state, no winit event loop reference — easy to unit-test.

### 4.3 Module Dependency Direction

```
main.rs ──▶ app.rs ──▶ render/ ──▶ (wgpu, egui)
               │
               ├──▶ input.rs
               │
               └──▶ sim/     (zero platform imports)
```

`sim/` must not import from `app`, `render`, or `input`. Everything else may import from `sim`. Rust's visibility rules (no `pub use` of the wrong direction) and the CI grep check enforce this.

### 4.4 Threading & Async Model

- **Phase 1 (skeleton):** single-threaded. Winit event loop + immediate-mode render.
- **Phase 2 (game logic):** keep simulation on the same thread initially. Reassess if a 1000×1000 grid stutters at 60 Hz.
- **Future:** `app.rs` may dispatch `tick()` to a worker thread (or `wgpu` compute shader) once profiling justifies it. Keep `Grid` `Send + Sync` where possible to leave that door open.
- **WASM constraint:** the browser main thread cannot block. `tick()` must be cheap enough or run via `requestAnimationFrame`-driven slicing. Web Workers are an option but add complexity; defer.

### 4.5 State Snapshot Contract

The contract between simulation and renderer is the most important interface in this codebase. Pin it early:

- The renderer receives a **read-only view** of the current generation each frame.
- The view exposes cell state in a layout the GPU can consume directly (e.g., a flat `&[u8]` bitset, or a sparse list of live cells, depending on density).
- The renderer never mutates the grid.
- The simulation never calls into the renderer.

Both representations (dense + sparse) should be considered; the choice depends on the rules, which are deferred. The bootstrap reserves the API shape without committing to one storage.

### 4.6 Logging, Errors, Configuration

- **Logging:** `log` facade everywhere; `env_logger` bound in `main.rs` for desktop. Web and mobile bind platform-specific backends in their future entry points.
- **Errors:** `thiserror` for typed errors in `sim/`; `anyhow` in `app.rs` and `main.rs` at the binary edge.
- **Config:** defer until Phase 2. When introduced, a `Config` struct lives in `app.rs`; the loader is platform-specific.

### 4.7 Phase 4 Migration Path (for record)

When Phase 4 (WASM) begins, the single-crate splits into a workspace:
- `src/sim/` → `crates/gol-sim/` (library crate, no change to code)
- `src/app.rs` + `src/render/` → `crates/gol-app/` (desktop + shared logic)
- `src/main.rs` → `crates/gol-desktop/src/main.rs`
- New `crates/gol-web/` with a `wasm-bindgen` entry point

Module paths become crate paths; the code is otherwise unchanged. This migration is the payoff of keeping `sim/` clean from day one.

---

## 5. Input Handling

Mouse and keyboard support is required from Phase 1.

### 5.1 Abstraction

- `input.rs` defines an `AppAction` enum (e.g., `TogglePause`, `Step`, `Reset`, `ToggleFullscreen`, `Quit`, `ToggleCellAt(WorldPos)`, `Pan(Vec2)`, `Zoom(f32)`).
- `app.rs` maps native `winit` events to `AppAction`s. Examples for desktop:
  - `Space` → `TogglePause`
  - `N` or `Right` → `Step`
  - `R` → `Reset`
  - `F11` → `ToggleFullscreen`
  - `Esc` → on first press, exit fullscreen; on second, no-op (close button is the canonical quit).
  - Left mouse button → `ToggleCellAt(world_pos)`
  - Mouse drag with middle button → `Pan`
  - Scroll wheel → `Zoom`
- Keybindings live in `input.rs`. Future: load from a config file.

### 5.2 Mouse-to-world coordinate mapping

- Belongs in `app.rs`'s camera/viewport logic so it is shared across platforms (touch on mobile, click on desktop, click in browser).
- Inputs: screen coordinates + viewport state (pan, zoom, window size).
- Output: world cell coordinates.

### 5.3 Touch (future)

- On mobile, `winit` reports `Touch` events. The platform crate maps single tap → `ToggleCellAt`, two-finger drag → `Pan`, pinch → `Zoom`.
- The `Command` enum stays the same; only the mapping layer differs.

---

## 6. Rendering Strategy (high level)

Game logic is deferred, but the rendering approach must be sketched because it constrains the WASM and mobile decisions.

- **Small grids (≤ 512×512):** instanced quads, one per live cell, or a single fullscreen quad sampling a state texture. Simple, runs everywhere including WebGL2.
- **Large grids (≥ 1024×1024):** state stored in a GPU texture (R8 or bitset packed); update via compute shader (`wgpu` compute) or by re-uploading a CPU-computed buffer per tick.
- **Compute-shader path** requires WebGPU on the web (Chrome stable; Firefox/Safari progressively). WebGL2 fallback path uses CPU tick + texture upload.

The bootstrap phase ships a minimal renderer: clear-to-color, draw a placeholder (a single quad or a checker pattern) to validate the pipeline. The grid renderer is Phase 2.

---

## 7. Test Strategy

Testing has to map onto the hexagonal layering. Different layers warrant different test types.

### 7.1 Unit Tests

- **Where:** `gol-core` and `gol-app`, in `#[cfg(test)]` modules colocated with code.
- **What:**
  - `gol-core`: rule correctness (once rules are specified), grid edge cases (empty, single cell, full grid, boundary cells), serialization round-trips.
  - `gol-app`: command dispatch, state machine transitions (paused → running, speed changes), camera math (screen-to-world, zoom limits, pan clamps).
- **Tooling:** `cargo nextest`. Property-based tests via `proptest` for rule invariants (e.g., still-life patterns remain stable; symmetry is preserved under symmetric initial conditions — once rules are defined).
- **Coverage target:** 90%+ on `gol-core`, 75%+ on `gol-app`. Not enforced as a hard gate initially; reported in CI.

### 7.2 Integration Tests

- **Where:** `tests/` directory at the crate root (standard Rust integration tests).
- **What:**
  - `app` state driving `sim` over many ticks, verifying snapshot stability.
  - Action sequences (e.g., toggle cell → pause → step → assert state).
- **Tooling:** standard Rust integration tests; no GUI needed. `tests/sim_tests.rs` is the first file; add `tests/app_tests.rs` in Phase 2.

### 7.3 Renderer Tests

The renderer is the hardest layer to test. Two complementary approaches:

- **Headless rendering tests** in `render/`:
  - Use `wgpu` with a software backend (`Backends::GL` via `llvmpipe`/`SwiftShader`, or `wgpu`'s `Backends::all() - VULKAN` fallback).
  - Render a known snapshot, read back the framebuffer, hash it.
  - Compare against a stored hash. CI matrix runs Linux only initially; macOS/Windows later.
- **Snapshot/visual tests:**
  - Render a fixed pattern at a fixed zoom; save PNG.
  - Compare against a golden PNG with a tolerance (perceptual or per-pixel epsilon).
  - Tooling: `image` crate + a lightweight diff (or `insta` for non-image snapshots like serialized snapshot structs).
  - Caveat: GPU drivers produce subtly different output across vendors and OSes. Run snapshot tests on a single canonical CI image (e.g., Ubuntu LTS + llvmpipe) to keep them deterministic; do not require them green on contributor laptops.

### 7.4 End-to-end / Smoke Tests

- **Desktop smoke test:** a CI job launches the binary in a virtual display (`xvfb-run` on Linux), waits for the window to appear, sends a `Quit` command via stdin or a debug socket, asserts clean shutdown. Initially, accept "binary runs and exits 0" as the gate; expand later.
- **WASM smoke test (Phase 4):** load the WASM bundle in headless Chromium via Playwright; assert canvas is created and a `requestAnimationFrame` tick fires.

### 7.5 Benchmarks

- `criterion` benchmarks on `sim::tick()` for representative grid sizes. Run on demand, not in CI on every PR (too noisy). Run nightly or on tagged commits.

### 7.6 CI Pipeline

GitHub Actions (or GitLab CI — choose at bootstrap; both supported by the same `cargo` commands).

Pipeline stages, in order, fail-fast within a stage, parallel across stages where possible:

1. **Lint & format**
   - `cargo fmt --check`
   - `cargo clippy --all-targets -- -D warnings`
   - `grep -rn "winit::\|wgpu::\|egui::" src/sim/` must return empty (sim purity check)
2. **Build**
   - `cargo build --all-targets`
   - Cache `~/.cargo` and `target/` keyed on `Cargo.lock`.
3. **Test**
   - `cargo nextest run`
4. **Audit**
   - `cargo deny check`
   - `cargo audit`
5. **WASM build (Phase 4 onwards)**
   - `trunk build --release` (after workspace split).
6. **Snapshot tests (Linux only, gated)**
   - Headless GPU via `xvfb-run` + `llvmpipe`.
7. **Cross-platform smoke (Phase 3 onwards)**
   - Matrix: Ubuntu, Windows, macOS.
   - Build only initially; smoke launch later.

---

## 8. Portability Roadmap

### 8.1 Phase Plan

The phasing is engineered so that each phase delivers a runnable, demo-able artifact and so that platform expansion does not require revisiting earlier layers.

#### Phase 1 — Skeleton (this bootstrap)

**Deliverable:** a Linux desktop binary that opens an 800×600 window, renders a dark background, shows a **Quit** button (egui), accepts F11 to toggle fullscreen, accepts Esc to exit fullscreen, and closes cleanly on close-button click or Quit button press.

**Done when:**
- Single crate compiles clean with clippy `-D warnings`.
- Binary launches on Ubuntu with a visible window.
- Quit button closes the application.
- F11 toggles fullscreen reliably.
- Window closes cleanly with no leaked GPU resources (validated by `wgpu` validation layers in debug builds).
- CI runs lint + build + test; sim purity grep passes.
- All four modules exist with correct visibility.

#### Phase 2 — Game Logic

**Deliverable:** the simulation actually runs. Grid is editable with the mouse, simulation can be paused/stepped/run-at-speed via keyboard, a simple egui HUD shows generation count and FPS.

**Prerequisite:** game rules document.

**Done when:**
- `gol-core` has full unit + property test coverage of the rules.
- Renderer draws the grid at 60 fps for a 256×256 grid on a mid-range Linux laptop.
- Mouse cell-toggling is pixel-accurate at all zoom levels.
- Snapshot tests pass on CI.

#### Phase 3 — Cross-platform desktop (Windows, macOS)

**Deliverable:** the same binary, built and runnable on Windows and macOS.

**Risks & work:**
- File path handling (none expected in Phase 3; if config arrives, use `directories` crate).
- macOS code signing & notarization (deferred unless distribution is needed).
- Windows DPI awareness — handled by winit; verify on a high-DPI display.
- CI matrix expansion: add `windows-latest` and `macos-latest` runners. Build + test only; manual smoke for now.

**Done when:**
- CI green on three OSes.
- Manual smoke run on a real Windows machine and a real macOS machine confirms parity.

#### Phase 4 — Web (WebAssembly)

**Deliverable:** a static web bundle (HTML + WASM + JS shim) playable in modern browsers.

**Approach:**
- New crate `gol-platform-web`, `cdylib` crate type, depends on `gol-app` + `gol-render`.
- Entry point uses `wasm-bindgen` to expose a `start(canvas_id)` function.
- `winit` builds for `wasm32-unknown-unknown` and uses the canvas as its surface.
- `wgpu` selects WebGPU when available, WebGL2 otherwise.
- Build via `trunk build --release`. Output is `dist/` containing `index.html`, the `.wasm`, and assets.

**Constraints:**
- No filesystem, no threads (without explicit Web Workers + SharedArrayBuffer + COOP/COEP headers).
- Bundle size matters: aim for <2 MB compressed `.wasm` after `wasm-opt -Oz`.
- Frame budget is browser-controlled (`requestAnimationFrame`); long ticks block the main thread.
- Keyboard focus on `<canvas>` elements requires `tabindex` and explicit focus management.
- Touch on mobile browsers must be wired via the same `Command` abstraction.

**Done when:**
- Bundle loads in Chrome, Firefox, and Safari (latest stable).
- F11 (or browser fullscreen API) toggles fullscreen.
- A documented hosting target (GitHub Pages or static S3) serves the bundle.

#### Phase 5 — Mobile (Android, iOS)

**Deliverable:** an Android APK and an iOS app bundle running the same simulation with touch input.

**Approach:**
- Android: `gol-platform-android` crate using `android-activity` + `winit`. Built via `cargo-ndk` and packaged into an APK with a thin Java/Kotlin shim or via `cargo-mobile2`.
- iOS: `gol-platform-ios` crate using `winit`'s iOS backend. Built via `cargo-mobile2` into an Xcode-compatible static library, wrapped in a thin Objective-C/Swift shim.
- Touch input mapped to `Command`s as described in §5.3.
- Fullscreen on mobile means immersive mode (Android) / hiding status bar (iOS); F11 has no meaning. Define mobile-specific UX in a follow-up.

**Constraints:**
- Mobile CI is hard. Recommend manual builds + on-device testing for a long time before adding device farms.
- App store policies, signing, provisioning profiles — outside the scope of engineering; budget time for ops.
- Performance ceiling is lower; benchmark target a 256×256 grid at 30 fps on a mid-range device.

**Done when:**
- APK runs on a real Android 10+ device.
- IPA runs on a real iOS 16+ device.
- Touch interactions feel correct (subjective; tracked as manual QA).

### 8.2 Roadmap Diagram (textual)

```
Phase 1: Skeleton (Linux)              ──► closeable window, fullscreen, input plumbing
Phase 2: Game Logic                    ──► simulation runs, HUD, snapshot tests
Phase 3: Windows + macOS               ──► CI matrix expanded, parity validated
Phase 4: Web (WASM)                    ──► browser bundle, hosted demo
Phase 5: Mobile (Android, iOS)         ──► touch, app bundles
```

Phases 3 and 4 may run in parallel — they share no code paths and depend only on Phase 2.

---

## 9. WebAssembly Feasibility Assessment

**Verdict: feasible and well-precedented.** This is one of the strongest reasons to choose Rust.

### 9.1 What works out of the box

- **Compilation:** `cargo build --target wasm32-unknown-unknown` is a one-line install (`rustup target add`).
- **Bindings:** `wasm-bindgen` is the de-facto standard, mature, and used by most Rust web projects.
- **Windowing:** `winit` has a `web` backend that uses an HTML `<canvas>` as the window.
- **Rendering:** `wgpu` runs on WebGPU (where available) and falls back to WebGL2.
- **Input:** keyboard and mouse map cleanly to `winit` events; the `Command` abstraction in `gol-app` is unchanged.

### 9.2 What requires care

- **No threads by default.** Rust's standard threading does not work in WASM without Web Workers + SharedArrayBuffer, which require the page to be served with COOP and COEP HTTP headers. Plan: keep the simulation single-threaded for the web target; revisit if profiling demands it.
- **No filesystem.** Save/load of patterns must use `localStorage` or the File System Access API. Abstract behind a `PatternStore` trait in `gol-app`; provide platform-specific impls.
- **No blocking.** Anything that blocks the main thread freezes the tab. This is already the case for the desktop event loop, so the discipline of "tick is cheap, render is cheap" carries over.
- **Bundle size.** Without `wasm-opt -Oz` and `--release`, bundles balloon. Target: <2 MB compressed. Strip debug info, enable LTO, use `opt-level = "z"` or `"s"` for the WASM profile.
- **Driver / browser variance.** WebGPU is not yet on every browser by default. The renderer must select a backend at runtime; integration tests should cover both WebGPU and WebGL2 paths.
- **Audio (if introduced later).** Requires user gesture before the first sample plays, on all browsers. Not in scope now.
- **Loading model.** WASM modules load asynchronously. The HTML shim must show a loading state and gracefully report compile errors.

### 9.3 What does not work

- Synchronous file I/O.
- Native dialogs (file picker, color picker) — must use HTML equivalents.
- Direct OS clipboard access without a user gesture.
- `std::time::Instant` works but is monotonic only at millisecond granularity in some browsers; use `web_time` crate for portability.

### 9.4 Comparison with Go for WASM

| Aspect | Go (stdlib WASM) | Go (TinyGo) | Rust |
|---|---|---|---|
| Bundle size (hello world) | ~2 MB | ~50 KB | ~30 KB (optimized) |
| Realistic game bundle | 5–10 MB | 1–3 MB (with stdlib gaps) | 1–3 MB |
| GC | Yes, runs in WASM | Yes, smaller | None |
| Ecosystem alignment | Limited | Limited | First-class |

The Rust path is materially better. Confirmed.

---

## 10. Risks & Mitigations

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | `wgpu` API breaking changes between versions | Medium | Medium | Pin minor version; review changelog before bumps; isolate `wgpu` usage in `gol-render`. |
| R2 | Team unfamiliarity with Rust borrow checker slows Phase 1 | Medium | Medium | Pair-program first PRs; allocate explicit ramp-up time; prefer simpler ownership patterns over clever ones early. |
| R3 | Snapshot tests flaky across GPU drivers | High | Low | Pin snapshot CI to a single OS + software renderer; treat dev-machine snapshot diffs as informational. |
| R4 | WASM bundle exceeds size budget | Medium | Medium | Set a CI gate that fails if `dist/*.wasm` exceeds threshold. Run `wasm-opt -Oz`. Use `cargo-bloat` to find offenders. |
| R5 | Mobile build toolchains drift / break | Medium | High | Defer until Phase 5; document toolchain versions; pin `cargo-mobile2` revision. |
| R6 | Game rules introduce coupling that breaks the layering | Low | High | Code review checklist enforces dependency direction; add a `cargo-deny` rule or a simple script that greps for forbidden imports in `gol-core`. |
| R7 | Compile times degrade developer experience | Medium | Low | Use `sccache`; favor smaller crates; use `cargo check` during dev; keep `gol-core` lean. |
| R8 | `winit` event loop API changes (it has historically) | Medium | Medium | Isolate winit usage in platform crates; pin minor version. |

---

## 11. Open Questions

These need stakeholder or team input before the relevant phase begins.

1. **Game rules document.** Required before Phase 2. Will it be classic Conway only, or also Highlife / Day & Night / arbitrary B/S notation? This affects `gol-core` API shape.
2. **Maximum grid size target.** 256×256? 4096×4096? Determines whether a CPU tick suffices or a compute shader is required.
3. **Toroidal vs. bounded grid.** Affects rule implementation and renderer.
4. **Persistence.** Will users save/load patterns? RLE format? Drag-and-drop file support on desktop?
5. **Multiplayer / shared canvas.** In or out? If in, networking layer needs design now (it would live as a sibling of `gol-render`).
6. **Distribution.** AppImage / Snap / Flatpak for Linux? `.dmg` for macOS? Microsoft Store? Affects Phase 3 ops budget.
7. **Telemetry.** Any opt-in usage analytics? If yes, design with privacy and a kill switch from day one.
8. **Branding / assets.** Icon, splash, color palette? Not blocking but should be agreed before Phase 2 ships externally.
9. **License.** MIT / Apache-2.0 / GPL? Affects dependency selection (some game-engine crates are not permissive).

---

## 12. Estimated Complexity

T-shirt sizing per phase, assuming one experienced Rust developer working at sustainable pace.

| Phase | Size | Justification |
|---|---|---|
| Phase 1 — Skeleton | S–M | Standard winit + wgpu boilerplate. Most time is in CI setup and workspace structuring, not code. |
| Phase 2 — Game Logic | M | Rules are simple but the renderer for arbitrary grid sizes, the camera, and the HUD take real time. Property tests are an investment. |
| Phase 3 — Windows + macOS | S | Mostly CI matrix and manual validation if no platform-specific features were introduced. |
| Phase 4 — Web (WASM) | M | The build pipeline (trunk, wasm-opt, hosting) is non-trivial. Cross-browser quirks consume time. |
| Phase 5 — Mobile | L | iOS and Android each have their own toolchain, signing, and UX adjustments. Touch UX iteration is open-ended. |

If the team is new to Rust, add roughly +50% to Phase 1 and +25% to Phase 2.

---

## 13. Acceptance Criteria for Bootstrap (Phase 1)

The bootstrap is considered complete and ready for Phase 2 hand-off when **all** of the following are true:

1. Single `Cargo.toml` with the four modules described in §4.2.
2. `grep -rn "winit::\|wgpu::\|egui::" src/sim/` returns empty.
3. `cargo build --all-targets` succeeds on a clean Ubuntu 22.04+ machine.
4. `cargo clippy --all-targets -- -D warnings` succeeds.
5. `cargo fmt --check` succeeds.
6. `cargo nextest run` runs and all tests pass (unit tests in `sim/`, integration tests in `tests/sim_tests.rs`).
7. Running the desktop binary opens an 800×600 window on Linux.
8. A **Quit** button rendered by egui closes the application cleanly.
9. `F11` toggles fullscreen; `Esc` exits fullscreen; the OS close button also quits cleanly.
10. Keyboard events reach `app.rs` and are logged at `debug` level (visible with `RUST_LOG=debug`).
11. CI pipeline green on a fresh push, including lint, sim-purity check, build, test, and audit stages.
12. `README.md` documents how to build and run.
13. This plan is linked from the `README.md`.

---

## 14. Appendix A — Alternative Stack (Go + Ebiten), for record

If, after review, the team chooses Go over Rust, the high-level mapping is:

- **Language:** Go 1.22+.
- **Framework:** Ebiten v2 — supports Linux/Windows/macOS desktop, Android, iOS, and WASM (with a heavier bundle).
- **Architecture:** identical layered-module split (`sim`, `app`, `render`, `input`), expressed as Go packages.
- **Trade-offs:** simpler onboarding, faster iteration, larger WASM bundle, weaker mobile story, GC pauses to monitor at high cell counts.
- **WASM:** Ebiten supports `GOOS=js GOARCH=wasm`; bundle is several MB even after compression. Adequate for a hobby web demo, marginal for a polished experience.

This appendix exists so the decision is reversible without re-doing the architectural work.

---

## 15. Appendix B — Glossary

- **Hexagonal architecture / ports & adapters:** an architectural style where the domain is at the center and all I/O lives in adapters at the edge.
- **Snapshot test:** a test that compares an output (image, serialized struct) against a stored "golden" version.
- **WebGPU / WebGL2:** browser GPU APIs. WebGPU is newer and more capable; WebGL2 is the universal fallback.
- **wgpu:** a Rust GPU abstraction implementing the WebGPU spec, but also targeting native Vulkan / Metal / DX12 / GL.
- **winit:** a Rust cross-platform window creation and event handling library.
- **egui:** an immediate-mode GUI library in Rust, easy to integrate with winit + wgpu.
- **MSRV:** minimum supported Rust version.
- **`cargo-deny`, `cargo-audit`:** supply-chain auditing tools for Rust.

---

*End of plan. This document will be revised once the game rules document is delivered and Phase 2 begins.*
