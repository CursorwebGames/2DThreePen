class Solver {
    /**
     * @param {BullPen} bp
     */
    constructor(bp) {
        this.bp = bp;
        this.board = bp.board;
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
}