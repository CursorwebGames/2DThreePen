//! Double bullpen (Star Battle 2*, K = 2) generator — Algorithm M experiment.
//!
//! Same "random regions + targeted repair" strategy as genpenai.rs and
//! research/gendouble.py; the piece under test here is the SOLVER.
//! Knuth's Algorithm M generalizes Algorithm X (exact cover, cf.
//! single_solver.rs) to *multiplicities*: every item must be covered
//! exactly K times instead of exactly once. That is precisely "K bulls
//! per row, per column, per region" with no encoding tricks — the naive
//! "duplicate each column K times" hack counts every solution 2^n times
//! (a region's two bulls are interchangeable) and silently breaks the
//! uniqueness check.
//!
//! Branching follows Knuth's tweak idea: each search node places ONE
//! bull — the first remaining candidate of the most constrained item —
//! then re-runs item selection, so a partially filled item can be
//! interrupted by whatever became tightest. Duplicate-freeness comes
//! from candidate order: when the search moves past a candidate x of
//! item p, x stays excluded from *every* list for the rest of that
//! node's loop ("tweaked" off), so deeper picks for p only come from
//! below x and each K-subset is enumerated exactly once. An item is
//! only fully covered when its need hits 0. The branch choice is a
//! deterministic function of the search state, so each solution is
//! reached by exactly one path and the count (capped at 2) stays
//! trustworthy.
//!
//! A pleasant consequence: because the branch item is never covered
//! early, every option still in any list is fully placeable, so item
//! sizes are always honest and `size < need` is a sound dead-branch
//! test at every node — no special cases.
//!
//! What this buys over the region-MRV backtracker in gendouble.py:
//! - MRV runs over all 3n items (rows, columns, AND regions), not just
//!   regions — a starved row can become the branch point.
//! - Candidate elimination is incremental (dancing links), not a full
//!   grid scan per node.
//!
//! Adjacency lives in the links too (unlike single_solver.rs, which
//! checks a `placed` bitmap per candidate): committing a bull detaches
//! every still-listed option within Chebyshev distance 1 of it. So
//! list sizes only count genuinely placeable cells, which makes the
//! `size < need` check exactly the Python solver's adjacency-aware
//! live-count prune — maintained incrementally instead of recomputed
//! by a full grid scan per node — and it means candidates drawn from a
//! list never need an adjacency test at all.
//!
//! Undo is a trail: every detached node (and de-listed header) is
//! pushed onto one stack, and backtracking pops to a mark. The search
//! is strictly LIFO, so popping the trail restores links in exactly
//! the reverse order they were broken, no matter which mechanism
//! (item cover or adjacency hiding) broke them.
//!
//! The solver is built on the crate's shared dancing-links structures
//! (matrix.rs / llist.rs / cell.rs) — the same Matrix the K=1
//! SingleSolver searches; test with
//! `cargo test --release --lib gendoubleai -- --nocapture`.

use crate::cell::Cell;
use crate::matrix::{Matrix, H};

/// Bulls per row, column, and region. Kept as a runtime parameter on
/// the solver so K=3 ("threepen") falls out later; the generator's
/// constants below are tuned for K=2.
const K: usize = 2;

/// A bull position: (row, col).
type Pos = (usize, usize);

/// Region grid, `grid[r][c]` = region index in `0..n`.
type Grid = Vec<Vec<usize>>;

/// Marker for cells not yet claimed by any region during growth.
const UNASSIGNED: usize = usize::MAX;

/// Give up repairing one board after this many edits and reroll.
const MAX_REPAIRS: usize = 50;

/// Per-solve work budget (committed bull placements). Boards whose
/// uniqueness or unsolvability proof is pathologically expensive get
/// discarded rather than paid for (cf. genpenai.rs). Swept at n=14
/// (fixed seeds, medians): unimodal with the optimum at 25k — half the
/// time of the old 200k, while 1k censors so hard that almost no board
/// survives (~4x worse). Retune with sweep_budget_*.
const SOLVE_BUDGET: usize = 25_000;

/// How many regions get a size cap (inclusive range). Swept at n=10,
/// 12, and 14 (fixed seeds, medians): n=10 is insensitive, n=12's
/// optimum is (7,9), n=14's is (9,11) — capping about two thirds of
/// the regions, more than the half the weaker Python solver liked, and
/// the curve is unimodal on both sides (all 14 capped at n=14 is ~10x
/// worse). Retune with sweep_caps_*.
fn num_capped(n: usize) -> (usize, usize) {
    (n.saturating_sub(5), n.saturating_sub(3))
}

/// Size cap for capped regions (inclusive range). The floor is 3, not
/// 1 as in the single generator: a region must hold 2 non-touching
/// bulls, impossible below 3 cells. Measured in Python at n=10: very
/// tight caps (3,5) backfire — most rolls become unsolvable.
const CAP_SIZE: (usize, usize) = (3, 9);

/// Growth brings every region to this size before free growth starts.
/// 3 cells are necessary for 2 non-touching bulls, not sufficient (an
/// L-tromino still fails) — `regions_feasible` is the real check.
const MIN_REGION: usize = 3;

/// Small deterministic RNG (SplitMix64), duplicated from genpenai.rs
/// because its methods are private there (as are the generator helpers
/// below: orth_neighbors, shuffle, stays_contiguous, kill_solution).
/// TODO: make genpenai's pub(crate) and share them instead.
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

// ------------------------------------------------- Algorithm M solver

