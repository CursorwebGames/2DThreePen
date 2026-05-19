import "p5";
import { BullPen } from "./bullpen";
import { Solver } from "./solver";

let pen: BullPen;
let solver: Solver;

const hintDesc = document.querySelector(".hint-desc") as HTMLDivElement;

window.setup = () => {
    const parent = document.querySelector(".canvas")!;
    parent.addEventListener("contextmenu", e => e.preventDefault());
    createCanvas(50 * 8 + 4, 50 * 8 + 4).parent(parent);
    pen = new BullPen();
    solver = new Solver(pen);
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