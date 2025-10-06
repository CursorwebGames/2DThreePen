const RECT_SIZE = 50;
const PADDING = 2;

let pen;
let solutions;
let solutionIndex = 0;

function setup() {
    const c = createCanvas(500, 500);
    c.elt.addEventListener("contextmenu", e => e.preventDefault());
    pen = new BullPen();
    solutions = new Solver(pen).solveBoard();
    console.log(solutions.length)
}

function draw() {
    background('pink')
    const bulls = pen.bulls;
    strokeWeight(2);

    for (let y = 0; y < bulls.length; y++) {
        for (let x = 0; x < bulls[0].length; x++) {
            const rx = (RECT_SIZE + PADDING / 2) * x;
            const ry = (RECT_SIZE + PADDING / 2) * y;
            if (mouseOver(rx, ry, RECT_SIZE, RECT_SIZE)) {
                fill(pen.colors[pen.board[y][x]] || 0);
            } else {
                fill(pen.colors[pen.board[y][x]] || 0);
            }

            rect(rx, ry, RECT_SIZE, RECT_SIZE);
            if (pen.bulls[y][x] == BULL) {
                fill(0);
                circle(rx + RECT_SIZE / 2, ry + RECT_SIZE / 2, 30, 30);
            } else if (pen.bulls[y][x] == DOT) {
                fill(123);
                circle(rx + RECT_SIZE / 2, ry + RECT_SIZE / 2, 10, 10);
            }
        }
    }
}

function mousePressed() {
    const bulls = pen.bulls;

    // -----

    // for (let y = 0; y < bulls.length; y++) {
    //     for (let x = 0; x < bulls[0].length; x++) {
    //         const rx = (RECT_SIZE + PADDING / 2) * x;
    //         const ry = (RECT_SIZE + PADDING / 2) * y;
    //         if (mouseOver(rx, ry, RECT_SIZE, RECT_SIZE) && mouseButton == LEFT) {
    //             pen.board[y][x] = 0;
    //         }
    //     }
    // }


    // solutions = new Solver(pen).solveBoard();
    // console.log(solutions.length)
    // return;

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

function keyPressed() {
    pen = new BullPen();
    solutions = new Solver(pen).solveBoard();
    console.log(solutions.length)
}

function mouseOver(x, y, w, h) {
    return mouseX >= x && mouseX <= x + w && mouseY >= y && mouseY <= y + h;
}