/// Exact cover with multiplicities over the crate's dancing-links
/// `Matrix` (matrix.rs / llist.rs / cell.rs) — the same structure the
/// K=1 `SingleSolver` searches with Algorithm X.
///
/// Items = matrix columns (3n): one per board row, per board column,
/// per region; each must be covered exactly `k` times. Options =
/// matrix rows (n^2): one per cell — a bull on (r, c) in region g
/// covers columns r, n+c, 2n+g. A set of matrix rows covering every
/// column exactly k times is a solution.
///
/// What Algorithm M adds on top of the shared Matrix:
/// - `need` per column: a column leaves the header list only when its
///   need hits 0 (Algorithm X is the special case need = 1).
/// - `blocked` counts: placing a bull detaches every option within
///   Chebyshev distance 1 from all lists, so column sizes only count
///   genuinely placeable cells.
/// - a trail instead of Matrix::cover/uncover: covering and adjacency
///   hiding can race for the same node, so every unlink (guarded by
///   `attached`) is pushed onto one stack and backtracking pops to a
///   mark — strict LIFO makes the mixed undo exact.
///
/// One layout fact this leans on: `Matrix` allocates node ids
/// sequentially (0 = H, 1..=3n = column headers, then 3 nodes per
/// added row), so option o's nodes are exactly `Cell(3n+1 + 3o + t)`
/// — siblings by arithmetic, complementing the x-links.
pub struct DoubleSolver {
    m: Matrix,
    n: usize,

    /// Id of the first option node (= 3n + 1).
    first: usize,

    /// Per column: bulls it still needs. This is Algorithm M's
    /// multiplicity — Algorithm X is the special case need = 1.
    need: Vec<usize>,

    /// Per node: currently linked into its vertical list. Both
    /// unlinking mechanisms (cover and adjacency hiding) can race for
    /// the same node; this flag makes "detach if attached" safe.
    attached: Vec<bool>,

    /// Per cell: how many placed bulls have it in their 3x3
    /// neighborhood. A count, not a boolean — two bulls can block the
    /// same cell, and removing one must leave it blocked (cf.
    /// gendouble.py). A cell's options are hidden while blocked > 0.
    blocked: Vec<u32>,

    /// Undo trail: nodes detached from y lists (and column headers
    /// removed from the x list, told apart by id < first), in order.
    trail: Vec<Cell>,

    /// Stop searching after this many solutions (2 for counting, 1
    /// when hunting for a solution other than a known one).
    max_sols: usize,

    /// A known solution (sorted) that doesn't count when found — the
    /// repair loop already has it, so "find up to 2" becomes the
    /// strictly cheaper "find 1 that differs".
    skip: Option<Vec<Pos>>,

    /// Remaining work budget for the current solve call, charged per
    /// committed placement — that is where the cover/hide work scales,
    /// so the budget censors expensive boards where they actually pay.
    steps: usize,
}

impl DoubleSolver {
    /// Takes an n x n region grid with labels 0..n.
    ///
    /// TODO: bring back `reset(&grid)` (relink in place, reusing every
    /// buffer). The repair loop re-solves dozens of times per board
    /// with one cell's region changed, and paying a full Matrix
    /// construction per solve cost ~10-15% end to end when measured.
    /// Needs a small clear/rebuild-in-place helper on Matrix — which
    /// would benefit SingleSolver equally, since genpenai.rs also
    /// constructs a fresh solver per repair re-solve.
    pub fn new(grid: &Grid, k: usize) -> DoubleSolver {
        let n = grid.len();
        let mut m = Matrix::new(3 * n);
        let mut mrow = vec![false; 3 * n];
        for r in 0..n {
            assert_eq!(grid[r].len(), n, "grid must be an n x n square");
            for c in 0..n {
                let g = grid[r][c];
                assert!(g < n, "region labels must be 0..n");
                mrow.fill(false);
                mrow[r] = true; // row constraint
                mrow[n + c] = true; // column constraint
                mrow[2 * n + g] = true; // region constraint
                m.add_row(&mrow);
            }
        }

        let first = 3 * n + 1;
        let total = m.c.len();
        // the sibling-by-arithmetic shortcut relies on this layout
        debug_assert_eq!(total, first + 3 * n * n);

        let mut need = vec![k; first];
        need[H.0] = 0; // the sentinel is not an item

        DoubleSolver {
            m,
            n,
            first,
            need,
            attached: vec![true; total],
            blocked: vec![0; n * n],
            trail: Vec::new(),
            max_sols: 2,
            skip: None,
            steps: usize::MAX,
        }
    }

    /// Whether the puzzle has exactly one solution.
    pub fn is_unique(&mut self) -> bool {
        self.solve().len() == 1
    }

    /// Returns at most 2 solutions.
    pub fn solve(&mut self) -> Vec<Vec<Pos>> {
        self.solve_within(usize::MAX).unwrap()
    }

    /// Like `solve`, but gives up once the search has placed `budget`
    /// bulls, returning None. Aborting unwinds cleanly, so the solver
    /// stays reusable.
    pub fn solve_within(&mut self, budget: usize) -> Option<Vec<Vec<Pos>>> {
        self.max_sols = 2;
        self.skip = None;
        self.run(budget)
    }

