use crate::matrix::{Matrix, H};

pub struct BullpenSolver {
    m: Matrix,

    /// Board width
    n: usize,

    /// Bitmap of board cells currently holding a bull,
    /// indexed by row_id = r * n + c
    placed: Vec<bool>,

    /// Remaining work budget for the current solve call;
    /// `solve_rec` stops exploring when it reaches 0
    steps: usize,
}

impl BullpenSolver {
    /// Takes in `[[1, 1], [2, 2]]` etc.
    pub fn new(regions: &[Vec<usize>]) -> BullpenSolver {
        let n = regions.len();

        let mut labels: Vec<usize> = Vec::with_capacity(n);

        let mut m = Matrix::new(3 * n);
        for (r, row) in regions.iter().enumerate() {
            assert_eq!(row.len(), n, "regions must be an n x n grid");
            for (c, &label) in row.iter().enumerate() {
                let region = labels.iter().position(|&l| l == label).unwrap_or_else(|| {
                    labels.push(label);
                    labels.len() - 1
                });

                let mut mrow = vec![false; 3 * n];
                mrow[r] = true;
                mrow[n + c] = true;
                mrow[2 * n + region] = true;
                m.add_row(&mrow);
            }
        }

        assert_eq!(labels.len(), n);

        BullpenSolver {
            m,
            n,
            placed: vec![false; n.pow(2)],
            steps: usize::MAX,
        }
    }

    /// Whether the puzzle has exactly one solution
    pub fn is_unique(&mut self) -> bool {
        self.count_sol() == 1
    }

    fn conflicts(&self, id: usize) -> bool {
        let n = self.n;
        let (y, x) = (id / n, id % n);
        for ny in y.saturating_sub(1)..=(y + 1).min(n - 1) {
            for nx in x.saturating_sub(1)..=(x + 1).min(n - 1) {
                if (ny, nx) != (y, x) && self.placed[ny * n + nx] {
                    return true;
                }
            }
        }

        false
    }

    /// Count solutions, short-circuiting when multiple exist.
    /// Returns 0, 1, or 2
    fn count_sol(&mut self) -> usize {
        let mut out = 0;
        self.count_sol_rec(&mut out);
        out
    }

    fn count_sol_rec(&mut self, n_answers: &mut usize) {
        let m = &mut self.m;

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
        while let Some(i) = i.next(&self.m.y) {
            // skip candidates that touch a bull already on the board;
            // must happen before any covering so there is nothing to restore
            let id = self.m.row_id[i];
            if self.conflicts(id) {
                continue;
            }

            // go through each item in the candidate
            // and cover that column (since this potential solution satisfies that constraint)
            let m = &mut self.m;
            let mut j = m.x.cursor(i);
            while let Some(j) = j.next(&m.x) {
                m.cover(m.c[j]);
            }

            // now keep solving
            self.placed[id] = true;
            self.count_sol_rec(n_answers);
            self.placed[id] = false;

            // restore
            let m = &mut self.m;
            let mut j = m.x.cursor(i);
            while let Some(j) = j.prev(&m.x) {
                m.uncover(m.c[j]);
            }

            // a second solution exists; no point searching further
            // (break only after restoring, so the matrix unwinds cleanly)
            if *n_answers >= 2 {
                break;
            }
        }

        self.m.uncover(c);
    }

    /// Returns at most 2 soltuions.
    pub fn solve(&mut self) -> Vec<Vec<(usize, usize)>> {
        self.solve_within(usize::MAX).unwrap()
    }

    /// Like `solve`, but gives up once the search has taken `budget`
    /// recursion steps, returning None. Exhaustively proving "0 solutions"
    /// or "exactly 1 solution" can take seconds on a pathological board;
    /// a caller that only wants *cheap* boards can discard one rather
    /// than pay to learn its exact answer. Aborting unwinds cleanly, so
    /// the solver stays reusable.
    pub fn solve_within(&mut self, budget: usize) -> Option<Vec<Vec<(usize, usize)>>> {
        self.steps = budget;
        let mut out = vec![];
        self.solve_rec(&mut Vec::new(), &mut out);
        if self.steps == 0 {
            None
        } else {
            Some(out)
        }
    }

