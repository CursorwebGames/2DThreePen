const RECT_SIZE = 50;
const PADDING = 1;

const REARRANGING_MODE = false; // rearrange the tiles
const MANUAL_BOARD = true; // manually enter in board regions

let pen;
/** @type {{ solutions: any, solutionMask: any } & Solver} */
let solver;

// step-through 14 "mode"
let solutionIndex = 0;

// rearranging mode
let prevClick = [-1, -1];
let steps = 0;

function setup() {
    const c = createCanvas(500, 500);
    c.elt.addEventListener("contextmenu", e => e.preventDefault());
    pen = new BullPen();
    solver = new Solver(pen);
    console.log(solver.solutions.length);
}

function getEliminatedCells(hy, hx) {
    const eliminated = new Set();
    const regionColor = pen.board[hy][hx];

    for (let y = 0; y < SIZE; y++) {
        for (let x = 0; x < SIZE; x++) {
            if (y == hy && x == hx) continue;
            const sameRow = y == hy;
            const sameCol = x == hx;
            const adjacent = Math.abs(y - hy) <= 1 && Math.abs(x - hx) <= 1;
            const sameRegion = pen.board[y][x] == regionColor;
            if (sameRow || sameCol || adjacent || sameRegion) {
                eliminated.add(`${y},${x}`);
            }
        }
    }

    return eliminated;
}

function draw() {
    background('pink');
    const bulls = pen.bulls;
    strokeWeight(PADDING);
    stroke(50);

    textSize(20);
    textAlign(CENTER);

    // find hovered empty cell for preview
    let hoverEliminated = null;
    let hoverCell = null;
    if (!REARRANGING_MODE) {
        for (let y = 0; y < bulls.length; y++) {
            for (let x = 0; x < bulls[0].length; x++) {
                const rx = (RECT_SIZE + PADDING / 2) * x;
                const ry = (RECT_SIZE + PADDING / 2) * y;
                if (mouseOver(rx, ry, RECT_SIZE, RECT_SIZE) && bulls[y][x] == EMPTY) {
                    hoverCell = [y, x];
                    hoverEliminated = getEliminatedCells(y, x);
                }
            }
        }
    }

    for (let y = 0; y < bulls.length; y++) {
        for (let x = 0; x < bulls[0].length; x++) {
            const rx = (RECT_SIZE + PADDING / 2) * x;
            const ry = (RECT_SIZE + PADDING / 2) * y;

            fill(pen.colors[pen.board[y][x]] || 0);
            rect(rx, ry, RECT_SIZE, RECT_SIZE, 2);

            if (REARRANGING_MODE) {
                if (solver.onesColor.has(pen.board[y][x])) {
                    line(rx, ry, rx + RECT_SIZE, ry + RECT_SIZE);
                }

                push();
                strokeWeight(2);
                fill(255);
                text(solver.solutionMask[y][x], rx + RECT_SIZE / 2, ry + RECT_SIZE / 2 + 10);
                pop();
            }

            if (!REARRANGING_MODE) {
                // draw hover preview overlay
                if (hoverEliminated && hoverEliminated.has(`${y},${x}`) && bulls[y][x] == EMPTY) {
                    noStroke();
                    fill(0, 0, 0, 120);
                    rect(rx, ry, RECT_SIZE, RECT_SIZE, 2);
                    stroke(50);
                    strokeWeight(PADDING);
                }

                if (hoverCell && y == hoverCell[0] && x == hoverCell[1]) {
                    noStroke();
                    fill(0, 180);
                    circle(rx + RECT_SIZE / 2, ry + RECT_SIZE / 2, 30, 30);
                    stroke(50);
                    strokeWeight(PADDING);
                } else if (pen.bulls[y][x] == BULL) {
                    fill(0);
                    circle(rx + RECT_SIZE / 2, ry + RECT_SIZE / 2, 30, 30);
                } else if (pen.bulls[y][x] == DOT) {
                    fill(0, 0, 0, 120);
                    rect(rx, ry, RECT_SIZE, RECT_SIZE, 2);
                    fill(123);
                    circle(rx + RECT_SIZE / 2, ry + RECT_SIZE / 2, 10, 10);
                }
            }

        }
    }
}

function mousePressed() {
    const bulls = pen.bulls;

    if (REARRANGING_MODE) {
        let currClick;

        for (let y = 0; y < bulls.length; y++) {
            for (let x = 0; x < bulls[0].length; x++) {
                const rx = (RECT_SIZE + PADDING / 2) * x;
                const ry = (RECT_SIZE + PADDING / 2) * y;
                if (mouseOver(rx, ry, RECT_SIZE, RECT_SIZE)) {
                    if (mouseButton == LEFT) {
                        pen.board[y][x]++;
                    } else {
                        pen.board[y][x]--;
                        pen.board[y][x] += SIZE;
                    }

                    currClick = [y, x];

                    pen.board[y][x] %= SIZE;
                }
            }
        }

        if (currClick && prevClick.toString() != currClick.toString()) {
            steps++;
        }

        if (currClick) {
            prevClick = currClick;
        }

        solver.solve();
        console.log('solutions:', solver.solutions.length, 'steps:', steps);
    } else {
        for (let y = 0; y < bulls.length; y++) {
            for (let x = 0; x < bulls[0].length; x++) {
                const rx = (RECT_SIZE + PADDING / 2) * x;
                const ry = (RECT_SIZE + PADDING / 2) * y;
                if (mouseOver(rx, ry, RECT_SIZE, RECT_SIZE) && mouseButton == LEFT) {
                    if (pen.bulls[y][x] != BULL) {
                        pen.bulls[y][x] = BULL;
                    } else {
                        pen.bulls[y][x] = EMPTY;
                    }
                }

                if (mouseOver(rx, ry, RECT_SIZE, RECT_SIZE) && mouseButton == RIGHT) {
                    if (pen.bulls[y][x] != DOT) {
                        pen.bulls[y][x] = DOT;
                    } else {
                        pen.bulls[y][x] = EMPTY;
                    }
                }
            }
        }
    }
}

function keyPressed() {
    // if (REARRANGING_MODE) {
    //     pen = new BullPen();
    //     solver = new Solver(pen);
    //     solver.solve();
    //     console.log(solver.solutions.length);
    // }

    // pen.bulls = genSolutions.solutions[solutionIndex];
    // solutionIndex++;
    // solutionIndex %= genSolutions.solutions.length;
}

function mouseOver(x, y, w, h) {
    return mouseX >= x && mouseX <= x + w && mouseY >= y && mouseY <= y + h;
}