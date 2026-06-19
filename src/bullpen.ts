import type { Point } from "./solver";

const BULL = 0;
const EMPTY = -1;
const DOT = 1;

// const RECT_SIZE = 50;
const PADDING = 1;
export const REGION_BORDER = 3;

type Mask = typeof BULL | typeof EMPTY | typeof DOT;

export class BullPen {
    board: number[][];
    colors: p5.Color[] = [];
    size: number;
    rectSize: number;
    private canvasSize: number;

    onBoardChange!: () => void;
    onComplete!: () => void;

    private undoStack: Mask[][][] = [];
    private redoStack: Mask[][][] = [];
    /** Snapshot to keep track of undo stack */
    private beforeSnapshot: Mask[][] | null = null;

    private dragMode: typeof DOT | typeof EMPTY | null = null;
    private hasDragged = false;
    private pressedCell: { x: number; y: number } | null = null;

    mask: (Mask)[][];

    constructor(canvasSize: number) {
        this.canvasSize = canvasSize;
        this.board = `
4 4 4 5 5 5 6 6 6 6 
0 4 4 4 4 5 5 5 5 6 
0 0 0 0 0 5 9 9 9 6 
0 0 0 0 0 9 9 9 9 6 
0 0 0 0 0 9 9 7 7 6 
1 1 1 0 0 7 7 7 7 6 
3 1 8 8 0 7 7 7 7 6 
3 1 8 8 8 2 2 2 6 6 
3 3 3 8 2 2 2 2 2 2 
3 3 8 8 2 2 2 2 2 2 
`.trim().split('\n').map(x => x.trim().split(/\s+/).map(Number));
        /*
        1 1 1 1 1 1 11 11 11 11 11 11 11 11 11 11 11 11 11 11 
        1 1 1 1 1 1 1 11 11 11 11 11 11 11 11 11 11 11 11 11 
        1 1 1 1 1 1 1 1 1 1 11 11 11 13 17 11 11 11 11 11 
        1 1 1 1 1 1 1 1 1 11 11 2 13 13 13 11 11 11 11 11 
        1 1 1 1 1 1 1 1 1 1 1 2 2 2 13 13 11 11 11 11 
        8 8 1 1 1 1 1 1 1 1 2 2 2 13 13 13 11 11 11 10 
        8 1 1 1 1 1 1 1 1 1 2 2 2 2 13 14 14 10 10 10 
        8 1 1 1 1 1 1 1 1 1 2 2 2 2 13 13 14 10 10 10 
        8 8 1 1 1 1 1 1 1 1 2 2 2 2 13 19 19 19 10 10 
        8 8 5 5 1 1 1 1 12 12 12 12 12 19 19 19 19 19 19 10 
        4 5 5 5 5 5 12 12 12 12 12 12 12 19 19 19 19 19 19 19 
        9 9 3 3 3 3 12 12 12 12 12 12 19 19 19 19 19 19 19 19 
        9 9 3 3 3 12 12 12 12 12 12 12 19 19 19 19 19 19 19 19 
        9 9 9 3 3 3 12 12 12 12 12 12 12 19 19 19 19 19 19 19 
        15 9 15 3 3 3 12 12 7 6 6 18 18 18 19 19 19 19 19 19 
        15 15 15 15 15 18 18 7 7 18 18 18 18 18 19 19 19 19 0 0 
        15 15 15 15 18 18 18 7 18 18 18 18 18 18 19 19 0 0 0 0 
        15 15 15 15 15 18 18 18 18 18 16 18 18 19 19 0 0 0 0 0 
        15 15 15 15 18 18 18 18 18 18 18 18 18 0 0 0 0 0 0 0 
        15 15 15 15 18 18 18 18 18 18 18 0 0 0 0 0 0 0 0 0
        */
        /*
        25 25 25 25 23 23 15 15 15 15 24 24 24 24 24 24 9 9 9 9 9 9 9 9 9 9 9 
 25 25 25 25 15 15 15 15 15 24 24 24 8 24 13 13 9 9 9 9 9 9 9 9 9 9 9 
 25 25 25 25 25 25 25 15 15 15 15 8 8 24 13 13 13 13 9 9 9 9 9 9 9 9 9 
 25 25 25 25 25 25 25 15 15 13 13 13 13 13 13 13 13 13 9 9 9 9 9 9 9 9 3 
 25 25 25 25 25 25 15 15 15 15 13 13 13 13 13 13 13 13 13 9 9 9 9 9 3 3 3 
 25 25 25 25 25 25 25 15 15 13 13 13 13 13 13 13 13 13 13 9 9 9 9 9 9 9 9 
 25 25 25 25 25 25 0 0 15 0 13 13 13 13 13 13 13 13 9 9 9 9 9 9 9 9 9 
 25 25 25 25 25 25 0 0 0 0 13 13 13 13 13 13 18 18 18 9 9 6 6 9 6 6 9 
 25 25 5 25 25 25 0 0 0 0 13 13 18 13 13 13 18 18 18 9 9 9 6 6 6 6 6 
 25 25 25 25 25 25 0 0 0 0 13 13 18 18 18 18 18 18 18 9 6 9 6 6 6 6 6 
 25 25 25 25 25 25 25 25 25 0 18 18 18 18 18 18 18 18 18 18 6 6 6 6 6 6 6 
 25 25 25 25 25 25 25 25 25 0 0 18 18 18 18 18 18 18 18 18 6 6 6 6 6 6 6 
 25 25 25 25 25 25 25 1 1 1 1 1 1 18 21 11 11 18 18 18 18 6 6 6 6 6 6 
 12 25 25 25 12 12 12 1 1 1 1 1 1 1 21 21 21 21 18 6 6 6 6 6 6 6 6 
 12 22 22 22 12 12 12 12 12 1 1 1 1 1 21 21 21 21 21 6 6 6 6 6 6 6 6 
 12 12 12 12 12 12 1 1 1 1 1 1 21 21 21 21 21 21 21 6 6 6 6 6 6 6 6 
 2 12 12 12 12 12 12 1 1 1 1 1 1 21 21 21 21 21 21 21 21 6 20 20 6 6 6 
 2 2 12 12 12 12 12 1 1 1 1 1 1 21 21 21 21 21 21 20 21 20 20 20 20 6 6 
 12 12 12 12 12 12 12 16 16 1 1 1 1 1 21 21 21 21 20 20 20 20 20 20 20 20 20 
 12 12 12 12 12 12 12 12 16 1 1 1 1 1 1 17 17 21 20 20 20 20 20 20 20 19 19 
 12 12 12 12 12 12 12 12 16 1 1 1 1 17 17 17 17 17 17 20 20 20 20 20 20 20 19 
 12 12 12 12 12 12 12 12 1 1 1 17 17 17 17 17 17 20 20 20 20 20 20 14 14 14 19 
 12 12 12 26 26 12 12 12 1 1 17 17 17 17 17 17 17 20 20 14 20 20 20 14 14 14 19 
 12 12 12 12 12 12 12 17 17 17 17 17 17 17 17 17 17 20 14 14 20 20 14 14 14 19 19 
 12 10 12 12 12 12 12 17 17 17 17 17 17 17 17 17 17 17 17 14 14 14 14 7 14 14 14 
 12 10 12 12 12 17 17 17 17 17 17 17 17 17 17 4 4 17 14 14 14 14 14 14 14 14 14 
 12 12 17 17 17 17 17 17 17 17 17 17 17 17 17 4 14 14 14 14 14 14 14 14 14 14 14
        */
        const size = this.board.length;
        this.size = size;
        this.rectSize = (canvasSize - 2 * REGION_BORDER) / this.size;

        this.mask = Array.from({ length: size }, () => Array(size).fill(EMPTY));

        for (let i = 0; i < this.board.length; i++) {
            const hue = floor(map(i, 0, size, 0, 360));
            this.colors.push(color(`hsl(${hue}, 80%, 60%)`));
        }
    }

