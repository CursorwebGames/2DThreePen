# maker_flood but with capped regions

import random
from collections import deque
from typing import Optional

from algox import AlgoXSolver
from research.maker_preplace import Board, count_solutions, _a_star, show_board

SIZE = 8

# ---------------------------------------------------------------------------
# Random flood-fill: no star pre-placement, just N random seeds
# ---------------------------------------------------------------------------


NUM_CAPPED = (5, 7)  # number of small regions per board
CAP_SIZE = (1, 3)  # cell count for each small region


def _random_flood_fill(
    size: int,
    num_capped: tuple[int, int] = NUM_CAPPED,
    cap_size: tuple[int, int] = CAP_SIZE,
) -> Board:
    """Pick `size` random seed cells, grow regions via interleaved random BFS.

    num_capped regions (chosen randomly in the given range) are capped at a
    random size within cap_size to reduce the solver search space. Pass
    num_capped=(0,0) to disable the feature entirely.
    Any cells left unreachable by the cap are filled in a second pass.
    """
    board = [[-1] * size for _ in range(size)]

    all_cells = [(y, x) for y in range(size) for x in range(size)]
    seeds = random.sample(all_cells, size)

    n_capped = random.randint(*num_capped)
    capped_regions = set(random.sample(range(size), min(n_capped, size)))
    max_size: dict[int, int] = {r: random.randint(*cap_size) for r in capped_regions}
    region_sizes = [0] * size

    queues: list[deque] = []
    for region, (y, x) in enumerate(seeds):
        board[y][x] = region
        region_sizes[region] = 1
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
            if region in max_size and region_sizes[region] >= max_size[region]:
                continue  # this region is full
            while q:
                ny, nx = q.popleft()
                if board[ny][nx] != -1:
                    continue
                board[ny][nx] = region
                region_sizes[region] += 1
                changed = True
                dirs = [(ny - 1, nx), (ny + 1, nx), (ny, nx - 1), (ny, nx + 1)]
                random.shuffle(dirs)
                for nny, nnx in dirs:
                    if 0 <= nny < size and 0 <= nnx < size and board[nny][nnx] == -1:
                        q.append((nny, nnx))
                break  # one cell per region per round

    # Second pass: cells isolated by capped regions get absorbed into a neighbour.
    filled = True
    while filled:
        filled = False
        for y in range(size):
            for x in range(size):
                if board[y][x] != -1:
                    continue
                for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                    ny, nx = y + dy, x + dx
                    if 0 <= ny < size and 0 <= nx < size and board[ny][nx] != -1:
                        board[y][x] = board[ny][nx]
                        filled = True
                        break

    return board


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def generate_puzzle(
    size: int = SIZE,
    max_attempts: int = 5,
    num_capped: tuple[int, int] = NUM_CAPPED,
    cap_size: tuple[int, int] = CAP_SIZE,
) -> Optional[Board]:
    """Generate a board with exactly one solution.

    Each attempt flood-fills a fresh random board and runs A* for up to max_attempts
    If cannot find one by then, try a different board.
    """
    for _ in range(max_attempts):
        board = _random_flood_fill(size, num_capped=num_capped, cap_size=cap_size)
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
        print(f"unique: {solver.has_unique_solution()}")
        break