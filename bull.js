const SIZE = 5;
const BULL = 0;
const EMPTY = -1;


class BullPen {
    constructor() {
        this.bulls = Array.from({ length: SIZE }, () => Array(SIZE).fill(EMPTY));
        this.board = Array.from({ length: SIZE }, () => Array(SIZE).fill(EMPTY));
    }
}