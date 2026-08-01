import "p5";
import { BullPen, REGION_BORDER, type BoardStatus } from "./bullpen";
import { Solver } from "./solver";
import { genSingle } from "./genpen";
import { DIFFICULTY } from "./config";
// import { crabIcon } from "./crabIcon";

let pen: BullPen;
let solver: Solver;
let currentSize = 8;

const MAX_CANVAS = 50 * 8 + 2 * REGION_BORDER;

const menuScreen = document.querySelector("#menu-screen") as HTMLDivElement;
const playScreen = document.querySelector("#play-screen") as HTMLDivElement;
const menuStatus = document.querySelector(".menu-status") as HTMLDivElement;
const sizeGrid = document.querySelector(".size-grid") as HTMLDivElement;
const statusEl = document.querySelector(".status") as HTMLDivElement;
const hintDesc = document.querySelector(".hint-desc") as HTMLDivElement;
const playMenu = document.querySelector(".play-menu") as HTMLDivElement;
const canvasParent = document.querySelector(".canvas") as HTMLDivElement;
const easyPlaceButton = document.querySelector(".easy-place") as HTMLButtonElement;

/** Builds the menu's size buttons from BOARD_SIZES (src/config.ts). */
function renderSizeButtons() {
    for (const { label, size } of DIFFICULTY) {
        const btn = document.createElement("button");
        btn.className = "size-btn";
        btn.innerHTML = `${label} (${size} &times; ${size})`;
        btn.addEventListener("click", () => startGame(size));
        sizeGrid.appendChild(btn);
    }
}

function getCanvasSize() {
    return Math.min(window.innerWidth - 32, window.innerHeight - 200, MAX_CANVAS);
}

function showMenu() {
    playScreen.classList.add("hidden");
    menuScreen.classList.remove("hidden");
    noLoop();
}

async function startGame(size: number) {
    currentSize = size;
    menuStatus.textContent = "Generating puzzle...";
    try {
        const board = await genSingle(size);
        pen.setBoard(board);
        hintDesc.textContent = "";
        menuStatus.textContent = "";
        menuScreen.classList.add("hidden");
        playScreen.classList.remove("hidden");
        playMenu.classList.add("hidden");
        loop();
        windowResized();
    } catch (e) {
        console.error(e);
        menuStatus.textContent = "Failed to generate puzzle, try again.";
    }
}

async function newPuzzle() {
    hintDesc.textContent = "Generating puzzle...";
    try {
        const board = await genSingle(currentSize);
        pen.setBoard(board);
        hintDesc.textContent = "";
        windowResized();
    } catch (e) {
        console.error(e);
        hintDesc.textContent = "Failed to generate puzzle, try again.";
    }
}

function describeViolations(violations: BoardStatus["violations"]): string {
    const parts: string[] = [];
    if (violations.rows.length) parts.push(`row ${violations.rows.map(r => r + 1).join(", ")} has too many crabs`);
    if (violations.cols.length) parts.push(`column ${violations.cols.map(c => c + 1).join(", ")} has too many crabs`);
    if (violations.regions.length) parts.push(`${violations.regions.length} pen(s) have too many crabs`);
    if (violations.adjacent.length) parts.push(`${violations.adjacent.length} crab(s) are touching`);
    return parts.join("; ");
}

function setStatus(status: BoardStatus) {
    statusEl.classList.remove("playing", "invalid", "won");
    statusEl.classList.add(status.state);
    if (status.state == "won") statusEl.textContent = "You win! 🦀";
    else if (status.state == "invalid") statusEl.textContent = `Invalid: ${describeViolations(status.violations)}`;
    else statusEl.textContent = "";
}

window.setup = () => {
    canvasParent.addEventListener("contextmenu", e => e.preventDefault());
    const size = getCanvasSize();
    createCanvas(size, size).parent(canvasParent);
    pen = new BullPen(size);
    pen.onBoardChange = () => { solver = new Solver(pen); };
    pen.onStatusChange = setStatus;
    solver = new Solver(pen);
    renderSizeButtons();
    showMenu();
};

window.windowResized = () => {
    if (!pen) return;
    const size = getCanvasSize();
    resizeCanvas(size, size);
    pen.setCanvasSize(size);
};

window.draw = () => {
    background(222);
    pen.draw();
};

window.mouseDragged = () => {
    if (playScreen.classList.contains("hidden")) return;
    pen.mouseDragged();
};

window.mousePressed = () => {
    if (playScreen.classList.contains("hidden")) return;
    pen.mousePressed();
};

window.mouseReleased = () => {
    if (playScreen.classList.contains("hidden")) return;
    pen.mouseReleased();
};

(document.querySelector(".back") as HTMLButtonElement).addEventListener("click", () => showMenu());

(document.querySelector(".settings-toggle") as HTMLButtonElement).addEventListener("click", () => {
    playMenu.classList.toggle("hidden");
});

(document.querySelector(".new-puzzle") as HTMLButtonElement).addEventListener("click", () => newPuzzle());

(document.querySelector(".undo") as HTMLButtonElement).addEventListener("click", () => pen.undo());
(document.querySelector(".redo") as HTMLButtonElement).addEventListener("click", () => pen.redo());
(document.querySelector(".clear") as HTMLButtonElement).addEventListener("click", () => pen.clear());

easyPlaceButton.addEventListener("click", () => {
    pen.toggleEasyPlaceMode();
    easyPlaceButton.textContent = pen.easyPlaceMode ? "Turn off Easy Place" : "Turn on Easy Place";
});

document.addEventListener("keydown", (e) => {
    if (playScreen.classList.contains("hidden")) return;
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
