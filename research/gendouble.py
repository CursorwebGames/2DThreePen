"""Two-bull bullpen (Star Battle 2*) prototype: K bulls per row, column,
and region. Target size n=12, fallback n=10. K is a parameter so
threepen (K=3) falls out later.

What changes from genpen-fable.py (the K=1 generator), and why:

1. The solver is no longer exact cover. Each row/column/region needs
   exactly K bulls, and dancing-links exact cover has no clean encoding
   for "exactly 2" (duplicating region columns counts every solution
   2^n times, which silently breaks uniqueness checking). Replacement:
   backtracking that branches on the most constrained region -- the
   MRV idea that made the k=1 DLX fast -- assigning a region all K of
   its bulls per node, with live-cell counts (adjacency-aware) pruning
   regions, rows, and columns that can no longer reach K. Details on
   solve_up_to_two.

2. The repair trick survives K=2 verbatim. Moving one KILL-bull cell
   (that is not a KEEP bull) into an adjacent region: the donor region
   held exactly K of KILL's bulls and now holds K-1, so KILL dies; all
   K of KEEP's bulls in the donor stayed and the receiver only gains a
   non-KEEP-bull cell, so KEEP survives. The donor stays nonempty (and
   even spread-feasible) automatically -- KEEP's bulls live there.

3. Two prefilters before paying for a solve:
     - every region must contain K mutually non-adjacent cells. Size
       alone is not enough: an L-tromino has 3 cells but every pair
       touches, so it can never hold 2 bulls;
     - regions<->rows and regions<->cols matching, generalized to
       capacities (each region supplies K, each row absorbs K) by
       duplicating every node K times and reusing plain matching.
       Note a region's two bulls MAY share a row, and the duplicated
       graph correctly allows that.

Region labels are assumed to be 0..n-1 (random_regions guarantees it).
"""

import random
import time
from itertools import combinations

Board = list[list[int]]
Pos = tuple[int, int]

UNASSIGNED = -1
K = 2

# Give up repairing one board after this many edits and reroll.
MAX_REPAIRS = 50

# Per-solve step budget; boards whose uniqueness proof is pathologically
# expensive get discarded rather than paid for (cf. genpenai.rs).
SOLVE_BUDGET = 200_000

# Small regions are strong constraints that cut the solver's branching
# factor and push boards toward fewer solutions (validated for k=1 in
# gen2.py / genpenai.rs). For k=2 the size floor is 3, not 1: a region
# must hold 2 non-touching bulls, and even some 3-cell shapes
# (L-tromino) can't -- those get caught by regions_feasible.
# Measured at n=10: very tight caps backfire -- CAP_SIZE (3,5) was ~4x
# slower than no caps because most rolls became unsolvable. This looser
# range is untested against uncapped (~600ms avg); tune with sweep().
NUM_CAPPED = (5, 7)  # how many regions get a size cap (inclusive)
CAP_SIZE = (3, 9)  # cell count for each capped region (inclusive)

# A region needs K=2 non-touching bulls, which is impossible below 3
# cells, so growth brings every region to this size before free growth
# starts. (3 cells are necessary, not sufficient -- an L-tromino still
# fails -- so regions_feasible stays as the real check.)
MIN_REGION = 3


def _orth_neighbors(r: int, c: int, n: int) -> list[Pos]:
    """Up/down/left/right neighbors of (r, c) inside an n x n board."""
    out = []
    if r > 0:
        out.append((r - 1, c))
    if r + 1 < n:
        out.append((r + 1, c))
    if c > 0:
        out.append((r, c - 1))
    if c + 1 < n:
        out.append((r, c + 1))
    return out