    /// The repair loop's question: does any solution other than
    /// `known` exist? `known` must be sorted and must be a solution of
    /// the current board (the keep/kill edit guarantees the surviving
    /// one is). Finding one differing solution needs a single find
    /// instead of two, and only the final "no, it's unique" answer
    /// pays for a full-tree proof.
    ///
    /// Returns None over budget, Some(None) if `known` is the unique
    /// solution, Some(Some(other)) otherwise.
    pub fn solve_other_within(
        &mut self,
        budget: usize,
        known: &[Pos],
    ) -> Option<Option<Vec<Pos>>> {
        debug_assert!(known.windows(2).all(|w| w[0] <= w[1]));
        self.max_sols = 1;
        self.skip = Some(known.to_vec());
        let sols = self.run(budget)?;
        Some(sols.into_iter().next())
    }

    fn run(&mut self, budget: usize) -> Option<Vec<Vec<Pos>>> {
        self.steps = budget;
        let mut sols = Vec::new();
        self.solve_rec(&mut Vec::new(), &mut sols);
        if self.steps == 0 {
            None
        } else {
            Some(sols)
        }
    }

    /// Record a completed placement set, unless it is the caller's
    /// already-known solution.
    fn record(&mut self, csol: &[Pos], sols: &mut Vec<Vec<Pos>>) {
        if let Some(skip) = &self.skip {
            let mut sorted = csol.to_vec();
            sorted.sort_unstable();
            if sorted == *skip {
                return;
            }
        }
        sols.push(csol.to_vec());
    }

    /// The three nodes of option o, one per covered column.
    fn nodes(&self, o: usize) -> [Cell; 3] {
        let base = self.first + 3 * o;
        [Cell(base), Cell(base + 1), Cell(base + 2)]
    }

    /// Detach node j from its column's vertical list, recording it on
    /// the trail so `undo_to` can relink it later.
    fn detach(&mut self, j: Cell) {
        debug_assert!(self.attached[j.0]);
        self.attached[j.0] = false;
        self.m.y.remove(j);
        self.m.size[self.m.c[j]] -= 1;
        self.trail.push(j);
    }

    /// Undo every detach/de-list since the trail was `mark` entries
    /// long. Popping restores in exact reverse order, which is what
    /// dancing-links relinking requires (a removed node's own links
    /// still point at its old neighbors).
    fn undo_to(&mut self, mark: usize) {
        while self.trail.len() > mark {
            let j = self.trail.pop().unwrap();
            if j.0 < self.first {
                self.m.x.restore(j); // column header back into the x list
            } else {
                self.attached[j.0] = true;
                self.m.y.restore(j);
                self.m.size[self.m.c[j]] += 1;
            }
        }
    }

    /// Column h is saturated: remove it from the header list and hide
    /// all its remaining options from other columns' lists. Identical
    /// to Algorithm X's cover — the difference is only *when* it is
    /// called (need hitting 0, not the first pick) and that it is
    /// undone via the trail, not a mirrored uncover.
    fn cover(&mut self, h: Cell) {
        self.trail.push(h);
        self.m.x.remove(h);
        let mut i = self.m.y.cursor(h);
        while let Some(i) = i.next(&self.m.y) {
            for j in self.nodes(self.m.row_id[i]) {
                // adjacency hiding may have beaten us to a sibling
                if j != i && self.attached[j.0] {
                    self.detach(j);
                }
            }
        }
    }

    /// A bull was placed on cell o: block its 3x3 neighborhood. Each
    /// cell newly blocked (count 0 -> 1) has its whole option pulled
    /// out of every list it is still in, so column sizes stay an exact
    /// count of placeable cells. o's own option is never touched: it
    /// was already detached by the tweak, and no future bull can land
    /// adjacent to o (those cells are blocked right here), so nothing
    /// ever detaches o's nodes out from under an ancestor's iteration.
    fn block_neighbors(&mut self, o: usize, sign: i32) {
        let n = self.n;
        let (y, x) = (o / n, o % n);
        for ny in y.saturating_sub(1)..=(y + 1).min(n - 1) {
            for nx in x.saturating_sub(1)..=(x + 1).min(n - 1) {
                let q = ny * n + nx;
                if q == o {
                    continue;
                }
                if sign > 0 {
                    self.blocked[q] += 1;
                    if self.blocked[q] == 1 {
                        for j in self.nodes(q) {
                            if self.attached[j.0] {
                                self.detach(j);
                            }
                        }
                    }
                } else {
                    // relinking is the trail's job (undo_to)
                    self.blocked[q] -= 1;
                }
            }
        }
    }

