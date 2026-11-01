//! Drawing mazes as text and as SVG.

use crate::maze::{Cell, Maze, E, S};
use std::collections::HashSet;
use std::fmt::Write;

/// Text drawing. Each cell is 3 characters wide; `path` cells get a dot.
pub fn ascii(maze: &Maze, path: &[Cell]) -> String {
    let on_path: HashSet<Cell> = path.iter().copied().collect();
    let mut out = String::new();
    out.push('+');
    for _ in 0..maze.width {
        out.push_str("---+");
    }
    out.push('\n');
    for y in 0..maze.height {
        // cell row: the west border, then each cell's content and its east wall
        let mut row = String::from(if y == 0 { " " } else { "|" }); // entrance gap at top-left
        let mut below = String::from("+");
        for x in 0..maze.width {
            row.push_str(if on_path.contains(&(x, y)) {
                " o "
            } else {
                "   "
            });
            let exit = x == maze.width - 1 && y == maze.height - 1;
            row.push(if maze.is_open((x, y), E) || exit {
                ' '
            } else {
                '|'
            });
            below.push_str(if maze.is_open((x, y), S) {
                "   +"
            } else {
                "---+"
            });
        }
        out.push_str(&row);
        out.push('\n');
        out.push_str(&below);
        out.push('\n');
    }
    out
}

/// Cell size in pixels and the colours used for the SVG output.
pub struct SvgStyle {
    pub cell: usize,
    pub wall: &'static str,
    pub background: &'static str,
    pub path: &'static str,
    pub explored: &'static str,
}

impl Default for SvgStyle {
    fn default() -> Self {
        SvgStyle {
            cell: 14,
            wall: "#1f2937",
            background: "#fdfcf8",
            path: "#e8590c",
            explored: "#c5dcf5",
        }
    }
}

/// SVG drawing with optional explored cells (shaded) and a solution line.
pub fn svg(maze: &Maze, path: &[Cell], explored: &[Cell], style: &SvgStyle) -> String {
    let c = style.cell;
    let pad = c / 2;
    let (w, h) = (maze.width * c + 2 * pad, maze.height * c + 2 * pad);
    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#
    );
    let _ = writeln!(
        out,
        r#"<rect width="{w}" height="{h}" fill="{}"/>"#,
        style.background
    );

    if !explored.is_empty() {
        let _ = write!(out, r#"<g fill="{}">"#, style.explored);
        for &(x, y) in explored {
            let _ = write!(
                out,
                r#"<rect x="{}" y="{}" width="{c}" height="{c}"/>"#,
                pad + x * c,
                pad + y * c
            );
        }
        out.push_str("</g>\n");
    }

    if path.len() > 1 {
        let points: Vec<String> = path
            .iter()
            .map(|&(x, y)| format!("{},{}", pad + x * c + c / 2, pad + y * c + c / 2))
            .collect();
        let _ = writeln!(
            out,
            r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
            points.join(" "),
            style.path,
            (c as f32 * 0.35).max(2.0)
        );
    }

    // walls: the top and left borders, then each cell's south and east wall if closed
    let mut d = format!(
        "M{} {}H{}M{} {}V{}",
        pad + c, // leave an entrance at the top-left
        pad,
        pad + maze.width * c,
        pad,
        pad,
        pad + maze.height * c
    );
    for y in 0..maze.height {
        for x in 0..maze.width {
            let (x0, y0) = (pad + x * c, pad + y * c);
            if !maze.is_open((x, y), S) {
                let _ = write!(d, "M{} {}H{}", x0, y0 + c, x0 + c);
            }
            let exit = x == maze.width - 1 && y == maze.height - 1;
            if !maze.is_open((x, y), E) && !exit {
                let _ = write!(d, "M{} {}V{}", x0 + c, y0, y0 + c);
            }
        }
    }
    let _ = writeln!(
        out,
        r#"<path d="{d}" stroke="{}" stroke-width="2" stroke-linecap="square" fill="none"/>"#,
        style.wall
    );
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maze::{Algorithm, Maze};
    use crate::rng::Rng;
    use crate::solve;

    #[test]
    fn ascii_size_and_entrances() {
        let m = Maze::generate(5, 3, Algorithm::Backtracker, &mut Rng::new(1));
        let text = ascii(&m, &[]);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3 * 2 + 1);
        assert!(lines.iter().all(|l| l.len() == 5 * 4 + 1));
        assert!(lines[1].starts_with(' '), "entrance at top-left");
        assert!(lines[5].ends_with(' '), "exit at bottom-right");
    }

    #[test]
    fn ascii_marks_the_path() {
        let m = Maze::generate(6, 6, Algorithm::Prim, &mut Rng::new(2));
        let sol = solve::bfs(&m, (0, 0), (5, 5)).unwrap();
        let text = ascii(&m, &sol.path);
        assert_eq!(text.matches(" o ").count(), sol.path.len());
    }

    #[test]
    fn svg_has_walls_path_and_explored() {
        let m = Maze::generate(8, 8, Algorithm::Prim, &mut Rng::new(4));
        let sol = solve::astar(&m, (0, 0), (7, 7)).unwrap();
        let out = svg(&m, &sol.path, &sol.explored, &SvgStyle::default());
        assert!(out.starts_with("<svg"));
        assert!(out.trim_end().ends_with("</svg>"));
        assert!(out.contains("<polyline"));
        assert_eq!(out.matches("<rect x=").count(), sol.explored.len());
    }

    #[test]
    fn svg_without_solution() {
        let m = Maze::generate(3, 3, Algorithm::Prim, &mut Rng::new(4));
        let out = svg(&m, &[], &[], &SvgStyle::default());
        assert!(!out.contains("<polyline"));
        assert!(!out.contains("<g "));
    }
}
