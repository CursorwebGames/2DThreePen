import type { BullPen } from "./bullpen";

/** Point is [y, x] */
export type Point = [number, number];
type Region = Point[];

function hashPt(y: number, x: number): string {
    return `${y},${x}`;
}

function combinations(arr: number[], k: number): number[][] {
    if (k == 0) return [[]];
    if (arr.length < k) return [];
    const [first, ...rest] = arr;
    return [
        ...combinations(rest, k - 1).map(c => [first, ...c]),
        ...combinations(rest, k),
    ];
}

export class Solver {
    board: number[][];
    size: number;
    mask: Set<string>;
    regions: Region[];
    hadToGuess = false;

    private solution: Point[] = [];
    hintFns = [this.singlePen, this.oneDirection, this.penOverlap, this.overcounting];
    hintDesc = [
        "There is only one available spot in this pen",
        "All other cells would make it so there cannot be any bulls here",
        "Placing a bull here would make it so there cannot be any bulls here",
        "These regions encompass the entirety of these rows/cols"
    ];

    constructor(pen: BullPen) {
        this.board = pen.board;
        this.size = pen.size;
        this.mask = new Set();
        this.regions = this.getRegions();
    }

    solve(): Point[] {
        if (this._solve()) {
            this.solution = this.regions.filter(ps => ps.length > 0).map(ps => ps[0]);
            return this.solution;
        }
        return [];
    }

    nextHint(): { hint: string, dots: Point[] } | null {
        // TODO: check if the user made any mistakes

        let prevSize = this.mask.size;
        const oldMask = new Set(this.mask);

        for (let c = 0; c < this.hintFns.length; c++) {
            this.hintFns[c].bind(this)();
            if (prevSize != this.mask.size) {
                // this hint did something!
                const newPts: Point[] = [...this.mask.difference(oldMask)]
                    .map(k => k.split(',').map(Number) as Point);
                return { hint: this.hintDesc[c], dots: newPts };
            }
        }

        // just solve it, and then see...
        return null;
    }

    private _solve(): boolean {
        let prevSize = -1;
        while (this.mask.size != prevSize) {
            prevSize = this.mask.size;
            this.singlePen();
            this.oneDirection();
            this.penOverlap();
            this.overcounting();
            if (this.regions.some(ps => ps.length == 0)) return false;
            if (this.mask.size == this.size ** 2 - this.size) return true;
        }

        this.hadToGuess = true;
        const target = this.regions
            .filter(ps => ps.length >= 2)
            .reduce((a, b) => (a.length <= b.length ? a : b));

        for (const [y, x] of [...target]) {
            const savedMask = new Set(this.mask);
            const savedPensets = this.regions.map(ps => [...ps]);

            for (const [cy, cx] of [...target]) {
                if (cy != y || cx != x) this.mask.add(hashPt(cy, cx));
            }
            for (const [ay, ax] of this.getAdjacent(y, x)) {
                this.mask.add(hashPt(ay, ax));
            }
            this.readjustRegions();

            if (this._solve()) return true;

            this.mask.clear();
            for (const k of savedMask) this.mask.add(k);
            for (let i = 0; i < this.regions.length; i++) {
                this.regions[i].length = 0;
                this.regions[i].push(...savedPensets[i]);
            }
        }

        return false;
    }

    private singlePen() {
        for (const penset of this.regions) {
            if (penset.length == 1) {
                for (const [ay, ax] of this.getAdjacent(penset[0][0], penset[0][1])) {
                    this.mask.add(hashPt(ay, ax));
                }
            }
        }
        this.readjustRegions();
    }

    private oneDirection() {
        for (const penset of this.regions) {
            if (!penset.length) continue;
            const vert = this.pensetAllVert(penset);
            let changed = false;
            if (vert != null) {
                const [x, color] = vert;
                for (let y = 0; y < this.size; y++) {
                    if (this.board[y][x] != color) {
                        this.mask.add(hashPt(y, x));
                        changed = true;
                    }
                }
            }

            if (changed) {
                break;
            }

            const horiz = this.pensetAllHoriz(penset);
            if (horiz != null) {
                const [y, color] = horiz;
                for (let x = 0; x < this.size; x++) {
                    if (this.board[y][x] != color) {
                        this.mask.add(hashPt(y, x));
                    }
                }
            }
        }
        this.readjustRegions();
    }

