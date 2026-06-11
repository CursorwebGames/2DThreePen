mod cell;
mod genpenai;
mod llist;
mod matrix;
mod solver;
mod utils;

use wasm_bindgen::prelude::*;

// use crate::solver::BullpenSolver;

pub use genpenai::generate;
pub use solver::BullpenSolver;

// #[wasm_bindgen(start)]
// pub fn main() {
//     #[cfg(debug_assertions)]
//     utils::set_panic_hook();
// }
