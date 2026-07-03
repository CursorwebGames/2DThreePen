use bullpen::{Aggregator, GenPen};
use fastrand::Rng;

const DEFAULT_RUNS: usize = 100;

fn main() {
    let mut args = std::env::args().skip(1);
    let n: usize = args
        .next()
        .expect("usage: bench <n> [runs]")
        .parse()
        .expect("n must be a positive integer");
    let runs: usize = args
        .next()
        .map(|s| s.parse().expect("runs must be a positive integer"))
        .unwrap_or(DEFAULT_RUNS);

    let mut agg = Aggregator::default();

    for i in 0..runs {
        let mut gen = GenPen::new(Rng::with_seed(i as u64));
        let grid = gen.gen(n);

        // let rust know that grid is used (important on fast profiles)
        std::hint::black_box(&grid);

        agg.push(gen.stats);
    }

    println!("{}", agg.to_json());
}
