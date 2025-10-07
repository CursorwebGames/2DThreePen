import random
from collections import deque
import copy
import json

ROWS = 6
COLS = 6

BULL = 0
EMPTY = -1

bulls = [[EMPTY for _ in range(ROWS)] for _ in range(COLS)]
# 0-4 are the colors
# board = [[EMPTY for _ in range(ROWS)] for _ in range(COLS)]
board = [
    [3, 3, 5, 5, 5, 4],
    [3, 3, 1, 5, 5, 0],
    [3, 3, 1, 1, 0, 0],
    [3, 3, 1, 0, 0, 0],
    [1, 1, 1, 0, 0, 0],
    [2, 0, 0, 0, 0, 0],
]


def main():
    # create_board()

    print_bulls()
    print_board()

    solutions = solve_board()

    print(len(solutions), "solutions found")
    # print_bulls_with_board(solutions[0], board)

    print_solution_stats(solutions)

    with open("solutions.json", "w") as f:
        json.dump({"solutions": solutions, "board": board}, f)


### printing ###
def print_bulls(bulls=bulls):
    print()
    for row in bulls:
        for col in row:
            if col == EMPTY:
                print(". ", end="")
            else:
                print("# ", end="")
        print()


def print_bulls_with_board(bulls, board):
    print()
    for row in range(ROWS):
        for col in range(COLS):
            if bulls[row][col] != EMPTY:
                print("#", end="")
                print(" ", end="")
            else:
                print(f"{board[row][col]} ", end="")
        print(f"  {' '.join([str(x) for x in board[row]])}")


def print_board(board=board, end=""):
    print()
    for row in board:
        for col in row:
            print(col, end=end)
        print()


def print_solution_stats(solutions):
    counts = get_solution_stats(solutions)
    for k, v in counts.items():
        print(f"({k}: {v})", end=" ")

    map = get_solution_mask(solutions)
    print_board(map, " ")


### SOLVER ###
PenSet = list[tuple[int, int]]


def solve_board(board=board):
    pen_sets = get_pen_sets(board)

    if len(pen_sets) != ROWS:
        print("could not solve (invalid board)")
        return []

    # heuristic: start with smallest pens first
    pen_sets.sort(key=lambda pen: len(pen))

    solutions = solve_pensets(pen_sets)
    if len(solutions) == 0:
        print("could not solve")

    return solutions


def new_bull_sol():
    return [[EMPTY for _ in range(ROWS)] for _ in range(COLS)]


def solve_pensets(pen_sets: list[PenSet], limit=-1):
    # for each pen in pen sets, try out a pen,
    # and then solve the rest of them with the remaining of the pens
    # NOTE: this will get all rows and columns, just because there are 10 pens,
    # and we already check that there can't be more than one bull per row,
    # so "pigeonhole principle" tells us that the 10 pens force 10 rows and 10 cols

    # (pen_idx, solution)
    queue = deque([(0, new_bull_sol())])
    solutions: list[list[list[int]]] = []

    while len(queue) > 0:
        pen_idx, bull_sol = queue.popleft()
        if pen_idx == len(pen_sets):
            solutions.append(bull_sol)
            if limit > 0 and len(solutions) >= limit:
                break
            continue

        candidates = pen_sets[pen_idx]
        for y, x in candidates:
            bs = copy.deepcopy(bull_sol)
            if valid_placement(y, x, bs):
                bs[y][x] = BULL
                queue.append((pen_idx + 1, bs))

    return solutions


def get_pen_sets(board: list[list[int]] = board):
    """
    turn regions into a list of coords,
    `0: [# of cells in color 0], etc.`
    """
    pen_sets: list[PenSet] = [[] for _ in range(ROWS)]
    for row in range(ROWS):
        for col in range(COLS):
            number = board[row][col]
            pen_sets[number].append((row, col))

    return pen_sets


