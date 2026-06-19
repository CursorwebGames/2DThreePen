//! Bullpen puzzle generator.
//!
//! Strategy: "random regions + targeted repair".
//!
//! 1. Grow n random contiguous regions over the n x n board.
//! 2. Solve it. Random boards usually land near 1 solution already.
//! 3. If there are 2+ solutions, make one tiny region edit that is
//!    *guaranteed* to kill one solution while *guaranteed* to preserve
//!    another (see [`kill_solution`] for why). Re-solve and repeat.
//! 4. If the board has 0 solutions, or repair gets stuck, reroll.
//!
//! Because every repair step preserves a known solution, the count can
//! never drop to 0 mid-repair — it only walks down toward 1.
//!
//! Tried and rejected: seeding regions from a pre-placed bull layout.
//! It guarantees >= 1 solution (no wasted zero-solution proofs), but
//! boards grown around a planted solution start with *many* solutions,
//! so repair takes far longer than the zero-solution rerolls it saves —
//! benchmarked much slower at n=15 than random seeding.

use crate::single_solver::SingleSolver;

/// A bull position: (row, col).
type Pos = (usize, usize);

/// Region grid, `grid[r][c]` = region index in `0..n`.
type Grid = Vec<Vec<usize>>;

/// Marker for cells not yet claimed by any region during growth.
const UNASSIGNED: usize = usize::MAX;

/// Give up repairing one board after this many edits and reroll.
/// In practice repair converges in a handful of steps.
const MAX_REPAIRS: usize = 50;

/// Per-solve work budget (recursion steps). Generously above what a
/// typical board needs, so only the pathological tail — boards whose
/// uniqueness or unsolvability proof would take seconds — gets cut off
/// and discarded. That tail dominates wall time from n=20 up.
const SOLVE_BUDGET: usize = 50_000;

/// Size cap for capped regions (inclusive range). Tiny regions are
/// strong constraints: they cut the solver's branching factor and push
/// boards toward fewer solutions (validated in research/gen2.py).
const CAP_SIZE: (usize, usize) = (1, 3);

/// How many regions get a size cap (inclusive range). The sweet spot is
/// a tradeoff: more caps make every solver call cheaper, but too many
/// make almost every roll unmatchable so all the time goes to growing
/// garbage boards (measured: scaling gen2.py's 5-7-of-8 ratio up to
/// n=15 produced 92k rerolls per 30 boards). This formula matches the
/// measured optima at n=15 (5,7) and n=20 (6,8); tune with sweep_caps_*.
fn num_capped(n: usize) -> (usize, usize) {
    (n / 3, n / 3 + 2)
}

/// Small deterministic RNG (SplitMix64) so we need no external crates
/// and runs are reproducible from a seed.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform integer in `0..k` (modulo bias is negligible for our k).
    fn below(&mut self, k: usize) -> usize {
        (self.next() % k as u64) as usize
    }

    /// Uniform integer in `lo..=hi`.
    fn range(&mut self, (lo, hi): (usize, usize)) -> usize {
        lo + self.below(hi - lo + 1)
    }
}

/// Generate an n x n bullpen with exactly one solution.
pub fn generate(n: usize, seed: u64) -> Grid {
    generate_capped(n, seed, num_capped(n))
}

