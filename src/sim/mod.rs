/// Game of Life simulation — Grid storage and Conway's rules.
///
/// INVARIANT: this module must never import platform crates (rendering, windowing, or GUI).

pub mod patterns;
pub mod session;

#[derive(Clone)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub(crate) cells: Vec<bool>,
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

impl std::hash::Hash for Grid {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.width.hash(state);
        self.height.hash(state);
        self.cells.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_are_stored() {
        let g = Grid::new(10, 20);
        assert_eq!(g.width, 10);
        assert_eq!(g.height, 20);
    }

    #[test]
    fn cell_count_is_width_times_height() {
        let g = Grid::new(4, 8);
        assert_eq!(g.cell_count(), 32);
    }

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

    // Helper: set a list of (x,y) cells alive on a grid.
    fn seed(g: &mut Grid, cells: &[(usize, usize)]) {
        for &(x, y) in cells { g.set(x, y, true); }
    }

    #[test]
    fn block_is_a_still_life() {
        // 2x2 square — the simplest still life.
        let mut g = Grid::new(10, 10);
        seed(&mut g, &[(4,4),(5,4),(4,5),(5,5)]);
        let before: Vec<bool> = g.cells.clone();
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

    #[test]
    fn toroidal_wrap_top_and_bottom() {
        // Cell at top row (y=0): accessing y=5 (one past bottom) wraps back to top.
        let mut g = Grid::new(5, 5);
        g.set(2, 0, true);
        assert!(g.get(2, 5));  // one past bottom wraps to top (row 0)

        // Cell at bottom row (y=4): accessing y=-1 (one above top) wraps to bottom.
        let mut g2 = Grid::new(5, 5);
        g2.set(2, 4, true);
        assert!(g2.get(2, -1)); // -1 rem_euclid 5 = 4. Correct.
    }
}