    draw() {
        push();
        translate(REGION_BORDER, REGION_BORDER);
        textSize(this.rectSize - 10);
        textAlign(CENTER, CENTER);
        for (let y = 0; y < this.mask.length; y++) {
            for (let x = 0; x < this.mask[0].length; x++) {
                const rx = this.rectSize * x;
                const ry = this.rectSize * y;

                const cellColor = this.colors[this.board[y][x]];
                const darkColor = lerpColor(cellColor, color(0), 0.2);

                noStroke();
                fill(darkColor);
                rect(rx, ry, this.rectSize, this.rectSize);
                strokeWeight(1);
                fill(cellColor);
                stroke(darkColor);
                rect(rx, ry, this.rectSize, this.rectSize, 4);

                const cell = this.mask[y][x];
                if (cell == BULL) {
                    fill(255);
                    circle(rx + this.rectSize / 2, ry + this.rectSize / 2, this.rectSize - 5);
                    text("🦀", rx + this.rectSize / 2, ry + this.rectSize / 2);
                }
            }
        }

        // border and dots (they need to be overlayed)
        strokeCap(ROUND);
        for (let y = 0; y < this.mask.length; y++) {
            for (let x = 0; x < this.mask[0].length; x++) {
                const rx = this.rectSize * x;
                const ry = this.rectSize * y;

                stroke(0);
                strokeWeight(REGION_BORDER);
                const regionId = this.board[y][x];

                // right
                if (x + 1 >= this.mask[0].length || this.board[y][x + 1] != regionId) {
                    line(rx + this.rectSize, ry, rx + this.rectSize, ry + this.rectSize);
                }

                // bottom
                if (y + 1 >= this.mask.length || this.board[y + 1][x] != regionId) {
                    line(rx, ry + this.rectSize, rx + this.rectSize, ry + this.rectSize);
                }

                // left
                if (x - 1 < 0 || this.board[y][x - 1] != regionId) {
                    line(rx, ry, rx, ry + this.rectSize);
                }

                // top
                if (y - 1 < 0 || this.board[y - 1][x] != regionId) {
                    line(rx, ry, rx + this.rectSize, ry);
                }

                noStroke();

                const cell = this.mask[y][x];
                const cellColor = this.colors[this.board[y][x]];
                if (cell == DOT) {
                    noStroke();
                    fill(0, 0, 0, 125);
                    rect(rx, ry, this.rectSize, this.rectSize);
                    fill(cellColor);
                    circle(rx + this.rectSize / 2, ry + this.rectSize / 2, 10);
                }
            }
        }
        pop();
    }

