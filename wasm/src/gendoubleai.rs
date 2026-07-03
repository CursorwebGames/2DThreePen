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
//! One deliberate simplification vs. Knuth's TAOCP 7.2.2.1 formulation:
//! Knuth picks ONE option per search node and may return to a partially
//! filled item later, relying on the tweak/untweak dance to keep the
//! enumeration duplicate-free. Here the chosen item is *saturated in
//! place* instead: a single search node picks all `need` remaining
//! options for the item at once, in vertical-list order, so every
//! K-subset is enumerated exactly once with no extra machinery. The
//! branch choice is a deterministic function of the search state, so
//! each solution is reached by exactly one path and the count (capped
//! at 2) stays trustworthy.
//!
//! What this buys over the region-MRV backtracker in gendouble.py:
//! - MRV runs over all 3n items (rows, columns, AND regions), not just
//!   regions — a starved row can become the branch point.
//! - Candidate elimination is incremental (dancing links), not a full
//!   grid scan per node.
//! What it loses: the per-node live-cell counts of the Python solver
//! see adjacency; DLX list sizes don't (a listed option may still
//! conflict with a placed bull). Adjacency stays imperative, exactly as
//! in single_solver.rs: a `placed` bitmap plus `conflicts` skips any
//! candidate touching a bull already on the board.
//!
//! This file is a self-contained prototype (not yet declared in
//! lib.rs): its own RNG, solver, and generator, so it can be compiled
//! and tested standalone with
//! `rustc --test -O wasm/src/gendoubleai.rs -o gendouble_test`.

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

/// Per-solve work budget (recursion steps). Boards whose uniqueness or
/// unsolvability proof is pathologically expensive get discarded rather
/// than paid for (cf. genpenai.rs).
const SOLVE_BUDGET: usize = 200_000;

/// How many regions get a size cap (inclusive range). Tuned at n=10 in
/// research/gendouble.py; retune with sweep_caps_* for other sizes.
const NUM_CAPPED: (usize, usize) = (5, 7);

/// Size cap for capped regions (inclusive range). The floor is 3, not
/// 1 as in the single generator: a region must hold 2 non-touching
/// bulls, impossible below 3 cells. Measured in Python at n=10: very
/// tight caps (3,5) backfire — most rolls become unsolvable.
const CAP_SIZE: (usize, usize) = (3, 9);

/// Growth brings every region to this size before free growth starts.
/// 3 cells are necessary for 2 non-touching bulls, not sufficient (an
/// L-tromino still fails) — `regions_feasible` is the real check.
const MIN_REGION: usize = 3;

/// Small deterministic RNG (SplitMix64), duplicated from genpenai.rs so
/// this prototype stays standalone.
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

/// Exact cover with multiplicities over dancing links.
///
/// Items (3n of them): one per board row, per board column, per region;
/// each must be covered exactly `k` times. Options (n^2 of them): one
/// per cell — placing a bull on (r, c) covers items r, n+c, 2n+g once
/// each. A set of options covering every item exactly k times is a
/// solution, minus adjacency, which is enforced imperatively.
///
/// Node layout: everything lives in flat arrays indexed by node id.
/// Id 0 is the horizontal sentinel, ids 1..=3n are the item headers,
/// and option o (cell r*n+c) owns the three consecutive ids
/// `first + 3o .. first + 3o + 3` — so an option's sibling nodes are
/// found by arithmetic instead of horizontal links.
pub struct DoubleSolver {
    n: usize,

    /// Id of the first option node (= 3n + 1).
    first: usize,

    /// Horizontal doubly-linked list of *active* item headers (an item
    /// leaves this list exactly when its `need` hits 0).
    hprev: Vec<usize>,
    hnext: Vec<usize>,

    /// Vertical doubly-linked list per item: header + available option
    /// nodes.
    up: Vec<usize>,
    down: Vec<usize>,

    /// Option node -> its item header id.
    col: Vec<usize>,

    /// Option node -> its option id (cell index r*n + c).
    opt: Vec<usize>,

    /// Per header: option nodes still in its vertical list.
    size: Vec<usize>,

    /// Per header: bulls this item still needs. This is Algorithm M's
    /// multiplicity — Algorithm X is the special case need = 1.
    need: Vec<usize>,

    /// Bitmap of board cells currently holding a bull, indexed by cell
    /// id r*n + c.
    placed: Vec<bool>,

