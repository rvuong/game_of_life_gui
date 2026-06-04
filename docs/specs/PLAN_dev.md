# Game of Life — Technical Development Plan

> Companion to `PLAN_rules.md`.
> Methodology: Test-Driven Development (TDD).
> For each sub-step: write the test → watch it fail → write the minimum code to pass → move on.

---

## How to read this document

Every sub-step follows the same three-part rhythm:

1. **Red** — copy the test block into the codebase and run `cargo test`. It must fail (or not compile). That is the goal at this stage.
2. **Green** — write only the code described in "What to implement". Run `cargo test` again. It must pass.
3. **Next** — move to the next sub-step. Never write code that no test is asking for yet.

When all sub-steps of a big step are green, the integration check at the bottom of that section gates moving to the next big step.

---

## Step 1 — Grid storage and Conway's rules `[IMPLEMENTED]`

### What you are replacing

`src/sim/mod.rs` currently holds a placeholder `Grid` with no cell storage. You will replace it entirely. The existing tests (`dimensions_are_stored`, `cell_count_is_width_times_height`) must continue to pass after the rewrite.

### Files to modify

- `src/sim/mod.rs` — replace the placeholder, add all new tests.

### New Cargo.toml dependencies

None.

---

### Sub-step 1.1 — Grid constructor and flat cell storage

**Write this test first** (add inside `#[cfg(test)] mod tests` in `src/sim/mod.rs`):

```rust
#[test]
fn new_grid_is_all_dead() {
    let g = Grid::new(5, 4);
    for y in 0..4 {
        for x in 0..5 {
            assert!(!g.get(x as i32, y as i32), "cell ({x},{y}) should be dead");
        }
    }
}

#[test]
fn set_and_get_cell() {
    let mut g = Grid::new(10, 10);
    g.set(3, 7, true);
    assert!(g.get(3, 7));
    assert!(!g.get(2, 7));
}

#[test]
fn toggle_cell() {
    let mut g = Grid::new(5, 5);
    g.toggle(2, 2);
    assert!(g.get(2, 2));
    g.toggle(2, 2);
    assert!(!g.get(2, 2));
}
```

**What to implement:**

Replace the current `Grid` struct with:

```rust
pub struct Grid {
    pub width: usize,
    pub height: usize,
    cells: Vec<bool>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![false; width * height],
        }
    }

    // x and y accept i32 so callers can pass negative offsets (used in neighbour counting).
    // Wrapping is applied here, making the grid toroidal.
    pub fn get(&self, x: i32, y: i32) -> bool {
        let x = x.rem_euclid(self.width as i32) as usize;
        let y = y.rem_euclid(self.height as i32) as usize;
        self.cells[y * self.width + x]
    }

    pub fn set(&mut self, x: usize, y: usize, alive: bool) {
        self.cells[y * self.width + x] = alive;
    }

    pub fn toggle(&mut self, x: usize, y: usize) {
        let idx = y * self.width + x;
        self.cells[idx] = !self.cells[idx];
    }

    pub fn cell_count(&self) -> usize {
        self.width * self.height
    }

    pub fn live_cell_count(&self) -> u32 {
        self.cells.iter().filter(|&&c| c).count() as u32
    }

    pub fn clear(&mut self) {
        self.cells.fill(false);
    }
}
```

> **Note on `rem_euclid`:** standard `%` in Rust returns a negative result for negative inputs. `rem_euclid` always returns a non-negative value. For example, `(-1_i32).rem_euclid(20)` returns `19`, which is exactly what toroidal wrapping needs.

**Verify:** `cargo test` — all tests including the pre-existing ones must pass.

> **Note:** The pre-existing tests use `g.width` and `g.height`. Changing those fields from `u32` to `usize` is fine — the literal integer comparisons still compile.

---

### Sub-step 1.2 — Toroidal wrapping is correct at all four edges

**Write this test first:**

```rust
#[test]
fn toroidal_wrap_right_edge() {
    let mut g = Grid::new(5, 5);
    g.set(4, 2, true); // rightmost column
    assert!(g.get(4, 2));
    assert!(g.get(-1, 2)); // one step past left edge wraps to right
}

#[test]
fn toroidal_wrap_left_edge() {
    let mut g = Grid::new(5, 5);
    g.set(0, 2, true);
    assert!(g.get(5, 2)); // one step past right wraps to left
}

#[test]
fn toroidal_wrap_top_and_bottom() {
    let mut g = Grid::new(5, 5);
    g.set(2, 0, true);
    assert!(g.get(2, -1)); // one above top wraps to bottom
    assert!(g.get(2, 5));  // one below bottom wraps to top

    let mut g2 = Grid::new(5, 5);
    g2.set(2, 4, true);
    assert!(g2.get(2, -1)); // wait: -1 rem_euclid 5 = 4. Correct.
}
```

