//! Tail investigation + data dump for the statistical test.
//!
//!   cargo run --profile fast --bin tails
//!
//! Prints the slowest GenPen boards with their internal counters (so you
//! can see whether a tail is a reroll storm or an expensive solve), and
//! writes every per-board timing to `times.csv` for compare_stats.py.

use bullpen::{genpenai, GenPen, Stats};
use fastrand::Rng;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;

/// Board size to investigate. Tails get dramatic from ~15 up.
const N: usize = 16;
const RUNS: usize = 300;
const TOP_K: usize = 12;

struct Record {
    seed: u64,
    ms: f64,
    stats: Stats,
}

fn main() {
    // GenPen: time + internal counters per seed.
    let mut records: Vec<Record> = (0..RUNS as u64)
        .map(|seed| {
            let mut gen = GenPen::new(Rng::with_seed(seed));
            let t = Instant::now();
            let grid = gen.gen(N);
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            std::hint::black_box(&grid);
            Record {
                seed,
                ms,
                stats: gen.stats,
            }
        })
        .collect();

    // genpenai: time only (no instrumentation exposed).
    let ai_times: Vec<(u64, f64)> = (0..RUNS as u64)
        .map(|seed| {
            let t = Instant::now();
            let grid = genpenai::generate(N, seed);
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            std::hint::black_box(&grid);
            (seed, ms)
        })
        .collect();

    // --- Tail report: slowest GenPen boards and what made them slow ---
    records.sort_by(|a, b| b.ms.partial_cmp(&a.ms).unwrap());
    println!("Slowest {TOP_K} GenPen boards at n={N} (of {RUNS}):\n");
    println!(
        "{:>6}  {:>9}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}",
        "seed", "ms", "rolls", "unsolv", "solves", "budget", "repair"
    );
    for r in records.iter().take(TOP_K) {
        let s = r.stats;
        println!(
            "{:>6}  {:>9.2}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}",
            r.seed, r.ms, s.rolls, s.unsolvable, s.solves, s.over_budget, s.repairs
        );
    }

    // Reproduce any single board with: GenPen::new(Rng::with_seed(SEED)).gen(N)
    println!("\nReproduce the worst: seed={} at n={N}", records[0].seed);

    // --- Where does the time go? Per-phase breakdown across all boards ---
    let wall: f64 = records.iter().map(|r| r.ms).sum();
    let phase = |f: fn(&Stats) -> std::time::Duration| -> f64 {
        records
            .iter()
            .map(|r| f(&r.stats).as_secs_f64() * 1000.0)
            .sum()
    };
    let regions = phase(|s| s.t_regions);
    let solvable = phase(|s| s.t_solvable);
    let solve = phase(|s| s.t_solve);
    let kill = phase(|s| s.t_kill);
    let other = wall - regions - solvable - solve - kill;
    println!("\nPhase breakdown over {RUNS} boards at n={N} (total wall {wall:.0} ms):");
    for (name, ms) in [
        ("gen_regions", regions),
        ("solvable", solvable),
        ("solve", solve),
        ("kill_sol", kill),
        ("other", other),
    ] {
        println!("  {name:<12} {ms:>9.1} ms  ({:>4.1}%)", 100.0 * ms / wall);
    }

    // --- Dump raw times for the statistical test ---
    let f = File::create("times.csv").expect("create times.csv");
    let mut w = BufWriter::new(f);
    writeln!(w, "method,n,seed,ms").unwrap();
    for r in &records {
        writeln!(w, "genpen,{},{},{:.4}", N, r.seed, r.ms).unwrap();
    }
    for (seed, ms) in &ai_times {
        writeln!(w, "genpenai,{},{},{:.4}", N, seed, ms).unwrap();
    }
    println!("\nWrote {} rows to times.csv", 2 * RUNS);
}