fn generate_capped(n: usize, seed: u64, caps: (usize, usize)) -> Grid {
    let mut rng = Rng::new(seed);

    // Outer loop: reroll from scratch when a board is hopeless.
    loop {
        let mut grid = random_regions(n, &mut rng, caps);

        // Cheap necessary condition before paying for a real solve:
        // most fresh rolls are provably unsolvable and this catches
        // ~96% of them in microseconds (see matching_filter_catch_rate)
        if !matchable(&grid) {
            continue;
        }

        for _ in 0..MAX_REPAIRS {
            let sols = match SingleSolver::new(&grid).solve_within(SOLVE_BUDGET) {
                Some(sols) => sols,
                None => break, // proof too expensive — discard the board
            };
            match sols.len() {
                0 => break, // dead board (only possible on a fresh roll)
                1 => return grid,
                // 2 solutions: edit the grid to kill one and keep the
                // other. The edit only works on bulls at region borders,
                // so if neither of sols[1]'s differing bulls is movable,
                // try the symmetric edit (kill sols[0], keep sols[1]).
                _ => {
                    if !kill_solution(&mut grid, &sols[0], &sols[1], &mut rng)
                        && !kill_solution(&mut grid, &sols[1], &sols[0], &mut rng)
                    {
                        break; // no legal edit in either direction, reroll
                    }
                }
            }
        }
    }
}

