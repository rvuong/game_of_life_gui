use crate::sim::Grid;

pub const BLOCK: &[(i32, i32)] = &[(0, 0), (1, 0), (0, 1), (1, 1)];

pub const BLINKER: &[(i32, i32)] = &[(-1, 0), (0, 0), (1, 0)];

pub const GLIDER: &[(i32, i32)] = &[(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];

pub const PULSAR: &[(i32, i32)] = &[
    (-4, -6), (-3, -6), (-2, -6), (2, -6), (3, -6), (4, -6),
    (-4, -1), (-3, -1), (-2, -1), (2, -1), (3, -1), (4, -1),
    (-4,  1), (-3,  1), (-2,  1), (2,  1), (3,  1), (4,  1),
    (-4,  6), (-3,  6), (-2,  6), (2,  6), (3,  6), (4,  6),
    (-6, -4), (-6, -3), (-6, -2), (-6,  2), (-6,  3), (-6,  4),
    (-1, -4), (-1, -3), (-1, -2), (-1,  2), (-1,  3), (-1,  4),
    ( 1, -4), ( 1, -3), ( 1, -2), ( 1,  2), ( 1,  3), ( 1,  4),
    ( 6, -4), ( 6, -3), ( 6, -2), ( 6,  2), ( 6,  3), ( 6,  4),
];

pub const R_PENTOMINO: &[(i32, i32)] = &[(0, -1), (1, -1), (-1, 0), (0, 0), (0, 1)];

pub struct PatternDef {
    pub name: &'static str,
    pub cells: &'static [(i32, i32)],
    pub description: &'static str,
}

pub const PRESETS: &[PatternDef] = &[
    PatternDef { name: "Block",       cells: BLOCK,       description: "Still life. Score: 4." },
    PatternDef { name: "Blinker",     cells: BLINKER,     description: "Period-2 oscillator. Score: 9." },
    PatternDef { name: "Glider",      cells: GLIDER,      description: "Travels diagonally." },
    PatternDef { name: "Pulsar",      cells: PULSAR,      description: "Period-3 oscillator. Requires 40×40." },
    PatternDef { name: "R-pentomino", cells: R_PENTOMINO, description: "Chaotic. Try on 60×60 with max 1000 steps." },
];

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn pulsar_places_48_live_cells() {
        let mut g = Grid::new(20, 20);
        place(PULSAR, &mut g);
        assert_eq!(g.live_cell_count(), 48);
    }

    #[test]
    fn place_centres_on_grid() {
        let mut g = Grid::new(20, 20);
        place(&[(0, 0)], &mut g);
        assert!(g.get(10, 10));
    }

    #[test]
    fn place_wraps_toroidally() {
        let mut g = Grid::new(5, 5);
        place(&[(-3, 0)], &mut g);
        // cx=2, cy=2: x=(2-3).rem_euclid(5)=4, y=2
        assert!(g.get(4, 2));
    }
}
