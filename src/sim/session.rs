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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_result_variants_exist() {
        let _ = GameResult::StillRunning;
        let _ = GameResult::CycleDetected;
        let _ = GameResult::CapReached;
    }

    #[test]
    fn new_session_starts_at_step_zero() {
        let g = Grid::new(10, 10);
        let session = GameSession::new(g, 300);
        assert_eq!(session.step_count, 0);
        assert_eq!(session.max_steps, 300);
        // The initial state is already hashed and stored.
        assert_eq!(session.population_history.len(), 1);
    }

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
}
