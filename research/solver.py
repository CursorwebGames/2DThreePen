from itertools import combinations

Point = tuple[int, int]
PenSet = list[tuple[int, int]]

_COLORS = [31, 32, 33, 34, 35, 36, 91, 92, 93, 94, 95, 96, 97, 90]


def _colored(text: str, color: int) -> str:
    return f"\033[{color}m{text}\033[0m"


class Solver:
    def __init__(self, board: list[list[int]]):
        self.board = board
        self.SIZE = len(board)
        self.mask: set[Point] = set()
        self.pensets: list[PenSet] = self._get_pen_sets()
        self._solution: list[Point] = []
        self.had_to_guess = False

    def solve(self) -> list[Point]:
        """Returns one bull position per region, or [] if unsolvable."""
        if self._solve():
            self._solution = [ps[0] for ps in self.pensets if ps]
            return self._solution
        return []

    # --- display ---

    def show_board(self):
        """Prints the board with each region in its own color."""
        print()
        for y in range(self.SIZE):
            for x in range(self.SIZE):
                region = self.board[y][x]
                print(_colored(str(region), _COLORS[region % len(_COLORS)]), end=" ")
            print()

    def show_mask(self):
        """Prints the board with region colors and masked cells shown as #."""
        print()
        for y in range(self.SIZE):
            for x in range(self.SIZE):
                if (y, x) in self.mask:
                    print(_colored("#", 90), end=" ")
                else:
                    region = self.board[y][x]
                    print(
                        _colored(str(region), _COLORS[region % len(_COLORS)]), end=" "
                    )
            print()

    def show_solution(self):
        """Prints the board with bull positions marked as ★."""
        solution_set = set(self._solution)
        print()
        for y in range(self.SIZE):
            for x in range(self.SIZE):
                region = self.board[y][x]
                color = _COLORS[region % len(_COLORS)]
                mark = "★" if (y, x) in solution_set else str(region)
                print(_colored(mark, color), end=" ")
            print()

    # --- backtracking ---

    def _solve(self) -> bool:
        prev_size = -1
        while len(self.mask) != prev_size:
            prev_size = len(self.mask)
            self._single_pen()
            self._one_direction()
            self._pen_overlap()
            self._overcounting()
            if any(len(ps) == 0 for ps in self.pensets):
                return False
            if len(self.mask) == self.SIZE**2 - self.SIZE:
                return True

        # Stuck: pick region with fewest candidates (must have ≥2 to make a real choice)
        self.had_to_guess = True
        target = min((ps for ps in self.pensets if len(ps) >= 2), key=len)

        for y, x in list(target):
            saved_mask = set(self.mask)
            saved_pensets = [list(ps) for ps in self.pensets]

            for cy, cx in list(target):
                if (cy, cx) != (y, x):
                    self.mask.add((cy, cx))
            for pt in self._get_adjacent(y, x):
                self.mask.add(pt)
            self._readjust_pensets()

            if self._solve():
                return True

            self.mask.clear()
            self.mask.update(saved_mask)
            for i, ps in enumerate(self.pensets):
                ps.clear()
                ps.extend(saved_pensets[i])

        return False

    # --- strategies ---

    def _single_pen(self):
        for penset in self.pensets:
            if len(penset) == 1:
                for pt in self._get_adjacent(*penset[0]):
                    self.mask.add(pt)
        self._readjust_pensets()

    def _one_direction(self):
        for penset in self.pensets:
            if not penset:
                continue
            vert = self._penset_all_vert(penset)
            if vert is not None:
                x, color = vert
                for y in range(self.SIZE):
                    if self.board[y][x] != color:
                        self.mask.add((y, x))
            horiz = self._penset_all_horiz(penset)
            if horiz is not None:
                y, color = horiz
                for x in range(self.SIZE):
                    if self.board[y][x] != color:
                        self.mask.add((y, x))
        self._readjust_pensets()

    def _pen_overlap(self):
        for penset in self.pensets:
            if not penset or len(penset) > 8:
                continue
            intersect = set(self._get_adjacent(*penset[0]))
            for y, x in penset:
                intersect &= set(self._get_adjacent(y, x))
            self.mask |= intersect
        self._readjust_pensets()

    def _overcounting(self):
        SIZE = self.SIZE
        row_regions: list[set] = [set() for _ in range(SIZE)]
        col_regions: list[set] = [set() for _ in range(SIZE)]
        color_to_ps: dict[int, PenSet] = {}

        for ps in self.pensets:
            if not ps:
                continue
            color = self.board[ps[0][0]][ps[0][1]]
            color_to_ps[color] = ps
            for y, x in ps:
                row_regions[y].add(color)
                col_regions[x].add(color)

        for k in range(1, SIZE):
            for row_subset in combinations(range(SIZE), k):
                regions: set = set()
                for y in row_subset:
                    regions |= row_regions[y]
                    if len(regions) > k:
                        break
                if len(regions) != k:
                    continue
                for color in regions:
                    for y, x in color_to_ps[color]:
                        if y not in row_subset:
                            self.mask.add((y, x))

            for col_subset in combinations(range(SIZE), k):
                regions = set()
                for x in col_subset:
                    regions |= col_regions[x]
                    if len(regions) > k:
                        break
                if len(regions) != k:
                    continue
                for color in regions:
                    for y, x in color_to_ps[color]:
                        if x not in col_subset:
                            self.mask.add((y, x))

        self._readjust_pensets()

    # --- utils ---

    def _readjust_pensets(self):
        for penset in self.pensets:
            for y, x in penset:
                if (y, x) in self.mask:
                    penset.remove((y, x))

    def _get_pen_sets(self) -> list[PenSet]:
        pen_sets: list[PenSet] = [[] for _ in range(self.SIZE)]
        for row in range(self.SIZE):
            for col in range(self.SIZE):
                pen_sets[self.board[row][col]].append((row, col))
        return pen_sets

    def _get_adjacent(self, y: int, x: int) -> list[Point]:
        out = []
        for dy in range(-1, 2):
            for dx in range(-1, 2):
                if dy == 0 and dx == 0:
                    continue
                ny, nx = y + dy, x + dx
                if 0 <= ny < self.SIZE and 0 <= nx < self.SIZE:
                    out.append((ny, nx))
        return out

    def _penset_all_horiz(self, penset: PenSet):
        ypos, xpos = penset[0]
        color = self.board[ypos][xpos]
        for i in range(1, len(penset)):
            if penset[i][0] != ypos:
                return None
        return ypos, color

    def _penset_all_vert(self, penset: PenSet):
        ypos, xpos = penset[0]
        color = self.board[ypos][xpos]
        for i in range(1, len(penset)):
            if penset[i][1] != xpos:
                return None
        return xpos, color


