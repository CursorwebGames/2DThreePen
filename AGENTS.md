# AGENTS.md

## Project Priorities
Order of importance:
1. Puzzle generation throughput
2. Puzzle uniqueness correctness
3. Solver speed (oftentimes speeds puzzle generation throughput)
4. Code maintainability

This is a performance-sensitive project. Avoid introducing allocations, cloning, or abstraction layers in hot paths unless benchmarks demonstrate no meaningful regression.

Code should be maintainable and readable enough that someone is able to learn from it and implement it from scratch without the help of AI.

## Algorithm
A *bullpen* (aka Star Battle, aka Queens) puzzle is an `n x n` grid partitioned into
`n` contiguous regions. A solution places exactly **K** "bulls" in every row,
every column, and every region, with no two bulls touching (Chebyshev distance
\>= 2, i.e. not even diagonally adjacent). A puzzle must also contain exactly **one**
solution.

- **Single** (K=1): one bull per row/col/region. Canonical implementation is
  [wasm/src/genpenai.rs](wasm/src/genpenai.rs) (Rust, shipped) with the solver
  in [wasm/src/solver.rs](wasm/src/solver.rs). A Python port lives in
  [research/gensingle.py](research/gensingle.py).
- **Double** (K=2): two bulls per row/col/region.
  [research/gendouble.py](research/gendouble.py) — a research prototype, K is a
  parameter so K=3 ("threepen") falls out later.

The two share one generation strategy. They differ in **the solver** and in a
**handful of K-driven feasibility checks/growth tweaks**. Everything below is
organized around that split.


## 1. The shared strategy: random regions + targeted repair
Both generators run the same loop (`generate` in each file):

```
loop forever:                       # reroll from scratch when a board is hopeless
    grid = random_regions(n)        # grow n contiguous regions from n seeds
    if not cheap_prefilters(grid):  # provably unsolvable? skip without solving
        continue
    repeat up to MAX_REPAIRS times:
        sols = solve(grid, cap=2)   # find up to 2 solutions (or give up on recursive depth)
        if sols is None:  break     # proof too expensive -> discard board
        if len(sols) == 0: break    # dead board -> reroll
        if len(sols) == 1: return grid          # done!
        # exactly 2 solutions: edit one cell to kill one, keep the other
        if not kill_solution(...):  break        # stuck -> reroll
```

**Every repair edit is guaranteed to preserve one known solution while destroying another**, so
the solution count can only walk *down* toward 1 — it can never accidentally hit
0 mid-repair. See 2. Keep + Kill.

### Region growth (`random_regions`)
Drop `n` seed cells, one per region, then flood outward. The frontier of legal
growth moves is kept **incrementally**: claiming a cell pushes a move for each
unassigned neighbor, and stale moves (cell already claimed) are skipped when
drawn. Each cell pushes <=4 moves ever, so filling the board is `O(n^2)` total
instead of rescanning the whole board per claim (`O(n^4)`).

**Size caps.** A random subset of regions is capped to a small size before
growth. Tiny regions are strong constraints: they shrink the solver's branching
factor and push boards toward fewer solutions. Too many caps, though, make
almost every roll unsolvable so all the time goes to growing garbage — there's a
measured optimum (`num_capped` in Rust; `NUM_CAPPED`/`CAP_SIZE` in Python).
After growth, cells walled off by full capped regions are absorbed into any
assigned neighbor (the cap is a bias, not a hard invariant).

