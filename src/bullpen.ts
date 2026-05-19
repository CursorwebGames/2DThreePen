import type { Point } from "./solver";

const BULL = 0;
const EMPTY = -1;
const DOT = 1;

const RECT_SIZE = 50;
const PADDING = 1;

const MOBILE = true;

export class BullPen {
    board: number[][];
    colors: p5.Color[] = [];
    size: number;

    private dragMode: typeof DOT | typeof EMPTY | null = null;
    private hasDragged = false;
    private pressedButton: 'left' | 'right' | null = null;

    /** True: there is a dot */
    mask: (typeof BULL | typeof EMPTY | typeof DOT)[][];

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

        this.mask = Array.from({ length: size }, () => Array(size).fill(EMPTY));

        for (let i = 0; i < this.board.length; i++) {
            const hue = floor(map(i, 0, size, 0, 360));
            this.colors.push(color(`hsl(${hue}, 80%, 60%)`));
        }
    }

    draw() {
        noStroke();
        textSize(RECT_SIZE - 10);
        textAlign(CENTER, CENTER);
        for (let y = 0; y < this.mask.length; y++) {
            for (let x = 0; x < this.mask[0].length; x++) {
                const rx = (RECT_SIZE + PADDING / 2) * x;
                const ry = (RECT_SIZE + PADDING / 2) * y;

                const cellColor = this.colors[this.board[y][x]];

                fill(cellColor);
                rect(rx, ry, RECT_SIZE, RECT_SIZE, 2);

                const cell = this.mask[y][x];
                if (cell == BULL) {
                    fill(255);
                    circle(rx + RECT_SIZE / 2, ry + RECT_SIZE / 2, 45);
                    text("🦀", rx + RECT_SIZE / 2, ry + RECT_SIZE / 2);
                } else if (cell == DOT) {
                    noStroke();
                    fill(0, 0, 0, 120);
                    rect(rx, ry, RECT_SIZE, RECT_SIZE, 2);
                    fill(cellColor);
                    circle(rx + RECT_SIZE / 2, ry + RECT_SIZE / 2, 10);
                }
            }
        }

    }

    private cellAt(mx: number, my: number): { x: number; y: number } | null {
        const x = Math.floor(mx / (RECT_SIZE + PADDING / 2));
        const y = Math.floor(my / (RECT_SIZE + PADDING / 2));
        if (x >= 0 && x < this.mask[0].length && y >= 0 && y < this.mask.length) {
            return { x, y };
        }
        return null;
    }

    addDots(dots: Point[]) {
        for (const [y, x] of dots) {
            this.mask[y][x] = DOT;
        }
    }

    setBoard(board: number[][]) {
        this.board = board;
        const size = this.board.length;
        this.size = size;

        this.mask = Array.from({ length: size }, () => Array(size).fill(EMPTY));
        this.colors = [];

        for (let i = 0; i < this.board.length; i++) {
            const hue = floor(map(i, 0, size, 0, 360));
            this.colors.push(color(`hsl(${hue}, 80%, 60%)`));
        }
    }

    mouseDragged() {
        if (!MOBILE && this.pressedButton != 'right') return;
        this.hasDragged = true;
        const cell = this.cellAt(mouseX, mouseY);
        if (!cell) return;
        const { x, y } = cell;

        if (this.dragMode == null) {
            this.dragMode = this.mask[y][x] == DOT ? EMPTY : DOT;
        }

        if (this.dragMode == DOT) {
            if (this.mask[y][x] == EMPTY) {
                this.mask[y][x] = DOT;
            }
        } else { // dragMode == EMPTY
            if (this.mask[y][x] != BULL) {
                this.mask[y][x] = EMPTY;
            }
        }
    }

    mousePressed() {
        this.hasDragged = false;
        this.dragMode = null;
        this.pressedButton = mouseButton.left ? 'left' : mouseButton.right ? 'right' : null;
    }

    mouseReleased() {
        this.dragMode = null;
    }

    mouseClicked() {
        if (!MOBILE && this.pressedButton != 'left') return;
        if (this.hasDragged) return;
        const cell = this.cellAt(mouseX, mouseY);
        if (!cell) return;
        const { x, y } = cell;
        const cur = this.mask[y][x];
        if (MOBILE) {
            if (cur == EMPTY) this.mask[y][x] = DOT;
            else if (cur == DOT) this.mask[y][x] = BULL;
            else this.mask[y][x] = EMPTY;
        } else {
            this.mask[y][x] = cur == BULL ? EMPTY : BULL;
        }
    }
}
