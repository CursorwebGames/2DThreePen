"""Bullpen generator: random regions + targeted repair.

Python port of wasm/src/genpenai.rs, with the follow-up lessons applied:

1. Region growth keeps an *incremental* frontier of legal growth moves
   instead of rescanning the whole board after every assignment
   (O(n^2) total pushes instead of O(n^4) rescans).
2. The contiguity check BFSes from the removed cell's own same-region
   neighbors ("anchors") instead of collecting the entire region first.
3. The solver call is capped at 2 solutions (AlgoXSolver already does
   this) — the repair step only ever looks at two of them.

The repair idea, in one paragraph: when a board has two solutions KEEP
and KILL, pick a bull cell of KILL that is not a bull of KEEP and move
that single cell into an adjacent region. KILL dies (that cell was its
only bull in the donor region, which no longer contains it) and KEEP
survives (its bull in the donor region is some other cell, and the
receiving region gains no second KEEP-bull). So every edit removes at
least one solution and can never remove them all: the solution count
walks down toward 1 instead of being searched for blindly (cf. the A*
in maker.py, which pays one solver call per *candidate* edit).
"""

import random

from algox import AlgoXSolver

Board = list[list[int]]
Pos = tuple[int, int]

UNASSIGNED = -1

# Give up repairing one board after this many edits and reroll.
# In practice repair converges in a handful of steps.
MAX_REPAIRS = 50


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


def random_regions(n: int) -> Board:
    """Grow n contiguous regions from n random seed cells until the board
    is covered. Every region is contiguous and nonempty by construction.

    The frontier is a list of (cell, region) growth moves maintained
    incrementally: assigning a cell pushes moves for its unassigned
    neighbors; moves whose cell got claimed in the meantime are stale
    and simply skipped when drawn. Each cell pushes at most 4 moves
    ever, so total work is O(n^2) instead of rescanning the board
    (O(n^2)) once per assigned cell (O(n^4) total).
    """
    grid = [[UNASSIGNED] * n for _ in range(n)]

    all_cells = [(r, c) for r in range(n) for c in range(n)]
    seeds = random.sample(all_cells, n)

    frontier: list[tuple[Pos, int]] = []

    def claim(r: int, c: int, region: int) -> None:
        grid[r][c] = region
        for nr, nc in _orth_neighbors(r, c, n):
            if grid[nr][nc] == UNASSIGNED:
                frontier.append(((nr, nc), region))

    for region, (r, c) in enumerate(seeds):
        claim(r, c, region)

    while frontier:
        # swap-pop a random move: O(1) removal, order doesn't matter
        i = random.randrange(len(frontier))
        frontier[i], frontier[-1] = frontier[-1], frontier[i]
        (r, c), region = frontier.pop()
        if grid[r][c] == UNASSIGNED:  # skip stale moves
            claim(r, c, region)

    return grid


def solve_up_to_two(board: Board) -> list[list[Pos]]:
    """At most 2 solutions, each as a list of bull positions.

    AlgoXSolver._solve already short-circuits after the second solution,
    which is all the repair step ever needs.
    """
    solver = AlgoXSolver(board)
    solver.solve()
    return [
        [solver._candidates[row]["pos"] for row in sol] for sol in solver._solutions
    ]


def stays_contiguous(board: Board, r: int, c: int) -> bool:
    """Would the region of (r, c) stay connected if (r, c) left it?

    Anchor trick: if removing (r, c) splits its region, every fragment
    must contain one of (r, c)'s own same-region orthogonal neighbors —
    the fragments were only ever connected *through* (r, c). So instead
    of collecting the whole region and counting it, BFS from one anchor
    (skipping (r, c)) and check the other anchors are reached.
    """
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
    moving one differing bull cell to an adjacent region does exactly this.
    """
    n = len(board)
    keep_set = set(keep)

    # bulls of `kill` that aren't bulls of `keep` — each is a valid target;
    # random order so repeated repairs don't gnaw at the same corner
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


def generate(n: int) -> Board:
    """Generate an n x n bullpen with exactly one solution."""
    while True:  # reroll from scratch when a board is hopeless
        board = random_regions(n)

        for _ in range(MAX_REPAIRS):
            sols = solve_up_to_two(board)
            if len(sols) == 0:
                break  # dead board (only possible on a fresh roll)
            if len(sols) == 1:
                return board
            # 2 solutions: edit the board to kill sols[1] but keep sols[0]
            if not kill_solution(board, sols[0], sols[1]):
                break  # no legal edit found, reroll


_COLORS = [31, 32, 33, 34, 35, 36, 91, 92, 93, 94, 95, 96, 97, 90]


def show(board: Board, sol: list[Pos] | None = None) -> None:
    bulls = set(sol or [])
    n = len(board)
    width = len(str(n - 1))
    print()
    for y in range(n):
        for x in range(n):
            region = board[y][x]
            mark = "★" if (y, x) in bulls else str(region)
            print(f"\033[{_COLORS[region % len(_COLORS)]}m{mark.rjust(width)}\033[0m", end=" ")
        print()


if __name__ == "__main__":
    import os
    import sys
    import time

    sys.stdout.reconfigure(encoding="utf-8")
    os.system("")  # enable ANSI on legacy Windows console

    SIZE = 8
    RUNS = 20

    total = 0.0
    last = None
    for i in range(1, RUNS + 1):
        t0 = time.perf_counter()
        last = generate(SIZE)
        elapsed = (time.perf_counter() - t0) * 1000
        total += elapsed
        print(f"board {i:2}: {elapsed:.1f} ms")

    print(f"\naverage: {total / RUNS:.1f} ms over {RUNS} boards of size {SIZE}")

    sols = solve_up_to_two(last)
    print("\npuzzle:")
    show(last)
    print("\nsolution:")
    show(last, sols[0])
    print(f"unique: {len(sols) == 1}")