    /// Remaining work budget for the current solve call.
    steps: usize,
}

impl DoubleSolver {
    /// Takes an n x n region grid with labels 0..n.
    pub fn new(grid: &Grid, k: usize) -> DoubleSolver {
        let n = grid.len();
        let cols = 3 * n;
        let first = cols + 1;
        let total = first + 3 * n * n;

        let mut s = DoubleSolver {
            n,
            first,
            hprev: (0..=cols).map(|i| if i == 0 { cols } else { i - 1 }).collect(),
            hnext: (0..=cols).map(|i| if i == cols { 0 } else { i + 1 }).collect(),
            up: (0..total).collect(),
            down: (0..total).collect(),
            col: vec![0; total],
            opt: vec![0; total],
            size: vec![0; cols + 1],
            need: vec![k; cols + 1],
            placed: vec![false; n * n],
            steps: usize::MAX,
        };
        s.need[0] = 0; // sentinel is not an item

        for r in 0..n {
            assert_eq!(grid[r].len(), n, "grid must be an n x n square");
            for c in 0..n {
                let g = grid[r][c];
                assert!(g < n, "region labels must be 0..n");
                let o = r * n + c;
                // items covered by a bull on this cell
                let items = [1 + r, 1 + n + c, 1 + 2 * n + g];
                for (t, &h) in items.iter().enumerate() {
                    let nd = first + 3 * o + t;
                    s.col[nd] = h;
                    s.opt[nd] = o;
                    // append at the bottom of h's vertical list
                    let bottom = s.up[h];
                    s.down[bottom] = nd;
                    s.up[nd] = bottom;
                    s.down[nd] = h;
                    s.up[h] = nd;
                    s.size[h] += 1;
                }
            }
        }
        s
    }

    /// Whether the puzzle has exactly one solution.
    pub fn is_unique(&mut self) -> bool {
        self.solve().len() == 1
    }

    /// Returns at most 2 solutions.
    pub fn solve(&mut self) -> Vec<Vec<Pos>> {
        self.solve_within(usize::MAX).unwrap()
    }

    /// Like `solve`, but gives up once the search has taken `budget`
    /// recursion steps, returning None. Aborting unwinds cleanly, so
    /// the solver stays reusable.
    pub fn solve_within(&mut self, budget: usize) -> Option<Vec<Vec<Pos>>> {
        self.steps = budget;
        let mut sols = Vec::new();
        self.solve_rec(&mut Vec::new(), &mut sols);
        if self.steps == 0 {
            None
        } else {
            Some(sols)
        }
    }

    /// Detach option node j from its vertical list.
    fn detach(&mut self, j: usize) {
        self.down[self.up[j]] = self.down[j];
        self.up[self.down[j]] = self.up[j];
        self.size[self.col[j]] -= 1;
    }

    /// Restore previously detached node j (its own links still point at
    /// its old neighbors — the dancing links trick).
    fn restore(&mut self, j: usize) {
        self.down[self.up[j]] = j;
        self.up[self.down[j]] = j;
        self.size[self.col[j]] += 1;
    }

    /// Hide option (of node i) from every list except the one i is in.
    fn hide(&mut self, i: usize) {
        let base = self.first + 3 * self.opt[i];
        for j in base..base + 3 {
            if j != i {
                self.detach(j);
            }
        }
    }

    fn unhide(&mut self, i: usize) {
        let base = self.first + 3 * self.opt[i];
        for j in (base..base + 3).rev() {
            if j != i {
                self.restore(j);
            }
        }
    }

    /// Item h is saturated: remove it from the header list and hide all
    /// its remaining options from other items' lists. Identical to
    /// Algorithm X's cover — the difference is only *when* it is called
    /// (need hitting 0, not the first pick).
    fn cover(&mut self, h: usize) {
        self.hnext[self.hprev[h]] = self.hnext[h];
        self.hprev[self.hnext[h]] = self.hprev[h];
        let mut i = self.down[h];
        while i != h {
            self.hide(i);
            i = self.down[i];
        }
    }

    fn uncover(&mut self, h: usize) {
        let mut i = self.up[h];
        while i != h {
            self.unhide(i);
            i = self.up[i];
        }
        self.hnext[self.hprev[h]] = h;
        self.hprev[self.hnext[h]] = h;
    }

