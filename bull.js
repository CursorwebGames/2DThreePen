const SIZE = 6;
const BULL = 0;
const EMPTY = -1;


class BullPen {
    constructor() {
        this.bulls = Array.from({ length: SIZE }, () => Array(SIZE).fill(EMPTY));
        // this.board = Array.from({ length: SIZE }, () => Array(SIZE).fill(EMPTY));
        // this.board = [
        //     [0, 0, 1, 1, 1, 1],
        //     [0, 2, 1, 1, 3, 3],
        //     [2, 2, 2, 1, 3, 3],
        //     [2, 2, 5, 4, 3, 3],
        //     [2, 4, 4, 4, 4, 3],
        //     [4, 4, 4, 4, 4, 4],
        // ];
        this.board = `544223
544233
544233
522233
500111
500011`.split("\n").map(x => x.split("").map(Number));
        this.bullCoords = [];
        this.colors = [];
        for (let i = 0; i < this.board.length; i++) {
            this.colors.push(color(random(255), random(255), random(255)));
        }
    }

    createBoard() {
        this.bullCoords = this.createBulls();
        this.fillPen(this.bullCoords);
    }

    /**
     * returns true if successfully added legal bulls
     * from row `row`
     */
    createBulls(row = 0) {
        if (row >= SIZE) {
            return true;
        }


    }
}