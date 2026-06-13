import random
from collections import deque
from typing import Optional

from algox import AlgoXSolver
from research.maker_preplace import Board, SIZE, count_solutions, _a_star, show_board

# ---------------------------------------------------------------------------
# Random flood-fill: no star pre-placement, just N random seeds
# ---------------------------------------------------------------------------


def _random_flood_fill(size: int) -> Board:
    """Pick `size` random seed cells, grow regions via interleaved random BFS."""
    board = [[-1] * size for _ in range(size)]

    all_cells = [(y, x) for y in range(size) for x in range(size)]
    seeds = random.sample(all_cells, size)

    queues: list[deque] = []
    for region, (y, x) in enumerate(seeds):
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
# Public API
# ---------------------------------------------------------------------------


def generate_puzzle(size: int = SIZE, max_attempts: int = 5) -> Optional[Board]:
    """Generate a board with exactly one solution.

    Each attempt flood-fills a fresh random board and runs A* for up to max_attempts
    If cannot find one by then, try a different board.
    """
    for _ in range(max_attempts):
        board = _random_flood_fill(size)
        n = count_solutions(board)
        if n == 1:
            return board
        result = _a_star(board, size, max_iter=20)
        if result is not None:
            return result
    return None


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