    /// Does a bull on cell `id` touch a bull already on the board?
    fn conflicts(&self, id: usize) -> bool {
        let n = self.n;
        let (y, x) = (id / n, id % n);
        for ny in y.saturating_sub(1)..=(y + 1).min(n - 1) {
            for nx in x.saturating_sub(1)..=(x + 1).min(n - 1) {
                if (ny, nx) != (y, x) && self.placed[ny * n + nx] {
                    return true;
                }
            }
        }
        false
    }

    fn solve_rec(&mut self, csol: &mut Vec<Pos>, sols: &mut Vec<Vec<Pos>>) {
        // out of budget: abandon this subtree. Parents still restore
        // their covers on the way out, so everything unwinds cleanly.
        if self.steps == 0 {
            return;
        }
        self.steps -= 1;

        // Choose the item with the fewest ways to saturate it (MRV,
        // measured as C(size, need) — a 4-option item needing 2 has 6
        // candidate pairs, a full row has hundreds). Any item with more
        // needs than options kills the branch outright.
        let mut best = 0;
        let mut best_bf = u64::MAX;
        let mut h = self.hnext[0];
        while h != 0 {
            if self.size[h] < self.need[h] {
                return; // item can no longer be satisfied
            }
            let bf = choose_count(self.size[h], self.need[h]);
            if bf < best_bf {
                best_bf = bf;
                best = h;
            }
            h = self.hnext[h];
        }

        if best == 0 {
            // no active items left => every item got exactly k bulls
            sols.push(csol.clone());
            return;
        }

        // Saturate `best` in one node: cover it, then enumerate all
        // size-m subsets of its option list in list order (see module
        // docs for why this replaces Knuth's tweak/untweak).
        let m = self.need[best];
        self.need[best] = 0;
        self.cover(best);
        self.pick(best, self.down[best], m, csol, sols);
        self.uncover(best);
        self.need[best] = m;
    }

    /// Choose `remaining` more options for item p from its vertical
    /// list, starting at node `start` (list order => each subset is
    /// enumerated exactly once). p is already covered, so p's list is
    /// stable throughout: nested covers can never touch it — any option
    /// still listed under another item has no p-node (cover(p) hid it).
    fn pick(
        &mut self,
        p: usize,
        start: usize,
        remaining: usize,
        csol: &mut Vec<Pos>,
        sols: &mut Vec<Vec<Pos>>,
    ) {
        if remaining == 0 {
            self.solve_rec(csol, sols);
            return;
        }

        let mut i = start;
        while i != p {
            if sols.len() >= 2 || self.steps == 0 {
                return;
            }

            let o = self.opt[i];
            let base = self.first + 3 * o;
            // Legal iff the cell touches no placed bull (this also
            // rejects touching pairs within the current subset — the
            // earlier pick is already on the board) and its other two
            // items can still absorb a bull.
            let legal = !self.conflicts(o)
                && (base..base + 3).all(|j| j == i || self.need[self.col[j]] >= 1);

            if legal {
                // commit: the option's non-p items each get one bull;
                // an item hitting 0 needs is saturated and covered.
                // Nothing needs hiding here — cover(p) already hid this
                // option from its other lists.
                self.placed[o] = true;
                csol.push((o / self.n, o % self.n));
                for j in base..base + 3 {
                    if j != i {
                        let h = self.col[j];
                        self.need[h] -= 1;
                        if self.need[h] == 0 {
                            self.cover(h);
                        }
                    }
                }

                self.pick(p, self.down[i], remaining - 1, csol, sols);

                for j in (base..base + 3).rev() {
                    if j != i {
                        let h = self.col[j];
                        if self.need[h] == 0 {
                            self.uncover(h);
                        }
                        self.need[h] += 1;
                    }
                }
                csol.pop();
                self.placed[o] = false;
            }

            i = self.down[i];
        }
    }
}

/// Binomial coefficient C(s, m) for tiny m (callers guarantee s >= m).
/// Multiplying before dividing keeps every intermediate integral.
fn choose_count(s: usize, m: usize) -> u64 {
    let mut r = 1u64;
    for t in 0..m {
        r = r * (s - t) as u64 / (t as u64 + 1);
    }
    r
}

// ---------------------------------------------------------- generator

/// Generate an n x n double bullpen with exactly one K=2 solution.
pub fn generate(n: usize, seed: u64) -> Grid {
    generate_capped(n, seed, NUM_CAPPED)
}

