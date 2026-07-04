use std::collections::HashSet;
use std::time::Instant;

use bullpen::gendoubleai::{self, DoubleSolver};
use bullpen::{GenPen, SingleSolver};
use fastrand::Rng;

const COLORS: &[&str] = &[
    "\x1b[31m", "\x1b[32m", "\x1b[33m", "\x1b[34m", "\x1b[35m", "\x1b[36m", "\x1b[91m", "\x1b[92m",
    "\x1b[93m", "\x1b[94m", "\x1b[95m", "\x1b[96m",
];
const RESET: &str = "\x1b[0m";

fn main() {
    let mut n: usize = 10;
    let mut double = false;
    for arg in std::env::args().skip(1) {
        if arg == "--double" {
            double = true;
        } else {
            n = arg.parse().expect("usage: preview [n] [--double]");
        }
    }

    if double {
        preview_double(n);
    } else {
        preview_single(n);
    }
}

fn preview_single(n: usize) {
    let mut gen = GenPen::new(Rng::new());
    let t = Instant::now();
    let grid = gen.gen(n);
    let elapsed = t.elapsed().as_secs_f64() * 1000.0;

    let solution = SingleSolver::new(&grid).solve_within(usize::MAX).unwrap();
    show(&grid, solution.into_iter().next().unwrap_or_default());

    println!();
    println!("generated in {elapsed:.2} ms");
    let s = gen.stats;
    println!(
        "rolls: {}  solves: {}  repairs: {}  over_budget: {}  steps: {}",
        s.rolls, s.solves, s.repairs, s.over_budget, s.solver_steps
    );
    println!(
        "time:  regions {:.2}ms  solvable {:.2}ms  solve {:.2}ms  kill {:.2}ms",
        s.t_regions, s.t_solvable, s.t_solve, s.t_kill
    );
}

// todo: stats for double
fn preview_double(n: usize) {
    let seed = fastrand::u64(..);
    let t = Instant::now();
    let grid = gendoubleai::generate(n, seed);
    let elapsed = t.elapsed().as_secs_f64() * 1000.0;

    let solution = DoubleSolver::new(&grid, 2).solve();
    show(&grid, solution.into_iter().next().unwrap_or_default());

    println!();
    println!("generated in {elapsed:.2} ms (seed {seed})");
}

fn show(grid: &[Vec<usize>], solution: Vec<(usize, usize)>) {
    for row in grid {
        for &cell in row {
            let color = COLORS[cell % COLORS.len()];
            print!("{color}{cell}{RESET} ");
        }
        println!();
    }

    let stars: HashSet<(usize, usize)> = solution.into_iter().collect();
    println!("solution:");
    println!();
    for (y, row) in grid.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate() {
            let color = COLORS[cell % COLORS.len()];
            let ch = if stars.contains(&(y, x)) { "★" } else { "." };
            print!("{color}{ch}{RESET} ");
        }
        println!();
    }
}
