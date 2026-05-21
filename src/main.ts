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
    pen.onBoardChange = () => { solver = new Solver(pen); };
    solver = new Solver(pen);
    gen = new PenGenerator(6);
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