### Cheap prefilter: matching (`matchable`)
Before paying for a real solve, reject provably-unsolvable boards in
microseconds. In any solution the bulls occupy distinct rows and each region
holds its bulls, so **regions must be matchable against the rows they touch**
(and likewise columns). This is bipartite matching via augmenting paths. A board
failing it is unsolvable, no search needed. (Passing proves nothing — adjacency
can still rule everything out; that's what the real solver is for.) In Rust this
catches ~96% of zero-solution rolls (`matching_filter_catch_rate`).

## 2. Keep + kill (the repair step)
This is the heart of the generator and is **identical in both** (`kill_solution` + `stays_contiguous`).

When a board has exactly two solutions, label them **KEEP** (survives) and
**KILL** (to be destroyed). Pick a bull cell of KILL that is *not* a bull of
KEEP, and **move that single cell into an orthogonally adjacent region**. Then:

- **KILL dies.** Its bull at `(r, c)` was the only bull KILL had in the donor
  region (a solution has exactly K bulls per region, and this is the one in
  KILL's set that isn't shared). The donor region no longer contains `(r, c)`,
  so KILL is now one bull short there — invalid.
- **KEEP survives.** KEEP's bull(s) in the donor region are *other* cells that
  stayed put, and the receiving region only gains a cell that isn't one of
  KEEP's bulls. So KEEP still satisfies every region.

So each edit removes >=1 solution and can never remove KEEP. Two legality
guards keep the board well-formed:

- **`stays_contiguous`**: the donor region must stay connected after losing
  `(r, c)`. Anchor trick — if removal would split the region, the fragments must
  each contain one of `(r, c)`'s own same-region neighbors (they were only
  connected *through* `(r, c)`); flood-fill from one anchor skipping `(r, c)`
  and check the others are reached. (The donor stays *nonempty* automatically —
  KEEP's bull lives there.)
- The cell must join an **orthogonally adjacent** region, so the receiver stays
  contiguous too.

If none of KILL's differing bulls is movable, try the symmetric edit (kill KEEP,
keep KILL) before giving up and rerolling.

> Why this matters: the naive alternative (the A* in `research/maker.py`) pays
> one solver call *per candidate edit*. Keep/kill makes a guaranteed-progress
> edit with **no** solver call, then re-solves once. The solver dominates wall
> time, so this is the whole ballgame.


## 3. The single-board solver — exact cover / Algorithm X (DLX)

[wasm/src/solver.rs](wasm/src/solver.rs), `BullpenSolver`.

K=1 is a clean **exact cover** problem, so it uses Knuth's Algorithm X with
dancing links (the `Matrix` of doubly-linked nodes in
[wasm/src/matrix.rs](wasm/src/matrix.rs)).

**Encoding.** `3n` columns = one per row constraint, one per column constraint,
one per region constraint. Each cell `(r, c)` of region `g` becomes one matrix
row covering exactly three columns: `r`, `n + c`, `2n + g`. A set of matrix rows
that covers all `3n` columns exactly once = one bull per row, per column, per
region = a solution.

**Search (`count_sol_rec` / `solve_rec`).** Standard DLX:

1. **Pick the column with the fewest remaining candidates** (MRV) — minimizes
   branching.
