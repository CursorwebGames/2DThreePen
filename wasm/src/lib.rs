use crate::{
    cell::Cell,
    matrix::{Matrix, H},
};

mod cell;
mod llist;
mod matrix;
mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    utils::set_panic_hook();
}

/// Not truly useful function
#[cfg(not(target_arch = "wasm32"))]
fn solve(mut m: Matrix) -> Vec<Vec<usize>> {
    let mut out = vec![];
    let mut curr = vec![];

    solve_rec(&mut m, &mut curr, &mut out);

    out
}

fn solve_rec(m: &mut Matrix, csol: &mut Vec<usize>, sols: &mut Vec<Vec<usize>>) {
    // choose the column with the fewest remaining rows
    let c = {
        let mut i = m.x.cursor(H);
        let mut c = match i.next(&m.x) {
            Some(c) => c,
            None => {
                sols.push(csol.clone());
                return;
            }
        };
        while let Some(next_c) = i.next(&m.x) {
            if m.size[next_c] < m.size[c] {
                c = next_c;
            }
        }
        c
    };
    // if the chosen column has no rows, this branch cannot succeed
    if m.size[c] == 0 {
        return;
    }

    // temporarily remove the chosen column from consideration
    m.cover(c);

    // look at all potential rows = candidates that satisfy this constraint
    // and test them (recursively)
    let mut i = m.y.cursor(c);
    while let Some(i) = i.next(&m.y) {
        // go through each item in the candidate
        // and cover that column (since this potential solution satisfies that constraint)
        let mut j = m.x.cursor(i);
        while let Some(j) = j.next(&m.x) {
            m.cover(m.c[j]);
        }

        // now keep solving
        csol.push(m.row_id[i]);
        solve_rec(m, csol, sols);
        csol.pop();

        // restore
        let mut j = m.x.cursor(i);
        while let Some(j) = j.prev(&m.x) {
            m.uncover(m.c[j]);
        }
    }

    // restore the chosen column before returning
    m.uncover(c);
}

/// Count how many solutions (to guarantee uniqueness)
fn count_sol(mut m: Matrix) -> usize {
    let mut out = 0;
    count_sol_rec(&mut m, &mut out);
    out
}

fn count_sol_rec(m: &mut Matrix, n_answers: &mut usize) {
    // choose column c
    let c = {
        let mut i = m.x.cursor(H);
        let mut c = match i.next(&m.x) {
            Some(it) => it,
            None => {
                *n_answers += 1;
                return;
            }
        };
        while let Some(next_c) = i.next(&m.x) {
            // find smallest column
            if m.size[next_c] < m.size[c] {
                c = next_c;
            }
        }
        c
    };

    // if no rows, then invalid state, fail
    if m.size[c] == 0 {
        return;
    }

    m.cover(c);

    // look at all potential rows = candidates that satisfy this constraint
    // and test them (recursively)
    let mut i = m.y.cursor(c);
    while let Some(i) = i.next(&m.y) {
        // go through each item in the candidate
        // and cover that column (since this potential solution satisfies that constraint)
        let mut j = m.x.cursor(i);
        while let Some(j) = j.next(&m.x) {
            m.cover(m.c[j]);
        }

        // now keep solving
        count_sol_rec(m, n_answers);

        // restore
        let mut j = m.x.cursor(i);
        while let Some(j) = j.prev(&m.x) {
            m.uncover(m.c[j]);
        }
    }

    m.uncover(c);
}

#[test]
fn sample_problem() {
    let f = false;
    let t = true;

    let mut m = Matrix::new(7);
    /*
        A = {1, 4, 7};
        B = {1, 4};
        C = {4, 5, 7};
        D = {3, 5, 6};
        E = {2, 3, 6, 7}; and
        F = {2, 7}.
    */
    m.add_row(&[t, f, f, t, f, f, t]);
    m.add_row(&[t, f, f, t, f, f, f]);
    m.add_row(&[f, f, f, t, t, f, t]);
    m.add_row(&[f, f, t, f, t, t, f]);
    m.add_row(&[f, t, t, f, f, t, t]);
    m.add_row(&[f, t, f, f, f, f, t]);

    let solutions = count_sol(m);
    assert_eq!(solutions, 1);
}

#[test]
fn solve_problem() {
    let f = false;
    let t = true;

    let mut m = Matrix::new(7);
    /*
        A = {1, 4, 7};
        B = {1, 4};
        C = {4, 5, 7};
        D = {3, 5, 6};
        E = {2, 3, 6, 7}; and
        F = {2, 7}.
    */
    m.add_row(&[t, f, f, t, f, f, t]);
    m.add_row(&[t, f, f, t, f, f, f]);
    m.add_row(&[f, f, f, t, t, f, t]);
    m.add_row(&[f, f, t, f, t, t, f]);
    m.add_row(&[f, t, t, f, f, t, t]);
    m.add_row(&[f, t, f, f, f, f, t]);

    let solutions = solve(m);
    assert_eq!(solutions, vec![vec![1, 3, 5]]);
}

#[test]
fn exhaustive_test() {
    'matrix: for bits in 0..=0b1111_1111_1111_1111 {
        let mut rows = [0u32; 4];
        for (i, row) in rows.iter_mut().enumerate() {
            *row = (bits >> (i * 4)) & 0b1111;
            if *row == 0 {
                continue 'matrix;
            }
        }

        let brute_force = {
            let mut n_solutions = 0;
            for mask in 0..=0b1111 {
                let mut or = 0;
                let mut n_ones = 0;
                for (i, &row) in rows.iter().enumerate() {
                    if mask & (1 << i) != 0 {
                        or |= row;
                        n_ones += row.count_ones()
                    }
                }
                if or == 0b1111 && n_ones == 4 {
                    n_solutions += 1;
                }
            }
            n_solutions
        };

        let dlx = {
            let mut m = Matrix::new(4);
            for row_bits in rows.iter() {
                let mut row = [false; 4];
                for i in 0..4 {
                    row[i] = row_bits & (1 << i) != 0;
                }
                m.add_row(&row);
            }
            count_sol(m)
        };
        assert_eq!(brute_force, dlx)
    }
}
