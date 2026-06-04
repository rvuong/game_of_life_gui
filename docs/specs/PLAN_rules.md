# Game of Life — Rules and Implementation Plan

> Status: APPROVED
> Scope: Game rules, scoring, and a phased implementation guide for the developer.

---

## Part 1 — The Game Rules

### 1.1 What the player sees

The player opens the application and sees a **grid of empty cells**. They click on cells to fill them, then press **Start** to watch the simulation run. The simulation stops automatically when either a pattern repeats or the step limit is reached. A score is displayed.

### 1.2 Grid

- **Sizes available:** 20×20 (default), 40×40, 60×60.
- **Topology: toroidal.** There are no edges. Think of the old Snake or Pac-Man games: when a pattern goes off the right side of the screen, it reappears from the left. Same for top and bottom. Every cell always has exactly 8 neighbours — no cell is ever "near a wall."

### 1.3 Conway's Rules (applied every step)

Each cell is either **alive** or **dead**. Every step, the entire grid updates simultaneously according to these four rules, which are checked for every cell at the same time:

1. A **live** cell with **fewer than 2** live neighbours → **dies** (loneliness).
2. A **live** cell with **2 or 3** live neighbours → **stays alive**.
3. A **live** cell with **more than 3** live neighbours → **dies** (overcrowding).
4. A **dead** cell with **exactly 3** live neighbours → **comes to life** (reproduction).

"Neighbours" means the 8 cells surrounding a cell (horizontally, vertically, and diagonally). On a toroidal grid, neighbours wrap around.

**Important:** the new state of every cell is computed from the *current* state of the grid, then all cells update at once. You never use the already-updated neighbours to compute a cell's next state within the same step.

### 1.4 Step timing

- One step = one full application of the rules to the whole grid.
- Default speed: **300ms per step**.
- The player can adjust speed between **50ms** (very fast) and **2000ms** (slow).

### 1.5 Termination conditions

The simulation stops as soon as **either** of these happens:

- **Cycle detected:** the current grid state is identical to a state that already occurred earlier in this run. The simulation has entered a loop — nothing new will ever happen.
- **Step cap reached:** the run has lasted the configured maximum number of steps without cycling. Default cap: **300 steps**, configurable from **100 to 10,000**.

The player cannot trigger a game over manually. The simulation always runs to its natural conclusion.

### 1.6 Scoring

At game over, the score is:

```
score = (N + 1)²
```

where **N is the number of unique grid states** visited during the run, starting from the initial placement (step 0) up to but not including the repeated state.

Examples:

| What happened | N | Score |
|---|---|---|
| Still life detected immediately (the pattern does not move) | 1 | 4 |
| Blinker (period-2 oscillator) detected | 2 | 9 |
| Pattern evolves through 50 unique states then dies out | 50 | 2,601 |
| Pattern evolves through 299 unique states then cap is reached | 300 | 90,601 |

The formula rewards both strategies: a pattern that loops with a high period, and a pattern that evolves through a long transient before dying or stabilising. Extinction (all cells die) is not a special case — it is simply one more state in the sequence, and the empty grid is detected as a repeated state on the following step.

### 1.7 Controls summary

| Control | Action |
|---|---|
| Click on a cell (before Start) | Toggle cell alive / dead |
| Start button | Begin simulation |
| Restart button | Return to blank grid, same settings |
| Speed slider or +/- keys | Adjust step duration |
| Grid size selector | Choose 20×20, 40×40, or 60×60 |
| Step cap input | Configure maximum steps (100–10,000) |

### 1.8 Post-game display

When the simulation ends, the application shows:
- The **final score**.
- The **reason for ending** (cycle detected / cap reached / extinction).
- A **population graph**: a simple line showing the number of live cells at each step. This helps the player understand what their pattern did over time.
- A **Play again** button (same grid, same settings, blank cells).

### 1.9 Preset patterns

The player can optionally load one of 3–5 built-in starting patterns instead of drawing from scratch. These are small, well-known GoL configurations that demonstrate interesting behaviours (oscillators, gliders, etc.). They are placed at the centre of the grid. The player can still modify the grid after loading a preset.

