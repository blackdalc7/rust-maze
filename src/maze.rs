//! The maze grid and two generation algorithms.
//!
//! Each cell stores which of its four walls are open as bit flags.

use crate::rng::Rng;

pub const N: u8 = 1;
pub const E: u8 = 2;
pub const S: u8 = 4;
pub const W: u8 = 8;

pub type Cell = (usize, usize); // (x, y)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algorithm {
    /// Depth-first "recursive backtracker": long, winding corridors.
    Backtracker,
    /// Randomised Prim's: lots of short dead ends, feels more "bushy".
    Prim,
}

impl Algorithm {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "backtracker" | "dfs" => Some(Self::Backtracker),
            "prim" | "prims" => Some(Self::Prim),
            _ => None,
        }
    }
}

pub struct Maze {
    pub width: usize,
    pub height: usize,
    cells: Vec<u8>,
}

impl Maze {
    /// A grid where every wall is closed.
    pub fn closed(width: usize, height: usize) -> Self {
        assert!(width > 0 && height > 0, "maze must be at least 1x1");
        Maze {
            width,
            height,
            cells: vec![0; width * height],
        }
    }

    pub fn generate(width: usize, height: usize, algorithm: Algorithm, rng: &mut Rng) -> Self {
        let mut maze = Maze::closed(width, height);
        match algorithm {
            Algorithm::Backtracker => maze.backtracker(rng),
            Algorithm::Prim => maze.prim(rng),
        }
        maze
    }

    fn idx(&self, (x, y): Cell) -> usize {
        y * self.width + x
    }

    pub fn is_open(&self, cell: Cell, dir: u8) -> bool {
        self.cells[self.idx(cell)] & dir != 0
    }

    /// The neighbour in a direction, if it's inside the grid.
    pub fn step(&self, (x, y): Cell, dir: u8) -> Option<Cell> {
        match dir {
            N if y > 0 => Some((x, y - 1)),
            S if y + 1 < self.height => Some((x, y + 1)),
            W if x > 0 => Some((x - 1, y)),
            E if x + 1 < self.width => Some((x + 1, y)),
            _ => None,
        }
    }

    /// Knock down the wall between a cell and its neighbour (both sides).
    pub fn carve(&mut self, from: Cell, dir: u8) -> Cell {
        let to = self.step(from, dir).expect("carving out of bounds");
        let (a, b) = (self.idx(from), self.idx(to));
        self.cells[a] |= dir;
        self.cells[b] |= opposite(dir);
        to
    }

    /// Cells you can walk to from here (no wall in between).
    pub fn open_neighbours(&self, cell: Cell) -> impl Iterator<Item = Cell> + '_ {
        [N, E, S, W]
            .into_iter()
            .filter(move |&d| self.is_open(cell, d))
            .filter_map(move |d| self.step(cell, d))
    }

    fn backtracker(&mut self, rng: &mut Rng) {
        // An explicit stack instead of recursion, so a 500x500 maze can't overflow the call stack.
        let mut visited = vec![false; self.width * self.height];
        let mut stack = vec![(0, 0)];
        visited[0] = true;
        while let Some(&current) = stack.last() {
            let mut dirs = [N, E, S, W];
            rng.shuffle(&mut dirs);
            let next = dirs.into_iter().find_map(|d| {
                self.step(current, d)
                    .filter(|&n| !visited[self.idx(n)])
                    .map(|n| (d, n))
            });
            match next {
                Some((dir, n)) => {
                    self.carve(current, dir);
                    visited[self.idx(n)] = true;
                    stack.push(n);
                }
                None => {
                    stack.pop();
                }
            }
        }
    }

    fn prim(&mut self, rng: &mut Rng) {
        let mut in_maze = vec![false; self.width * self.height];
        let mut frontier: Vec<(Cell, u8)> = Vec::new(); // (cell inside the maze, direction out)
        let start = (rng.below(self.width), rng.below(self.height));
        in_maze[self.idx(start)] = true;
        for d in [N, E, S, W] {
            frontier.push((start, d));
        }
        while !frontier.is_empty() {
            let pick = rng.below(frontier.len());
            let (cell, dir) = frontier.swap_remove(pick);
            let Some(next) = self.step(cell, dir) else {
                continue;
            };
            if in_maze[self.idx(next)] {
                continue;
            }
            self.carve(cell, dir);
            in_maze[self.idx(next)] = true;
            for d in [N, E, S, W] {
                frontier.push((next, d));
            }
        }
    }

    /// Count dead ends (cells with exactly one opening), a rough "texture" measure.
    pub fn dead_ends(&self) -> usize {
        (0..self.height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .filter(|&c| self.cells[self.idx(c)].count_ones() == 1)
            .count()
    }

    /// Number of openings carved. A perfect maze is a spanning tree, so this is cells - 1.
    pub fn passages(&self) -> usize {
        self.cells
            .iter()
            .map(|c| c.count_ones() as usize)
            .sum::<usize>()
            / 2
    }
}