if __name__ == "__main__":
    # board = [
    #     [3, 3, 5, 5, 5, 4],
    #     [3, 3, 1, 5, 5, 0],
    #     [3, 3, 1, 1, 0, 0],
    #     [3, 3, 1, 0, 0, 0],
    #     [1, 1, 1, 0, 0, 0],
    #     [2, 0, 0, 0, 0, 0],
    # ]

    # board = [
    #     [0, 1, 1, 2, 2, 2],
    #     [0, 3, 2, 2, 2, 2],
    #     [0, 3, 3, 2, 2, 2],
    #     [3, 3, 4, 2, 2, 2],
    #     [4, 4, 4, 5, 5, 5],
    #     [4, 4, 4, 4, 5, 5],
    # ]

    # board = [
    #     [0, 0, 1, 1, 1, 1, 2, 2, 2, 3],
    #     [0, 1, 1, 1, 1, 1, 2, 2, 3, 3],
    #     [0, 1, 0, 1, 1, 1, 1, 3, 3, 4],
    #     [0, 0, 0, 1, 1, 5, 5, 5, 4, 4],
    #     [6, 6, 0, 0, 5, 5, 5, 5, 4, 4],
    #     [6, 6, 0, 0, 5, 7, 7, 7, 7, 8],
    #     [6, 6, 6, 6, 5, 7, 7, 7, 8, 8],
    #     [6, 6, 6, 9, 9, 9, 7, 7, 8, 8],
    #     [6, 6, 6, 6, 9, 9, 9, 9, 8, 8],
    #     [6, 6, 6, 6, 9, 9, 9, 9, 9, 8],
    # ]

    # board = [
    #     [0, 0, 1, 1],
    #     [0, 0, 1, 2],
    #     [3, 3, 1, 2],
    #     [3, 3, 2, 2],
    # ]

    board = [
        [0, 0, 0, 1, 1, 1, 2, 2],
        [0, 0, 0, 1, 1, 1, 1, 2],
        [3, 0, 0, 0, 1, 2, 1, 2],
        [3, 3, 0, 0, 2, 2, 2, 2],
        [3, 3, 3, 0, 2, 2, 2, 4],
        [3, 5, 5, 6, 6, 2, 4, 4],
        [7, 7, 5, 6, 6, 4, 4, 4],
        [5, 5, 5, 5, 4, 4, 4, 4],
    ]

    solver = Solver(board)
    solver.show_board()
    solver.solve()
    solver.show_board()
    solver.show_mask()
    print("Had to guess?", solver.had_to_guess)
