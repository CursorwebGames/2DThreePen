import heapq
import random
from collections import deque
from typing import Optional

from algox import AlgoXSolver

SIZE = 6

Board = list[list[int]]


# ---------------------------------------------------------------------------
# Phase 1 – star placement: one per row, one per col, no 8-adjacency
# ---------------------------------------------------------------------------


def _place_stars(size: int) -> list[tuple[int, int]]:
    cols: list[int] = []
    used: set[int] = set()

    def bt(row: int) -> bool:
        if row == size:
            return True
        order = list(range(size))
        random.shuffle(order)
        for col in order:
            if col in used:
                continue
            if cols and abs(cols[-1] - col) <= 1:
                continue
            used.add(col)
            cols.append(col)
            if bt(row + 1):
                return True
            cols.pop()
            used.remove(col)
        return False

    if not bt(0):
        raise RuntimeError("star placement failed")
    return [(r, cols[r]) for r in range(size)]


# ---------------------------------------------------------------------------
# Phase 2 – flood-fill regions from star seeds (interleaved random BFS)
# ---------------------------------------------------------------------------


def _flood_fill(stars: list[tuple[int, int]], size: int) -> Board:
    board = [[-1] * size for _ in range(size)]
    queues: list[deque] = []

    for region, (y, x) in enumerate(stars):
        board[y][x] = region
        q: deque = deque()
        dirs = [(y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)]
        random.shuffle(dirs)
        for ny, nx in dirs:
            if 0 <= ny < size and 0 <= nx < size:
                q.append((ny, nx))
        queues.append(q)

    changed = True
    while changed:
        changed = False
        for region, q in enumerate(queues):
            while q:
                ny, nx = q.popleft()
                if board[ny][nx] != -1:
                    continue
                board[ny][nx] = region
                changed = True
                dirs = [(ny - 1, nx), (ny + 1, nx), (ny, nx - 1), (ny, nx + 1)]
                random.shuffle(dirs)
                for nny, nnx in dirs:
                    if 0 <= nny < size and 0 <= nnx < size and board[nny][nnx] == -1:
                        q.append((nny, nnx))
                break  # one cell per region per round

    return board


# ---------------------------------------------------------------------------
# Solution counting with configurable upper limit (for A* heuristic)
# ---------------------------------------------------------------------------


def count_solutions(board: Board, limit: int = 10) -> int:
    """Count solutions up to `limit` using AlgoXSolver internals."""
    solver = AlgoXSolver(board)
    found: list = []

    def _solve(active_rows: set, active_cols: dict, chosen: list) -> None:
        if not active_cols:
            found.append(None)
            return
        col = min(active_cols, key=lambda c: len(active_cols[c] & active_rows))
        possible = active_cols[col] & active_rows
        if not possible:
            return
        for row in possible:
            chosen.append(row)
            covered = {
                solver._candidates[row]["row"],
                solver._candidates[row]["col"],
                solver._candidates[row]["region"],
            }
            new_cols = {c: rs for c, rs in active_cols.items() if c not in covered}
            rows_to_remove = {row} | solver._conflicts[row]
            for ck in covered:
                rows_to_remove |= solver._columns[ck]
            _solve(active_rows - rows_to_remove, new_cols, chosen)
            chosen.pop()
            if len(found) >= limit:
                return

    _solve(set(range(len(solver._candidates))), dict(solver._columns), [])
    return len(found)


# ---------------------------------------------------------------------------
# Boundary-swap neighbours
# ---------------------------------------------------------------------------


def _board_key(board: Board) -> tuple:
    return tuple(tuple(row) for row in board)


