type Board = number[][];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function shuffle<T>(arr: T[]): T[] {
    for (let i = arr.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [arr[i], arr[j]] = [arr[j], arr[i]];
    }
    return arr;
}

function randomSample<T>(arr: T[], k: number): T[] {
    const copy = [...arr];
    shuffle(copy);
    return copy.slice(0, k);
}

// ---------------------------------------------------------------------------
// Min-heap
// ---------------------------------------------------------------------------

type HeapEntry = [number, number, number, Board]; // [f, g, ctr, board]

function heapPush(heap: HeapEntry[], entry: HeapEntry): void {
    heap.push(entry);
    let i = heap.length - 1;
    while (i > 0) {
        const parent = (i - 1) >> 1;
        if (heap[parent][0] <= heap[i][0]) break;
        [heap[parent], heap[i]] = [heap[i], heap[parent]];
        i = parent;
    }
}

function heapPop(heap: HeapEntry[]): HeapEntry {
    const top = heap[0];
    const last = heap.pop()!;
    if (heap.length) {
        heap[0] = last;
        let i = 0;
        while (true) {
            const l = 2 * i + 1, r = 2 * i + 2;
            let smallest = i;
            if (l < heap.length && heap[l][0] < heap[smallest][0]) smallest = l;
            if (r < heap.length && heap[r][0] < heap[smallest][0]) smallest = r;
            if (smallest == i) break;
            [heap[i], heap[smallest]] = [heap[smallest], heap[i]];
            i = smallest;
        }
    }
    return top;
}

// ---------------------------------------------------------------------------
// PenGen
// ---------------------------------------------------------------------------

export class PenGenerator {
    size: number;

    constructor(size: number) {
        this.size = size;
    }

    generate(maxAttempts = 5): Board | null {
        for (let attempt = 0; attempt < maxAttempts; attempt++) {
            const board = this.randomFloodFill();
            const n = this.countSolutions(board);
            if (n == 1) return board;
            const result = this.aStar(board, 20);
            if (result != null) return result;
        }
        return null;
    }

    private randomFloodFill(): Board {
        const { size } = this;
        const board: Board = Array.from({ length: size }, () => Array(size).fill(-1));

        const allCells: [number, number][] = [];
        for (let y = 0; y < size; y++)
            for (let x = 0; x < size; x++)
                allCells.push([y, x]);

        const seeds = randomSample(allCells, size);
        const queues: [number, number][][] = [];

        for (let region = 0; region < seeds.length; region++) {
            const [y, x] = seeds[region];
            board[y][x] = region;
            const q: [number, number][] = [];
            for (const [ny, nx] of shuffle([[y - 1, x], [y + 1, x], [y, x - 1], [y, x + 1]] as [number, number][]))
                if (ny >= 0 && ny < size && nx >= 0 && nx < size)
                    q.push([ny, nx]);
            queues.push(q);
        }

        let changed = true;
        while (changed) {
            changed = false;
            for (let region = 0; region < queues.length; region++) {
                const q = queues[region];
                while (q.length) {
                    const [ny, nx] = q.shift()!;
                    if (board[ny][nx] != -1) continue;
                    board[ny][nx] = region;
                    changed = true;
                    for (const [nny, nnx] of shuffle([[ny - 1, nx], [ny + 1, nx], [ny, nx - 1], [ny, nx + 1]] as [number, number][]))
                        if (nny >= 0 && nny < size && nnx >= 0 && nnx < size && board[nny][nnx] == -1)
                            q.push([nny, nnx]);
                    break;
                }
            }
        }

        return board;
    }