**What to implement:** nothing new — the `get` method you wrote in 1.1 already handles this via `rem_euclid`. These tests just prove it works.

**Verify:** `cargo test` passes.

---

### Sub-step 1.3 — Neighbour counting

**Write this test first:**

```rust
#[test]
fn isolated_cell_has_zero_neighbours() {
    let mut g = Grid::new(10, 10);
    g.set(5, 5, true);
    assert_eq!(g.count_neighbours(5, 5), 0);
}

#[test]
fn cell_surrounded_by_all_eight_has_eight_neighbours() {
    let mut g = Grid::new(10, 10);
    for dy in -1_i32..=1 {
        for dx in -1_i32..=1 {
            g.set(
                (5_i32 + dx) as usize,
                (5_i32 + dy) as usize,
                true,
            );
        }
    }
    // Centre cell (5,5) is alive, but we count its neighbours — not itself.
    assert_eq!(g.count_neighbours(5, 5), 8);
}

#[test]
fn neighbours_wrap_toroidally() {
    let mut g = Grid::new(5, 5);
    g.set(4, 2, true); // right edge
    // The cell at (0,2) has (4,2) as its left neighbour due to wrapping.
    assert_eq!(g.count_neighbours(0, 2), 1);
}
```

**What to implement:**

```rust
impl Grid {
    pub fn count_neighbours(&self, x: usize, y: usize) -> u8 {
        const OFFSETS: [(i32, i32); 8] = [
            (-1, -1), (0, -1), (1, -1),
            (-1,  0),          (1,  0),
            (-1,  1), (0,  1), (1,  1),
        ];
        let x = x as i32;
        let y = y as i32;
        OFFSETS.iter().filter(|(dx, dy)| self.get(x + dx, y + dy)).count() as u8
    }
}
```

**Verify:** `cargo test` passes.

---

### Sub-step 1.4 — `tick()` applies Conway's rules

**Write this test first:**

```rust
// Helper: set a list of (x,y) cells alive on a grid.
fn seed(g: &mut Grid, cells: &[(usize, usize)]) {
    for &(x, y) in cells { g.set(x, y, true); }
}

#[test]
fn block_is_a_still_life() {
    // 2x2 square — the simplest still life.
    let mut g = Grid::new(10, 10);
    seed(&mut g, &[(4,4),(5,4),(4,5),(5,5)]);
    let before: Vec<bool> = g.cells.clone(); // requires cells to be pub(crate) or accessible
    g.tick();
    assert_eq!(g.cells, before);
}

#[test]
fn blinker_oscillates_with_period_2() {
    // Horizontal blinker: (4,5) (5,5) (6,5)
    let mut g = Grid::new(10, 10);
    seed(&mut g, &[(4,5),(5,5),(6,5)]);

    g.tick(); // should become vertical: (5,4) (5,5) (5,6)
    assert!( g.get(5, 4));
    assert!( g.get(5, 5));
    assert!( g.get(5, 6));
    assert!(!g.get(4, 5));
    assert!(!g.get(6, 5));

    g.tick(); // should be horizontal again
    assert!( g.get(4, 5));
    assert!( g.get(5, 5));
    assert!( g.get(6, 5));
    assert!(!g.get(5, 4));
    assert!(!g.get(5, 6));
}

#[test]
fn isolated_cell_dies() {
    let mut g = Grid::new(10, 10);
    g.set(5, 5, true);
    g.tick();
    assert!(!g.get(5, 5));
}

#[test]
fn dead_cell_with_three_neighbours_comes_alive() {
    let mut g = Grid::new(10, 10);
    // Three live cells around (5,5), which is dead.
    seed(&mut g, &[(4,4),(5,4),(6,4)]);
    g.tick();
    assert!(g.get(5, 5)); // (5,5) had exactly 3 live neighbours
}
```

**What to implement:**

```rust
impl Grid {
    pub fn tick(&mut self) {
        let mut next = vec![false; self.width * self.height];
        for y in 0..self.height {
            for x in 0..self.width {
                let n = self.count_neighbours(x, y);
                let alive = self.get(x as i32, y as i32);
                next[y * self.width + x] = matches!(
                    (alive, n),
                    (true,  2) | (true,  3) | (false, 3)
                );
            }
        }
        self.cells = next;
    }
}
```

> This is the full application of the four Conway rules compressed into one `matches!` expression:
> - Live cell + 2 neighbours → stays alive
> - Live cell + 3 neighbours → stays alive
> - Dead cell + 3 neighbours → comes alive
> - Everything else → dead

