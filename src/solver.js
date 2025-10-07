class Solver {
    /**
     * @param {BullPen} bp
     */
    constructor(bp) {
        this.bp = bp;
        this.board = bp.board;

        this.solve();
    }

    solve() {
        this.solutions = this.solveBoard();
        this.solutionMask = this.getSolutionMask(this.solutions);
        this.maxColor = this.getMaxColor(this.solutions, this.board);
    }

    solveBoard() {
        const penSets = this.getPenSets(this.board);

        if (penSets.length != SIZE) {
            // invalid board: you need same number of pens as columns
            return [];
        }

        // heuristic: start with smallest pen
        penSets.sort((a, b) => a.length - b.length);

        const solutions = this.solvePenSets(penSets);
        return solutions;
    }

    newBullSol() {
        return Array.from({ length: SIZE }, () => Array(SIZE).fill(EMPTY));
    }

    getPenSets(board) {
        const penSets = Array.from({ length: SIZE }, () => []);

        for (let r = 0; r < SIZE; r++) {
            for (let c = 0; c < SIZE; c++) {
                const color = board[r][c];
                penSets[color].push([r, c]);
            }
        }

        return penSets;
    }

    solvePenSets(penSets) {
        // TODO: use an actual queue
        // tuple of (pen_idx, solution)
        const queue = [[0, this.newBullSol()]];
        const solutions = [];

        while (queue.length > 0) {
            const [penIdx, bullSol] = queue.shift();
            if (penIdx == penSets.length) {
                // base case: solved all pen sets
                solutions.push(bullSol);
                continue;
            }

            const candidates = penSets[penIdx];
            for (const [y, x] of candidates) {
                const bs = structuredClone(bullSol);
                if (this.bp.validPlacement(y, x, bs)) {
                    bs[y][x] = BULL;
                    queue.push([penIdx + 1, bs]);
                }
            }
        }

        return solutions;
    }

    getSolutionDist(solutions, board) {
        const counts = {};

        for (const solution of solutions) {
            for (let y = 0; y < SIZE; y++) {
                for (let x = 0; x < SIZE; x++) {
                    if (solution[y][x] == BULL) {
                        const color = board[y][x];
                        if (counts[color] == null) {
                            counts[color] = new Set();
                        }

                        counts[color].add(`${y},${x}`);
                    }
                }
            }
        }

        const out = {};
        for (const key in counts) {
            out[key] = counts[key].size;
        }

        return out;
    }

    getMaxColor(solutions, board) {
        const counts = this.getSolutionDist(solutions, board);
        let maxColor = null;
        let maxVal = -1;

        for (const k in counts) {
            if (counts[k] > maxVal) {
                maxVal = counts[k];
                maxColor = k;
            }
        }

        return maxColor;
    }

    getSolutionMask(solutions) {
        const counts = Array.from({ length: SIZE }, () => Array(SIZE).fill(0));

        for (const solution of solutions) {
            for (let y = 0; y < SIZE; y++) {
                for (let x = 0; x < SIZE; x++) {
                    if (solution[y][x] == BULL) {
                        counts[y][x] += 1;
                    }
                }
            }
        }

        return counts;
    }
}