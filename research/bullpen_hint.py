from itertools import combinations
from typing import Callable
from main import get_pen_sets, PenSet, in_board, print_board

import time

Point = tuple[int, int]

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

board = [
    [0, 0, 1, 1],
    [0, 0, 1, 2],
    [3, 3, 1, 2],
    [3, 3, 2, 2],
]

SIZE = len(board)

mask: set[Point] = set()
""" Set of places where a bull CANNOT be """
pensets = get_pen_sets(board, SIZE)


def main():
    print_mask()
    start = time.perf_counter()
    solved = solve()
    end = time.perf_counter()
    print(f"Elapsed time: {end - start:.4f} seconds")
    print(f"Solved: {solved}")
    print_mask()


### PRINTING ###
def print_mask(mask=mask):
    print()
    for y in range(SIZE):
        for x in range(SIZE):
            if (y, x) in mask:
                print("x", end=" ")
            else:
                print(board[y][x], end=" ")
        print()


### WRAPPER OPT ###
def opt(*, loop=True):
    """Loop: whether to loop through each penset or not"""

    def decorator(f: Callable[[PenSet], bool]) -> Callable[[], None]:
        def wrap():
            # start = time.perf_counter()
            if loop:
                for penset in pensets:
                    f(penset)
            else:
                f()

            readjust_pensets()
            # end = time.perf_counter()
            # print(f"Elapsed time for {f}: {end - start:.4f} seconds")

        return wrap

    return decorator


### OPTIMIZATION "Optimizations": Places that pens cannot be ###
@opt()
def single_pen(penset: PenSet):
    """Only one space is available"""
    if len(penset) == 1:
        px, py = penset[0]
        for y, x in get_adjacent(px, py):
            mask.add((y, x))
        return True

    return False


@opt()
def one_direction(penset: PenSet):
    """Cell is all vertical, etc."""
    yi, xi = (penset[0][0], penset[0][1])
    color = board[yi][xi]

    vert = penset_all_vert(penset)

    changed = False

    if vert != None:
        x, color = vert
        for y in range(0, SIZE):
            if board[y][x] != color:
                mask.add((y, x))
        changed = True

    horiz = penset_all_horiz(penset)

    if horiz != None:
        y, color = horiz
        for x in range(SIZE):
            if board[y][x] != color:
                mask.add((y, x))
        changed = True

    return changed


@opt()
def pen_overlap(penset: PenSet):
    global mask
    """
    if pen has <= 3 cells, there exists at least one cell
    which can eliminate them all (i.e. that cell is invalid)
    """

    # currently a very naive solution:
    if len(penset) <= 8:
        intersect: set[Point] = set(get_adjacent(*penset[0]))
        for y, x in penset:
            adjacents = set(get_adjacent(y, x))
            intersect &= adjacents
        mask |= intersect
        if len(intersect) > 0:
            return True

    return False


@opt(loop=False)
def overcounting():
    """
    If K rows/cols contain cells from exactly K regions,
    those regions' bulls must be in those rows/cols.
    """
    row_regions: list[set] = [set() for _ in range(SIZE)]
    col_regions: list[set] = [set() for _ in range(SIZE)]

    color_to_ps: dict[int, PenSet] = {}

    for ps in pensets:
        color = board[ps[0][0]][ps[0][1]]
        color_to_ps[color] = ps

        for y, x in ps:
            row_regions[y].add(color)
            col_regions[x].add(color)

    for k in range(1, SIZE):
        for row_subset in combinations(range(SIZE), k):
            regions = set()

            for y in row_subset:
                regions |= row_regions[y]

                # early rejection
                if len(regions) > k:
                    break

            if len(regions) != k:
                continue

            for color in regions:
                for y, x in color_to_ps[color]:
                    if y not in row_subset:
                        mask.add((y, x))

        for col_subset in combinations(range(SIZE), k):
            regions = set()

            for x in col_subset:
                regions |= col_regions[x]

                # early rejection
                if len(regions) > k:
                    break

            if len(regions) != k:
                continue

            for color in regions:
                for y, x in color_to_ps[color]:
                    if x not in col_subset:
                        mask.add((y, x))


functions = [single_pen, one_direction, pen_overlap, overcounting]


def solve() -> bool:
    """Constraint propagation until stuck, then backtrack."""
    prev_size = -1
    while len(mask) != prev_size:
        prev_size = len(mask)
        for fn in functions:
            fn()
        if any(len(ps) == 0 for ps in pensets):
            return False
        if len(mask) == SIZE**2 - SIZE:
            return True

    # Stuck: pick region with fewest candidates (MRV)
    target = min(pensets, key=len)

    for y, x in list(target):
        saved_mask = set(mask)
        saved_pensets = [list(ps) for ps in pensets]

        # Place bull at (y, x): mask rest of region and all adjacent cells
        for cy, cx in list(target):
            if (cy, cx) != (y, x):
                mask.add((cy, cx))
        for ay, ax in get_adjacent(y, x):
            mask.add((ay, ax))
        readjust_pensets()

        if solve():
            return True

        # Backtrack
        mask.clear()
        mask.update(saved_mask)
        for i, ps in enumerate(pensets):
            ps.clear()
            ps.extend(saved_pensets[i])

    return False


### UTIL ###
def readjust_pensets():
    """Readjust penset based on a mask"""
    for penset in pensets:
        for y, x in penset:
            if (y, x) in mask:
                penset.remove((y, x))


def get_adjacent(y: int, x: int):
    """Only returns valid positions!"""
    out = []
    for dy in range(-1, 2):
        for dx in range(-1, 2):
            if dy == 0 and dx == 0:
                continue

            point = (y + dy, x + dx)
            if in_board(y + dy, x + dx, SIZE):
                out.append(point)

    return out


def penset_all_horiz(penset: PenSet):
    """
    Returns a range which is the range of the squares it is horizontal,
    and None otherwise
    """
    ypos = penset[0][0]
    xpos = penset[0][1]
    color = board[ypos][xpos]

    for i in range(1, len(penset)):
        if penset[i][0] != ypos:
            return None

    return ypos, color


def penset_all_vert(penset: PenSet):
    ypos = penset[0][0]
    xpos = penset[0][1]
    color = board[ypos][xpos]

    for i in range(1, len(penset)):
        if penset[i][1] != xpos:
            return None

    return xpos, color


if __name__ == "__main__":
    main()