    fn solve_rec(&mut self, csol: &mut Vec<(usize, usize)>, sols: &mut Vec<Vec<(usize, usize)>>) {
        // out of budget: abandon this subtree. Parents still restore
        // their covers on the way out, so the matrix unwinds cleanly.
        if self.steps == 0 {
            return;
        }
        self.steps -= 1;

        let m = &mut self.m;

        // choose column c
        let c = {
            let mut i = m.x.cursor(H);
            let mut c = match i.next(&m.x) {
                Some(it) => it,
                None => {
                    sols.push(csol.clone());
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
        while let Some(i) = i.next(&self.m.y) {
            // skip candidates that touch a bull already on the board;
            // must happen before any covering so there is nothing to restore
            let id = self.m.row_id[i];
            if self.conflicts(id) {
                continue;
            }

            // go through each item in the candidate
            // and cover that column (since this potential solution satisfies that constraint)
            let m = &mut self.m;
            let mut j = m.x.cursor(i);
            while let Some(j) = j.next(&m.x) {
                m.cover(m.c[j]);
            }

            // now keep solving
            self.placed[id] = true;
            csol.push((id / self.n, id % self.n));
            self.solve_rec(csol, sols);
            csol.pop();
            self.placed[id] = false;

            // restore
            let m = &mut self.m;
            let mut j = m.x.cursor(i);
            while let Some(j) = j.prev(&m.x) {
                m.uncover(m.c[j]);
            }

            if sols.len() >= 2 || self.steps == 0 {
                break;
            }
        }

        self.m.uncover(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board<const N: usize>(b: [[usize; N]; N]) -> Vec<Vec<usize>> {
        b.iter().map(|row| row.to_vec()).collect()
    }

    /// Count solutions by trying every column permutation
    fn brute_force(regions: &[Vec<usize>]) -> usize {
        let n = regions.len();
        let mut count = 0;
        let mut perm = vec![0; n];
        let mut used = vec![false; n];
        brute_force_rec(regions, n, 0, &mut perm, &mut used, &mut count);
        count
    }

    fn brute_force_rec(
        regions: &[Vec<usize>],
        n: usize,
        r: usize,
        perm: &mut Vec<usize>,
        used: &mut Vec<bool>,
        count: &mut usize,
    ) {
        if r == n {
            // all columns distinct and non-adjacent by construction;
            // check regions are all distinct
            let mut seen = vec![false; n];
            for (r, &c) in perm.iter().enumerate() {
                let reg = regions[r][c];
                if seen[reg] {
                    return;
                }
                seen[reg] = true;
            }
            *count += 1;
            return;
        }
        for c in 0..n {
            // adjacency: queens in consecutive rows must not touch diagonally
            if used[c] || (r > 0 && perm[r - 1].abs_diff(c) == 1) {
                continue;
            }
            used[c] = true;
            perm[r] = c;
            brute_force_rec(regions, n, r + 1, perm, used, count);
            used[c] = false;
        }
    }

    #[test]
    fn unique_puzzle() {
        // for n=4 only two queen layouts avoid touching: columns 1,3,0,2
        // and columns 2,0,3,1; these regions allow the first and break
        // the second (its row-0 and row-2 queens would share region 1)
        let regions = board([[0, 0, 1, 1], [0, 2, 1, 1], [2, 2, 3, 1], [2, 3, 3, 3]]);
        assert_eq!(brute_force(&regions), 1);
        assert_eq!(BullpenSolver::new(&regions).count_sol(), 1);
        assert!(BullpenSolver::new(&regions).is_unique());
    }

    #[test]
    fn ambiguous_puzzle() {
        // region == board column: both non-touching layouts survive
        let regions = board([[0, 1, 2, 3], [0, 1, 2, 3], [0, 1, 2, 3], [0, 1, 2, 3]]);
        assert_eq!(brute_force(&regions), 2);
        assert_eq!(BullpenSolver::new(&regions).count_sol(), 2);
        assert!(!BullpenSolver::new(&regions).is_unique());
    }

    #[test]
    fn one_based_labels() {
        // same board as unique_puzzle, but with 1-based region labels
        let regions = board([[1, 1, 2, 2], [1, 3, 2, 2], [3, 3, 4, 2], [3, 4, 4, 4]]);
        assert!(BullpenSolver::new(&regions).is_unique());
    }

    #[test]
    fn solver_is_reusable_after_counting() {
        let regions = board([[0, 0, 1, 1], [0, 2, 1, 1], [2, 2, 3, 1], [2, 3, 3, 3]]);
        let mut solver = BullpenSolver::new(&regions);
        assert_eq!(solver.count_sol(), 1);
        // matrix and bitmap must be fully restored: counting again agrees
        assert_eq!(solver.count_sol(), 1);
    }

    /// Boards from src/jsold/solutions.js. `expected` is the full solution
    /// count: stated by the source names/comments where available
    /// (oneSol, sixSol, elevenSol, twelveSol, genSolutions, ...),
    /// measured by brute force otherwise. The solver is additionally
    /// cross-checked against brute force on every board.
    #[test]
    fn existing_puzzles() {
        #[rustfmt::skip]
        let suite: Vec<(&str, usize, Vec<Vec<usize>>)> = vec![
            ("oneSol", 1, board([[3, 3, 5, 5, 5, 4], [3, 3, 1, 5, 5, 0], [3, 3, 1, 1, 0, 0], [3, 3, 1, 0, 0, 0], [1, 1, 1, 0, 0, 0], [2, 0, 0, 0, 0, 0]])),
            ("elevenSol", 11, board([[4, 4, 4, 5, 5, 5], [3, 4, 4, 4, 2, 2], [3, 3, 1, 1, 2, 2], [3, 3, 1, 1, 2, 2], [0, 0, 1, 1, 1, 2], [0, 0, 0, 0, 0, 0]])),
            ("sixSol", 6, board([[4, 4, 4, 5, 5, 5], [3, 3, 4, 5, 1, 1], [3, 3, 5, 5, 1, 1], [0, 0, 5, 2, 2, 1], [0, 0, 0, 2, 2, 1], [0, 0, 0, 2, 2, 2]])),
            ("difficultOneSolution", 1, board([[5, 5, 5, 4, 4, 4], [5, 3, 5, 5, 2, 4], [3, 3, 5, 0, 2, 4], [3, 1, 1, 0, 2, 2], [3, 3, 1, 0, 0, 2], [3, 3, 1, 0, 0, 2]])),
            ("oneSevenBySeven", 1, board([[3, 3, 3, 3, 6, 6, 5], [3, 3, 4, 6, 6, 1, 5], [3, 4, 4, 4, 4, 1, 5], [3, 3, 4, 4, 4, 1, 5], [2, 3, 4, 4, 1, 1, 5], [2, 0, 1, 4, 4, 1, 1], [0, 0, 1, 1, 1, 1, 1]])),
            ("illini", 17, board([[6, 4, 4, 4, 4, 2, 5], [6, 6, 4, 3, 4, 2, 5], [6, 6, 3, 3, 2, 2, 2], [3, 3, 3, 3, 3, 2, 2], [1, 0, 0, 0, 3, 2, 2], [1, 1, 0, 3, 3, 2, 2], [1, 0, 0, 0, 2, 2, 2]])),
            ("maskHard", 7, board([[4, 4, 4, 4, 5, 5], [4, 4, 3, 3, 5, 5], [2, 2, 2, 3, 3, 5], [1, 2, 2, 2, 3, 5], [1, 2, 1, 0, 3, 0], [1, 1, 1, 0, 0, 0]])),
            ("maskZero", 3, board([[5, 1, 1, 4, 4, 4], [5, 1, 3, 4, 3, 4], [5, 1, 3, 3, 3, 3], [5, 1, 3, 3, 3, 2], [5, 1, 1, 2, 2, 2], [5, 1, 1, 0, 0, 0]])),
            ("twelveSol", 12, board([[3, 3, 5, 5, 5, 4], [3, 3, 1, 5, 5, 4], [3, 3, 1, 1, 5, 4], [3, 3, 1, 2, 4, 4], [1, 1, 1, 2, 0, 0], [2, 2, 2, 2, 0, 0]])),
            ("bullPen", 1, board([[0, 0, 1, 1, 1, 1], [0, 2, 1, 1, 3, 3], [2, 2, 2, 1, 3, 3], [2, 2, 5, 4, 3, 3], [2, 4, 4, 4, 4, 3], [4, 4, 4, 4, 4, 4]])),
        ];

        for (name, expected, regions) in suite {
            let full = brute_force(&regions);
            assert_eq!(full, expected, "{name}: expected count vs brute force");
            assert_eq!(
                BullpenSolver::new(&regions).count_sol(),
                full.min(2),
                "{name}: solver vs brute force"
            );
        }
    }

    #[test]
    fn random_grids_match_brute_force() {
        let n = 5;
        let mut seed: u64 = 0xdead_beef;
        let mut rand = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as usize
        };

        let mut tested = 0;
        while tested < 200 {
            let regions: Vec<Vec<usize>> = (0..n)
                .map(|_| (0..n).map(|_| rand() % n).collect())
                .collect();

            // every region must appear, otherwise the grid is rejected
            // by the constructor as malformed
            let mut seen = vec![false; n];
            for row in &regions {
                for &reg in row {
                    seen[reg] = true;
                }
            }
            if seen.iter().any(|&s| !s) {
                continue;
            }
            tested += 1;

            let expected = brute_force(&regions).min(2);
            assert_eq!(
                BullpenSolver::new(&regions).count_sol(),
                expected,
                "mismatch for regions {:?}",
                regions
            );
        }
    }
}