    fn solve_rec(&mut self, csol: &mut Vec<Pos>, sols: &mut Vec<Vec<Pos>>) {
        // Choose the branch column by MRV: fewest spare options
        // (size - need, Knuth's branching degree for multiplicities).
        // Any column with more needs than options kills the branch
        // outright — and since sizes are adjacency-aware, this is the
        // same live-count prune that carries the Python solver.
        let mut best = H;
        let mut best_spare = usize::MAX;
        let mut cur = self.m.x.cursor(H);
        while let Some(h) = cur.next(&self.m.x) {
            if self.m.size[h] < self.need[h] {
                return; // column can no longer be satisfied
            }
            let spare = self.m.size[h] - self.need[h];
            if spare < best_spare {
                best_spare = spare;
                best = h;
            }
        }

        if best == H {
            // no active columns left => every column got exactly k bulls
            self.record(csol, sols);
            return;
        }

        // Place ONE bull for `best`, trying its candidates in list
        // order. Tried candidates stay tweaked off (detached from
        // every list) while later ones run, so deeper picks for this
        // column only come from further down the list — each K-subset
        // is enumerated exactly once. All restored at loop end.
        let p = best;
        let mark_level = self.trail.len();
        let mut i = self.m.y[p].next;
        while i != p {
            if sols.len() >= self.max_sols || self.steps == 0 {
                break;
            }

            let o = self.m.row_id[i];
            // Being in p's list means the option is fully attached:
            // none of its columns is saturated and no placed bull
            // touches it (either would have detached it) — so it is
            // legal as-is, no checks needed.
            debug_assert!(self.nodes(o).iter().all(|j| self.attached[j.0]));
            debug_assert_eq!(self.blocked[o], 0);

            // tweak: pull the option out of every list (persists
            // across this loop), then commit the placement.
            for j in self.nodes(o) {
                self.detach(j);
            }

            self.steps -= 1; // budget is charged per placement
            let mark = self.trail.len();
            csol.push((o / self.n, o % self.n));
            self.block_neighbors(o, 1);
            for j in self.nodes(o) {
                let h = self.m.c[j];
                self.need[h] -= 1;
                if self.need[h] == 0 {
                    self.cover(h);
                }
            }

            self.solve_rec(csol, sols);

            for j in self.nodes(o) {
                self.need[self.m.c[j]] += 1;
            }
            self.block_neighbors(o, -1);
            self.undo_to(mark);
            csol.pop();

            // the removed node's own link still names its old
            // successor, which is back in the list by now
            i = self.m.y[i].next;
        }
        self.undo_to(mark_level); // untweak every tried candidate
    }
}

// ---------------------------------------------------------- generator

/// Generate an n x n double bullpen with exactly one K=2 solution.
pub fn generate(n: usize, seed: u64) -> Grid {
    generate_capped(n, seed, num_capped(n))
}

fn generate_capped(n: usize, seed: u64, caps: (usize, usize)) -> Grid {
    let mut rng = Rng::new(seed);

    // Outer loop: reroll from scratch when a board is hopeless.
    loop {
        let mut grid = random_regions(n, &mut rng, caps);

        // Two cheap necessary conditions before paying for a solve:
        // every region must fit K non-touching bulls, and regions must
        // be flow-feasible against rows and columns.
        if !regions_feasible(&grid, K) || !matchable(&grid, K) {
            continue;
        }

        // Fresh board: the full "0, 1, or 2+ solutions" question.
        let sols = match DoubleSolver::new(&grid, K).solve_within(SOLVE_BUDGET) {
            Some(sols) => sols,
            None => continue, // proof too expensive — discard the board
        };
        let (mut keep, mut other) = match &sols[..] {
            [] => continue, // dead board
            [_] => return grid,
            [a, b, ..] => (a.clone(), b.clone()),
        };

        // Repair loop. From here on one solution is always known
        // (keep survives every edit by construction), so each re-solve
        // only has to hunt for a solution that differs from it.
        for _ in 0..MAX_REPAIRS {
            // Edit the grid to kill `other` and keep `keep`; if none
            // of other's differing bulls is movable, try the symmetric
            // edit (then the surviving solution is `other`).
            if kill_solution(&mut grid, &keep, &other, &mut rng) {
                // keep survives
            } else if kill_solution(&mut grid, &other, &keep, &mut rng) {
                std::mem::swap(&mut keep, &mut other);
            } else {
                break; // no legal edit in either direction, reroll
            }

            let mut known = keep.clone();
            known.sort_unstable();
            match DoubleSolver::new(&grid, K).solve_other_within(SOLVE_BUDGET, &known) {
                None => break, // proof too expensive — discard the board
                Some(None) => return grid, // keep is the unique solution
                Some(Some(o)) => other = o,
            }
        }
    }
}