    private countSolutions(board: Board, limit = 10): number {
        const { size } = this;
        const regions: [number, number][][] = Array.from({ length: size }, () => []);
        for (let y = 0; y < size; y++)
            for (let x = 0; x < size; x++)
                regions[board[y][x]].push([y, x]);

        let found = 0;
        const usedRows = new Set<number>();
        const usedCols = new Set<number>();
        const forbidden = new Set<string>();

        const bt = (ri: number): void => {
            if (found >= limit) return;
            if (ri == size) { found++; return; }
            for (const [y, x] of regions[ri]) {
                if (usedRows.has(y) || usedCols.has(x) || forbidden.has(`${y},${x}`)) continue;
                usedRows.add(y);
                usedCols.add(x);
                const added: string[] = [];
                for (let dy = -1; dy <= 1; dy++)
                    for (let dx = -1; dx <= 1; dx++) {
                        if (dy == 0 && dx == 0) continue;
                        const k = `${y + dy},${x + dx}`;
                        if (!forbidden.has(k)) { forbidden.add(k); added.push(k); }
                    }
                bt(ri + 1);
                usedRows.delete(y);
                usedCols.delete(x);
                for (const k of added) forbidden.delete(k);
            }
        };

        bt(0);
        return found;
    }

    private boardKey(board: Board): string {
        return board.map(row => row.join(',')).join('|');
    }

    private regionConnected(board: Board, exclY: number, exclX: number, region: number): boolean {
        const { size } = this;
        let start: [number, number] | null = null;
        outer: for (let y = 0; y < size; y++)
            for (let x = 0; x < size; x++)
                if (board[y][x] == region && !(y == exclY && x == exclX)) { start = [y, x]; break outer; }

        if (!start) return false;

        const visited = new Set<string>([`${start[0]},${start[1]}`]);
        const q: [number, number][] = [start];
        while (q.length) {
            const [cy, cx] = q.shift()!;
            for (const [dy, dx] of [[-1, 0], [1, 0], [0, -1], [0, 1]] as [number, number][]) {
                const ny = cy + dy, nx = cx + dx;
                const k = `${ny},${nx}`;
                if (ny >= 0 && ny < size && nx >= 0 && nx < size
                    && board[ny][nx] == region
                    && !(ny == exclY && nx == exclX)
                    && !visited.has(k)) {
                    visited.add(k);
                    q.push([ny, nx]);
                }
            }
        }

        for (let y = 0; y < size; y++)
            for (let x = 0; x < size; x++)
                if (board[y][x] == region && !(y == exclY && x == exclX))
                    if (!visited.has(`${y},${x}`)) return false;

        return true;
    }

    private neighbours(board: Board): Board[] {
        const { size } = this;
        const result: Board[] = [];
        const seen = new Set<string>();

        for (let y = 0; y < size; y++) {
            for (let x = 0; x < size; x++) {
                const src = board[y][x];
                for (const [dy, dx] of [[-1, 0], [1, 0], [0, -1], [0, 1]] as [number, number][]) {
                    const ny = y + dy, nx = x + dx;
                    if (ny < 0 || ny >= size || nx < 0 || nx >= size) continue;
                    const dst = board[ny][nx];
                    if (dst == src) continue;
                    const swapKey = `${y},${x},${dst}`;
                    if (seen.has(swapKey)) continue;
                    seen.add(swapKey);
                    if (this.regionConnected(board, y, x, src)) {
                        const nb = board.map(row => [...row]);
                        nb[y][x] = dst;
                        result.push(nb);
                    }
                }
            }
        }

        return result;
    }

    private aStar(start: Board, maxIter: number): Board | null {
        const h0 = this.countSolutions(start);
        if (h0 == 1) return start;

        let ctr = 0;
        const heap: HeapEntry[] = [];
        heapPush(heap, [h0 - 1, 0, ctr, start]);
        const bestG = new Map<string, number>();
        bestG.set(this.boardKey(start), 0);

        for (let iter = 0; iter < maxIter; iter++) {
            if (!heap.length) break;
            const [, g, , board] = heapPop(heap);

            for (const nb of this.neighbours(board)) {
                const h = this.countSolutions(nb);
                if (h == 1) return nb;
                if (h == 0) continue;
                const ng = g + 1;
                const key = this.boardKey(nb);
                if ((bestG.get(key) ?? Infinity) <= ng) continue;
                bestG.set(key, ng);
                ctr++;
                heapPush(heap, [ng + h - 1, ng, ctr, nb]);
            }
        }

        return null;
    }
}
