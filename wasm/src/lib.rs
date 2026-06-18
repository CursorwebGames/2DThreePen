mod cell;
mod gen_single;
pub mod genpenai;
mod llist;
mod matrix;
mod single_solver;
mod utils;

use wasm_bindgen::prelude::*;

pub use gen_single::GenPen;
pub use single_solver::SingleSolver;

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    utils::set_panic_hook();
}
