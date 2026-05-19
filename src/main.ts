import "p5";
import { BullPen } from "./bullpen";
import { Solver } from "./solver";
import { PenGenerator } from "./genpen";

let pen: BullPen;
let solver: Solver;
let gen: PenGenerator;

const hintDesc = document.querySelector(".hint-desc") as HTMLDivElement;

window.setup = () => {
    const parent = document.querySelector(".canvas")!;
    parent.addEventListener("contextmenu", e => e.preventDefault());
    createCanvas(50 * 8 + 4, 50 * 8 + 4).parent(parent);
    pen = new BullPen();
    solver = new Solver(pen);
    gen = new PenGenerator(8);
};

window.draw = () => {
    background(0);
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

window.mouseClicked = () => {
    pen.mouseClicked();
};

(document.querySelector(".hint") as HTMLButtonElement).addEventListener("click", () => {
    let x = solver.nextHint();
    if (!x) return;
    const { hint, dots } = x;
    hintDesc.textContent = hint;
    pen.addDots(dots);
});

(document.querySelector(".gen") as HTMLButtonElement).addEventListener("click", () => {
    let board: number[][] | null = null;
    while (!board) {
        board = gen.generate();
    }
    pen.setBoard(board);
    solver = new Solver(pen);
});