from collections import defaultdict
import time

Point = tuple[int, int]

_COLORS = [31, 32, 33, 34, 35, 36, 91, 92, 93, 94, 95, 96, 97, 90]


def _colored(text: str, color: int) -> str:
    return f"\033[{color}m{text}\033[0m"


class AlgoXSolver:
    def __init__(self, board: list[list[int]]):
        self.board = board
        self.SIZE = len(board)
        self._solution: list[Point] = []
        self._solutions: list[list[int]] = []
        self._candidates: list[dict] = []
        self._conflicts: defaultdict[int, set[int]] = defaultdict(set)
        self._columns: defaultdict = defaultdict(set)
        self._build()

    def solve(self) -> list[Point]:
        self._solutions = []
        self._solve(
            set(range(len(self._candidates))),
            dict(self._columns),
            [],
        )
        if self._solutions:
            self._solution = [self._candidates[r]["pos"] for r in self._solutions[0]]
        return self._solution

    def has_unique_solution(self) -> bool:
        return len(self._solutions) == 1

    # --- display ---

    def show_board(self):
        print()
        for y in range(self.SIZE):
            for x in range(self.SIZE):
                region = self.board[y][x]
                print(_colored(str(region), _COLORS[region % len(_COLORS)]), end=" ")
            print()

    def show_solution(self):
        solution_set = set(self._solution)
        print()
        for y in range(self.SIZE):
            for x in range(self.SIZE):
                region = self.board[y][x]
                color = _COLORS[region % len(_COLORS)]
                mark = "★" if (y, x) in solution_set else str(region)
                print(_colored(mark, color), end=" ")
            print()

    # --- build ---

    def _build(self):
        for y in range(self.SIZE):
            for x in range(self.SIZE):
                region = self.board[y][x]
                self._candidates.append(
                    {
                        "pos": (y, x),
                        "row": ("row", y),
                        "col": ("col", x),
                        "region": ("region", region),
                    }
                )

        n = len(self._candidates)
        for i in range(n):
            ay, ax = self._candidates[i]["pos"]
            for j in range(i + 1, n):
                by, bx = self._candidates[j]["pos"]
                if abs(ay - by) <= 1 and abs(ax - bx) <= 1:
                    self._conflicts[i].add(j)
                    self._conflicts[j].add(i)

        for i, c in enumerate(self._candidates):
            self._columns[c["row"]].add(i)
            self._columns[c["col"]].add(i)
            self._columns[c["region"]].add(i)

    # --- Algorithm X ---

    def _solve(
        self,
        active_rows: set[int],
        active_cols: dict,
        chosen: list[int],
    ):
        if not active_cols:
            self._solutions.append(chosen.copy())
            return

        # MRV: pick column with fewest remaining candidates
        col = min(active_cols, key=lambda c: len(active_cols[c] & active_rows))
        possible = active_cols[col] & active_rows

        if not possible:
            return

        for row in possible:
            chosen.append(row)

            # remove columns satisfied by this choice
            covered = {
                self._candidates[row]["row"],
                self._candidates[row]["col"],
                self._candidates[row]["region"],
            }
            new_cols = {c: rows for c, rows in active_cols.items() if c not in covered}

            # remove: chosen row, 8-adjacent rows, AND all rows sharing a covered column
            rows_to_remove = {row} | self._conflicts[row]
            for col_key in covered:
                rows_to_remove |= self._columns[col_key]
            new_rows = active_rows - rows_to_remove

            self._solve(new_rows, new_cols, chosen)
            chosen.pop()

            if len(self._solutions) >= 2:
                return


if __name__ == "__main__":
    from research.hintsolver import Solver

    RUNS = 200

    boards = [
        (
            "4x4",
            [
                [0, 0, 1, 1],
                [0, 0, 1, 2],
                [3, 3, 1, 2],
                [3, 3, 2, 2],
            ],
        ),
        (
            "6x6",
            [
                [0, 1, 1, 2, 2, 2],
                [0, 3, 2, 2, 2, 2],
                [0, 3, 3, 2, 2, 2],
                [3, 3, 4, 2, 2, 2],
                [4, 4, 4, 5, 5, 5],
                [4, 4, 4, 4, 5, 5],
            ],
        ),
        (
            "8x8",
            [
                [0, 0, 0, 1, 1, 1, 2, 2],
                [0, 0, 0, 1, 1, 1, 1, 2],
                [3, 0, 0, 0, 1, 2, 1, 2],
                [3, 3, 0, 0, 2, 2, 2, 2],
                [3, 3, 3, 0, 2, 2, 2, 4],
                [3, 5, 5, 6, 6, 2, 4, 4],
                [7, 7, 5, 6, 6, 4, 4, 4],
                [5, 5, 5, 5, 4, 4, 4, 4],
            ],
        ),
        (
            "10x10",
            [
                [0, 0, 1, 1, 1, 1, 2, 2, 2, 3],
                [0, 1, 1, 1, 1, 1, 2, 2, 3, 3],
                [0, 1, 0, 1, 1, 1, 1, 3, 3, 4],
                [0, 0, 0, 1, 1, 5, 5, 5, 4, 4],
                [6, 6, 0, 0, 5, 5, 5, 5, 4, 4],
                [6, 6, 0, 0, 5, 7, 7, 7, 7, 8],
                [6, 6, 6, 6, 5, 7, 7, 7, 8, 8],
                [6, 6, 6, 9, 9, 9, 7, 7, 8, 8],
                [6, 6, 6, 6, 9, 9, 9, 9, 8, 8],
                [6, 6, 6, 6, 9, 9, 9, 9, 9, 8],
            ],
        ),
    ]

    for name, board in boards:
        print(f"\n=== {name} ({RUNS} runs each) ===")

        ax = AlgoXSolver(board)
        t0 = time.perf_counter()
        for _ in range(RUNS):
            ax = AlgoXSolver(board)
            ax.solve()
        t1 = time.perf_counter()
        avg_ax = (t1 - t0) / RUNS * 1000
        print(
            f"  AlgoX : {avg_ax:.3f} ms/run  unique={ax.has_unique_solution()}  solution={ax._solution}"
        )

        s = Solver(board)
        t2 = time.perf_counter()
        for _ in range(RUNS):
            s = Solver(board)
            s.solve()
        t3 = time.perf_counter()
        avg_s = (t3 - t2) / RUNS * 1000
        print(f"  Solver: {avg_s:.3f} ms/run  solution={s._solution}")

        winner = "AlgoX" if avg_ax < avg_s else "Solver"
        ratio = max(avg_ax, avg_s) / min(avg_ax, avg_s)
        print(f"  → {winner} wins by {ratio:.1f}x")