> **Important:** for `block_is_a_still_life`, the test accesses `g.cells` directly. Add `pub(crate)` to the `cells` field so tests within the crate can reach it: `pub(crate) cells: Vec<bool>`.

**Verify:** `cargo test` passes.

---

### Step 1 integration check

Before moving to Step 2, run:

```sh
cargo test
grep -rn "winit::\|wgpu::\|egui::" src/sim/   # must print nothing
```

Both must succeed. The glider test below is the final proof that the simulation is correct. Add it now:

```rust
#[test]
fn glider_returns_to_shape_after_4_ticks() {
    // Standard glider, top-left corner of a 20x20 toroidal grid.
    // After 4 ticks it has moved one cell diagonally and is identical in shape.
    let mut g = Grid::new(20, 20);
    seed(&mut g, &[(1,0),(2,1),(0,2),(1,2),(2,2)]);
    for _ in 0..4 { g.tick(); }
    // Shifted one right and one down:
    assert!( g.get(2, 1));
    assert!( g.get(3, 2));
    assert!( g.get(1, 3));
    assert!( g.get(2, 3));
    assert!( g.get(3, 3));
    assert_eq!(g.live_cell_count(), 5);
}
```

---

## Step 2 — Cycle detection and scoring `[IMPLEMENTED]`

### What you are building

A `GameSession` struct that wraps a `Grid`, tracks every grid state it has seen, and knows when to stop.

### Files to create or modify

- Create `src/sim/session.rs`.
- Modify `src/sim/mod.rs`: add `pub mod session;` and implement `Hash` for `Grid`.

### New Cargo.toml dependencies

None — `std::collections::HashSet` and `std::hash::DefaultHasher` are part of the standard library.

---

### Sub-step 2.1 — Grid implements Hash

The cycle detector needs to turn a grid into a number. The standard way in Rust is to implement `Hash` on `Grid`, then use `DefaultHasher` to produce a `u64`.

**Write this test first** (in `src/sim/mod.rs`):

```rust
#[test]
fn identical_grids_produce_same_hash() {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    fn hash_grid(g: &Grid) -> u64 {
        let mut h = DefaultHasher::new();
        g.hash(&mut h);
        h.finish()
    }

    let mut a = Grid::new(5, 5);
    a.set(2, 2, true);

    let mut b = Grid::new(5, 5);
    b.set(2, 2, true);

    assert_eq!(hash_grid(&a), hash_grid(&b));
}

#[test]
fn different_grids_produce_different_hash() {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    fn hash_grid(g: &Grid) -> u64 {
        let mut h = DefaultHasher::new();
        g.hash(&mut h);
        h.finish()
    }

    let mut a = Grid::new(5, 5);
    a.set(2, 2, true);

    let b = Grid::new(5, 5); // all dead

    assert_ne!(hash_grid(&a), hash_grid(&b));
}
```

**What to implement** (in `src/sim/mod.rs`):

```rust
impl std::hash::Hash for Grid {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.width.hash(state);
        self.height.hash(state);
        self.cells.hash(state);
    }
}
```

**Verify:** `cargo test` passes.

---

### Sub-step 2.2 — GameResult enum