pub fn opposite(dir: u8) -> u8 {
    match dir {
        N => S,
        S => N,
        E => W,
        W => E,
        _ => panic!("not a direction: {dir}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solve;

    fn all_cells(m: &Maze) -> Vec<Cell> {
        (0..m.height)
            .flat_map(|y| (0..m.width).map(move |x| (x, y)))
            .collect()
    }

    #[test]
    fn carving_opens_both_sides() {
        let mut m = Maze::closed(3, 3);
        let to = m.carve((1, 1), E);
        assert_eq!(to, (2, 1));
        assert!(m.is_open((1, 1), E));
        assert!(m.is_open((2, 1), W));
        assert!(!m.is_open((1, 1), N));
    }

    #[test]
    fn step_stays_inside_the_grid() {
        let m = Maze::closed(2, 2);
        assert_eq!(m.step((0, 0), N), None);
        assert_eq!(m.step((0, 0), W), None);
        assert_eq!(m.step((1, 1), S), None);
        assert_eq!(m.step((1, 1), E), None);
        assert_eq!(m.step((0, 0), E), Some((1, 0)));
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn carving_through_the_border_panics() {
        Maze::closed(2, 2).carve((0, 0), N);
    }

    #[test]
    fn both_algorithms_make_perfect_mazes() {
        // perfect = every cell reachable and exactly one path between any two cells,
        // which for a grid means: connected and exactly (cells - 1) passages
        for alg in [Algorithm::Backtracker, Algorithm::Prim] {
            for seed in 0..20 {
                let m = Maze::generate(17, 11, alg, &mut Rng::new(seed));
                assert_eq!(m.passages(), 17 * 11 - 1, "{alg:?} seed {seed}");
                let reachable = solve::reachable(&m, (0, 0));
                assert_eq!(reachable, all_cells(&m).len(), "{alg:?} seed {seed}");
            }
        }
    }

    #[test]
    fn same_seed_same_maze() {
        let a = Maze::generate(10, 10, Algorithm::Prim, &mut Rng::new(3));
        let b = Maze::generate(10, 10, Algorithm::Prim, &mut Rng::new(3));
        assert_eq!(a.cells, b.cells);
    }

    #[test]
    fn prim_has_more_dead_ends_than_backtracker() {
        let dfs = Maze::generate(40, 40, Algorithm::Backtracker, &mut Rng::new(1)).dead_ends();
        let prim = Maze::generate(40, 40, Algorithm::Prim, &mut Rng::new(1)).dead_ends();
        assert!(prim > dfs * 2, "prim {prim} vs dfs {dfs}");
    }

    #[test]
    fn big_maze_does_not_overflow_the_stack() {
        let m = Maze::generate(400, 400, Algorithm::Backtracker, &mut Rng::new(9));
        assert_eq!(m.passages(), 400 * 400 - 1);
    }

    #[test]
    fn one_by_one_maze() {
        let m = Maze::generate(1, 1, Algorithm::Prim, &mut Rng::new(0));
        assert_eq!(m.passages(), 0);
    }

    #[test]
    fn algorithm_names() {
        assert_eq!(Algorithm::parse("DFS"), Some(Algorithm::Backtracker));
        assert_eq!(Algorithm::parse("prim"), Some(Algorithm::Prim));
        assert_eq!(Algorithm::parse("kruskal"), None);
    }
}