def _region_connected(
    board: Board, size: int, excl_y: int, excl_x: int, region: int
) -> bool:
    """True if `region` stays 4-connected after removing (excl_y, excl_x)."""
    start = next(
        (
            (y, x)
            for y in range(size)
            for x in range(size)
            if board[y][x] == region and (y, x) != (excl_y, excl_x)
        ),
        None,
    )
    if start is None:
        return False  # region becomes empty

    visited = {start}
    q = deque([start])
    while q:
        cy, cx = q.popleft()
        for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            ny, nx = cy + dy, cx + dx
            if (
                0 <= ny < size
                and 0 <= nx < size
                and board[ny][nx] == region
                and (ny, nx) != (excl_y, excl_x)
                and (ny, nx) not in visited
            ):
                visited.add((ny, nx))
                q.append((ny, nx))

    return all(
        (y, x) in visited
        for y in range(size)
        for x in range(size)
        if board[y][x] == region and (y, x) != (excl_y, excl_x)
    )


def _neighbours(board: Board, size: int) -> list[Board]:
    """Boards reachable by moving one boundary cell to an adjacent region,
    keeping the source region 4-connected."""
    result = []
    seen: set[tuple] = set()

    for y in range(size):
        for x in range(size):
            src = board[y][x]
            for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                ny, nx = y + dy, x + dx
                if not (0 <= ny < size and 0 <= nx < size):
                    continue
                dst = board[ny][nx]
                if dst == src:
                    continue
                swap = (y, x, dst)
                if swap in seen:
                    continue
                seen.add(swap)
                if _region_connected(board, size, y, x, src):
                    nb = [row[:] for row in board]
                    nb[y][x] = dst
                    result.append(nb)

    return result


# ---------------------------------------------------------------------------
# A* search  –  f = g (swaps made) + h (solutions − 1)
# ---------------------------------------------------------------------------


def _a_star(start: Board, size: int, max_iter: int = 3000) -> Optional[Board]:
    h0 = count_solutions(start)
    if h0 == 1:
        return start

    ctr = 0
    # heap entry: (f, g, ctr, board)
    heap: list = [(h0 - 1, 0, ctr, start)]
    best_g: dict[tuple, int] = {_board_key(start): 0}

    for _ in range(max_iter):
        if not heap:
            break

        _, g, _, board = heapq.heappop(heap)

        for nb in _neighbours(board, size):
            h = count_solutions(nb)
            if h == 1:
                return nb
            if h == 0:
                continue  # dead end — no solution exists
            ng = g + 1
            key = _board_key(nb)
            if best_g.get(key, float("inf")) <= ng:
                continue
            best_g[key] = ng
            ctr += 1
            heapq.heappush(heap, (ng + h - 1, ng, ctr, nb))

    return None


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def generate_puzzle(size: int = SIZE) -> Optional[Board]:
    """Return a board with exactly one solution, or None if generation failed."""
    stars = _place_stars(size)
    board = _flood_fill(stars, size)
    n = count_solutions(board)
    if n == 1:
        return board
    return _a_star(board, size)


# ---------------------------------------------------------------------------
# Display
# ---------------------------------------------------------------------------

_COLORS = [31, 32, 33, 34, 35, 36, 91, 92, 93, 94, 95, 96, 97, 90]


def _colored(text: str, color: int) -> str:
    return f"\033[{color}m{text}\033[0m"


def show_board(board: Board) -> None:
    size = len(board)
    print()
    for y in range(size):
        for x in range(size):
            r = board[y][x]
            print(_colored(str(r), _COLORS[r % len(_COLORS)]), end=" ")
        print()


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    import time

    for attempt in range(1, 21):
        t0 = time.perf_counter()
        puzzle = generate_puzzle(SIZE)
        elapsed = (time.perf_counter() - t0) * 1000

        if puzzle is None:
            print(f"attempt {attempt}: failed ({elapsed:.0f} ms)")
            continue

        print(f"attempt {attempt}: found in {elapsed:.0f} ms")
        show_board(puzzle)

        solver = AlgoXSolver(puzzle)
        solver.solve()
        solver.show_solution()
        print(f"unique: {solver.has_unique_solution()}")
        break
