use bullpen::gendoubleai::{generate, DoubleSolver};

const COLORS: &[&str] = &[
    "\x1b[31m", "\x1b[32m", "\x1b[33m", "\x1b[34m", "\x1b[35m", "\x1b[36m", "\x1b[91m", "\x1b[92m",
    "\x1b[93m", "\x1b[94m", "\x1b[95m", "\x1b[96m",
];
const RESET: &str = "\x1b[0m";

fn main() {
    let n: usize = std::env::args()
        .nth(1)
        .map(|s| s.parse().expect("usage: previewdouble [n]"))
        .unwrap_or(10);

    let seed = fastrand::u64(..);
    let t = std::time::Instant::now();
    let grid = generate(n, seed);
    let elapsed = t.elapsed().as_secs_f64() * 1000.0;

    for row in &grid {
        for &cell in row {
            let color = COLORS[cell % COLORS.len()];
            print!("{color}{cell}{RESET} ");
        }
        println!();
    }

    // solve and overlay stars
    let solution = DoubleSolver::new(&grid, 2).solve();
    let stars: std::collections::HashSet<(usize, usize)> = solution
        .into_iter()
        .next()
        .unwrap_or_default()
        .into_iter()
        .collect();

    println!("solution:");
    println!();
    for (y, row) in grid.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate() {
            let color = COLORS[cell % COLORS.len()];
            let ch = if stars.contains(&(y, x)) {
                "★".to_string()
            } else {
                ".".to_string()
            };
            print!("{color}{ch}{RESET} ");
        }
        println!();
    }

    println!();
    println!("generated in {elapsed:.2} ms (seed {seed})");
}
