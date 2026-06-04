# Game of Life

Conway's Game of Life rendered with wgpu and an egui overlay, built in Rust.

## Prerequisites

- Rust stable toolchain (`rustup` recommended)
- `cargo` (included with Rust)

## Build

```sh
cargo build
```

## Run

```sh
cargo run
```

## Test

```sh
cargo test
```

## Keyboard shortcuts

| Key        | Action                                      |
|------------|---------------------------------------------|
| Q          | Quit                                        |
| F11        | Toggle fullscreen                           |
| Esc        | Exit fullscreen (no-op if already windowed) |
| Close button | Quit                                      |

## Specification

See [docs/specs/PLAN_initial.md](docs/specs/PLAN_initial.md) for the full Phase 1 plan and architecture decisions.