def random_regions(n: int, caps: tuple[int, int] | None = None) -> Board:
    """Grow n contiguous regions from n random seed cells until the board
    is covered (incremental frontier as in genpen-fable.py).

    With `caps`, a random caps[0]..caps[1] of the regions stop growing at
    a random CAP_SIZE size, biasing the board toward the small regions
    that make solves cheap and solutions scarce."""
    grid = [[UNASSIGNED] * n for _ in range(n)]
    sizes = [0] * n
    max_size = [n * n] * n
    if caps is not None:
        for region in random.sample(range(n), min(random.randint(*caps), n)):
            max_size[region] = random.randint(*CAP_SIZE)

    all_cells = [(r, c) for r in range(n) for c in range(n)]
    seeds = random.sample(all_cells, n)

    frontier: list[tuple[Pos, int]] = []

    def claim(r: int, c: int, region: int) -> None:
        grid[r][c] = region
        sizes[region] += 1
        for nr, nc in _orth_neighbors(r, c, n):
            if grid[nr][nc] == UNASSIGNED:
                frontier.append(((nr, nc), region))

    for region, (r, c) in enumerate(seeds):
        claim(r, c, region)

    # Phase 1: round-robin one cell per under-minimum region per round,
    # so every region reaches MIN_REGION cells before free growth -- and
    # no region can wall a slow starter in first. A region with no legal
    # move left is already walled in; give up on it (the board will die
    # in regions_feasible, same as before).
    while True:
        grew = False
        for region in range(n):
            if sizes[region] >= MIN_REGION:
                continue
            moves = [
                (r, c)
                for (r, c), reg in frontier
                if reg == region and grid[r][c] == UNASSIGNED
            ]
            if moves:
                r, c = random.choice(moves)
                claim(r, c, region)
                grew = True
        if not grew:
            break

    while frontier:
        i = random.randrange(len(frontier))
        frontier[i], frontier[-1] = frontier[-1], frontier[i]
        (r, c), region = frontier.pop()
        if grid[r][c] == UNASSIGNED and sizes[region] < max_size[region]:
            claim(r, c, region)

    # Cells walled off by full capped regions get absorbed into any
    # assigned neighbor. This may push a capped region past its cap --
    # fine, the cap is a bias, not an invariant (cf. genpenai.rs).
    if sum(sizes) < n * n:
        stranded = [p for p in all_cells if grid[p[0]][p[1]] == UNASSIGNED]
        while stranded:
            remaining = []
            for r, c in stranded:
                nbrs = [p for p in _orth_neighbors(r, c, n) if grid[p[0]][p[1]] != UNASSIGNED]
                if nbrs:
                    y, x = random.choice(nbrs)
                    grid[r][c] = grid[y][x]
                else:
                    remaining.append((r, c))
            stranded = remaining

    return grid


# ---------------------------------------------------------------- solver


def solve_up_to_two(
    board: Board, k: int = K, budget: int | None = None
) -> list[list[Pos]] | None:
    """At most 2 solutions, each a list of bull positions.

    Branches on the most constrained REGION (fewest live cells), not on
    rows in order -- the MRV idea that made the k=1 DLX solver fast, and
    the reason the generator's small capped regions pay off: a 4-cell
    region has a couple of valid bull-pairs, a row has ~36. Each node
    assigns one region all k of its bulls at once.

    The branching choice is a deterministic function of the search
    state, so every solution is reached by exactly one path and the
    count (capped at 2) stays trustworthy.

    Live cell = not within Chebyshev distance 1 of a placed bull, and
    its row, column, and region can still take a bull. One grid pass
    per node refreshes live counts per region/row/column, which both
    picks the branch region and prunes: any region needing more bulls
    than it has live cells -- or row/column that can no longer reach k
    -- kills the branch. This sees adjacency, unlike the old
    suffix-count prune.

    Returns None if the search used more than `budget` nodes -- the
    caller should discard the board rather than pay for the full proof.
    """
    n = len(board)
    region_cells: list[list[Pos]] = [[] for _ in range(n)]
    for r in range(n):
        for c in range(n):
            region_cells[board[r][c]].append((r, c))

    row_count = [0] * n
    col_count = [0] * n
    done = [False] * n  # region already holds its k bulls
    # blocked[r][c] = how many placed bulls have (r, c) in their 3x3
    # neighborhood (incl. their own cell); live requires 0. Maintained
    # incrementally so the per-node liveness test is O(1) per cell.
    blocked = [[0] * n for _ in range(n)]
    placed: list[Pos] = []
    sols: list[list[Pos]] = []
    steps = budget if budget is not None else (1 << 62)

    def live(r: int, c: int) -> bool:
        return blocked[r][c] == 0 and row_count[r] < k and col_count[c] < k

    def place(combo: tuple[Pos, ...], sign: int) -> None:
        """Add (sign=+1) or remove (sign=-1) one region's k bulls."""
        done[board[combo[0][0]][combo[0][1]]] = sign > 0
        for r, c in combo:
            if sign > 0:
                placed.append((r, c))
            else:
                placed.pop()
            row_count[r] += sign
            col_count[c] += sign
            for y in range(max(0, r - 1), min(n, r + 2)):
                for x in range(max(0, c - 1), min(n, c + 2)):
                    blocked[y][x] += sign

    def assignments(reg: int) -> list[tuple[Pos, ...]]:
        """All ways to put this region's k bulls on its live cells:
        pairwise non-touching, and no row/column pushed past k."""
        cells = [(r, c) for (r, c) in region_cells[reg] if live(r, c)]
        out = []
        for combo in combinations(cells, k):
            if any(
                max(abs(a[0] - b[0]), abs(a[1] - b[1])) <= 1
                for a, b in combinations(combo, 2)
            ):
                continue
            rows = [r for r, _ in combo]
            cols = [c for _, c in combo]
            if any(row_count[r] + rows.count(r) > k for r in rows):
                continue
            if any(col_count[c] + cols.count(c) > k for c in cols):
                continue
            out.append(combo)
        return out

    def rec() -> None:
        nonlocal steps
        if steps == 0:
            return
        steps -= 1

        # One grid pass: live counts per region/row/column. Doubles as
        # the prune and as the MRV branch choice.
        reg_live = [0] * n
        row_live = [0] * n
        col_live = [0] * n
        branch = -1
        for reg in range(n):
            if done[reg]:
                continue
            for r, c in region_cells[reg]:
                if live(r, c):
                    reg_live[reg] += 1
                    row_live[r] += 1
                    col_live[c] += 1
            if reg_live[reg] < k:
                return  # region can no longer hold its bulls
            if branch == -1 or reg_live[reg] < reg_live[branch]:
                branch = reg

        if branch == -1:  # every region filled => rows/cols exact too
            sols.append(placed.copy())
            return

        for i in range(n):
            if row_count[i] + row_live[i] < k or col_count[i] + col_live[i] < k:
                return

        for combo in assignments(branch):
            place(combo, +1)
            rec()
            place(combo, -1)
            if len(sols) >= 2 or steps == 0:
                return

    rec()
    return None if steps == 0 else sols