2. `cover` it, then try each candidate row that satisfies it.
3. Adjacency isn't an exact-cover column, so it's enforced *imperatively*: a
   `placed[r*n+c]` bitmap plus `conflicts(id)` skips any candidate touching an
   already-placed bull (checked before covering, so there's nothing to undo).
4. Cover the candidate's other columns, recurse, then **uncover** to backtrack
   (dancing links restores the matrix in O(1) per node).

**Capping at 2.** Both `count_sol` and `solve` short-circuit once a second
solution is found — the generator only ever needs to know "0, 1, or 2+".
`solve_within(budget)` aborts after `budget` recursion steps and returns `None`;
proving uniqueness on a pathological board can take seconds, and the generator
would rather discard it than pay. Aborts unwind cleanly so the solver is
reusable.


## 4. The double-board solver — region-MRV backtracking

[research/gendouble.py](research/gendouble.py), `solve_up_to_two`.

**Why not Algorithm X?** Exact cover means "cover each column *exactly once*."
K=2 needs "cover each constraint *exactly twice*." The naive hack — duplicate
each region column so it must be covered twice — counts every solution `2^n`
times (the two bulls of a region are interchangeable), which silently breaks the
uniqueness check. So plain Algorithm X is out.

**Algorithm M — an open question, not a closed door.** The principled fix for
multiplicities is Knuth's **Algorithm M** (exact cover *with multiplicities*),
which handles "cover each constraint exactly K times" directly. It's a chunk of
dancing-links machinery we haven't built yet, so whether it actually beats the
current solver for K=2 (and K=3) is **untested** — it's a promising future
direction to benchmark, not something we've ruled out. For now, double uses
plain **backtracking with forward checking + MRV**, which handles any K with no
duplication tricks and is cross-checked against the K=1 DLX solver
(`crosscheck_k1`).

The search branches on **one region at a time**, assigning all K of its bulls at
once. Tree depth is `n` (one level per region), not `K*n`.

### State kept incrementally (updated in `place(combo, sign)`)

- `row_count[r]`, `col_count[c]` — bulls placed so far in each row/column.
- `done[reg]` — region already has its K bulls.
- `blocked[r][c]` — **a count** of how many placed bulls have `(r, c)` in their
  3x3 neighborhood. Placing a bull does `+1` to each of its ~9 neighbors;
  undoing does `-1`. A cell is adjacency-free iff `blocked[r][c] == 0`. It must
  be a *count*, not a boolean: if two bulls both cover a cell, removing one
  should leave it blocked by the other — `2 -> 1`, still blocked.

So `live(r, c)` = `blocked[r][c] == 0 and row_count[r] < K and col_count[c] < K`
is O(1), no scanning placed bulls at query time.

### One node (`rec`)

1. **One grid pass that does triple duty** — loop over every unfinished region,
   counting live cells per region (and per row/col). In that single pass:
   - **prune**: if any region has fewer than K live cells, this branch is dead
     -> return immediately;
   - **choose the branch (MRV)**: track the region with the fewest live cells.
   - (After the loop, a second cheap check prunes if any row/col can no longer
     reach K bulls — forward checking.)

   This is the "combined prune + MRV" pass: branch choice and dead-branch
   detection share the same loop instead of two passes.
2. **Base case**: if every region is `done`, rows/cols are automatically exact
   too — record the solution.
3. **Enumerate placements** for the chosen region (`assignments`): all size-K
   subsets of its live cells that are pairwise non-touching and don't push any
   row/col past K. (Recomputed fresh each node from current live cells — nothing
   cached across nodes.)
4. For each placement: `place(+1)`, recurse, `place(-1)`. Stop once 2 solutions
   are found or the step budget runs out.

**Why MRV pays off here:** a 4-cell region has ~1-2 valid bull-pairs; a row-sized
region has dozens. Branching on the tightest region first means most bad lines
die within a couple of levels. The generator's small capped regions exist
precisely to create these tight branch points. Like the single solver, it caps
at 2 solutions and honors a step budget (returns `None` if exceeded).


## 5. Single vs double — exactly what differs

The generate loop, region growth, caps, the `matchable` prefilter idea, and the
**entire keep/kill repair** are shared. Double differs only where K=2 forces it:

| | Single (K=1) | Double (K=2) |
|---|---|---|
| **Solver** | Exact cover / Algorithm X with dancing links (`solver.rs`) | Region-MRV backtracking (`solve_up_to_two`) — DLX's "exactly-once" can't express "exactly K" cleanly |
| **Min region size** | 1 (a region just needs 1 cell) | 3, enforced by a round-robin growth phase — 2 non-touching bulls need >=3 cells |
| **Region feasibility check** | none needed | `regions_feasible`: every region must actually fit K pairwise non-adjacent bulls (size 3 isn't enough — an L-tromino has 3 cells but all touch) |
| **Matching prefilter** | plain 1-to-1 region<->row / region<->col matching | capacity-K matching (each region supplies K, each row/col absorbs K) via node-splitting (duplicate each node K times, reuse plain matching) |
| **Cap floor** | `CAP_SIZE` can go down to 1 | `CAP_SIZE` floor is 3 (same min-region reason) |

Everything else — incremental frontier growth, stranded-cell absorption, the
step budget, the reroll-on-hopeless logic, and keep/kill — is the same algorithm
in both.
