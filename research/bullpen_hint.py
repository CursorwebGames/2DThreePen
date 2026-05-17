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

board = [
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
]

SIZE = len(board)

mask: set[Point] = set()
""" Set of places where a bull CANNOT be """
pensets = get_pen_sets(board, SIZE)


def main():
    functions = [single_pen, one_direction, pen_overlap, overcounting]

    print_mask()

    iterations = 0
    start = time.perf_counter()
    while True:
        for fn in functions:
            fn()
        iterations += 1
        if len(mask) == SIZE**2 - SIZE:
            break
    end = time.perf_counter()
    print(f"Elapsed time: {end - start:.4f} seconds")
    print(f"Iterations: {iterations}")

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
            if loop:
                for penset in pensets:
                    f(penset)
            else:
                f()

            readjust_pensets()

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
    # algorithm: loop through # of rows, and check how many regions are present in those row subsections
    # if row count == # of regions then we have overcounting, and block off rest of region
    row_regions: list[set] = [set() for _ in range(SIZE)]
    col_regions: list[set] = [set() for _ in range(SIZE)]
    for ps in pensets:
        for y, x in ps:
            c = board[y][x]
            row_regions[y].add(c)
            col_regions[x].add(c)

    for k in range(1, SIZE):
        for row_subset in combinations(range(SIZE), k):
            regions = set().union(*(row_regions[y] for y in row_subset))
            if len(regions) == k:
                row_set = set(row_subset)
                for ps in pensets:
                    if ps and board[ps[0][0]][ps[0][1]] in regions:
                        for y, x in ps:
                            if y not in row_set:
                                mask.add((y, x))

        for col_subset in combinations(range(SIZE), k):
            regions = set().union(*(col_regions[x] for x in col_subset))
            if len(regions) == k:
                col_set = set(col_subset)
                for ps in pensets:
                    if ps and board[ps[0][0]][ps[0][1]] in regions:
                        for y, x in ps:
                            if x not in col_set:
                                mask.add((y, x))


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