/// Grow n contiguous regions from n random seed cells until the board
/// is covered. Every region is contiguous and nonempty by construction.
///
/// The frontier of growth moves is maintained incrementally: claiming a
/// cell pushes a move for each unassigned neighbor; moves whose cell got
/// claimed in the meantime are stale and skipped when drawn. Each cell
/// pushes at most 4 moves ever, so filling the board is O(n^2) total
/// instead of rescanning the whole board once per claimed cell (O(n^4)).
fn random_regions(n: usize, rng: &mut Rng, caps: (usize, usize)) -> Grid {
    let mut grid = vec![vec![UNASSIGNED; n]; n];
    let mut frontier: Vec<(Pos, usize)> = Vec::new();
    let mut sizes = vec![0usize; n];

    // Pick which regions are capped, and at what size.
    let mut max_size = vec![usize::MAX; n];
    let mut order: Vec<usize> = (0..n).collect();
    shuffle(&mut order, rng);
    for &region in order.iter().take(rng.range(caps).min(n)) {
        max_size[region] = rng.range(CAP_SIZE);
    }

    fn claim(
        grid: &mut Grid,
        frontier: &mut Vec<(Pos, usize)>,
        sizes: &mut [usize],
        (r, c): Pos,
        region: usize,
    ) {
        grid[r][c] = region;
        sizes[region] += 1;
        for (nr, nc) in orth_neighbors(r, c, grid.len()) {
            if grid[nr][nc] == UNASSIGNED {
                frontier.push(((nr, nc), region));
            }
        }
    }

    // Drop n seeds on distinct random cells, one per region.
    for region in 0..n {
        loop {
            let (r, c) = (rng.below(n), rng.below(n));
            if grid[r][c] == UNASSIGNED {
                claim(&mut grid, &mut frontier, &mut sizes, (r, c), region);
                break;
            }
        }
    }

    while !frontier.is_empty() {
        // swap_remove a random move: O(1) removal, order doesn't matter
        let (pos, region) = frontier.swap_remove(rng.below(frontier.len()));
        if grid[pos.0][pos.1] == UNASSIGNED && sizes[region] < max_size[region] {
            claim(&mut grid, &mut frontier, &mut sizes, pos, region);
        }
    }

    // Second pass: cells walled off by full capped regions get absorbed
    // into any assigned neighbor. This may push a capped region past its
    // cap — fine, the cap is a bias, not an invariant (cf. gen2.py).
    // Usually nothing is stranded, so gather the leftovers once and
    // sweep only that list instead of rescanning the whole board.
    if sizes.iter().sum::<usize>() < n * n {
        let mut stranded: Vec<Pos> = (0..n)
            .flat_map(|r| (0..n).map(move |c| (r, c)))
            .filter(|&(r, c)| grid[r][c] == UNASSIGNED)
            .collect();
        while !stranded.is_empty() {
            let mut i = 0;
            while i < stranded.len() {
                let (r, c) = stranded[i];
                let assigned = orth_neighbors(r, c, n).find(|&(y, x)| grid[y][x] != UNASSIGNED);
                if let Some((y, x)) = assigned {
                    grid[r][c] = grid[y][x];
                    stranded.swap_remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }

    grid
}

/// Edit `grid` so that solution `kill` is no longer valid while solution
/// `keep` still is. Returns false if no legal edit exists.
///
/// The trick: pick a bull cell (r, c) that is in `kill` but not in `keep`,
/// and move that one cell into an adjacent region. Then:
///
/// - `kill` dies: its bull at (r, c) was its only bull in region X
///   (solutions have exactly one bull per region), and X no longer
///   contains (r, c) — so `kill` leaves region X bull-less.
/// - `keep` survives: `keep`'s bull in X is some other cell (still in X),
///   and the region gaining (r, c) gains no new bull of `keep`'s.
///
/// The edit must not break the region invariants, so we require:
/// - the donor region stays contiguous after losing (r, c)
///   (it stays nonempty automatically — `keep`'s bull is in it), and
/// - the cell joins an *orthogonally adjacent* region (stays contiguous).
fn kill_solution(grid: &mut Grid, keep: &[Pos], kill: &[Pos], rng: &mut Rng) -> bool {
    let n = grid.len();

    // Bulls of `kill` that aren't bulls of `keep` — each is a valid target.
    // Visit them in random order so repeated repairs don't always gnaw at
    // the same corner of the board.
    let mut targets: Vec<Pos> = kill.iter().filter(|p| !keep.contains(p)).copied().collect();
    shuffle(&mut targets, rng);

    for (r, c) in targets {
        if !stays_contiguous(grid, r, c) {
            continue; // removing this cell would split its region
        }
        let donor = grid[r][c];
        let mut neighbors: Vec<Pos> = orth_neighbors(r, c, n).collect();
        shuffle(&mut neighbors, rng);
        for (nr, nc) in neighbors {
            if grid[nr][nc] != donor {
                grid[r][c] = grid[nr][nc];
                return true;
            }
        }
    }

    false
}

/// Cheap necessary condition for solvability: in any solution the n
/// bulls occupy n distinct rows, and each region holds exactly one
/// bull — so regions must be matchable one-to-one with rows they
/// touch, and likewise with columns. A board failing either check is
/// provably unsolvable, no search needed. (The converse doesn't hold:
/// passing says nothing, e.g. adjacency can still rule everything out.)
fn matchable(grid: &Grid) -> bool {
    let n = grid.len();
    let mut rows = vec![vec![false; n]; n];
    let mut cols = vec![vec![false; n]; n];
    for r in 0..n {
        for c in 0..n {
            rows[grid[r][c]][r] = true;
            cols[grid[r][c]][c] = true;
        }
    }
    perfect_matching(&rows) && perfect_matching(&cols)
}

/// Maximum bipartite matching between regions and rows (or columns)
/// via augmenting paths; true if a perfect matching exists.
/// `touch[region][x]` = region has a cell in row/col x.
fn perfect_matching(touch: &[Vec<bool>]) -> bool {
    fn augment(
        region: usize,
        touch: &[Vec<bool>],
        seen: &mut [bool],
        matched: &mut [usize],
    ) -> bool {
        for x in 0..touch.len() {
            if touch[region][x] && !seen[x] {
                seen[x] = true;
                if matched[x] == usize::MAX || augment(matched[x], touch, seen, matched) {
                    matched[x] = region;
                    return true;
                }
            }
        }
        false
    }

    let n = touch.len();
    let mut matched = vec![usize::MAX; n];
    (0..n).all(|region| augment(region, touch, &mut vec![false; n], &mut matched))
}

/// Would `region of (r, c)` remain connected if (r, c) left it?
///
/// Anchor trick: if removing (r, c) splits its region, every fragment
/// must contain one of (r, c)'s own same-region orthogonal neighbors
/// (the "anchors") — the fragments were only ever connected *through*
/// (r, c). So there's no need to collect the whole region and count it:
/// flood-fill from one anchor, skipping (r, c), and check the other
/// anchors are reached.
fn stays_contiguous(grid: &Grid, r: usize, c: usize) -> bool {
    let n = grid.len();
    let region = grid[r][c];

    let anchors: Vec<Pos> = orth_neighbors(r, c, n)
        .filter(|&(y, x)| grid[y][x] == region)
        .collect();

    if anchors.is_empty() {
        return false; // (r, c) is the whole region; removal would empty it
    }
    if anchors.len() == 1 {
        return true; // only one possible fragment, nothing to split
    }

    let mut seen = vec![vec![false; n]; n];
    let mut stack = vec![anchors[0]];
    seen[anchors[0].0][anchors[0].1] = true;
    while let Some((y, x)) = stack.pop() {
        for (ny, nx) in orth_neighbors(y, x, n) {
            if grid[ny][nx] == region && (ny, nx) != (r, c) && !seen[ny][nx] {
                seen[ny][nx] = true;
                stack.push((ny, nx));
            }
        }
    }

    anchors.iter().all(|&(y, x)| seen[y][x])
}

/// Up/down/left/right neighbors of (r, c) inside an n x n board.
/// Allocation-free: this runs in every hot loop. `wrapping_sub` turns
/// the off-board 0-1 case into usize::MAX, which the `< n` filter drops.
fn orth_neighbors(r: usize, c: usize, n: usize) -> impl Iterator<Item = Pos> {
    IntoIterator::into_iter([
        (r.wrapping_sub(1), c),
        (r + 1, c),
        (r, c.wrapping_sub(1)),
        (r, c + 1),
    ])
    .filter(move |&(y, x)| y < n && x < n)
}

/// Fisher-Yates shuffle.
fn shuffle<T>(items: &mut [T], rng: &mut Rng) {
    for i in (1..items.len()).rev() {
        items.swap(i, rng.below(i + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every region 0..n present, and each one contiguous.
    fn assert_valid_regions(grid: &Grid) {
        let n = grid.len();
        for region in 0..n {
            let cells: Vec<Pos> = (0..n)
                .flat_map(|y| (0..n).map(move |x| (y, x)))
                .filter(|&(y, x)| grid[y][x] == region)
                .collect();
            assert!(!cells.is_empty(), "region {} is empty", region);

            // flood fill from the first cell must reach the whole region
            let mut seen = vec![vec![false; n]; n];
            let mut stack = vec![cells[0]];
            seen[cells[0].0][cells[0].1] = true;
            let mut reached = 0;
            while let Some((y, x)) = stack.pop() {
                reached += 1;
                for (ny, nx) in orth_neighbors(y, x, n) {
                    if grid[ny][nx] == region && !seen[ny][nx] {
                        seen[ny][nx] = true;
                        stack.push((ny, nx));
                    }
                }
            }
            assert_eq!(reached, cells.len(), "region {region} is split");
        }
    }

    #[test]
    fn generated_puzzles_are_unique_and_valid() {
        for n in [5, 6, 8] {
            for seed in 0..10 {
                let grid = generate(n, seed);
                assert_valid_regions(&grid);
                assert!(
                    SingleSolver::new(&grid).is_unique(),
                    "n={} seed={} not unique: {:?}",
                    n,
                    seed,
                    grid
                );
            }
        }
    }

    #[test]
    fn deterministic_for_a_seed() {
        assert_eq!(generate(6, 42), generate(6, 42));
    }

    /// Not run by default. Find the cap-count sweet spot for a size:
    /// `cargo test --release sweep_caps_n20 -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn sweep_caps_n15() {
        sweep_caps(15, 20);
    }

    #[test]
    #[ignore]
    fn sweep_caps_n20() {
        sweep_caps(20, 20);
    }

    fn sweep_caps(n: usize, runs: u64) {
        for caps in [(6, 8), (8, 10), (10, 13), (12, 15)] {
            let start = std::time::Instant::now();
            for seed in 0..runs {
                generate_capped(n, seed, caps);
            }
            println!(
                "n={} caps={:?}: {:?} avg",
                n,
                caps,
                start.elapsed() / runs as u32
            );
        }
    }

    fn bench(n: usize, runs: u64) {
        let start = std::time::Instant::now();
        for seed in 0..runs {
            let grid = generate(n, seed);
            assert!(SingleSolver::new(&grid).is_unique());
        }
        let total = start.elapsed();
        println!(
            "n={}: {:?} avg over {} boards",
            n,
            total / runs as u32,
            runs
        );
    }

    /// Not run by default. Time generation with e.g.:
    /// `cargo test --release bench_n20 -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bench_n15() {
        bench(15, 50);
    }

    #[test]
    #[ignore]
    fn bench_n20() {
        bench(20, 50);
    }

    /// Not run by default. How many zero-solution boards does the
    /// regions<->rows/cols matching pre-filter catch without solving?
    /// `cargo test --release matching_filter -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn matching_filter_catch_rate() {
        let n = 15;
        let boards = 2000;
        let mut rng = Rng::new(7);

        let (mut zero, mut caught) = (0, 0);
        for _ in 0..boards {
            let grid = random_regions(n, &mut rng, num_capped(n));
            let matchable = matchable(&grid);
            let solvable = !SingleSolver::new(&grid).solve().is_empty();

            // the filter must never reject a solvable board
            assert!(matchable || !solvable, "filter rejected a solvable board");

            if !solvable {
                zero += 1;
                if !matchable {
                    caught += 1;
                }
            }
        }

        println!(
            "{} fresh boards: {} zero-solution, {} caught by matching filter ({:.1}%)",
            boards,
            zero,
            caught,
            100.0 * caught as f64 / zero.max(1) as f64
        );
    }

    /// Not run by default. Where does generation time go?
    /// `cargo test --release breakdown_n20 -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn breakdown_n15() {
        breakdown(15, 30);
    }

    #[test]
    #[ignore]
    fn breakdown_n20() {
        breakdown(20, 30);
    }

    fn breakdown(n: usize, target: usize) {
        use std::time::{Duration, Instant};

        let mut rng = Rng::new(123);

        let (mut rerolls, mut zero_sol, mut repairs, mut solves) = (0u32, 0u32, 0u32, 0u32);
        let mut filtered = 0u32;
        let mut over_budget = 0u32;
        let mut solve_time = Duration::ZERO;
        let mut grow_time = Duration::ZERO;

        let mut done = 0;
        while done < target {
            let t = Instant::now();
            let mut grid = random_regions(n, &mut rng, num_capped(n));
            grow_time += t.elapsed();

            if !matchable(&grid) {
                filtered += 1;
                rerolls += 1;
                continue;
            }

            let mut ok = false;
            for _ in 0..MAX_REPAIRS {
                let t = Instant::now();
                let sols = SingleSolver::new(&grid).solve_within(SOLVE_BUDGET);
                solve_time += t.elapsed();
                solves += 1;
                let sols = match sols {
                    Some(sols) => sols,
                    None => {
                        over_budget += 1;
                        break;
                    }
                };
                match sols.len() {
                    0 => {
                        zero_sol += 1;
                        break;
                    }
                    1 => {
                        ok = true;
                        break;
                    }
                    _ => {
                        repairs += 1;
                        if !kill_solution(&mut grid, &sols[0], &sols[1], &mut rng)
                            && !kill_solution(&mut grid, &sols[1], &sols[0], &mut rng)
                        {
                            break;
                        }
                    }
                }
            }
            if ok {
                done += 1;
            } else {
                rerolls += 1;
            }
        }

        println!(
            "{} boards: rerolls={} (matching-filtered {}, zero-solution {}, over-budget {}), repairs={}, solver calls={}",
            target, rerolls, filtered, zero_sol, over_budget, repairs, solves
        );
        println!(
            "time in solver: {:?}, in region growth: {:?}",
            solve_time, grow_time
        );
    }
}
