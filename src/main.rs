//! rust-maze: generate, solve and draw mazes.

mod maze;
mod render;
mod rng;
mod solve;

use maze::{Algorithm, Maze};
use rng::Rng;
use solve::Solver;
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const HELP: &str = "\
rust-maze - generate and solve mazes

USAGE:
    rust-maze [OPTIONS]

OPTIONS:
    -w, --width <N>      maze width in cells        [default: 20]
    -h, --height <N>     maze height in cells       [default: 10]
    -s, --seed <N>       random seed                [default: current time]
    -a, --algo <NAME>    backtracker | prim         [default: backtracker]
        --solve <NAME>   bfs | astar | none         [default: none]
        --svg <FILE>     also write an SVG picture
        --explored       shade the cells the solver looked at (SVG only)
        --quiet          don't print the maze, only stats
        --help           show this message
";

#[derive(Debug)]
struct Options {
    width: usize,
    height: usize,
    seed: u64,
    algorithm: Algorithm,
    solver: Option<Solver>,
    svg: Option<String>,
    explored: bool,
    quiet: bool,
}

fn parse_args(args: &[String]) -> Result<Option<Options>, String> {
    let mut o = Options {
        width: 20,
        height: 10,
        seed: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0),
        algorithm: Algorithm::Backtracker,
        solver: None,
        svg: None,
        explored: false,
        quiet: false,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg.as_str() {
            "--help" => return Ok(None),
            "-w" | "--width" => o.width = number(&value(arg)?, arg)?,
            "-h" | "--height" => o.height = number(&value(arg)?, arg)?,
            "-s" | "--seed" => {
                o.seed = value(arg)?
                    .parse()
                    .map_err(|_| "seed must be a number".to_string())?
            }
            "-a" | "--algo" => {
                let v = value(arg)?;
                o.algorithm = Algorithm::parse(&v).ok_or(format!("unknown algorithm {v:?}"))?;
            }
            "--solve" => {
                let v = value(arg)?;
                o.solver = if v == "none" {
                    None
                } else {
                    Some(Solver::parse(&v).ok_or(format!("unknown solver {v:?}"))?)
                };
            }
            "--svg" => o.svg = Some(value(arg)?),
            "--explored" => o.explored = true,
            "--quiet" => o.quiet = true,
            other => return Err(format!("unknown option {other:?} (try --help)")),
        }
    }
    Ok(Some(o))
}

fn number(s: &str, flag: &str) -> Result<usize, String> {
    match s.parse::<usize>() {
        Ok(n) if (1..=2000).contains(&n) => Ok(n),
        _ => Err(format!("{flag} must be a number from 1 to 2000")),
    }
}

fn run(o: &Options) -> Result<String, String> {
    let mut out = String::new();
    let t0 = Instant::now();
    let maze = Maze::generate(o.width, o.height, o.algorithm, &mut Rng::new(o.seed));
    let generated = t0.elapsed();

    let goal = (o.width - 1, o.height - 1);
    let solution = o.solver.map(|s| {
        let t = Instant::now();
        let sol = solve::solve(&maze, (0, 0), goal, s).expect("perfect mazes always have a path");
        (s, sol, t.elapsed())
    });
    let path = solution
        .as_ref()
        .map(|(_, s, _)| s.path.as_slice())
        .unwrap_or(&[]);

    if !o.quiet {
        out.push_str(&render::ascii(&maze, path));
    }
    let cells = o.width * o.height;
    let perfect = maze.passages() == cells - 1 && solve::reachable(&maze, (0, 0)) == cells;
    out.push_str(&format!(
        "{}x{} {:?} maze, seed {}, {} dead ends, perfect: {}, generated in {:.2?}\n",
        o.width,
        o.height,
        o.algorithm,
        o.seed,
        maze.dead_ends(),
        if perfect { "yes" } else { "no" },
        generated
    ));
    if let Some((s, sol, took)) = &solution {
        out.push_str(&format!(
            "{s:?}: path length {}, explored {} of {} cells ({:.0}%) in {took:.2?}\n",
            sol.path.len(),
            sol.explored.len(),
            o.width * o.height,
            100.0 * sol.explored.len() as f64 / (o.width * o.height) as f64
        ));
    }
    if let Some(file) = &o.svg {
        let explored = match (&solution, o.explored) {
            (Some((_, sol, _)), true) => sol.explored.as_slice(),
            _ => &[],
        };
        let svg = render::svg(&maze, path, explored, &render::SvgStyle::default());
        std::fs::write(file, svg).map_err(|e| format!("couldn't write {file}: {e}"))?;
        out.push_str(&format!("wrote {file}\n"));
    }
    Ok(out)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args).and_then(|o| o.map(|o| run(&o)).transpose()) {
        Ok(Some(text)) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Ok(None) => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn defaults() {
        let o = parse_args(&[]).unwrap().unwrap();
        assert_eq!((o.width, o.height), (20, 10));
        assert_eq!(o.algorithm, Algorithm::Backtracker);
        assert!(o.solver.is_none());
    }

    #[test]
    fn all_options() {
        let o = parse_args(&args(
            "-w 30 --height 12 -s 7 -a prim --solve astar --svg m.svg --explored --quiet",
        ))
        .unwrap()
        .unwrap();
        assert_eq!((o.width, o.height, o.seed), (30, 12, 7));
        assert_eq!(o.algorithm, Algorithm::Prim);
        assert_eq!(o.solver, Some(Solver::AStar));
        assert_eq!(o.svg.as_deref(), Some("m.svg"));
        assert!(o.explored && o.quiet);
        assert!(parse_args(&args("--solve none"))
            .unwrap()
            .unwrap()
            .solver
            .is_none());
    }

    #[test]
    fn help_and_errors() {
        assert!(parse_args(&args("--help")).unwrap().is_none());
        for bad in [
            "-w",
            "-w 0",
            "-w abc",
            "-s x",
            "-a kruskal",
            "--solve dijkstra",
            "--fast",
        ] {
            assert!(parse_args(&args(bad)).is_err(), "{bad} should fail");
        }
    }

    #[test]
    fn run_prints_maze_and_stats() {
        let o = parse_args(&args("-w 6 -h 4 -s 1 --solve bfs"))
            .unwrap()
            .unwrap();
        let out = run(&o).unwrap();
        assert!(out.contains(" o "));
        assert!(out.contains("6x4 Backtracker maze, seed 1"));
        assert!(out.contains("perfect: yes"));
        assert!(out.contains("Bfs: path length"));
    }

    #[test]
    fn run_writes_svg() {
        let file = std::env::temp_dir().join("rust-maze-test.svg");
        let path = file.to_str().unwrap();
        let o = parse_args(&args(&format!(
            "-w 5 -h 5 -s 2 --solve astar --svg {path} --explored --quiet"
        )))
        .unwrap()
        .unwrap();
        let out = run(&o).unwrap();
        assert!(!out.contains('+'), "quiet mode shouldn't print the maze");
        let svg = std::fs::read_to_string(&file).unwrap();
        assert!(svg.contains("<polyline"));
        std::fs::remove_file(file).ok();
    }

    #[test]
    fn run_reports_unwritable_svg() {
        let o = parse_args(&args("-w 2 -h 2 -s 1 --svg /no/such/dir/m.svg --quiet"))
            .unwrap()
            .unwrap();
        assert!(run(&o).unwrap_err().contains("couldn't write"));
    }
}
