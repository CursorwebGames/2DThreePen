# AGENTS.md

## Project Priorities
Order of importance:
1. Puzzle generation throughput
2. Puzzle uniqueness correctness
3. Solver speed (oftentimes speeds puzzle generation throughput)
4. Code maintainability (this includes statistics and data gathering)

This is a performance-sensitive project. Avoid introducing allocations, cloning, or abstraction layers in hot paths unless benchmarks demonstrate no meaningful regression.

Code should be maintainable and readable enough that someone is able to learn from it and implement it from scratch without the help of AI.

# Algorithm
A *bullpen* (aka Star Battle, aka Queens) puzzle is an `n x n` grid partitioned into
`n` contiguous regions. A solution places exactly **K** "bulls" in every row,
every column, and every region, with no two bulls touching (Chebyshev distance
\>= 2, i.e. not even diagonally adjacent). A puzzle must also contain exactly **one**
solution.

- **Single** (K=1): one bull per row/col/region. Canonical implementation is
  [wasm/src/genpenai.rs](wasm/src/genpenai.rs) (Rust, shipped) with the solver
  in [wasm/src/single_solver.rs](wasm/src/single_solver.rs). A Python port
  lives in [research/gensingle.py](research/gensingle.py).
- **Double** (K=2): two bulls per row/col/region. Canonical implementation is
  [wasm/src/gendoubleai.rs](wasm/src/gendoubleai.rs) (Rust, self-contained:
  generator + Algorithm M solver in one file); K is a runtime parameter on the
  solver so K=3 ("threepen") falls out later.
  [research/gendouble.py](research/gendouble.py) is the earlier Python
  prototype (region-MRV backtracking, superseded by the Rust solver).

The two share one generation strategy. They differ in **the solver** and in a
**handful of K-driven feasibility checks/growth tweaks**. Both generators run the same loop (`generate` in each file):

```yaml
loop forever: # reroll from scratch when a board is hopeless
    grid = random_regions(n)
    if not cheap_prefilters(grid):
        continue # provably unsolvable
    repeat MAX_REPAIRS times: # give up if board can't converge to single solution
        sols = solve(grid, cap=2)
        if sols is None: break # too hard to solve (recursion depth)
        if len(sols) == 0: break
        if len(sols) == 1: return grid
        if not kill_solution(...): break # exactly two solutions, prune (if possible)
```

## 1. Random Generation
Randomly generating regions produces better puzzles than generating regions around a fixed solution, since a fixed solution often spawns many alternative solutions.

### Region growth (`random_regions`)
Drop `n` seed cells, one per region, then flood outward. This is like flood-fill except the stack has random removal. Each cell pushes <=4 neighbors ever, so filling the board is `O(n^2)`.

**Size caps.** A random subset of regions is temporarily capped during growth to reduce the solver's branching factor and encourage fewer solutions. Too many caps hurt generation success, so the cap count is tuned empirically (`num_capped` in Rust; `NUM_CAPPED`/`CAP_SIZE` in Python). Caps are only a generation heuristic; isolated cells are merged afterward.

### Cheap prefilter: matching (`matchable`)
Before running an expensive solver, reject provably unsolvable boards using a fast bipartite matching check implemented with the Hopcroft–Karp augmenting-path algorithm. Any valid solution requires consistent assignment of constraints across rows, columns, and regions, forming a matching problem between regions and the rows/columns they intersect. If matching fails, the board is unsatisfiable; if it succeeds, nothing is guaranteed. In Rust, this filter removes ~96% of invalid boards at `n=20`.

## 2. Keep + kill (the repair step)
This is the heart of the generator and is **identical in both** (`kill_solution` + `stays_contiguous`).

When a board has exactly two solutions, label them **KEEP** (survives) and **KILL** (to be destroyed). Pick a bull cell of KILL that is *not* a bull of KEEP, and move that single cell into an orthogonally adjacent region. Then:

- **KILL dies.** Its bull at `(r, c)` was the only bull KILL had in the donor region. After swapping `(r, c)`, that region no longer satisfies KILL's constraints, so KILL is invalid.
- **KEEP survives.** KEEP's configuration is unaffected in the donor region, and the receiving region gains a non-KEEP bull cell, so all KEEP constraints remain satisfied.