    private penOverlap() {
        for (const penset of this.regions) {
            if (!penset.length || penset.length > 8) continue;
            let intersect = new Set(
                this.getAdjacent(penset[0][0], penset[0][1]).map(([ay, ax]) => hashPt(ay, ax))
            );
            for (const [y, x] of penset) {
                const adj = new Set(this.getAdjacent(y, x).map(([ay, ax]) => hashPt(ay, ax)));
                for (const k of [...intersect]) {
                    if (!adj.has(k)) intersect.delete(k);
                }
            }

            for (const k of intersect) this.mask.add(k);
        }
        this.readjustRegions();
    }

    private overcounting() {
        const SIZE = this.size;
        const rowRegions: Set<number>[] = Array.from({ length: SIZE }, () => new Set());
        const colRegions: Set<number>[] = Array.from({ length: SIZE }, () => new Set());
        const colorToPs = new Map<number, Region>();

        for (const ps of this.regions) {
            if (!ps.length) continue;
            const color = this.board[ps[0][0]][ps[0][1]];
            colorToPs.set(color, ps);
            for (const [y, x] of ps) {
                rowRegions[y].add(color);
                colRegions[x].add(color);
            }
        }

        const indices = Array.from({ length: SIZE }, (_, i) => i);
        for (let k = 1; k < SIZE; k++) {
            let changed = false;
            for (const rowSubset of combinations(indices, k)) {
                const regions = new Set<number>();
                let tooMany = false;
                for (const y of rowSubset) {
                    for (const c of rowRegions[y]) regions.add(c);
                    if (regions.size > k) { tooMany = true; break; }
                }
                if (tooMany || regions.size != k) continue;
                for (const color of regions) {
                    for (const [y, x] of colorToPs.get(color)!) {
                        if (!rowSubset.includes(y)) {
                            this.mask.add(hashPt(y, x));
                            changed = true;
                        }
                    }
                }
            }

            if (changed) {
                break;
            }

            for (const colSubset of combinations(indices, k)) {
                const regions = new Set<number>();
                let tooMany = false;
                for (const x of colSubset) {
                    for (const c of colRegions[x]) regions.add(c);
                    if (regions.size > k) { tooMany = true; break; }
                }
                if (tooMany || regions.size != k) continue;
                for (const color of regions) {
                    for (const [y, x] of colorToPs.get(color)!) {
                        if (!colSubset.includes(x)) this.mask.add(hashPt(y, x));
                    }
                }
            }
        }

        this.readjustRegions();
    }

    private readjustRegions() {
        for (const penset of this.regions) {
            for (let i = penset.length - 1; i >= 0; i--) {
                if (this.mask.has(hashPt(penset[i][0], penset[i][1]))) {
                    penset.splice(i, 1);
                }
            }
        }
    }

    private getRegions(): Region[] {
        const penSets: Region[] = Array.from({ length: this.size }, () => []);
        for (let row = 0; row < this.size; row++) {
            for (let col = 0; col < this.size; col++) {
                penSets[this.board[row][col]].push([row, col]);
            }
        }
        return penSets;
    }

    private getAdjacent(y: number, x: number): Point[] {
        const out: Point[] = [];
        for (let dy = -1; dy <= 1; dy++) {
            for (let dx = -1; dx <= 1; dx++) {
                if (dy == 0 && dx == 0) continue;
                const ny = y + dy, nx = x + dx;
                if (ny >= 0 && ny < this.size && nx >= 0 && nx < this.size) {
                    out.push([ny, nx]);
                }
            }
        }
        return out;
    }

    private pensetAllHoriz(penset: Region): [number, number] | null {
        const [ypos, _] = penset[0];
        const color = this.board[ypos][penset[0][1]];
        for (let i = 1; i < penset.length; i++) {
            if (penset[i][0] != ypos) return null;
        }
        return [ypos, color];
    }

    private pensetAllVert(penset: Region): [number, number] | null {
        const [_, xpos] = penset[0];
        const color = this.board[penset[0][0]][xpos];
        for (let i = 1; i < penset.length; i++) {
            if (penset[i][1] != xpos) return null;
        }
        return [xpos, color];
    }
}
