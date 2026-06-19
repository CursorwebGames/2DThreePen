import init, { generate_single } from "../wasm/pkg/bullpen";

// use wasmReady to gate the promise
const wasmReady = init();

export async function genSingle(n: number, seed: bigint = BigInt(Date.now())): Promise<number[][]> {
    await wasmReady;
    const flat = generate_single(n, seed);
    const board: number[][] = [];
    for (let y = 0; y < n; y++) {
        board.push(Array.from(flat.subarray(y * n, (y + 1) * n)));
    }
    return board;
}
