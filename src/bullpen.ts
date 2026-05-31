import type { Point } from "./solver";

const BULL = 0;
const EMPTY = -1;
const DOT = 1;

// const RECT_SIZE = 50;
const PADDING = 1;

const MOBILE = true;

type Mask = typeof BULL | typeof EMPTY | typeof DOT;

export class BullPen {
    board: number[][];
    colors: p5.Color[] = [];
    size: number;
    rectSize: number;

    onBoardChange!: () => void;

    private undoStack: Mask[][][] = [];
    private redoStack: Mask[][][] = [];
    private _beforeSnapshot: Mask[][] | null = null;

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
        this.rectSize = 50 * 8 / this.size;

        this.mask = Array.from({ length: size }, () => Array(size).fill(EMPTY));

        for (let i = 0; i < this.board.length; i++) {
            const hue = floor(map(i, 0, size, 0, 360));
            this.colors.push(color(`hsl(${hue}, 80%, 60%)`));
        }
    }

    draw() {
        noStroke();
        textSize(this.rectSize - 10);
        textAlign(CENTER, CENTER);
        for (let y = 0; y < this.mask.length; y++) {
            for (let x = 0; x < this.mask[0].length; x++) {
                const rx = (this.rectSize + PADDING / 2) * x;
                const ry = (this.rectSize + PADDING / 2) * y;

                const cellColor = this.colors[this.board[y][x]];

                fill(cellColor);
                rect(rx, ry, this.rectSize, this.rectSize, 2);

                const cell = this.mask[y][x];
                if (cell == BULL) {
                    fill(255);
                    circle(rx + this.rectSize / 2, ry + this.rectSize / 2, 45);
                    text("🦀", rx + this.rectSize / 2, ry + this.rectSize / 2);
                } else if (cell == DOT) {
                    noStroke();
                    fill(0, 0, 0, 120);
                    rect(rx, ry, this.rectSize, this.rectSize, 2);
                    fill(cellColor);
                    circle(rx + this.rectSize / 2, ry + this.rectSize / 2, 10);
                }
            }
        }

    }

    private cellAt(mx: number, my: number): { x: number; y: number } | null {
        const x = Math.floor(mx / (this.rectSize + PADDING / 2));
        const y = Math.floor(my / (this.rectSize + PADDING / 2));
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

    addDots(dots: Point[]) {
        const snapshot = this.copyMask();
        for (const [y, x] of dots) {
            this.mask[y][x] = DOT;
        }
        this.pushUndo(snapshot);
    }

    setBoard(board: number[][]) {
        this.board = board;
        const size = this.board.length;
        this.size = size;
        this.rectSize = 50 * 8 / this.size;

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
        if (!MOBILE && !mouseButton.right) return;
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
        this._beforeSnapshot = this.copyMask();
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
            this.pushUndo(this._beforeSnapshot!);
        } else if (this.pressedCell) {
            const { x, y } = this.pressedCell;
            const cur = this._beforeSnapshot![y][x];
            this.mask = this._beforeSnapshot!.map(row => [...row]) as Mask[][];
            if (MOBILE) {
                if (cur == EMPTY) this.mask[y][x] = DOT;
                else if (cur == DOT) this.mask[y][x] = BULL;
                else this.mask[y][x] = EMPTY;
            } else if (mouseButton.left) {
                if (cur == EMPTY) this.mask[y][x] = DOT;
                else if (cur == DOT) this.mask[y][x] = BULL;
                else this.mask[y][x] = EMPTY;
            } else if (mouseButton.right) {
                this.mask[y][x] = cur == BULL ? EMPTY : BULL;
            }
            this.pushUndo(this._beforeSnapshot!);
        }
        this.dragMode = null;
    }
}