# ------------------------------------------------------------ prefilters


def regions_feasible(board: Board, k: int = K) -> bool:
    """Every region must be able to host k mutually non-adjacent bulls
    (pairwise Chebyshev distance >= 2)."""
    n = len(board)
    cells: dict[int, list[Pos]] = {reg: [] for reg in range(n)}
    for r in range(n):
        for c in range(n):
            cells[board[r][c]].append((r, c))
    return all(_spread_exists(cs, k, ()) for cs in cells.values())


def _spread_exists(cells: list[Pos], k: int, chosen: tuple[Pos, ...]) -> bool:
    if k == 0:
        return True
    for i, (r, c) in enumerate(cells):
        if all(max(abs(r - y), abs(c - x)) >= 2 for (y, x) in chosen):
            if _spread_exists(cells[i + 1 :], k - 1, chosen + ((r, c),)):
                return True
    return False


def matchable(board: Board, k: int = K) -> bool:
    """Cheap necessary condition for solvability: regions must be
    matchable with the rows they touch (each region supplying k bulls,
    each row absorbing k), and likewise with columns. A board failing
    either check is provably unsolvable, no search needed."""
    n = len(board)
    rows = [[False] * n for _ in range(n)]
    cols = [[False] * n for _ in range(n)]
    for r in range(n):
        for c in range(n):
            rows[board[r][c]][r] = True
            cols[board[r][c]][c] = True
    return _capacity_matching(rows, k) and _capacity_matching(cols, k)


