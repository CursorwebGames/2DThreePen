use bullpen::genpenai;
use bullpen::{GenPen, SingleSolver};
use fastrand::Rng;
use std::collections::HashSet;
use std::time::Instant;

// Median is robust to the heavy reroll tail, so it stabilizes faster
// than the mean — but n>=16 boards are slow, so drop this if it drags.
const RUNS: usize = 100;

const COLORS: &[u8] = &[31, 32, 33, 34, 35, 36, 91, 92, 93, 94, 95, 96, 97, 90];

fn show(grid: &[Vec<usize>], sol: Option<&HashSet<(usize, usize)>>) {
    let n = grid.len();
    let width = (n - 1).to_string().len();
    println!();
    for r in 0..n {
        for c in 0..n {
            let region = grid[r][c];
            let color = COLORS[region % COLORS.len()];
            let is_bull = sol.map_or(false, |s| s.contains(&(r, c)));
            if is_bull {
                print!("\x1b[{color}m{:>width$}\x1b[0m ", "★");
            } else {
                print!("\x1b[{color}m{region:>width$}\x1b[0m ");
            }
        }
        println!();
    }
}

/// (median, mean) of a list of timings in ms.
fn stats(mut times: Vec<f64>) -> (f64, f64) {
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = times.len() / 2;
    let median = if times.len() % 2 == 0 {
        (times[mid - 1] + times[mid]) / 2.0
    } else {
        times[mid]
    };
    (median, mean)
}

fn main() {
    println!(
        "{:>4}  {:>9}  {:>9}  {:>9}  {:>9}",
        "n", "GP med", "GP avg", "AI med", "AI avg"
    );
    println!("{}", "-".repeat(52));

    for n in 6..=18 {
        let new_times: Vec<f64> = (0..RUNS)
            .map(|i| {
                let mut gen = GenPen::new(Rng::with_seed(i as u64));
                let t = Instant::now();
                let grid = gen.gen(n);
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                std::hint::black_box(&grid);
                ms
            })
            .collect();

        let ai_times: Vec<f64> = (0..RUNS)
            .map(|i| {
                let t = Instant::now();
                let grid = genpenai::generate(n, i as u64);
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                std::hint::black_box(&grid);
                ms
            })
            .collect();

        let (gp_med, gp_avg) = stats(new_times);
        let (ai_med, ai_avg) = stats(ai_times);
        println!(
            "{:>4}  {:>7.2}ms  {:>7.2}ms  {:>7.2}ms  {:>7.2}ms",
            n, gp_med, gp_avg, ai_med, ai_avg
        );
    }

    let mut gen = GenPen::new(Rng::new());
    let grid = gen.gen(10);
    let sol = SingleSolver::new(&grid).solve();
    let bulls: HashSet<(usize, usize)> = sol[0].iter().copied().collect();
    println!("\n=== n=10 ===");
    println!("\npuzzle:");
    show(&grid, None);
    println!("\nsolution:");
    show(&grid, Some(&bulls));
}
