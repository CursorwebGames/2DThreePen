mod cell;
mod gen_single;
pub mod gendouble_test;
pub mod genpen_test;
mod llist;
mod matrix;
mod single_solver;
mod stats;
#[cfg(debug_assertions)]
mod utils;

use fastrand::Rng;
use wasm_bindgen::prelude::*;

pub use gen_single::GenPen;
pub use single_solver::SingleSolver;
#[cfg(not(target_arch = "wasm32"))]
pub use stats::{Aggregator, Stats};

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    utils::set_panic_hook();
}

#[wasm_bindgen]
pub fn generate_single(n: usize, seed: u64) -> Vec<u8> {
    let mut gen = GenPen::new(Rng::with_seed(seed));
    gen.gen(n)
        .into_iter()
        .flatten()
        .map(|region| region as u8)
        .collect()
}
