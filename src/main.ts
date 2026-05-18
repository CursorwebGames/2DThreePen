import "p5";
import { BullPen } from "./bullpen";

let pen: BullPen;

window.setup = () => {
    const canvas = document.querySelector("canvas")!;
    canvas.addEventListener("contextmenu", e => e.preventDefault());
    createCanvas(50 * 8 + 4, 50 * 8 + 4, canvas);
    pen = new BullPen();
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