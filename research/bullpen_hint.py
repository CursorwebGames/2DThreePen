from typing import Callable
from main import get_pen_sets, PenSet, in_board, print_board


Point = tuple[int, int]

board = [
    [3, 3, 5, 5, 5, 4],
    [3, 3, 1, 5, 5, 0],
    [3, 3, 1, 1, 0, 0],
    [3, 3, 1, 0, 0, 0],
    [1, 1, 1, 0, 0, 0],
    [2, 0, 0, 0, 0, 0],
]

SIZE = len(board)

mask: set[Point] = set()
""" Set of places where a bull CANNOT be """
pensets = get_pen_sets(board, SIZE)


def main():
    functions = [single_pen, one_direction, pen_overlap]

    for fn in functions:
        print("running", fn)
        fn()

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
def opt(f: Callable[[PenSet], bool]) -> Callable[[], None]:
    def wrap():
        changed = False
        while not changed:
            for penset in pensets:
                if f(penset):
                    changed = True
                readjust_pensets()

    return wrap


### OPTIMIZATION "Optimizations": Places that pens cannot be ###
@opt
def single_pen(penset: PenSet):
    """Only one space is available"""
    if len(penset) == 1:
        px, py = penset[0]
        for y, x in get_adjacent(px, py):
            mask.add((y, x))
        return True

    return False


@opt
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


@opt
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


### UTIL ###
def readjust_pensets():
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
            if in_board(y, x, SIZE):
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
