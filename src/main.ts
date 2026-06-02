import "p5";
import { BullPen, REGION_BORDER } from "./bullpen";
import { Solver } from "./solver";
import { PenGenerator } from "./genpen";

import init, { sum } from "../wasm/pkg";

const wasmReady = init();

let pen: BullPen;
let solver: Solver;
let gen: PenGenerator;

const hintDesc = document.querySelector(".hint-desc") as HTMLDivElement;

const MAX_CANVAS = 50 * 8 + 2 * REGION_BORDER;
function getCanvasSize() {
    return Math.min(window.innerWidth - 32, window.innerHeight - 32, MAX_CANVAS);
}

window.setup = () => {
    const parent = document.querySelector(".canvas")!;
    parent.addEventListener("contextmenu", e => e.preventDefault());
    const size = getCanvasSize();
    createCanvas(size, size).parent(parent);
    pen = new BullPen(size);
    pen.onBoardChange = () => { solver = new Solver(pen); };
    solver = new Solver(pen);
    gen = new PenGenerator(6);
    wasmReady.then(() => {
        console.log(sum(6, 7));
    });
};

window.windowResized = () => {
    const size = getCanvasSize();
    resizeCanvas(size, size);
    pen.setCanvasSize(size);
};

window.draw = () => {
    background(222);
    pen.draw();
};

window.mouseDragged = () => {
    pen.mouseDragged();
};

window.mousePressed = () => {
    pen.mousePressed();
};

window.mouseReleased = () => {
    pen.mouseReleased();
};

(document.querySelector(".undo") as HTMLButtonElement).addEventListener("click", () => pen.undo());
(document.querySelector(".redo") as HTMLButtonElement).addEventListener("click", () => pen.redo());
(document.querySelector(".clear") as HTMLButtonElement).addEventListener("click", () => pen.clear());

document.addEventListener("keydown", (e) => {
    if (e.ctrlKey && e.key == "z" && !e.shiftKey) { e.preventDefault(); pen.undo(); }
    else if (e.ctrlKey && (e.key == "y" || e.key == "Z")) { e.preventDefault(); pen.redo(); }
});

(document.querySelector(".hint") as HTMLButtonElement).addEventListener("click", () => {
    let x = solver.nextHint();
    if (!x) return;
    const { hint, dots } = x;
    hintDesc.textContent = hint;
    pen.addDots(dots);
});

(document.querySelector(".export") as HTMLButtonElement).addEventListener("click", () => {
    console.log(JSON.stringify(pen.board));
});

(document.querySelector(".gen") as HTMLButtonElement).addEventListener("click", () => {
    let board: number[][] | null = null;
    // while (!board) {
    board = gen.generate();
    // }
    if (!board) {
        console.log('failed, try again');
        return;
    }
    pen.setBoard(board);
});