    checkBoard() {
        // todo: check for invalid bulls, check if it is all valid (do onComplete)
    }

    private cellAt(mx: number, my: number): { x: number; y: number } | null {
        const x = Math.floor((mx - REGION_BORDER) / (this.rectSize + PADDING / 2));
        const y = Math.floor((my - REGION_BORDER) / (this.rectSize + PADDING / 2));
        if (x >= 0 && x < this.mask[0].length && y >= 0 && y < this.mask.length) {
            return { x, y };
        }
        return null;
    }

    private copyMask(): Mask[][] {
        return this.mask.map(row => [...row]) as Mask[][];
    }

    private pushUndo(snapshot: Mask[][]) {
        this.undoStack.push(snapshot);
        if (this.undoStack.length > 50) this.undoStack.shift();
        this.redoStack = [];
    }

    undo() {
        if (!this.undoStack.length) return;
        this.redoStack.push(this.copyMask());
        this.mask = this.undoStack.pop()!;
    }

    redo() {
        if (!this.redoStack.length) return;
        this.undoStack.push(this.copyMask());
        this.mask = this.redoStack.pop()!;
    }

    clear() {
        this.pushUndo(this.copyMask());
        this.mask = Array.from({ length: this.size }, () => Array(this.size).fill(EMPTY));
    }

    /**
     * External function to add dots (from hint)
     * @todo extend to bulls too
     */
    addDots(dots: Point[]) {
        const snapshot = this.copyMask();
        for (const [y, x] of dots) {
            this.mask[y][x] = DOT;
        }
        this.pushUndo(snapshot);
    }

    /**
     * Set board with new puzzle
     */
    setCanvasSize(canvasSize: number) {
        this.canvasSize = canvasSize;
        this.rectSize = (canvasSize - 2 * REGION_BORDER) / this.size;
    }

    setBoard(board: number[][]) {
        this.board = board;
        const size = this.board.length;
        this.size = size;
        this.rectSize = (this.canvasSize - 2 * REGION_BORDER) / this.size;

        this.mask = Array.from({ length: size }, () => Array(size).fill(EMPTY));
        this.colors = [];
        this.undoStack = [];
        this.redoStack = [];

        for (let i = 0; i < this.board.length; i++) {
            const hue = floor(map(i, 0, size, 0, 360));
            this.colors.push(color(`hsl(${hue}, 80%, 60%)`));
        }

        this.onBoardChange();
    }

    mouseDragged() {
        const cell = this.cellAt(mouseX, mouseY);
        if (!cell) return;
        const { x, y } = cell;
        const pc = this.pressedCell;
        if (!this.hasDragged && pc && (cell.x != pc.x || cell.y != pc.y)) {
            this.hasDragged = true;
        }
        if (!this.hasDragged) return;
        this.applyDrag(x, y);
    }

    private applyDrag(x: number, y: number) {
        if (this.dragMode == DOT) {
            if (this.mask[y][x] == EMPTY) this.mask[y][x] = DOT;
        } else {
            if (this.mask[y][x] != BULL) this.mask[y][x] = EMPTY;
        }
    }

    mousePressed() {
        this.hasDragged = false;
        this.beforeSnapshot = this.copyMask();
        const cell = this.cellAt(mouseX, mouseY);
        this.pressedCell = cell;
        if (!cell) return;
        const { x, y } = cell;
        const cur = this.mask[y][x];
        this.dragMode = cur == DOT ? EMPTY : DOT;
        this.applyDrag(x, y); // immediate feedback; reverted on release if no drag
    }

    mouseReleased() {
        if (this.hasDragged) {
            this.pushUndo(this.beforeSnapshot!);
        } else if (this.pressedCell) {
            const { x, y } = this.pressedCell;
            const cur = this.beforeSnapshot![y][x];
            this.mask = this.beforeSnapshot!.map(row => [...row]) as Mask[][];
            if (cur == EMPTY) this.mask[y][x] = DOT;
            else if (cur == DOT) this.mask[y][x] = BULL;
            else this.mask[y][x] = EMPTY;
            this.pushUndo(this.beforeSnapshot!);
        }
        this.dragMode = null;
        this.checkBoard();
    }
}
