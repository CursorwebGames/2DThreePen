const SIZE = 6;
const BULL = 0;
const DOT = 1;
const EMPTY = -1;


class BullPen {
    constructor() {
        this.bulls = Array.from({ length: SIZE }, () => Array(SIZE).fill(EMPTY));
        this.board = Array.from({ length: SIZE }, () => Array(SIZE).fill(EMPTY));

        if (MANUAL_BOARD) {
            // SET YOUR BOARD HERE
            this.board =
                [[5, 5, 5, 4, 4, 4], [5, 3, 5, 5, 2, 4], [3, 3, 5, 0, 2, 4], [3, 1, 1, 0, 2, 2], [3, 3, 1, 0, 0, 2], [3, 3, 1, 0, 0, 2]];
            // genSolutions.board;
        }

        /**
         * @type {[number, number][]}
         */
        this.bullCoords = [];
        this.colors = [];

        if (!MANUAL_BOARD) {
            this.createBoard();
        }

        for (let i = 0; i < this.board.length; i++) {
            const hue = floor(map(i, 0, SIZE, 0, 360));
            this.colors.push(color(`hsl(${hue}, 80%, 60%)`));
        }
    }

    createBoard() {
        const result = this.createBulls(this.bulls, this.bullCoords);
        if (!result) {
            throw Error("Unable to place bulls");
        }

        this.createPens(this.board, this.bullCoords);
    }

    /**
     * On each bull coord, expand the region
     */
    createPens(board, bullCoords) {
        let stacks = [];
        for (let color = 0; color < bullCoords.length; color++) {
            const [y, x] = bullCoords[color];
            board[y][x] = color;
            stacks.push([color, this.nextDirections(y, x)]);
        }

        stacks = shuffle(stacks);

        while (true) {
            // if only one true exists, this exits
            // we want all false because that means
            // no board can keep expanding, i.e. board is full
            let allFalse = true;

            for (const [color, stack] of stacks) {
                if (this.floodfillPen(board, color, stack)) {
                    allFalse = false;
                }
            }

            if (allFalse) {
                break;
            }
        }
    }

    floodfillPen(board, color, stack) {
        while (stack.length > 0) {
            const [y, x] = stack.pop();
            if (this.inBoard(y, x) && board[y][x] == EMPTY) {
                board[y][x] = color;
                stack.push(...this.nextDirections(y, x));
                return true;
            }
        }
        return false;
    }

    /**
     * All adjacent cells that floodfillPen can expand to
     * NOTE: doesn't check if such cell is valid
     */
    nextDirections(y, x) {
        let out = [[y - 1, x], [y + 1, x], [y, x - 1], [y, x + 1]];
        out = shuffle(out);
        return out;
    }

    /**
     * returns true if successfully added legal bulls
     * from row `row`
     * @returns {boolean}
     */
    createBulls(bulls, bullCoords, row = 0) {
        if (row >= SIZE) {
            return true;
        }

        const tries = this.generateTries();

        while (tries.length > 0) {
            const col = tries.pop();

            if (!this.validPlacement(row, col)) {
                continue;
            }

            bulls[row][col] = BULL;

            if (this.createBulls(bulls, bullCoords, row + 1)) {
                bullCoords.push([row, col]);
                return true;
            }

            bulls[row][col] = EMPTY;
        }

        return false;
    }

    generateTries() {
        let tries = [];
        for (let i = 0; i < SIZE; i++) {
            tries.push(i);
        }

        tries = shuffle(tries);

        return tries;
    }

    validPlacement(row, col, bulls = this.bulls) {
        return !(this.checkRowBull(row, bulls) || this.checkColBull(col, bulls) || this.checkBullTouch(row, col, bulls));
    }

    checkRowBull(row, bulls) {
        for (let c = 0; c < SIZE; c++) {
            if (bulls[row][c] == BULL) {
                return true;
            }
        }

        return false;
    }

    checkColBull(col, bulls) {
        for (let r = 0; r < SIZE; r++) {
            if (bulls[r][col] == BULL) {
                return true;
            }
        }

        return false;
    }

    checkBullTouch(y, x, bulls) {
        for (let dy = -1; dy <= 1; dy++) {
            for (let dx = -1; dx <= 1; dx++) {
                if (dy == 0 && dx == 0) {
                    continue;
                }

                if (!this.inBoard(y + dy, x + dx)) {
                    continue;
                }

                if (bulls[y + dy][x + dx] == BULL) {
                    return true;
                }
            }
        }

        return false;
    }

    inBoard(y, x) {
        return y >= 0 && y < SIZE && x >= 0 && x < SIZE;
    }
}