---

## Part 2 — Implementation Plan

The implementation is divided into **5 independent steps**. Each step produces something you can run and verify before moving to the next. No step requires knowledge of the next one.

---

### Step 1 — Grid storage and Conway's rules

**What you are building:** the brain of the simulation. A `Grid` struct that holds cell states and a `tick()` function that advances the grid by one step.

**Why this step comes first:** everything else in the game — rendering, scoring, cycle detection — depends on having a correct simulation. Build and test this in complete isolation from the screen, the mouse, and the timer.

**Where this code lives:** entirely inside `src/sim/`. No imports from `winit`, `wgpu`, or `egui` are allowed here (the CI check enforces this).

**What to implement:**

1. **Grid storage.** A flat `Vec<bool>` of size `width × height`. Cell at column `x`, row `y` is stored at index `y * width + x`. Start with `bool` for clarity; you can optimise to a bitset later if needed.

2. **Toroidal indexing.** A helper that, given coordinates `(x, y)` that might be out of bounds, wraps them back into the grid. In Rust: `x.rem_euclid(width)`. This must be used every time you look up a neighbour.

3. **Neighbour count.** A function that, for a given `(x, y)`, counts how many of its 8 surrounding cells are alive. Use toroidal indexing for all 8 lookups.

4. **`tick()`.** Allocate a new `Vec<bool>` of the same size. For each cell, count its neighbours, apply the four Conway rules, and write the result into the new buffer. Replace the grid's internal buffer with the new one. Never read from the new buffer while still computing it.

5. **Grid sizes.** The `Grid` struct stores its own `width` and `height`. Constructing a 20×20 grid and a 40×40 grid should work identically.

**How to verify it works:**

Write unit tests in `src/sim/mod.rs` for the following known patterns. These are the canonical sanity checks for any GoL implementation:

- **Block (still life):** a 2×2 square of live cells. After one `tick()`, the grid must be identical to before.
- **Blinker (period-2 oscillator):** three cells in a horizontal line. After one tick, they form a vertical line. After a second tick, they are horizontal again.
- **Glider:** a 5-cell pattern that moves diagonally. After 4 ticks on a toroidal grid, it must be back to its original shape but shifted one cell diagonally.

If all three tests pass, your simulation is correct.

---

### Step 2 — Cycle detection and scoring

**What you are building:** the logic that decides when the game ends and what the player's score is.

**Why this step is separate from Step 1:** cycle detection is a pure algorithm applied on top of the simulation. It has no rendering logic, no timers. Keeping it separate makes it easy to test with known patterns.

**Where this code lives:** a new file `src/sim/cycle.rs` (or directly in `src/sim/mod.rs` if kept small). Still no platform imports.

**Key concept — hashing a grid state.** To check whether a grid state has been seen before, you convert the entire grid into a single number (a hash). Two identical grids produce the same hash; two different grids almost certainly produce different hashes. In Rust, you can implement `Hash` on `Grid` by hashing its `Vec<bool>` buffer, then store these hashes in a `HashSet<u64>`.

**What to implement:**

1. **`GameSession` struct.** Wraps a `Grid` and holds:
   - A `HashSet<u64>` of all hashes seen so far.
   - A `Vec<u32>` called `population_history` that records the number of live cells at each step.
   - A `step_count: u32` counter.
   - A `max_steps: u32` cap (configurable, default 300).

2. **`advance()` method on `GameSession`.** This is the single method that drives the game:
   - Hash the current grid state. If the hash is already in the set → return `GameOver::CycleDetected`.
   - Insert the hash into the set.
   - Record the current live cell count into `population_history`.
   - Call `grid.tick()`.
   - Increment `step_count`. If `step_count >= max_steps` → return `GameOver::CapReached`.
   - Otherwise return `GameRunning`.

3. **`score()` method.** Returns `(unique_states + 1).pow(2)` as a `u64`. `unique_states` is simply `self.seen_states.len()`.

4. **`GameOver` enum.** Three variants: `CycleDetected`, `CapReached`, `StillRunning`.

**How to verify it works:**

