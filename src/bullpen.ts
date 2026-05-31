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

    constructor() {
        this.board = `
6 6 6 4 4 4 0 0
3 6 6 6 3 4 4 0
3 3 3 3 3 4 4 0
3 3 5 5 3 2 2 2
5 5 5 5 5 2 2 2
7 7 7 7 5 1 1 1
7 7 7 7 1 1 1 1
7 7 7 7 1 1 1 1`.trim().split('\n').map(x => x.split(' ').map(Number));
        const size = this.board.length;
        this.size = size;
        this.rectSize = (50 * 8) / this.size;

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
                    circle(rx + this.rectSize / 2, ry + this.rectSize / 2, 45);
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
    setBoard(board: number[][]) {
        this.board = board;
        const size = this.board.length;
        this.size = size;
        this.rectSize = (50 * 8 - 2 * REGION_BORDER) / this.size;

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
