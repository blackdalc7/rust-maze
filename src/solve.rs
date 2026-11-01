//! Solving mazes: breadth-first search and A*.

use crate::maze::{Cell, Maze};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// Which search algorithm to solve with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Solver {
    Bfs,
    AStar,
}

impl Solver {
    /// Parse a command line name (`bfs`, `astar` or `a*`), ignoring case.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "bfs" => Some(Self::Bfs),
            "astar" | "a*" => Some(Self::AStar),
            _ => None,
        }
    }
}

/// A solved path and the search that found it.
pub struct Solution {
    /// Cells from start to goal, inclusive.
    pub path: Vec<Cell>,
    /// Every cell the algorithm looked at, in order (used to visualise the search).
    pub explored: Vec<Cell>,
}

/// Solve from `start` to `goal` with `solver`, or `None` if the goal is unreachable.
pub fn solve(maze: &Maze, start: Cell, goal: Cell, solver: Solver) -> Option<Solution> {
    match solver {
        Solver::Bfs => bfs(maze, start, goal),
        Solver::AStar => astar(maze, start, goal),
    }
}

fn index(maze: &Maze, (x, y): Cell) -> usize {
    y * maze.width + x
}

fn rebuild(maze: &Maze, came_from: &[Option<Cell>], start: Cell, goal: Cell) -> Vec<Cell> {
    let mut path = vec![goal];
    let mut current = goal;
    while current != start {
        current = came_from[index(maze, current)].expect("broken path");
        path.push(current);
    }
    path.reverse();
    path
}

/// Breadth-first search: explores outwards in rings and always finds a shortest path.
pub fn bfs(maze: &Maze, start: Cell, goal: Cell) -> Option<Solution> {
    let mut came_from = vec![None; maze.width * maze.height];
    let mut seen = vec![false; maze.width * maze.height];
    let mut queue = VecDeque::from([start]);
    let mut explored = Vec::new();
    seen[index(maze, start)] = true;
    while let Some(cell) = queue.pop_front() {
        explored.push(cell);
        if cell == goal {
            return Some(Solution {
                path: rebuild(maze, &came_from, start, goal),
                explored,
            });
        }
        for next in maze.open_neighbours(cell) {
            let i = index(maze, next);
            if !seen[i] {
                seen[i] = true;
                came_from[i] = Some(cell);
                queue.push_back(next);
            }
        }
    }
    None
}

fn manhattan((ax, ay): Cell, (bx, by): Cell) -> usize {
    ax.abs_diff(bx) + ay.abs_diff(by)
}

/// A* with the Manhattan distance heuristic: a shortest path, usually exploring fewer cells than BFS.
pub fn astar(maze: &Maze, start: Cell, goal: Cell) -> Option<Solution> {
    let n = maze.width * maze.height;
    let mut came_from = vec![None; n];
    let mut cost = vec![usize::MAX; n];
    let mut closed = vec![false; n];
    let mut explored = Vec::new();
    // BinaryHeap is a max-heap, so wrap in Reverse to pop the smallest f = g + h first.
    // The tie-breaker prefers larger g (deeper cells), which explores fewer cells in mazes.
    let mut open = BinaryHeap::new();
    cost[index(maze, start)] = 0;
    open.push(Reverse((manhattan(start, goal), Reverse(0usize), start)));
    while let Some(Reverse((_, _, cell))) = open.pop() {
        let i = index(maze, cell);
        if closed[i] {
            continue;
        }
        closed[i] = true;
        explored.push(cell);
        if cell == goal {
            return Some(Solution {
                path: rebuild(maze, &came_from, start, goal),
                explored,
            });
        }
        for next in maze.open_neighbours(cell) {
            let j = index(maze, next);
            let g = cost[i] + 1;
            if g < cost[j] {
                cost[j] = g;
                came_from[j] = Some(cell);
                open.push(Reverse((
                    g + manhattan(next, goal),
                    Reverse(usize::MAX - g),
                    next,
                )));
            }
        }
    }
    None
}

/// How many cells can be reached from `start`.
pub fn reachable(maze: &Maze, start: Cell) -> usize {
    let mut seen = vec![false; maze.width * maze.height];
    let mut stack = vec![start];
    seen[index(maze, start)] = true;
    let mut count = 0;
    while let Some(cell) = stack.pop() {
        count += 1;
        for next in maze.open_neighbours(cell) {
            let i = index(maze, next);
            if !seen[i] {
                seen[i] = true;
                stack.push(next);
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maze::{Algorithm, E, S};
    use crate::rng::Rng;

    #[test]
    fn straight_corridor() {
        let mut m = Maze::closed(4, 1);
        m.carve((0, 0), E);
        m.carve((1, 0), E);
        m.carve((2, 0), E);
        for solver in [Solver::Bfs, Solver::AStar] {
            let s = solve(&m, (0, 0), (3, 0), solver).unwrap();
            assert_eq!(s.path, vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
        }
    }

    #[test]
    fn no_path_returns_none() {
        let mut m = Maze::closed(3, 3);
        m.carve((0, 0), S);
        assert!(bfs(&m, (0, 0), (2, 2)).is_none());
        assert!(astar(&m, (0, 0), (2, 2)).is_none());
        assert_eq!(reachable(&m, (0, 0)), 2);
    }

    #[test]
    fn start_equals_goal() {
        let m = Maze::closed(2, 2);
        assert_eq!(bfs(&m, (1, 1), (1, 1)).unwrap().path, vec![(1, 1)]);
    }

    #[test]
    fn bfs_and_astar_agree_on_length_and_path_is_valid() {
        for seed in 0..15 {
            let m = Maze::generate(30, 20, Algorithm::Prim, &mut Rng::new(seed));
            let goal = (29, 19);
            let a = bfs(&m, (0, 0), goal).unwrap();
            let b = astar(&m, (0, 0), goal).unwrap();
            assert_eq!(a.path.len(), b.path.len(), "seed {seed}");
            for pair in b.path.windows(2) {
                assert!(
                    m.open_neighbours(pair[0]).any(|c| c == pair[1]),
                    "path walks through a wall"
                );
            }
        }
    }

    #[test]
    fn astar_explores_less_than_bfs_on_average() {
        let (mut bfs_total, mut astar_total) = (0, 0);
        for seed in 0..20 {
            let m = Maze::generate(40, 40, Algorithm::Prim, &mut Rng::new(seed));
            bfs_total += bfs(&m, (0, 0), (39, 39)).unwrap().explored.len();
            astar_total += astar(&m, (0, 0), (39, 39)).unwrap().explored.len();
        }
        assert!(
            astar_total < bfs_total,
            "A* {astar_total} vs BFS {bfs_total}"
        );
    }

    #[test]
    fn solver_names() {
        assert_eq!(Solver::parse("BFS"), Some(Solver::Bfs));
        assert_eq!(Solver::parse("a*"), Some(Solver::AStar));
        assert_eq!(Solver::parse("dijkstra"), None);
    }
}