**Write this test first** (in `src/sim/session.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_result_variants_exist() {
        // This test simply checks the enum compiles with the expected variants.
        let _ = GameResult::StillRunning;
        let _ = GameResult::CycleDetected;
        let _ = GameResult::CapReached;
    }
}
```

**What to implement** (new file `src/sim/session.rs`):

```rust
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use crate::sim::Grid;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GameResult {
    StillRunning,
    CycleDetected,
    CapReached,
}
```

Also add `pub mod session;` to `src/sim/mod.rs`.

**Verify:** `cargo test` passes.

---

### Sub-step 2.3 — GameSession constructor

**Write this test first** (in `src/sim/session.rs` tests):

```rust
#[test]
fn new_session_starts_at_step_zero() {
    let g = Grid::new(10, 10);
    let session = GameSession::new(g, 300);
    assert_eq!(session.step_count, 0);
    assert_eq!(session.max_steps, 300);
    // The initial state is already hashed and stored.
    assert_eq!(session.population_history.len(), 1);
}
```

**What to implement:**

```rust
fn hash_grid(grid: &Grid) -> u64 {
    let mut hasher = DefaultHasher::new();
    grid.hash(&mut hasher);
    hasher.finish()
}

pub struct GameSession {
    pub grid: Grid,
    seen_hashes: HashSet<u64>,
    pub population_history: Vec<u32>,
    pub step_count: u32,
    pub max_steps: u32,
}

impl GameSession {
    pub fn new(grid: Grid, max_steps: u32) -> Self {
        let initial_hash = hash_grid(&grid);
        let initial_pop = grid.live_cell_count();
        let mut seen_hashes = HashSet::new();
        seen_hashes.insert(initial_hash);
        Self {
            grid,
            seen_hashes,
            population_history: vec![initial_pop],
            step_count: 0,
            max_steps,
        }
    }

    pub fn score(&self) -> u64 {
        let n = self.seen_hashes.len() as u64;
        (n + 1).pow(2)
    }
}
```

**Verify:** `cargo test` passes.

---

### Sub-step 2.4 — advance() drives the simulation

**Write this test first:**

```rust
fn make_blinker() -> Grid {
    let mut g = Grid::new(10, 10);
    g.set(4, 5, true);
    g.set(5, 5, true);
    g.set(6, 5, true);
    g
}

fn make_block() -> Grid {
    let mut g = Grid::new(10, 10);
    g.set(4, 4, true); g.set(5, 4, true);
    g.set(4, 5, true); g.set(5, 5, true);
    g
}

#[test]
fn still_life_detected_after_one_advance() {
    let mut s = GameSession::new(make_block(), 300);
    let r = s.advance();
    assert_eq!(r, GameResult::CycleDetected);
    assert_eq!(s.score(), 4); // (1+1)^2
}

#[test]
fn blinker_detected_after_two_advances() {
    let mut s = GameSession::new(make_blinker(), 300);
    assert_eq!(s.advance(), GameResult::StillRunning);
    assert_eq!(s.advance(), GameResult::CycleDetected);
    assert_eq!(s.score(), 9); // (2+1)^2
}

#[test]
fn cap_reached_stops_the_game() {
    let mut s = GameSession::new(make_blinker(), 2);
    assert_eq!(s.advance(), GameResult::StillRunning);
    assert_eq!(s.advance(), GameResult::CapReached);
    // Score at cap: 2 unique states seen → (2+1)^2 = 9
    assert_eq!(s.score(), 9);
}

#[test]
fn population_history_grows_each_advance() {
    let mut s = GameSession::new(make_blinker(), 300);
    assert_eq!(s.population_history.len(), 1); // initial state
    s.advance();
    assert_eq!(s.population_history.len(), 2);
    s.advance(); // cycle detected — no new state added
    assert_eq!(s.population_history.len(), 2); // still 2
}
```

> Note on the last test: when a cycle is detected, the repeated state is not added to `population_history`. The history only records unique states.

**What to implement:**

```rust
impl GameSession {
    pub fn advance(&mut self) -> GameResult {
        self.grid.tick();
        self.step_count += 1;

        if self.step_count >= self.max_steps {
            return GameResult::CapReached;
        }

        let h = hash_grid(&self.grid);
        if self.seen_hashes.contains(&h) {
            return GameResult::CycleDetected;
        }

        self.seen_hashes.insert(h);
        self.population_history.push(self.grid.live_cell_count());
        GameResult::StillRunning
    }
}
```

**Verify:** `cargo test` passes.

---

### Step 2 integration check

```sh
cargo test
```

All tests pass. Confirm manually that the blinker scores 9 and the block scores 4 by reading the test output. If both are green, the game logic is complete and correct. Steps 3–5 add no changes to `src/sim/`.

---

## Step 3 — Grid rendering and cell editing `[IMPLEMENTED]`

### What you are building

The grid on screen, cell toggling with the mouse, the grid size selector, and the Start/Restart buttons.

### Files to modify

- `src/app.rs` — add `AppPhase`, grid state, mouse handling.
- `src/render/mod.rs` — add the grid draw pass.

### New Cargo.toml dependencies

None yet.

---

### Sub-step 3.1 — App state machine and grid ownership

**What to implement** (in `src/app.rs`):

Add the application phase enum and extend `AppInner` to own a `Grid` and track the current phase.

```rust
use game_of_life::sim::Grid;
use game_of_life::sim::session::{GameSession, GameResult};

pub enum AppPhase {
    Editing,
    Running,
    Ended(EndState),
}

pub struct EndState {
    pub reason: GameResult,
    pub score: u64,
    pub population_history: Vec<u32>,
    pub step_count: u32,
}
```

Add to `AppInner`:
```rust
pub grid: Grid,
pub phase: AppPhase,
pub game_session: Option<GameSession>,
pub max_steps: u32,
```

Initialise them in `AppInner::new`:
```rust
grid: Grid::new(20, 20),
phase: AppPhase::Editing,
game_session: None,
max_steps: 300,
```

**Write this test** (in `src/app.rs` as a unit test, using `#[cfg(test)]`):

```rust
#[test]
fn start_transitions_to_running() {
    // Create an AppInner-like struct just for the state machine logic.
    // This test does not open a window.
    let grid = Grid::new(5, 5);
    let max_steps = 300;
    let session = GameSession::new(grid.clone(), max_steps);
    // Simulate what pressing Start does:
    let phase = AppPhase::Running;
    assert!(matches!(phase, AppPhase::Running));
    let _ = session; // session is ready to use
}
```

> This is a minimal smoke test to confirm the types compile and connect correctly. More meaningful tests come from the full game loop in Step 4.

**Verify:** `cargo build` compiles without errors.

---

### Sub-step 3.2 — World-to-screen coordinate conversion

This is the most important pure-logic piece in Step 3. A wrong conversion causes misclicks and broken gameplay. Test it independently.

**Write this test first** (in `src/app.rs`):

```rust
#[cfg(test)]
mod coord_tests {
    use super::*;

    // Standalone helper (mirrors what the renderer computes).
    fn screen_to_cell(
        mouse_x: f32, mouse_y: f32,
        grid_origin_x: f32, grid_origin_y: f32,
        cell_size: f32,
        grid_width: usize, grid_height: usize,
    ) -> Option<(usize, usize)> {
        if mouse_x < grid_origin_x || mouse_y < grid_origin_y {
            return None;
        }
        let col = ((mouse_x - grid_origin_x) / cell_size) as usize;
        let row = ((mouse_y - grid_origin_y) / cell_size) as usize;
        if col >= grid_width || row >= grid_height { None } else { Some((col, row)) }
    }

    #[test]
    fn top_left_corner_maps_to_cell_0_0() {
        assert_eq!(
            screen_to_cell(10.0, 10.0, 10.0, 10.0, 20.0, 5, 5),
            Some((0, 0))
        );
    }

    #[test]
    fn click_in_second_cell_row_and_column() {
        // Grid starts at (10,10), each cell is 20px. Click at (55,35):
        // col = (55-10)/20 = 2, row = (35-10)/20 = 1
        assert_eq!(
            screen_to_cell(55.0, 35.0, 10.0, 10.0, 20.0, 5, 5),
            Some((2, 1))
        );
    }

    #[test]
    fn click_outside_grid_returns_none() {
        assert_eq!(
            screen_to_cell(5.0, 5.0, 10.0, 10.0, 20.0, 5, 5),
            None
        );
        // Past the right edge: col = (120-10)/20 = 5, which equals grid_width → None
        assert_eq!(
            screen_to_cell(120.0, 10.0, 10.0, 10.0, 20.0, 5, 5),
            None
        );
    }
}
```

**What to implement:**

Extract the function above into a free function `screen_to_cell` in `src/app.rs` (not inside `AppInner` — keep it pure so it is testable). `AppInner` will call it when handling mouse click events, passing the current grid origin and cell size from the renderer.

**Verify:** `cargo test` passes.

---

### Sub-step 3.3 — Grid rendering with egui Painter

**What to implement** (in `src/render/mod.rs`):

Add a method `draw_grid` that takes the grid and an egui `Ui` and draws it using `egui::Painter`. Do not use a custom wgpu shader — egui's `painter.rect_filled` is sufficient for Phase 2.

```rust
pub fn draw_grid(ui: &mut egui::Ui, grid: &game_of_life::sim::Grid) -> (egui::Pos2, f32) {
    let available = ui.available_size();
    let cell_size = (available.x / grid.width as f32)
        .min(available.y / grid.height as f32)
        .floor()
        .max(4.0); // minimum 4px per cell

    let grid_w = cell_size * grid.width as f32;
    let grid_h = cell_size * grid.height as f32;

    // Centre the grid in the available area.
    let origin = ui.cursor().min
        + egui::vec2(
            (available.x - grid_w) / 2.0,
            (available.y - grid_h) / 2.0,
        );

    let painter = ui.painter();

    for y in 0..grid.height {
        for x in 0..grid.width {
            let top_left = origin + egui::vec2(x as f32 * cell_size, y as f32 * cell_size);
            let rect = egui::Rect::from_min_size(top_left, egui::vec2(cell_size - 1.0, cell_size - 1.0));
            let color = if grid.get(x as i32, y as i32) {
                egui::Color32::from_rgb(220, 220, 220) // alive
            } else {
                egui::Color32::from_rgb(30, 30, 30) // dead
            };
            painter.rect_filled(rect, 0.0, color);
        }
    }

    (origin, cell_size) // returned so AppInner can do coordinate conversion
}
```

The return value `(origin, cell_size)` is stored in `AppInner` after each draw call, so that when a mouse click arrives, `screen_to_cell` has the values it needs.

**Manual verify:** run `cargo run`. The dark grid must appear on screen. Resize the window — the grid must scale.

---

### Sub-step 3.4 — Start, Restart, and grid size buttons

**What to implement** (in `src/app.rs`, inside the egui panel rendered each frame):

```rust
// Inside AppInner::render(), in the egui panel for AppPhase::Editing:

egui::SidePanel::left("controls").show(ctx, |ui| {
    ui.label("Grid size:");
    if ui.button("20 × 20").clicked() {
        self.grid = Grid::new(20, 20);
    }
    if ui.button("40 × 40").clicked() {
        self.grid = Grid::new(40, 40);
    }
    if ui.button("60 × 60").clicked() {
        self.grid = Grid::new(60, 60);
    }
    ui.separator();
    if ui.button("▶  Start").clicked() {
        let session = GameSession::new(self.grid.clone(), self.max_steps);
        self.game_session = Some(session);
        self.phase = AppPhase::Running;
    }
});
```

Add a Restart button visible in all phases:

```rust
if ui.button("⟳  Restart").clicked() {
    self.grid.clear();
    self.game_session = None;
    self.phase = AppPhase::Editing;
}
```

**Handle mouse click → cell toggle in `AppPhase::Editing`:**

In `AppInner`, store the last known grid origin and cell size:
```rust
grid_origin: egui::Pos2,
grid_cell_size: f32,
```

Update them after every `draw_grid` call. Then in `window_event`, for `WindowEvent::MouseInput` with left button pressed:

```rust
if matches!(self.phase, AppPhase::Editing) {
    if let Some((col, row)) = screen_to_cell(
        self.last_mouse_pos.x,
        self.last_mouse_pos.y,
        self.grid_origin.x,
        self.grid_origin.y,
        self.grid_cell_size,
        self.grid.width,
        self.grid.height,
    ) {
        self.grid.toggle(col, row);
    }
}
```

Track `last_mouse_pos` by handling `WindowEvent::CursorMoved { position, .. }`.

**Manual verify:** click cells — they appear and disappear. Press Start — app enters Running phase (grid stops responding to clicks). Press Restart — grid clears.

---

### Step 3 integration check

Run the application. Manually verify:
- [ ] Grid displays correctly in all three sizes.
- [ ] Clicking a dead cell makes it alive; clicking again makes it dead.
- [ ] Start transitions to Running (no more click-to-toggle).
- [ ] Restart returns to Editing with a blank grid.

---

## Step 4 — Game loop and speed control `[IMPLEMENTED]`

### What you are building

The timer that advances the simulation at the correct pace, the live display of step count and population, and the speed control.

### Files to modify

- `src/app.rs` — add timer fields, handle advance() result, add speed UI.

### New Cargo.toml dependencies

None — `std::time::Instant` and `std::time::Duration` are in the standard library.

---

### Sub-step 4.1 — Timer fields in AppInner

**What to implement:**

Add to `AppInner`:
```rust
last_tick: std::time::Instant,
step_duration: std::time::Duration,
```

Initialise in `AppInner::new`:
```rust
last_tick: std::time::Instant::now(),
step_duration: std::time::Duration::from_millis(300),
```

---

### Sub-step 4.2 — Elapsed-time tick in `about_to_wait`

The `about_to_wait` callback fires every time winit has no more events to process. This is where the game advances, not inside an OS timer.

**Write this test first** (pure logic, no window):

```rust
#[test]
fn advance_called_when_enough_time_has_elapsed() {
    use std::time::{Duration, Instant};
    use game_of_life::sim::Grid;
    use game_of_life::sim::session::{GameSession, GameResult};

    let mut g = Grid::new(10, 10);
    g.set(4, 5, true); g.set(5, 5, true); g.set(6, 5, true); // blinker

    let mut session = GameSession::new(g, 300);
    let step_duration = Duration::from_millis(1); // very short for testing
    let mut last_tick = Instant::now() - step_duration; // already elapsed

    // Simulate what about_to_wait does:
    if last_tick.elapsed() >= step_duration {
        last_tick = Instant::now();
        let result = session.advance();
        assert_eq!(result, GameResult::StillRunning);
        assert_eq!(session.step_count, 1);
    }
}
```

**What to implement** (in `ApplicationHandler::about_to_wait` in `src/app.rs`):

```rust
fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    let Some(inner) = &mut self.inner else { return };

    if matches!(inner.phase, AppPhase::Running) {
        if inner.last_tick.elapsed() >= inner.step_duration {
            inner.last_tick = std::time::Instant::now();
            let session = inner.game_session.as_mut().unwrap();
            match session.advance() {
                GameResult::StillRunning => {}
                reason => {
                    inner.phase = AppPhase::Ended(EndState {
                        reason,
                        score: session.score(),
                        population_history: session.population_history.clone(),
                        step_count: session.step_count,
                    });
                }
            }
        }
    }

    inner.window.request_redraw();
}
```

**Verify:** place a blinker, press Start, watch it oscillate. The game must transition to Ended after 2 steps.

---

### Sub-step 4.3 — Live step count and speed control

**What to implement** (in the egui panel for `AppPhase::Running`):

```rust
egui::SidePanel::left("controls").show(ctx, |ui| {
    let session = self.game_session.as_ref().unwrap();
    ui.label(format!("Step: {} / {}", session.step_count, self.max_steps));
    ui.label(format!("Live cells: {}", session.population_history.last().copied().unwrap_or(0)));
    ui.separator();

    let mut ms = self.step_duration.as_millis() as u32;
    ui.label("Speed (ms/step):");
    if ui.add(egui::Slider::new(&mut ms, 50..=2000).logarithmic(true)).changed() {
        self.step_duration = std::time::Duration::from_millis(ms as u64);
    }
    ui.separator();
    if ui.button("⟳  Restart").clicked() {
        // ... same as before
    }
});
```

> The slider uses `logarithmic(true)` so that the fine control is at the slow end (where the player watches step by step) and the fast end compresses toward 50ms.

**Manual verify:** place any pattern, press Start, move the speed slider from slow to fast. The simulation must visibly change pace.

---

### Sub-step 4.4 — Max steps configuration

**What to implement** (in the egui panel for `AppPhase::Editing`):

```rust
ui.label("Max steps:");
ui.add(egui::Slider::new(&mut self.max_steps, 100..=10_000).logarithmic(true));
```

When Start is pressed, pass `self.max_steps` to `GameSession::new`. This is already how it was written in Sub-step 3.4; just confirm the value flows through.

**Manual verify:** set max steps to 5, place a glider, press Start. The game must end at step 5 with reason CapReached.

---

### Step 4 integration check

Manually run through this sequence:

| Action | Expected result |
|---|---|
| Place a block (2×2), press Start | Game ends at step 1, score 4, reason CycleDetected |
| Restart, place a blinker (3 in a row), press Start | Game ends at step 2, score 9, reason CycleDetected |
| Set max steps to 5, place a glider, press Start | Game ends at step 5, score ≥ 36 (`(5+1)²`), reason CapReached |
| Move speed slider to 50ms | Simulation runs visibly faster |

If all four pass, Step 4 is done.

---

## Step 5 — Preset patterns and post-game display `[PLANNED]`

### What you are building

Built-in patterns the player can load from the edit screen, and the post-game screen with score, reason, and population graph.

### Files to create or modify

- Create `src/sim/patterns.rs` — pattern data and placement function.
- Modify `src/sim/mod.rs` — add `pub mod patterns;`.
- Modify `src/app.rs` — preset buttons, end screen.
- Modify `Cargo.toml` — add `egui_plot`.

### New Cargo.toml dependency

```toml
egui_plot = "0.29"
```

---

### Sub-step 5.1 — Pattern data and placement

**Write this test first** (in `src/sim/patterns.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Grid;

    #[test]
    fn block_pattern_places_four_live_cells() {
        let mut g = Grid::new(20, 20);
        place(BLOCK, &mut g);
        assert_eq!(g.live_cell_count(), 4);
    }

    #[test]
    fn blinker_pattern_places_three_live_cells() {
        let mut g = Grid::new(20, 20);
        place(BLINKER, &mut g);
        assert_eq!(g.live_cell_count(), 3);
    }

    #[test]
    fn placement_is_centred_and_toroidal() {
        // A pattern placed on a 5x5 grid with offset (-3, 0) must wrap correctly.
        let mut g = Grid::new(5, 5);
        place(&[(-3, 0)], &mut g); // -3 rem_euclid 5 = 2
        assert!(g.get(2, 2)); // centre is (2,2) on a 5x5
    }
}
```

**What to implement** (new file `src/sim/patterns.rs`):

```rust
use crate::sim::Grid;

pub const BLOCK: &[(i32, i32)] = &[
    (0,0),(1,0),
    (0,1),(1,1),
];

pub const BLINKER: &[(i32, i32)] = &[(-1,0),(0,0),(1,0)];

pub const GLIDER: &[(i32, i32)] = &[
    (1,0),
    (2,1),
    (0,2),(1,2),(2,2),
];

// Period-3 oscillator, requires at least a 15x15 grid.
pub const PULSAR: &[(i32, i32)] = &[
    (-6,-4),(-5,-4),(-4,-4),(-2,-4),(-1,-4),(0,-4),
    // ... (full list of 48 cells — look up "pulsar life wiki" for coordinates)
    // abbreviated here for length; paste the full coordinate list
];

// Chaotic 5-cell pattern: takes 1103 steps to stabilise on a large grid.
pub const R_PENTOMINO: &[(i32, i32)] = &[
    (0,-1),(1,-1),(-1,0),(0,0),(0,1),
];

pub struct PatternDef {
    pub name: &'static str,
    pub cells: &'static [(i32, i32)],
    pub description: &'static str,
}

pub const PRESETS: &[PatternDef] = &[
    PatternDef { name: "Block",      cells: BLOCK,      description: "Still life. Score: 4." },
    PatternDef { name: "Blinker",    cells: BLINKER,    description: "Period-2 oscillator. Score: 9." },
    PatternDef { name: "Glider",     cells: GLIDER,     description: "Travels diagonally." },
    PatternDef { name: "Pulsar",     cells: PULSAR,     description: "Period-3 oscillator. Requires 40×40." },
    PatternDef { name: "R-pentomino",cells: R_PENTOMINO,description: "Chaotic. Try on 60×60 with max 1000 steps." },
];

/// Place a pattern centred on the grid. Existing cells are cleared first.
pub fn place(pattern: &[(i32, i32)], grid: &mut Grid) {
    grid.clear();
    let cx = (grid.width / 2) as i32;
    let cy = (grid.height / 2) as i32;
    for (dx, dy) in pattern {
        let x = (cx + dx).rem_euclid(grid.width as i32) as usize;
        let y = (cy + dy).rem_euclid(grid.height as i32) as usize;
        grid.set(x, y, true);
    }
}
```

**Verify:** `cargo test` passes.

---

### Sub-step 5.2 — Preset buttons in the editing panel

**What to implement** (in the egui panel for `AppPhase::Editing`):

```rust
use game_of_life::sim::patterns::{PRESETS, place};

ui.separator();
ui.label("Presets:");
for preset in PRESETS {
    if ui.button(preset.name).on_hover_text(preset.description).clicked() {
        place(preset.cells, &mut self.grid);
    }
}
```

**Manual verify:** click each preset button. The grid must clear and show the pattern centred.

---

### Sub-step 5.3 — Post-game screen

**What to implement** (in `AppPhase::Ended` rendering, in `src/app.rs`):

```rust
use egui_plot::{Line, Plot, PlotPoints};

// Called when phase is AppPhase::Ended(state):
egui::CentralPanel::default().show(ctx, |ui| {
    ui.heading("Game Over");
    ui.separator();

    let reason_text = match state.reason {
        GameResult::CycleDetected => "A pattern repeated — the simulation has looped.",
        GameResult::CapReached    => "Step limit reached.",
    };
    ui.label(reason_text);
    ui.separator();

    ui.label(format!("Steps: {}", state.step_count));
    ui.label(format!("Unique states: {}", (state.score as f64).sqrt() as u64 - 1));
    ui.heading(format!("Score: {}", state.score));

    ui.separator();
    ui.label("Population over time:");

    let points: PlotPoints = state.population_history
        .iter()
        .enumerate()
        .map(|(i, &count)| [i as f64, count as f64])
        .collect();

    Plot::new("pop_graph")
        .height(200.0)
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new(points).name("Live cells"));
        });

    ui.separator();
    if ui.button("▶  Play again").clicked() {
        self.grid.clear();
        self.game_session = None;
        self.phase = AppPhase::Editing;
    }
});
```

**Manual verify:**

1. Place a block, press Start. End screen shows: score 4, step 1, reason "pattern repeated", graph shows a flat line of 4 cells.
2. Load R-pentomino on 60×60, set max steps 500, press Start. Let it run. The graph should show a wild, chaotic population curve before flattening.

---

### Step 5 integration check

Run through this final checklist:

| Check | Expected |
|---|---|
| Block preset → Start | Score 4, immediate end |
| Blinker preset → Start | Score 9, 2 steps |
| Glider preset on 20×20 → Start | Ends when glider returns to start position |
| R-pentomino on 60×60 → Start | Runs many steps, high score |
| Population graph is visible and non-empty | Yes |
| Play again returns to editing with blank grid | Yes |
| All existing `cargo test` still pass | Yes |

When this checklist is fully green, the game described in `PLAN_rules.md` is fully implemented.

---

## Quick reference — file map after all 5 steps

```
src/
  lib.rs                    — pub mod sim
  main.rs                   — event loop entry point
  app.rs                    — App, AppInner, AppPhase, EndState, screen_to_cell
  input.rs                  — AppAction enum and key mapping
  render/
    mod.rs                  — Renderer (wgpu), draw_grid (egui Painter)
  sim/
    mod.rs                  — Grid, Hash impl
    session.rs              — GameSession, GameResult
    patterns.rs             — PRESETS, place()
tests/
  sim_tests.rs              — integration tests (no GPU)
```