- Test with a **block**: `advance()` should return `CycleDetected` on the very first call (the initial state and the post-tick state are the same → the hash is already in the set after step 0). Score = `(1 + 1)² = 4`.
- Test with a **blinker**: `CycleDetected` after 2 calls. Score = `(2 + 1)² = 9`.
- Test with a **glider** on a 20×20 toroidal grid and a cap of 1000: the glider cycles back to its start position after a known number of steps. Verify the step count and score match expectations.
- Test with `max_steps = 5` and a glider: `advance()` must return `CapReached` at step 5.

---

### Step 3 — Grid rendering and cell editing

**What you are building:** the visual grid on screen and the ability to click cells to toggle them before the simulation starts.

**Why this step is independent from Steps 1 and 2:** rendering depends on `Grid` (for the cell state) but not on `GameSession`. You can build and test this step by simply creating a `Grid`, toggling cells by hand in code, and verifying the display looks right. No timer, no scoring, no cycle detection needed yet.

**Where this code lives:** `src/render/mod.rs` (new rendering pass for the grid) and `src/app.rs` (mouse handling and app state).

**Key concept — world coordinates vs. screen coordinates.** The mouse gives you a pixel position on the screen. You need to convert that to a cell column and row. This conversion depends on how large each cell is drawn (e.g., each cell = 20 pixels), and where the grid is positioned. This mapping function belongs in `app.rs` and will be reused in the simulation phase.

**What to implement:**

1. **Cell rendering pass.** In `src/render/mod.rs`, add a method that receives the grid state and draws a rectangle for each cell. Dead cells: dark grey. Live cells: white (or a bright accent colour). The egui grid can be drawn using `egui::Painter` with `rect_filled` calls, which avoids writing custom shaders for this phase.

2. **Grid size UI.** Three buttons or a dropdown: 20×20, 40×40, 60×60. Changing the grid size resets the grid to all-dead and resizes the rendered area. This sits in the egui panel.

3. **Mouse click → cell toggle.** In `app.rs`, when the application is in `EditMode` (before Start is pressed) and the player clicks, convert the screen position to a cell `(x, y)` and call a `toggle_cell(x, y)` method on the grid.

4. **App state machine.** `app.rs` now needs to track which phase the game is in:
   - `EditMode`: player is placing cells. Show the grid, a Start button, the grid size selector.
   - `RunMode`: simulation is running. Show the grid (live, updating), step count, current live cell count, speed control.
   - `EndMode`: simulation has ended. Show the final grid, final score, reason for ending, population graph, Play again button.

5. **Start and Restart buttons.** Start transitions `EditMode → RunMode` and creates a `GameSession`. Restart from any state returns to `EditMode` with a blank grid.

**How to verify it works:**

Run the application. You should be able to:
- Click cells to make them appear and disappear.
- Switch grid sizes and see the grid reset.
- Press Start and see the state transition (even if the simulation does not tick yet — that comes in Step 4).

---

### Step 4 — Game loop and speed control

**What you are building:** the timer that drives the simulation forward, the speed control, and the wiring that connects `GameSession::advance()` to the screen and to the end-of-game transition.

**Why this step is separate:** the game logic (Steps 1–2) and the rendering (Step 3) are both complete. This step is purely about *time* — advancing the simulation at the right pace and reacting to the result.

**Where this code lives:** `src/app.rs`. The `about_to_wait` callback (which winit calls every frame) is where you check whether enough time has elapsed to advance one step.

**Key concept — frame-based timing.** The application does not have a "sleep for 300ms" anywhere. Instead, every time winit gives you a frame (the `about_to_wait` event), you check the system clock. If enough time has passed since the last tick, you call `advance()`. Otherwise you do nothing. This keeps the window responsive at all times (it can still handle mouse clicks, resize, and quit while waiting for the next tick).

In Rust: `std::time::Instant::now()` gives the current time. Store `last_tick: Instant` in the app state and compare with `last_tick.elapsed() >= step_duration`.

**What to implement:**

1. **Elapsed-time tick in `about_to_wait`.** Check the clock. If enough time has passed and the app is in `RunMode`, call `game_session.advance()`.