Each edit eliminates at least one solution and never eliminates KEEP. Two legality guards keep the board well-formed:

- **`stays_contiguous`**: the donor region must remain connected after removing `(r, c)`. This is checked via a connectivity test (flood-fill from an anchor, ensuring all remaining cells are still reachable without `(r, c)`).
- The moved cell must enter an **orthogonally adjacent** region, preserving contiguity in the receiver as well.

If no differing KILL bull is movable, try the symmetric operation (kill KEEP, keep KILL) before abandoning and rerolling.

> **Why this matters**: Keep/kill guarantees a valid reduction step, minimizing solves which dominate runtime.

## 3. The single-board solver -- exact cover / Algorithm X (DLX)
[wasm/src/single_solver.rs](wasm/src/single_solver.rs), `SingleSolver`.

K=1 is a clean **exact cover** problem, so it uses Knuth's Algorithm X with
dancing links (the `Matrix` of doubly-linked nodes in
[wasm/src/matrix.rs](wasm/src/matrix.rs)).

**Encoding.** `3n` columns = one per row constraint, one per column constraint,
one per region constraint. Each cell `(r, c)` of region `g` becomes one matrix
row covering exactly three columns: `r`, `n + c`, `2n + g`. A set of matrix rows
that covers all `3n` columns exactly once = one bull per row, per column, per
region = a solution.

**Search (`count_sol_rec` / `solve_rec`).** Standard DLX:

1. **Pick the column with the fewest remaining candidates** (MRV) minimum remaining values.
2. `cover` it, then try each candidate row that satisfies it.
3. Adjacency isn't an exact-cover column, so it's enforced *imperatively*: a `placed[r*n+c]` bitmap plus `conflicts(id)` skips any candidate touching an already-placed bull (checked before covering, so there's nothing to undo).
4. Cover the candidate's other columns, recurse, then **uncover** to backtrack (dancing links restores the matrix in `O(1)` per node).

**Capping at 2.** Both `count_sol` and `solve` short-circuit once a second solution is found -- the generator only ever needs to know "0, 1, or 2+". `solve_within(budget)` aborts after `budget` recursion steps and returns `None`; the generator would rather discard hard boards than pay. Aborts unwind cleanly so the solver is reusable.

## 4. The double-board solver -- Algorithm M (DLX with multiplicities)
[wasm/src/gendoubleai.rs](wasm/src/gendoubleai.rs), `DoubleSolver`.

**Why not plain Algorithm X?** Exact cover means "cover each column *exactly
once*." K=2 needs "cover each constraint *exactly twice*." The naive hack --
duplicate each region column so it must be covered twice -- counts every
solution `2^n` times (the two bulls of a region are interchangeable), which
silently breaks the uniqueness check. The principled fix is Knuth's
**Algorithm M** (exact cover *with multiplicities*): every item carries a
`need` count starting at K, and an item is only *covered* (removed from the
header list, its options hidden) when its `need` reaches 0. Algorithm X is the
special case need = 1. K is a runtime parameter, so K=1 works too (used for
cross-checking) and K=3 falls out for free.

Cross-checked against two independent brute forces: k=1 vs column
permutations, k=2 vs row-by-row column-pair enumeration. Also replaces the
Python prototype's region-MRV backtracker; the earlier version of this section
described that solver ([research/gendouble.py](research/gendouble.py),
`solve_up_to_two`).

**Encoding.** Same `3n` items as the single solver (row, column, region), each
needing K covers. One option per cell `(r, c)`: it covers items `r`, `n+c`,
`2n+g` once each. Nodes live in flat arrays; option `o` owns 3 consecutive
node ids, so an option's siblings are found by arithmetic, not horizontal
links.

**Search (`solve_rec`).** Deviates from textbook DLX in three load-bearing
ways:

1. **MRV over all `3n` items, by spare = `size - need`** (Knuth's branching
   degree for multiplicities) -- a starved *row* can become the branch point,
   not just a tight region. The same scan doubles as the dead-branch prune:
   any item with `size < need` kills the node immediately. No items left on
   the header list = every item got exactly K bulls = record a solution.
2. **One bull per node, tweak-style.** Each node places ONE bull for the
   chosen item (not all K at once), then recurses with full re-MRV -- so a
   half-filled item can be interrupted by whatever became tightest. Tried
   candidates stay detached from *every* list for the rest of the node's loop
   ("tweaked" off), so deeper picks for the same item only come from further
   down its list -- each K-subset is enumerated exactly once, no duplicates.
   Because the branch item is never covered early, list sizes stay honest
   everywhere and the `size < need` prune is sound at every node.
3. **Adjacency lives in the links** (unlike the single solver's per-candidate
   bitmap check): committing a bull detaches every still-listed option in its
   3x3 neighborhood (`block_neighbors`, gated by a per-cell `blocked` *count*
   -- two bulls can block the same cell, and removing one must leave it
   blocked). So item sizes count only genuinely placeable cells, `size < need`
   is exactly the Python solver's adjacency-aware live-count prune maintained
   incrementally, and candidates drawn from a list never need an adjacency
   test at all.

**Undo is a trail, not paired cover/uncover.** Cover and adjacency hiding both
detach nodes, and can race for the same node (an `attached` flag makes "detach
if attached" safe) -- so backtracking can't just re-run covers in reverse.
Instead every detached node (and de-listed header) is pushed onto one stack,
and backtracking pops to a mark; the search is strictly LIFO, so popping
restores links in exactly the reverse order they were broken, whichever
mechanism broke them.

**Generator-facing API.** Caps at 2 solutions and honors a step budget charged
per *committed placement* (`solve_within`, returns `None` over budget; aborts
unwind cleanly so the solver stays reusable). Two extras the repair loop
leans on: `reset(&grid)` relinks the whole structure in place so re-solves
across dozens of repair edits are allocation-free, and
`solve_other_within(budget, known)` hunts for ONE solution differing from the
already-known survivor instead of re-finding two -- only the final "yes, it's
unique" answer pays for a full-tree proof.

**Single vs double DLX raced at K=1** (`bench_double_solver_k1_*`): the
Algorithm M solver is ~1.6x faster at n=15 but ~1.3x slower at n=20 --
adjacency-in-the-links bookkeeping (~24 detaches per placement) outweighs its
pruning when need = 1 covers items immediately anyway. So the single generator
keeps its dedicated solver.


## 5. Single vs double - exactly what differs
The generate loop, region growth, caps, the `matchable` prefilter idea, and the
**entire keep/kill repair** are shared. Double differs only where K=2 forces it:

| | Single (K=1) | Double (K=2) |
|---|---|---|
| **Solver** | Exact cover / Algorithm X with dancing links (`single_solver.rs`); adjacency imperative via `placed` bitmap + `conflicts` | Algorithm M -- DLX with per-item `need` counts (`gendoubleai.rs`); adjacency in the links, trail undo |
| **Min region size** | 1 (a region just needs 1 cell) | 3, enforced by a round-robin growth phase -- 2 non-touching bulls need >=3 cells |
| **Region feasibility check** | none needed | `regions_feasible`: every region must actually fit K pairwise non-adjacent bulls (size 3 isn't enough -- an L-tromino has 3 cells but all touch) |
| **Matching prefilter** | plain 1-to-1 region<->row / region<->col bipartite matching (Hopcroft-Karp) | max-flow (Edmonds-Karp): region supplies K, row/col absorbs K, and the region->line edge cap is the max *spread-out* (non-touching) bulls the region fits in that line -- stronger than matching, catches most rerolls pre-solve |
| **Cap floor** | `CAP_SIZE` can go down to 1 | `CAP_SIZE` floor is 3 (same min-region reason) |

Everything else -- incremental frontier growth, stranded-cell absorption, the
step budget, the reroll-on-hopeless logic, and keep/kill -- is the same algorithm
in both.

# Data Collection
We need to collect as much useful data as possible -- such as benchmarks and number of failures. This is important to iterating magic constants and finding possible optimizations. However, data collection must be as non-intrusive as possible, and **cannot** affect production performance in any way.

# AI Double Notes
How the K=2 generator (`wasm/src/gendoubleai.rs`) got to ~200 ms/board at n=14,
in order, with what each step was worth. Every step was validated against two
independent brute forces (k=1 permutations, k=2 row-pairs) before benchmarking.

1. **First Rust port** (Algorithm M, saturation branching; adjacency imperative
   like `single_solver.rs`): n=14 at 13.2 s avg. Roughly tied with CPython's
   region-MRV solver -- meaning the search tree was ~50-100x larger and the
   language win was being spent masking a weaker search.
2. **Adjacency into the links** (3-4.4x): placing a bull detaches every option
   in its 3x3 from all lists (`blocked` counts). Column sizes become true live
   counts, so `size < need` is the adjacency-aware prune, maintained
   incrementally. Undo unified into one trail stack (strict LIFO).
3. **Tweak-style branching** (with 4-6 below, 13.2 s -> 605 ms): one bull per
   node, full re-MRV between the picks of a pair; tried candidates stay
   tweaked off so each K-subset is enumerated once. *Deleted* ~100 lines and
   removed a soundness trap: under saturation branching, mid-pick `size < need`
   wrongly killed live branches because cover(p) hid supply that pending picks
   could still deliver. The k=1 crosscheck caught it immediately.
4. **`solve_other_within(budget, known)`**: repair re-solves hunt for ONE
   solution differing from the known survivor instead of re-finding two.
5. **Flow prefilter** (`matchable` in gendoubleai.rs): proving a fresh roll
   has zero solutions costs a full search of the whole tree, and thousands of
   rolls per finished board are dead. The replacement for plain matching is a
   max-flow feasibility check: source -> each region (cap K) -> each row
   (edge cap) -> sink (cap K), solvable only if flow reaches K*n; run again
   for columns. The edge cap is what adds power over bipartite matching: a
   region's supply into a row is the most *non-adjacent* cells it has there
   (bulls in one row can't sit in touching columns), so a region meeting a
   row in 2 adjacent cells supplies at most 1. Failing the check is a proof
   of unsolvability in microseconds. At n=14 this raised the pre-solve catch
   rate to ~92% of rerolls and cut full zero-solution proofs ~5x (2231 ->
   417 per 10 boards).
6. **Budget charged per placement**: `steps` used to tick once per recursion
   node, but the expensive work (covering columns, hiding neighbors) happens
   per *committed bull*, and rejected candidates cost nothing -- so a
   pathological board could burn seconds while barely touching its 200k
   budget (n=14 saw 1107 over-budget solves AND multi-second tails at once).
   Decrementing on each committed placement makes the counter roughly
   proportional to wall time, so SOLVE_BUDGET actually censors the boards it
   is meant to censor -- which is also what made the budget sweep meaningful
   (optimum 25k, step 7).
7. **Sweeps** (fixed seeds, medians, timeout-censored -- `sweep` harness in the
   test module): `num_capped(n) = (n-5, n-3)` (optima measured at n=12 and
   n=14; ~2/3 of regions, more than the weaker Python solver liked) and
   `SOLVE_BUDGET = 25_000` (unimodal; old 200k was 2x worse). 605 -> ~200 ms.
8. **Rewrite onto the shared `Matrix`/`LList`/`Cell`**: deleted the duplicated
   flat-array DLX (~200 lines) in favor of the structures `single_solver.rs`
   uses -- construction via `Matrix::add_row`, node unlink/relink via
   `LList::remove/restore`, sizes/column-of/row-of from the Matrix fields.
   Two deliberate departures: the trail bypasses `Matrix::cover/uncover`
   (two unlink mechanisms race for the same nodes, see Learnings), and
   sibling lookup uses Matrix's sequential node allocation
   (`Cell(3n+1 + 3o + t)`, asserted in the constructor) instead of x-links.
   Measured cost of the rewrite: ~10-15% (n=14 avg ~200 -> 226 ms), largely
   because `reset()` (relink-in-place between the repair loop's re-solves)
   was lost -- Matrix has no clear/rebuild-in-place, so every solve pays a
   full construction. Accepted for the shared substrate, but it is NOT a
   settled loss: **TODO -- add a relink-in-place helper to Matrix and restore
   `reset()`** (first item in the TODO list below; benefits single's repair
   loop equally, and the TODO on `DoubleSolver::new` marks the spot).

Cross-solver race (`bench_double_solver_k1_*` in `genpenai.rs`): DoubleSolver
at k=1 is 1.6x faster than SingleSolver at n=15 but 1.3x slower at n=20
(measured on the pre-rewrite flat-array solver; re-run after Matrix changes) --
adjacency-in-links bookkeeping outweighs its pruning at k=1 on big boards,
since need=1 covers columns immediately anyway. So single keeps its solver.

## Learnings (transferable)
- A stronger solver shifts generator constants: cap optimum moved from ~n/2
  (Python) to ~2n/3 (Rust). Retune magic numbers after solver changes.
- Compare fixed-seed MEDIANS with timeout censoring; small-sample means lie
  (heavy tails), and wall-clock medians drift ~2x between runs under load --
  only compare settings within one run.
- Per-node speed means nothing if the search tree is bigger. The first Rust
  port ran each node ~50-100x faster than CPython yet finished n=12 in the
  same ~1 s wall time -- its tree was that much larger, because DLX column
  sizes couldn't see adjacency and MRV kept branching on the wrong items.
  Diagnosis: if a Rust port only ties the Python prototype, don't profile
  the code, count the nodes -- the search is missing a prune.
- The uniqueness check counts solutions capped at 2, so any solution the
  search can reach by two different paths is silently double-counted and a
  unique puzzle gets rejected as ambiguous. Two paths arise from (a) an
  item's K bulls being chosen in different orders -- prevented by picking an
  item's candidates in fixed list order and leaving tried candidates
  detached ("tweaked off") so deeper picks only see later ones, giving
  {a,b} only as a-then-b; and (b) the branch item itself varying between
  visits to the same state -- prevented by making the MRV choice a pure
  function of the search state (no randomness, fixed tie-break).
- Run the K-generic solver with k=1 and diff it against the known-good
  Algorithm X solver on a few hundred random boards
  (`k1_matches_brute_force`). k=1 exercises the same cover/undo/adjacency
  machinery but has an independent oracle, so bugs surface as count
  mismatches in seconds. The one real bug of this effort (a mid-pick prune
  that killed live branches) failed this test instantly while every k=2
  test passed -- at k=2 the same bug only showed as generation hanging.
- `Matrix::cover`/`uncover` are safe because they are the *only* thing that
  unlinks nodes, so "uncover undoes exactly what cover did" holds. Adding
  adjacency hiding broke that assumption: a node could already be unlinked
  by the other mechanism, and a second unlink corrupts the list (and a
  paired uncover then restores the wrong state). Fix in `gendoubleai.rs`:
  every unlink checks an `attached` flag first and pushes the node onto one
  shared trail; backtracking pops the trail to a mark. Since the search is
  LIFO, pop order is exactly reverse unlink order, which is what
  dancing-links relinking requires -- no per-mechanism undo logic at all.

## TODO
- [ ] `reset()` / relink-in-place on Matrix (worth ~10-15% for double; the
      same construction-per-solve cost exists in single's repair loop).
- [ ] Port `solve_other` to single's repair loop (find-one-differing; pure
      win, no per-node cost).
- [ ] Sweep `CAP_SIZE` (still Python-era (3,9), never re-swept in Rust).
- [ ] Stats/Aggregator story for double (single has one; double has none).
- [ ] Wasm export `generate_double` in lib.rs + TS integration.
- [ ] K=3 ("threepen"): solver is K-generic; needs MIN_REGION=5,
      spread/flow prefilters already parameterized -- benchmark it.
- [ ] MAX_REPAIRS=50 is uninspected; add repair-stuck stats before tuning.
- [ ] Dedupe the generator helpers double copies from single (Rng,
      orth_neighbors, shuffle, stays_contiguous, kill_solution) -- blocked
      only on genpenai's being private; make them pub(crate).
- [ ] `generate_limited` (test-only sweep harness) mirrors generate_capped's
      loop by hand; keep them in sync or the sweeps tune the wrong algorithm.
