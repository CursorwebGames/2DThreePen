const RECT_SIZE = 50;
const PADDING = 1;

const REARRANGING_MODE = true; // rearrange the tiles
const MANUAL_BOARD = true; // manually enter in board regions

let pen;
let solutions;
let solutionMask;

// step-through 14 "mode"
let solutionIndex = 0;

// rearranging mode
let prevClick = [-1, -1];
let steps = 0;

function setup() {
    const c = createCanvas(500, 500);
    c.elt.addEventListener("contextmenu", e => e.preventDefault());
    pen = new BullPen();
    solutions = new Solver(pen).solveBoard();
    solutionMask = Solver.getSolutionMask(solutions);
    console.log(solutions.length)
}

function draw() {
    background('pink');
    const bulls = pen.bulls;
    strokeWeight(PADDING);
    stroke(50);

    textSize(20);
    textAlign(CENTER);

    for (let y = 0; y < bulls.length; y++) {
        for (let x = 0; x < bulls[0].length; x++) {
            const rx = (RECT_SIZE + PADDING / 2) * x;
            const ry = (RECT_SIZE + PADDING / 2) * y;

            fill(pen.colors[pen.board[y][x]] || 0);
            rect(rx, ry, RECT_SIZE, RECT_SIZE, 2);

            if (!REARRANGING_MODE) {
                if (pen.bulls[y][x] == BULL) {
                    fill(0);
                    circle(rx + RECT_SIZE / 2, ry + RECT_SIZE / 2, 30, 30);
                } else if (pen.bulls[y][x] == DOT) {
                    fill(123);
                    circle(rx + RECT_SIZE / 2, ry + RECT_SIZE / 2, 10, 10);
                }
            }

            push();
            strokeWeight(2);
            fill(255);
            text(solutionMask[y][x], rx + RECT_SIZE / 2, ry + RECT_SIZE / 2 + 10);
            pop();
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

        solutions = new Solver(pen).solveBoard();
        solutionMask = Solver.getSolutionMask(solutions);
        console.log('solutions:', solutions.length, 'steps:', steps);
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
    if (REARRANGING_MODE) {
        pen = new BullPen();
        solutions = new Solver(pen).solveBoard();
        console.log(solutions.length);
    }

    // pen.bulls = genSolutions.solutions[solutionIndex];
    // solutionIndex++;
    // solutionIndex %= genSolutions.solutions.length;
}

function mouseOver(x, y, w, h) {
    return mouseX >= x && mouseX <= x + w && mouseY >= y && mouseY <= y + h;
}