2. **Handle the result of `advance()`.**
   - `StillRunning` → update the display (live cell count, step number) and wait for the next tick.
   - `CycleDetected` or `CapReached` → transition the app to `EndMode` and record the reason.

3. **Speed control.** Add a slider (egui) or keyboard shortcuts (`+` / `-`) that change `step_duration`. Range: 50ms to 2000ms. The change takes effect immediately on the next elapsed-time check.

4. **Live display during `RunMode`.** Show the current step number and the number of live cells. This uses `game_session.step_count` and the last entry in `population_history`.

**How to verify it works:**

Place a blinker on a 20×20 grid, press Start, and watch. You should see:
- The blinker flipping between horizontal and vertical every 300ms.
- The step counter incrementing.
- After 2 steps, the game transitions to `EndMode` with score 9 and reason "Cycle detected."

Try the speed control — at 50ms/step the blinker should flicker almost invisibly fast; at 2000ms it should feel very slow.

---

### Step 5 — Preset patterns and post-game display

**What you are building:** the finishing touches. Three built-in patterns the player can load, and the post-game screen with the population graph.

**Why this step is last:** it requires no new architectural decisions. Steps 1–4 have built everything the game needs to function correctly. This step is about making the game more welcoming and more informative.

**Where this code lives:** pattern data in `src/sim/patterns.rs`. Post-game graph in `src/app.rs` using egui's built-in `egui::plot` widget.

**What to implement:**

1. **Pattern data.** Create a file `src/sim/patterns.rs` that defines 3 to 5 patterns as static arrays of `(i32, i32)` offsets from a centre point. Suggested patterns:
   - **Block** — 2×2 still life. Period 1.
   - **Blinker** — 3 cells in a line. Period 2. Shows the simplest oscillator.
   - **Glider** — 5-cell pattern. Travels diagonally. Period 4 on an infinite grid.
   - **Pulsar** — a large, beautiful period-3 oscillator. Impressive on 40×40 or larger.
   - **R-pentomino** — a chaotic 5-cell pattern that takes 1103 steps to stabilise. On a large enough grid this will run for a long time.

   Each pattern is placed by computing `(centre_x + offset_x) % width` and `(centre_y + offset_y) % height` (toroidal placement). This ensures no pattern can be placed out of bounds.

2. **Preset buttons in `EditMode`.** In the egui panel, add one button per pattern. Clicking a button clears the grid and places the pattern at the centre. The player can still edit the grid afterwards.

3. **Population graph in `EndMode`.** `game_session.population_history` is a `Vec<u32>` with one entry per step. Use `egui_plot::Plot` with a `Line` series to display it. Label the axes: x = step number, y = live cells. This gives the player instant visual feedback on how their pattern evolved.

4. **End-of-game summary panel.** Show:
   - Score: `(N + 1)²` in large text.
   - Unique states visited: `N`.
   - Steps elapsed.
   - Reason: "Cycle detected" / "Cap reached" / "Extinction" (extinction is the specific case where the last unique state was all-dead).
   - A **Play again** button that returns to `EditMode` with a blank grid and the same settings.

**How to verify it works:**

- Load the R-pentomino preset on a 40×40 grid with a cap of 500. Press Start and let it run. The population graph should show wild variation before stabilising. The score should be noticeably higher than a blinker.
- Load the Block preset. Press Start. Score should be 4, step count 1, reason "Cycle detected."
- Verify that "Play again" resets the grid to blank and returns to the editing phase.

---

## Summary table

| Step | What it builds | Depends on | Deliverable you can demo |
|---|---|---|---|
| 1 | Grid + Conway's rules | nothing | `cargo test` — all unit tests green |
| 2 | Cycle detection + scoring | Step 1 | `cargo test` — blinker scores 9, block scores 4 |
| 3 | Grid rendering + cell editing | Step 1 | Click cells, see them toggle on screen |
| 4 | Game loop + speed control | Steps 1, 2, 3 | Full playable game, no presets or graph |
| 5 | Presets + post-game display | Steps 1–4 | Complete, polished game |