def get_solution_stats(solutions: list[list[list[int]]], board=board):
    """
    Gets the number of unique possible placements per set
    - some tiles are just never valid, and will never lead to a solution
    In other words, the number of tiles in each region where a bull hits that tile at least once in a solution

    Could open up to: try targeting pens with the least number of "variance" (multiple bull placement solutions in a pen)
    """
    counts: dict[int, set[tuple[int, int]]] = {}

    for solution in solutions:
        for y in range(ROWS):
            for x in range(COLS):
                if solution[y][x] == BULL:
                    color = board[y][x]
                    if not color in counts:
                        counts[color] = set()

                    counts[color].add((y, x))

    out = {}
    for k, v in counts.items():
        out[k] = len(v)

    return out


def get_solution_mask(solutions: list[list[list[int]]]):
    """
    Gets number of solutions where a bull is in that tile

    Could open up to: try ceding bulls with more solutions/bulls with less solutions
    """
    counts = [[0 for _ in range(ROWS)] for _ in range(COLS)]

    for solution in solutions:
        for y in range(ROWS):
            for x in range(COLS):
                if solution[y][x] == BULL:
                    counts[y][x] += 1

    return counts


### BOARD GENERATOR ###
def create_board():
    bull_coords = []
    success = create_bulls(bull_coords)
    if not success:
        raise Exception("skill issue lmao")
    create_pens(bull_coords)


def create_bulls(bull_coords: list[tuple[int, int]], row=0, bulls=bulls):
    # recursively add bulls, one per each row, backtracking if a bull placement is impossible
    # so like a dfs
    # stores bull coords in `bull_coords`
    if row >= ROWS:
        return True

    tries = list(range(0, COLS))
    random.shuffle(tries)

    while len(tries) > 0:
        col = tries.pop()

        if not valid_placement(row, col):
            continue

        bulls[row][col] = BULL

        if create_bulls(bull_coords, row + 1):
            bull_coords.append((row, col))
            return True

        bulls[row][col] = EMPTY

    return False


def create_pens(bull_coords: list[tuple[int, int]], board=board):
    """On each bull coord, expand the region"""
    # careful that the coords are (y, x)
    # while still can expand, for each bull region, choose to expand it

    # [(0, stack), (1, stack)] :
    stacks: list[tuple[int, list[tuple[int, int]]]] = []
    for color, (y, x) in enumerate(bull_coords):
        board[y][x] = color
        stacks.append((color, next_directions(y, x)))

    random.shuffle(stacks)

    while True:
        all_false = True
        for stack in stacks:
            if floodfill_pen(*stack) == True:
                all_false = False

        if all_false:
            break


def floodfill_pen(color: int, stack: list[tuple[int, int]], board=board):
    """Directly assign color to board. Returns False if no board was assigned"""
    while len(stack) > 0:
        y, x = stack.pop()
        if in_board(y, x) and board[y][x] == EMPTY:
            board[y][x] = color
            stack.extend(next_directions(y, x))
            return True

    return False


#   ### BOARD FIXER ###
# def fix_board(board: list[list[int]]):
#     pensets = get_pen_sets(board)
#     solutions

#     stack = [pensets]

#     while not board_unique(board):
#         expand_penset(pensets[0])

#     pass


### UTILITY ###
def next_directions(y: int, x: int):
    """Gets the adjacent cells that floodfill_pen can expand to"""
    # doesn't check if next is valid
    o = [(y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)]
    random.shuffle(o)
    return o


def in_board(y: int, x: int):
    return y >= 0 and y < ROWS and x >= 0 and x < COLS


def valid_placement(row: int, col: int, bulls=bulls):
    return not (
        check_row_bull(row, bulls)
        or check_col_bull(col, bulls)
        or check_bull_touch(row, col, bulls)
    )


def check_row_bull(row: int, bulls=bulls):
    """Check if row has a bull"""
    for i in range(COLS):
        if bulls[row][i] == BULL:
            return True

    return False


def check_col_bull(col: int, bulls=bulls):
    """Check if row has a bull"""
    for i in range(COLS):
        if bulls[i][col] == BULL:
            return True

    return False


def check_bull_touch(y: int, x: int, bulls=bulls):
    for dy in range(-1, 2):
        for dx in range(-1, 2):
            if dy == 0 and dx == 0:
                continue

            if not in_board(y + dy, x + dx):
                continue

            if bulls[y + dy][x + dx] == BULL:
                return True

    return False


main()