fn generate_capped(n: usize, seed: u64, caps: (usize, usize)) -> Grid {
    let mut rng = Rng::new(seed);

    // Outer loop: reroll from scratch when a board is hopeless.
    loop {
        let mut grid = random_regions(n, &mut rng, caps);

        // Two cheap necessary conditions before paying for a solve:
        // every region must fit K non-touching bulls, and regions must
        // be capacity-K matchable against rows and columns.
        if !regions_feasible(&grid, K) || !matchable(&grid, K) {
            continue;
        }

        for _ in 0..MAX_REPAIRS {
            let sols = match DoubleSolver::new(&grid, K).solve_within(SOLVE_BUDGET) {
                Some(sols) => sols,
                None => break, // proof too expensive — discard the board
            };
            match sols.len() {
                0 => break, // dead board (only possible on a fresh roll)
                1 => return grid,
                // 2 solutions: edit the grid to kill one and keep the
                // other; if neither of sols[1]'s differing bulls is
                // movable, try the symmetric edit.
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

/// Cheap necessary condition for solvability: regions must be matchable
/// with the rows they touch (each region supplying k bulls, each row
/// absorbing k), and likewise with columns. A board failing either
/// check is provably unsolvable, no search needed. Note a region's two
/// bulls MAY share a row, and the duplicated graph correctly allows it.
fn matchable(grid: &Grid, k: usize) -> bool {
    let n = grid.len();
    let mut rows = vec![vec![false; n]; n];
    let mut cols = vec![vec![false; n]; n];
    for r in 0..n {
        for c in 0..n {
            rows[grid[r][c]][r] = true;
            cols[grid[r][c]][c] = true;
        }
    }
    capacity_matching(&rows, k) && capacity_matching(&cols, k)
}

/// Perfect matching after duplicating every node k times — the standard
/// node-splitting reduction of capacity-k b-matching to plain matching
/// via augmenting paths.
fn capacity_matching(touch: &[Vec<bool>], k: usize) -> bool {
    fn augment(
        a: usize,
        k: usize,
        touch: &[Vec<bool>],
        seen: &mut [bool],
        matched: &mut [usize],
    ) -> bool {
        for b in 0..matched.len() {
            if touch[a / k][b / k] && !seen[b] {
                seen[b] = true;
                if matched[b] == usize::MAX || augment(matched[b], k, touch, seen, matched) {
                    matched[b] = a;
                    return true;
                }
            }
        }
        false
    }

    let nk = touch.len() * k;
    let mut matched = vec![usize::MAX; nk];
    (0..nk).all(|a| augment(a, k, touch, &mut vec![false; nk], &mut matched))
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
            let grid = random_regions(n, &mut rng, NUM_CAPPED);
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
        let grid = random_regions(10, &mut rng, NUM_CAPPED);
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
            let grid = random_regions(n, &mut rng, NUM_CAPPED);
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

    /// Not run by default. Find the cap-count sweet spot for a size.
    #[test]
    #[ignore]
    fn sweep_caps_n10() {
        sweep_caps(10, 20);
    }

    #[test]
    #[ignore]
    fn sweep_caps_n12() {
        sweep_caps(12, 20);
    }

    fn sweep_caps(n: usize, runs: u64) {
        for caps in [(0, 0), (2, 4), (4, 6), (5, 7), (6, 8), (8, 10)] {
            let start = std::time::Instant::now();
            for seed in 0..runs {
                generate_capped(n, seed, caps);
            }
            println!(
                "n={} caps={:?} size={:?}: {:?} avg",
                n,
                caps,
                CAP_SIZE,
                start.elapsed() / runs as u32
            );
        }
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
            let mut grid = random_regions(n, &mut rng, NUM_CAPPED);
            grow_time += t.elapsed();

            if !regions_feasible(&grid, K) || !matchable(&grid, K) {
                filtered += 1;
                rerolls += 1;
                continue;
            }

            let mut ok = false;
            for _ in 0..MAX_REPAIRS {
                let t = Instant::now();
                let sols = DoubleSolver::new(&grid, K).solve_within(SOLVE_BUDGET);
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
            "{} boards: rerolls={} (prefiltered {}, zero-solution {}, over-budget {}), repairs={}, solver calls={}",
            target, rerolls, filtered, zero_sol, over_budget, repairs, solves
        );
        println!(
            "time in solver: {:?}, in region growth: {:?}",
            solve_time, grow_time
        );
    }
}