def _capacity_matching(touch: list[list[bool]], k: int) -> bool:
    """Perfect matching after duplicating every node k times -- the
    standard node-splitting reduction of capacity-k b-matching."""
    nk = len(touch) * k
    matched = [-1] * nk

    def augment(a: int, seen: set[int]) -> bool:
        for b in range(nk):
            if touch[a // k][b // k] and b not in seen:
                seen.add(b)
                if matched[b] == -1 or augment(matched[b], seen):
                    matched[b] = a
                    return True
        return False

    return all(augment(a, set()) for a in range(nk))


# ---------------------------------------------------------------- repair


def stays_contiguous(board: Board, r: int, c: int) -> bool:
    """Would the region of (r, c) stay connected if (r, c) left it?
    (Anchor trick, identical to genpen-fable.py.)"""
    n = len(board)
    region = board[r][c]
    anchors = [p for p in _orth_neighbors(r, c, n) if board[p[0]][p[1]] == region]

    if not anchors:
        return False  # (r, c) is the whole region; removal would empty it
    if len(anchors) == 1:
        return True  # only one possible fragment, nothing to split

    seen = {anchors[0]}
    stack = [anchors[0]]
    while stack:
        y, x = stack.pop()
        for p in _orth_neighbors(y, x, n):
            if board[p[0]][p[1]] == region and p != (r, c) and p not in seen:
                seen.add(p)
                stack.append(p)

    return all(a in seen for a in anchors)


def kill_solution(board: Board, keep: list[Pos], kill: list[Pos]) -> bool:
    """Edit `board` so solution `kill` dies while `keep` survives.
    Returns False if no legal edit exists. See module docstring for why
    this works unchanged for any K."""
    n = len(board)
    keep_set = set(keep)

    targets = [p for p in kill if p not in keep_set]
    random.shuffle(targets)

    for r, c in targets:
        if not stays_contiguous(board, r, c):
            continue  # removing this cell would split its region
        donor = board[r][c]
        neighbors = _orth_neighbors(r, c, n)
        random.shuffle(neighbors)
        for nr, nc in neighbors:
            if board[nr][nc] != donor:
                board[r][c] = board[nr][nc]
                return True

    return False


# -------------------------------------------------------------- generator


def generate(
    n: int,
    k: int = K,
    caps: tuple[int, int] = NUM_CAPPED,
    time_limit: float | None = None,
) -> Board | None:
    """Generate an n x n board with exactly one k-bull solution.
    Pass caps=(0, 0) to disable region size caps entirely.

    With `time_limit` (seconds), gives up and returns None once the
    limit passes -- generation time has a heavy tail, and a benchmark
    would rather record a timeout than wait out one unlucky board."""
    deadline = time.monotonic() + time_limit if time_limit else None
    while True:  # reroll from scratch when a board is hopeless
        if deadline is not None and time.monotonic() > deadline:
            return None
        board = random_regions(n, caps)

        if not regions_feasible(board, k) or not matchable(board, k):
            continue

        for _ in range(MAX_REPAIRS):
            sols = solve_up_to_two(board, k, SOLVE_BUDGET)
            if sols is None or len(sols) == 0:
                break  # too expensive, or dead board: reroll
            if len(sols) == 1:
                return board
            # 2 solutions: edit to kill one and keep the other; if no
            # bull of sols[1] is movable, try the symmetric direction
            if not (
                kill_solution(board, sols[0], sols[1])
                or kill_solution(board, sols[1], sols[0])
            ):
                break  # no legal edit in either direction, reroll


# ------------------------------------------------------------ validation


def assert_valid_solution(board: Board, sol: list[Pos], k: int = K) -> None:
    n = len(board)
    assert len(sol) == k * n
    rows = [0] * n
    cols = [0] * n
    regs = [0] * n
    for r, c in sol:
        rows[r] += 1
        cols[c] += 1
        regs[board[r][c]] += 1
    assert all(v == k for v in rows + cols + regs), "count constraint broken"
    bulls = sorted(sol)
    for i, (r, c) in enumerate(bulls):
        for y, x in bulls[i + 1 :]:
            assert max(abs(r - y), abs(c - x)) >= 2, f"bulls touch: {(r, c)} {(y, x)}"


def crosscheck_k1(runs: int = 200, n: int = 5) -> None:
    """The new solver at k=1 must agree with the known-good AlgoXSolver
    on random label grids (solution count capped at 2, and the actual
    bull set when unique)."""
    from algox import AlgoXSolver

    tested = 0
    while tested < runs:
        board = [[random.randrange(n) for _ in range(n)] for _ in range(n)]
        if len({v for row in board for v in row}) < n:
            continue  # AlgoXSolver requires every region present
        tested += 1

        mine = solve_up_to_two(board, k=1)
        ref = AlgoXSolver(board)
        ref.solve()
        ref_sols = [
            sorted(ref._candidates[row]["pos"] for row in sol)
            for sol in ref._solutions
        ]
        assert len(mine) == len(ref_sols), f"count mismatch on {board}"
        if len(mine) == 1:
            assert sorted(mine[0]) == ref_sols[0], f"solution mismatch on {board}"
    print(f"crosscheck vs AlgoXSolver: {runs} random k=1 boards agree")


def breakdown(n: int = 10, target: int = 10, caps: tuple[int, int] = (0, 0)) -> None:
    """Where does generation time go? (cf. breakdown_* in genpenai.rs)"""
    rolls = filtered = zero = over_budget = stuck = repairs = solves = 0
    solve_time = 0.0
    t_start = time.perf_counter()

    done = 0
    while done < target:
        rolls += 1
        board = random_regions(n, caps)
        if not regions_feasible(board) or not matchable(board):
            filtered += 1
            continue
        for _ in range(MAX_REPAIRS):
            t0 = time.perf_counter()
            sols = solve_up_to_two(board, K, SOLVE_BUDGET)
            solve_time += time.perf_counter() - t0
            solves += 1
            if sols is None:
                over_budget += 1
                break
            if len(sols) == 0:
                zero += 1
                break
            if len(sols) == 1:
                done += 1
                break
            repairs += 1
            if not (kill_solution(board, sols[0], sols[1])
                    or kill_solution(board, sols[1], sols[0])):
                stuck += 1
                break
        else:
            stuck += 1

    total = time.perf_counter() - t_start
    print(f"{target} boards in {total:.1f}s: rolls={rolls} "
          f"(filtered={filtered}, zero-sol={zero}, over-budget={over_budget}, "
          f"repair-stuck={stuck}), repairs={repairs}, solver calls={solves}, "
          f"solver time={solve_time:.1f}s ({100 * solve_time / total:.0f}%)")


def sweep(
    n: int = 10,
    runs: int = 10,
    options: tuple[tuple[int, int], ...] = ((0, 0), (2, 4), (4, 6), (5, 7), (6, 8), (8, 10)),
    time_limit: float = 2.0,
) -> None:
    """Find the cap-count sweet spot for a size (cf. sweep_caps_* in
    genpenai.rs). (0, 0) is the uncapped baseline. Every setting replays
    the same seeds, and medians matter more than means -- generation
    time has a heavy tail, so one unlucky board dominates a mean.
    Boards slower than `time_limit` are cut off and counted instead of
    waited out (their time still enters the stats as the limit)."""
    import statistics

    for caps in options:
        random.seed(42)
        times = []
        timeouts = 0
        for _ in range(runs):
            t0 = time.perf_counter()
            if generate(n, caps=caps, time_limit=time_limit) is None:
                timeouts += 1
            times.append((time.perf_counter() - t0) * 1000)
        print(f"n={n} caps={caps} size={CAP_SIZE}: "
              f"median={statistics.median(times):.0f} ms, "
              f"mean={statistics.mean(times):.0f} ms, "
              f"timeouts={timeouts}/{runs}", flush=True)


_COLORS = [31, 32, 33, 34, 35, 36, 91, 92, 93, 94, 95, 96, 97, 90]


def _colored(text: str, color: int) -> str:
    return f"\033[{color}m{text}\033[0m"


def show(board: Board, sol: list[Pos] | None = None) -> None:
    """Print each region number in its own color (cf. show_board in
    maker.py); pass a solution and its bulls print as ★ instead of
    their number."""
    bulls = set(sol or [])
    n = len(board)
    width = len(str(n - 1))
    print()
    for y in range(n):
        for x in range(n):
            region = board[y][x]
            mark = "★" if (y, x) in bulls else str(region)
            print(_colored(mark.rjust(width), _COLORS[region % len(_COLORS)]), end=" ")
        print()


if __name__ == "__main__":
    import os
    import sys
    import time

    # Windows console defaults to cp1252, which can't print "★"
    sys.stdout.reconfigure(encoding="utf-8")
    os.system("")  # nudge the legacy Windows console into ANSI mode

    for size in (10,):
        runs = 1
        total = 0.0
        last = None
        for i in range(1, runs + 1):
            t0 = time.perf_counter()
            last = generate(size)
            elapsed = (time.perf_counter() - t0) * 1000
            total += elapsed
            print(f"n={size} board {i}: {elapsed:.0f} ms")

        sols = solve_up_to_two(last)
        assert len(sols) == 1, "generated board is not unique?!"
        assert_valid_solution(last, sols[0])
        print(f"\nn={size} average: {total / runs:.0f} ms over {runs} boards")
        print("\npuzzle:")
        show(last)
        print("\nsolution:")
        show(last, sols[0])
        print()