/// Grow n contiguous regions from n random seed cells until the board
/// is covered (incremental frontier, cf. genpenai.rs). New for K=2: a
/// round-robin phase grows every region to MIN_REGION cells before free
/// growth, so no region gets walled in below the size 2 bulls require.
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

    // Phase 1: round-robin one cell per under-minimum region per round.
    // A region with no legal move left is already walled in; give up on
    // it (the board will die in regions_feasible, same as before).
    loop {
        let mut grew = false;
        for region in 0..n {
            if sizes[region] >= MIN_REGION {
                continue;
            }
            let moves: Vec<Pos> = frontier
                .iter()
                .filter(|&&(p, reg)| reg == region && grid[p.0][p.1] == UNASSIGNED)
                .map(|&(p, _)| p)
                .collect();
            if !moves.is_empty() {
                let pos = moves[rng.below(moves.len())];
                claim(&mut grid, &mut frontier, &mut sizes, pos, region);
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }

    // Phase 2: free growth, random frontier moves until exhausted.
    while !frontier.is_empty() {
        let (pos, region) = frontier.swap_remove(rng.below(frontier.len()));
        if grid[pos.0][pos.1] == UNASSIGNED && sizes[region] < max_size[region] {
            claim(&mut grid, &mut frontier, &mut sizes, pos, region);
        }
    }

    // Cells walled off by full capped regions get absorbed into any
    // assigned neighbor. This may push a capped region past its cap —
    // fine, the cap is a bias, not an invariant.
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

// ---------------------------------------------------------- prefilters

/// Every region must be able to host k mutually non-adjacent bulls
/// (pairwise Chebyshev distance >= 2). Size alone is not enough: an
/// L-tromino has 3 cells but every pair touches.
fn regions_feasible(grid: &Grid, k: usize) -> bool {
    let n = grid.len();
    let mut cells: Vec<Vec<Pos>> = vec![Vec::new(); n];
    for r in 0..n {
        for c in 0..n {
            cells[grid[r][c]].push((r, c));
        }
    }
    cells.iter().all(|cs| spread_exists(cs, k, &mut Vec::new()))
}

fn spread_exists(cells: &[Pos], k: usize, chosen: &mut Vec<Pos>) -> bool {
    if k == 0 {
        return true;
    }
    for (i, &(r, c)) in cells.iter().enumerate() {
        if chosen
            .iter()
            .all(|&(y, x)| r.abs_diff(y).max(c.abs_diff(x)) >= 2)
        {
            chosen.push((r, c));
            if spread_exists(&cells[i + 1..], k - 1, chosen) {
                return true;
            }
            chosen.pop();
        }
    }
    false
}

/// Cheap necessary condition for solvability: k bulls per region must
/// be routable to rows needing k bulls each (and likewise columns) —
/// a flow problem: source -> region (cap k) -> row (cap = how many
/// bulls that region can physically put in that row) -> sink (cap k).
/// A solvable board admits flow k*n, so anything less is a proof of
/// unsolvability, no search needed.
///
/// The edge capacity is what makes this stronger than plain matching:
/// bulls in one row can't sit in adjacent columns, so a region's supply
/// into a row is its max spread-out cell count there, not just "touches
/// it". A region meeting a row in 2 adjacent cells supplies at most 1.
fn matchable(grid: &Grid, k: usize) -> bool {
    let n = grid.len();
    // caps[g][x] = max non-touching bulls region g can place in line x;
    // within one line, cells touch iff consecutive, so a greedy skip
    // count over the line's cells (visited in order) is exact.
    let mut row_caps = vec![vec![0usize; n]; n];
    let mut col_caps = vec![vec![0usize; n]; n];
    let mut last = vec![[-2isize; 2]; n]; // per region: last counted row-/col-index
    for r in 0..n {
        for reg in last.iter_mut() {
            reg[0] = -2;
        }
        for c in 0..n {
            let g = grid[r][c];
            if c as isize > last[g][0] + 1 {
                row_caps[g][r] += 1;
                last[g][0] = c as isize;
            }
        }
    }
    for c in 0..n {
        for reg in last.iter_mut() {
            reg[1] = -2;
        }
        for r in 0..n {
            let g = grid[r][c];
            if r as isize > last[g][1] + 1 {
                col_caps[g][c] += 1;
                last[g][1] = r as isize;
            }
        }
    }
    feasible_flow(&row_caps, k) && feasible_flow(&col_caps, k)
}

/// Does k*n flow fit through source -> regions (cap k) -> lines (edge
/// caps) -> sink (cap k)? Edmonds-Karp on a dense residual matrix —
/// the graph has 2n+2 nodes and the target flow is only k*n.
fn feasible_flow(edge_caps: &[Vec<usize>], k: usize) -> bool {
    let n = edge_caps.len();
    let v = 2 * n + 2;
    let (s, t) = (2 * n, 2 * n + 1);
    let mut cap = vec![vec![0usize; v]; v];
    for g in 0..n {
        cap[s][g] = k;
        for x in 0..n {
            cap[g][n + x] = edge_caps[g][x].min(k);
        }
    }
    for x in 0..n {
        cap[n + x][t] = k;
    }

    let mut flow = 0;
    loop {
        // BFS for an augmenting path in the residual graph
        let mut prev = vec![usize::MAX; v];
        prev[s] = s;
        let mut queue = vec![s];
        let mut qi = 0;
        while qi < queue.len() && prev[t] == usize::MAX {
            let u = queue[qi];
            qi += 1;
            for w in 0..v {
                if prev[w] == usize::MAX && cap[u][w] > 0 {
                    prev[w] = u;
                    queue.push(w);
                }
            }
        }
        if prev[t] == usize::MAX {
            break;
        }
        let mut bottleneck = usize::MAX;
        let mut w = t;
        while w != s {
            bottleneck = bottleneck.min(cap[prev[w]][w]);
            w = prev[w];
        }
        let mut w = t;
        while w != s {
            cap[prev[w]][w] -= bottleneck;
            cap[w][prev[w]] += bottleneck;
            w = prev[w];
        }
        flow += bottleneck;
    }
    flow == k * n
}

// -------------------------------------------------------------- repair

/// Edit `grid` so that solution `kill` is no longer valid while `keep`
/// still is; false if no legal edit exists. Identical to genpenai.rs —
/// the argument survives any K: the moved cell held KILL's *only*
/// differing bull in the donor region, so KILL comes up one bull short
/// there, while all of KEEP's donor-region bulls stayed put and the
/// receiver only gains a non-KEEP-bull cell.
fn kill_solution(grid: &mut Grid, keep: &[Pos], kill: &[Pos], rng: &mut Rng) -> bool {
    let n = grid.len();

    // Bulls of `kill` that aren't bulls of `keep` — each a valid target.
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

/// Would `region of (r, c)` remain connected if (r, c) left it?
/// (Anchor trick, identical to genpenai.rs.)
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
/// Allocation-free; `wrapping_sub` turns the off-board 0-1 case into
/// usize::MAX, which the `< n` filter drops.
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

    /// Count k=1 solutions by trying every column permutation
    /// (independent reference, ported from single_solver.rs tests).
    fn brute_force_k1(regions: &Grid) -> usize {
        fn rec(
            regions: &Grid,
            r: usize,
            perm: &mut Vec<usize>,
            used: &mut Vec<bool>,
            count: &mut usize,
        ) {
            let n = regions.len();
            if r == n {
                let mut seen = vec![false; n];
                for (r, &c) in perm.iter().enumerate() {
                    let reg = regions[r][c];
                    if seen[reg] {
                        return;
                    }
                    seen[reg] = true;
                }
                *count += 1;
                return;
            }
            for c in 0..n {
                if used[c] || (r > 0 && perm[r - 1].abs_diff(c) == 1) {
                    continue;
                }
                used[c] = true;
                perm[r] = c;
                rec(regions, r + 1, perm, used, count);
                used[c] = false;
            }
        }

        let n = regions.len();
        let mut count = 0;
        rec(regions, 0, &mut vec![0; n], &mut vec![false; n], &mut count);
        count
    }

    /// All k=2 solutions by choosing a column pair per board row —
    /// an independent reference implementation for cross-checking.
    fn brute_force_k2(grid: &Grid) -> Vec<Vec<Pos>> {
        fn rec(
            grid: &Grid,
            r: usize,
            col_count: &mut Vec<usize>,
            reg_count: &mut Vec<usize>,
            rows: &mut Vec<(usize, usize)>,
            sols: &mut Vec<Vec<Pos>>,
        ) {
            let n = grid.len();
            if r == n {
                if col_count.iter().all(|&v| v == 2) && reg_count.iter().all(|&v| v == 2) {
                    sols.push(
                        rows.iter()
                            .enumerate()
                            .flat_map(|(y, &(a, b))| [(y, a), (y, b)])
                            .collect(),
                    );
                }
                return;
            }
            for c1 in 0..n {
                for c2 in c1 + 2..n {
                    if r > 0 {
                        let (p1, p2) = rows[r - 1];
                        if p1.abs_diff(c1) <= 1
                            || p1.abs_diff(c2) <= 1
                            || p2.abs_diff(c1) <= 1
                            || p2.abs_diff(c2) <= 1
                        {
                            continue; // touches a bull in the row above
                        }
                    }
                    if col_count[c1] >= 2 || col_count[c2] >= 2 {
                        continue;
                    }
                    let (g1, g2) = (grid[r][c1], grid[r][c2]);
                    if g1 == g2 {
                        if reg_count[g1] > 0 {
                            continue;
                        }
                    } else if reg_count[g1] >= 2 || reg_count[g2] >= 2 {
                        continue;
                    }

                    col_count[c1] += 1;
                    col_count[c2] += 1;
                    reg_count[g1] += 1;
                    reg_count[g2] += 1;
                    rows.push((c1, c2));
                    rec(grid, r + 1, col_count, reg_count, rows, sols);
                    rows.pop();
                    reg_count[g2] -= 1;
                    reg_count[g1] -= 1;
                    col_count[c2] -= 1;
                    col_count[c1] -= 1;
                }
            }
        }

        let n = grid.len();
        let mut sols = Vec::new();
        rec(
            grid,
            0,
            &mut vec![0; n],
            &mut vec![0; n],
            &mut Vec::new(),
            &mut sols,
        );
        sols
    }

    fn assert_valid_solution(grid: &Grid, sol: &[Pos], k: usize) {
        let n = grid.len();
        assert_eq!(sol.len(), k * n);
        let mut rows = vec![0; n];
        let mut cols = vec![0; n];
        let mut regs = vec![0; n];
        for &(r, c) in sol {
            rows[r] += 1;
            cols[c] += 1;
            regs[grid[r][c]] += 1;
        }
        for i in 0..n {
            assert!(
                rows[i] == k && cols[i] == k && regs[i] == k,
                "count constraint broken"
            );
        }
        for (i, &(r, c)) in sol.iter().enumerate() {
            for &(y, x) in &sol[i + 1..] {
                assert!(
                    r.abs_diff(y).max(c.abs_diff(x)) >= 2,
                    "bulls touch: {:?} {:?}",
                    (r, c),
                    (y, x)
                );
            }
        }
    }

    /// Every region 0..n present, and each one contiguous.
    fn assert_valid_regions(grid: &Grid) {
        let n = grid.len();
        for region in 0..n {
            let cells: Vec<Pos> = (0..n)
                .flat_map(|y| (0..n).map(move |x| (y, x)))
                .filter(|&(y, x)| grid[y][x] == region)
                .collect();
            assert!(!cells.is_empty(), "region {} is empty", region);

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

    /// Random n x n label grid with every region present.
    fn random_label_grid(n: usize, rng: &mut Rng) -> Option<Grid> {
        let grid: Grid = (0..n)
            .map(|_| (0..n).map(|_| rng.below(n)).collect())
            .collect();
        let mut seen = vec![false; n];
        for row in &grid {
            for &reg in row {
                seen[reg] = true;
            }
        }
        seen.iter().all(|&s| s).then_some(grid)
    }

    /// At multiplicity 1 Algorithm M *is* Algorithm X, so it must agree
    /// with a permutation brute force on random label grids.
    #[test]
    fn k1_matches_brute_force() {
        let n = 5;
        let mut rng = Rng::new(0xdead_beef);
        let mut tested = 0;
        while tested < 200 {
            let grid = match random_label_grid(n, &mut rng) {
                Some(g) => g,
                None => continue,
            };
            tested += 1;
            let expected = brute_force_k1(&grid).min(2);
            assert_eq!(
                DoubleSolver::new(&grid, 1).solve().len(),
                expected,
                "mismatch for regions {:?}",
                grid
            );
        }
    }

    /// The multiplicity machinery must count k=2 solutions exactly once
    /// each — this is precisely what the naive duplicated-column
    /// encoding gets wrong (2^n paths per solution).
    #[test]
    fn k2_matches_brute_force_on_random_labels() {
        let n = 6;
        let mut rng = Rng::new(42);
        let mut tested = 0;
        while tested < 200 {
            let grid = match random_label_grid(n, &mut rng) {
                Some(g) => g,
                None => continue,
            };
            tested += 1;
            let expected = brute_force_k2(&grid);
            let got = DoubleSolver::new(&grid, 2).solve();
            assert_eq!(
                got.len(),
                expected.len().min(2),
                "count mismatch for regions {:?}",
                grid
            );
            if expected.len() == 1 {
                let mut mine = got[0].clone();
                mine.sort();
                let mut theirs = expected[0].clone();
                theirs.sort();
                assert_eq!(mine, theirs, "solution mismatch for regions {:?}", grid);
            }
        }
    }

    /// Same cross-check on contiguous grown boards — closer to what the
    /// generator actually feeds the solver, and more likely to have
    /// 1 or 2 solutions than random label soup.
    #[test]
    fn k2_matches_brute_force_on_region_boards() {
        let n = 7;
        let mut rng = Rng::new(7);
        for _ in 0..50 {
            let grid = random_regions(n, &mut rng, num_capped(n));
            let expected = brute_force_k2(&grid);
            let got = DoubleSolver::new(&grid, 2).solve();
            assert_eq!(
                got.len(),
                expected.len().min(2),
                "count mismatch for regions {:?}",
                grid
            );
        }
    }

    #[test]
    fn solver_is_reusable_after_solving() {
        let mut rng = Rng::new(3);
        let grid = random_regions(10, &mut rng, num_capped(10));
        let mut solver = DoubleSolver::new(&grid, 2);
        let first = solver.solve().len();
        // links, needs, and bitmap must be fully restored: solving
        // again agrees
        assert_eq!(solver.solve().len(), first);
    }

    /// The prefilters must never reject a solvable board.
    #[test]
    fn prefilters_are_sound() {
        let n = 7;
        let mut rng = Rng::new(11);
        for _ in 0..200 {
            let grid = random_regions(n, &mut rng, num_capped(n));
            if !regions_feasible(&grid, K) || !matchable(&grid, K) {
                assert!(
                    DoubleSolver::new(&grid, K).solve().is_empty(),
                    "prefilter rejected a solvable board: {:?}",
                    grid
                );
            }
        }
    }

    #[test]
    fn generated_puzzles_are_unique_and_valid() {
        for seed in 0..4 {
            let grid = generate(10, seed);
            assert_valid_regions(&grid);
            let sols = DoubleSolver::new(&grid, K).solve();
            assert_eq!(sols.len(), 1, "seed={} not unique: {:?}", seed, grid);
            assert_valid_solution(&grid, &sols[0], K);
        }
    }

    #[test]
    fn deterministic_for_a_seed() {
        assert_eq!(generate(10, 42), generate(10, 42));
    }

    fn bench(n: usize, runs: u64) {
        let start = std::time::Instant::now();
        for seed in 0..runs {
            let grid = generate(n, seed);
            assert!(DoubleSolver::new(&grid, K).is_unique());
        }
        let total = start.elapsed();
        println!("n={}: {:?} avg over {} boards", n, total / runs as u32, runs);
    }

    /// Not run by default. Time generation with e.g.:
    /// `./gendouble_test bench_n10 --ignored --nocapture`
    #[test]
    #[ignore]
    fn bench_n10() {
        bench(10, 50);
    }

    #[test]
    #[ignore]
    fn bench_n12() {
        bench(12, 50);
    }

    #[test]
    #[ignore]
    fn bench_n14() {
        bench(14, 10);
    }

    /// Like generate_capped, but with a parameterized solve budget and
    /// a wall-clock limit: returns None once the limit passes.
    /// Generation time has a heavy tail, and a sweep would rather count
    /// a timeout than wait one out (its time enters stats as the limit).
    /// NOTE: this mirrors generate_capped's loop — if that loop changes,
    /// change this one too or the sweeps tune the wrong algorithm.
    fn generate_limited(
        n: usize,
        seed: u64,
        caps: (usize, usize),
        budget: usize,
        limit: std::time::Duration,
    ) -> Option<Grid> {
        let deadline = std::time::Instant::now() + limit;
        let mut rng = Rng::new(seed);

        loop {
            if std::time::Instant::now() > deadline {
                return None;
            }
            let mut grid = random_regions(n, &mut rng, caps);
            if !regions_feasible(&grid, K) || !matchable(&grid, K) {
                continue;
            }
            let sols = match DoubleSolver::new(&grid, K).solve_within(budget) {
                Some(sols) => sols,
                None => continue,
            };
            let (mut keep, mut other) = match &sols[..] {
                [] => continue,
                [_] => return Some(grid),
                [a, b, ..] => (a.clone(), b.clone()),
            };
            'repair: for _ in 0..MAX_REPAIRS {
                if kill_solution(&mut grid, &keep, &other, &mut rng) {
                } else if kill_solution(&mut grid, &other, &keep, &mut rng) {
                    std::mem::swap(&mut keep, &mut other);
                } else {
                    break 'repair;
                }
                let mut known = keep.clone();
                known.sort_unstable();
                match DoubleSolver::new(&grid, K).solve_other_within(budget, &known) {
                    None => break 'repair,
                    Some(None) => return Some(grid),
                    Some(Some(o)) => other = o,
                }
            }
        }
    }

    /// Fixed seeds, medians, timeout-censored — heavy-tailed times make
    /// small-sample means lie (one unlucky board dominates).
    fn sweep<S: std::fmt::Debug + Copy>(
        label: &str,
        n: usize,
        runs: u64,
        settings: &[S],
        limit_ms: u64,
        gen: impl Fn(u64, S) -> Option<Grid>,
    ) {
        let limit = std::time::Duration::from_millis(limit_ms);
        for &setting in settings {
            let mut times: Vec<f64> = Vec::new();
            let mut timeouts = 0;
            for seed in 0..runs {
                let t = std::time::Instant::now();
                if gen(seed, setting).is_none() {
                    timeouts += 1;
                }
                times.push((t.elapsed().min(limit)).as_secs_f64() * 1000.0);
            }
            times.sort_by(f64::total_cmp);
            let median = times[times.len() / 2];
            let mean = times.iter().sum::<f64>() / times.len() as f64;
            println!(
                "n={} {}={:?}: median={:.0} ms, mean={:.0} ms, timeouts={}/{}",
                n, label, setting, median, mean, timeouts, runs
            );
        }
    }

    /// Not run by default. Find the cap-count sweet spot for a size.
    #[test]
    #[ignore]
    fn sweep_caps_n10() {
        let caps = [(3, 5), (4, 6), (5, 7), (6, 8), (7, 9), (8, 10)];
        sweep("caps", 10, 60, &caps, 1_000, |seed, c| {
            generate_limited(10, seed, c, SOLVE_BUDGET, std::time::Duration::from_millis(1_000))
        });
    }

    #[test]
    #[ignore]
    fn sweep_caps_n12() {
        let caps = [(5, 7), (6, 8), (7, 9), (8, 10), (9, 11), (10, 12)];
        sweep("caps", 12, 40, &caps, 3_000, |seed, c| {
            generate_limited(12, seed, c, SOLVE_BUDGET, std::time::Duration::from_millis(3_000))
        });
    }

    #[test]
    #[ignore]
    fn sweep_caps_n14() {
        let caps = [(9, 11), (10, 12), (11, 13), (12, 14), (13, 14), (14, 14)];
        sweep("caps", 14, 30, &caps, 8_000, |seed, c| {
            generate_limited(14, seed, c, SOLVE_BUDGET, std::time::Duration::from_millis(8_000))
        });
    }

    /// Not run by default. How hard should the budget censor tails?
    #[test]
    #[ignore]
    fn sweep_budget_n14() {
        let budgets = [1_000usize, 2_000, 5_000, 10_000, 25_000];
        sweep("budget", 14, 30, &budgets, 8_000, |seed, b| {
            generate_limited(14, seed, num_capped(14), b, std::time::Duration::from_millis(8_000))
        });
    }

    /// Not run by default. Where does generation time go?
    #[test]
    #[ignore]
    fn breakdown_n10() {
        breakdown(10, 30);
    }

    #[test]
    #[ignore]
    fn breakdown_n12() {
        breakdown(12, 30);
    }

    #[test]
    #[ignore]
    fn breakdown_n14() {
        breakdown(14, 10);
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

            if !regions_feasible(&grid, K) || !matchable(&grid, K) {
                filtered += 1;
                rerolls += 1;
                continue;
            }

            let t = Instant::now();
            let sols = DoubleSolver::new(&grid, K).solve_within(SOLVE_BUDGET);
            solve_time += t.elapsed();
            solves += 1;

            let (mut keep, mut other) = match sols.as_deref() {
                None => {
                    over_budget += 1;
                    rerolls += 1;
                    continue;
                }
                Some([]) => {
                    zero_sol += 1;
                    rerolls += 1;
                    continue;
                }
                Some([_]) => {
                    done += 1;
                    continue;
                }
                Some([a, b, ..]) => (a.clone(), b.clone()),
            };

            let mut ok = false;
            for _ in 0..MAX_REPAIRS {
                if kill_solution(&mut grid, &keep, &other, &mut rng) {
                } else if kill_solution(&mut grid, &other, &keep, &mut rng) {
                    std::mem::swap(&mut keep, &mut other);
                } else {
                    break;
                }
                repairs += 1;

                let mut known = keep.clone();
                known.sort_unstable();
                let t = Instant::now();
                let res = DoubleSolver::new(&grid, K).solve_other_within(SOLVE_BUDGET, &known);
                solve_time += t.elapsed();
                solves += 1;
                match res {
                    None => {
                        over_budget += 1;
                        break;
                    }
                    Some(None) => {
                        ok = true;
                        break;
                    }
                    Some(Some(o)) => other = o,
                }
            }
            if ok {
                done += 1;
            } else {
                rerolls += 1;
            }
        }

        println!(
            "{} boards: rerolls={} (prefiltered {}, zero-solution {}, over-budget {}), repairs={}, solver calls={}",
            target, rerolls, filtered, zero_sol, over_budget, repairs, solves
        );
        println!(
            "time in solver: {:?}, in region growth: {:?}",
            solve_time, grow_time
        );
    }
}
