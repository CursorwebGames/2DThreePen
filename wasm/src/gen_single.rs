use std::{collections::VecDeque, ops::RangeInclusive};

use fastrand::Rng;

use crate::single_solver::SingleSolver;

const EMPTY: usize = usize::MAX;

// TODO: investigate if changing these numbers affects speed
const MAX_REPAIRS: usize = 50;
const SOLVE_BUDGET: usize = 50_000;

// TODO: how fun is it if we have an optimal number?
const CAP_SIZE: RangeInclusive<usize> = 1..=3;

/// (y, x)
type Pos = (usize, usize);

pub struct GenPen {
    rng: Rng,
    grid: Vec<Vec<usize>>,
    n: usize,
}

impl GenPen {
    pub fn new(rng: Rng) -> Self {
        Self {
            n: 0,
            grid: vec![],
            rng,
        }
    }

    pub fn gen(&mut self, n: usize) -> Vec<Vec<usize>> {
        self.n = n;
        self.init_grid();
        self.gen_puzzle();
        std::mem::take(&mut self.grid)
    }

    fn gen_puzzle(&mut self) {
        loop {
            self.clear();
            self.gen_regions();

            // ~96% of boards are NOT SOLVABLE!
            if !self.solvable() {
                continue;
            }

            for _ in 0..MAX_REPAIRS {
                let sols = match SingleSolver::new(&self.grid).solve_within(SOLVE_BUDGET) {
                    Some(sols) => sols,
                    None => break, // too expensive
                };
                match sols.len() {
                    0 => break,
                    1 => return, // yay!
                    _ => {
                        if !self.kill_sol(&sols[0], &sols[1]) && !self.kill_sol(&sols[1], &sols[0])
                        {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Generate random regions
    fn gen_regions(&mut self) {
        let mut frontier: Vec<(Pos, usize)> = Vec::new();
        let n = self.n;

        // maps region -> size
        let mut sizes = vec![0usize; n];
        let mut max_size = vec![usize::MAX; n];
        let mut order: Vec<usize> = (0..n).collect();
        self.rng.shuffle(&mut order);

        let num_caps = self.num_caps();

        // take <num caps> regions
        for &region in order.iter().take(self.rng.usize(num_caps)) {
            // cap them
            max_size[region] = self.rng.usize(CAP_SIZE);
        }

        // add seeds
        for region in 0..n {
            loop {
                let (y, x) = self.rand_pos();
                if self.grid[y][x] == EMPTY {
                    self.claim(&mut frontier, &mut sizes, (y, x), region);
                    break;
                }
            }
        }

        while !frontier.is_empty() {
            let ((y, x), region) = frontier.swap_remove(self.rng.usize(0..frontier.len()));
            if self.grid[y][x] == EMPTY && sizes[region] < max_size[region] {
                self.claim(&mut frontier, &mut sizes, (y, x), region);
            }
        }

        // real chance capped regions don't fully fill the board (walled off)
        if sizes.iter().sum::<usize>() < n * n {
            let mut queue: VecDeque<Pos> = (0..n)
                .flat_map(|y| (0..n).map(move |x| (y, x)))
                .filter(|&(y, x)| self.grid[y][x] == EMPTY)
                .collect();

            while let Some((y, x)) = queue.pop_front() {
                if let Some((ny, nx)) = self
                    .neighbors(y, x)
                    .find(|&(y, x)| self.grid[y][x] != EMPTY)
                {
                    self.grid[y][x] = self.grid[ny][nx];
                } else {
                    queue.push_back((y, x)); // still surrounded, retry later
                }
            }
        }
    }

    fn claim(
        &mut self,
        frontier: &mut Vec<(Pos, usize)>,
        sizes: &mut [usize],
        (y, x): Pos,
        region: usize,
    ) {
        self.grid[y][x] = region;
        sizes[region] += 1;
        for (nr, nc) in self.neighbors(y, x) {
            if self.grid[nr][nc] == EMPTY {
                frontier.push(((nr, nc), region));
            }
        }
    }

    /// Remove all targets in kill not in keep as long as region stays connected
    /// Returns `false` if can't kill any target
    fn kill_sol(&mut self, keep: &[Pos], kill: &[Pos]) -> bool {
        let mut targets: Vec<Pos> = kill
            .iter()
            .filter(|pos| !keep.contains(pos))
            .copied()
            .collect();
        self.rng.shuffle(&mut targets);

        for (y, x) in targets {
            if !self.stays_connected(y, x) {
                continue;
            }

            let donor_region = self.grid[y][x];
            let mut neighbors: Vec<Pos> = self.neighbors(y, x).collect();
            self.rng.shuffle(&mut neighbors);
            for (ny, nx) in neighbors {
                if self.grid[ny][nx] != donor_region {
                    self.grid[y][x] = self.grid[ny][nx];
                    return true;
                }
            }
        }

        return false;
    }

    /// Cheap sanity check: each row/col has a unique region color
    fn solvable(&self) -> bool {
        let n = self.n;
        // row[region][x] == true means region touches at row x
        let mut rows = vec![vec![false; n]; n];
        let mut cols = vec![vec![false; n]; n];
        for y in 0..n {
            for x in 0..n {
                rows[self.grid[y][x]][y] = true;
                cols[self.grid[y][x]][x] = true;
            }
        }

        self.perfect_matching(&rows) && self.perfect_matching(&cols)
    }

    fn perfect_matching(&self, touch: &[Vec<bool>]) -> bool {
        let n = self.n;
        let mut matched = vec![EMPTY; n];

        // Hopcroft-Karp should succeed on every row in assigning a region
        (0..n).all(|region| self.augment(region, touch, &mut vec![false; n], &mut matched))
    }

    /// Hopcroft-Karp augmenting path algorithm
    /// Assign `regions` to `rows` with edge being assignment
    fn augment(
        &self,
        region: usize,
        touch: &[Vec<bool>],
        seen: &mut [bool],     // fresh attempt tracker each time
        matched: &mut [usize], // running tracker of who goes where
    ) -> bool {
        for x in 0..self.n {
            if touch[region][x] && !seen[x] {
                // try this row
                seen[x] = true;

                // row x is free, or its current owner can find another row
                if matched[x] == EMPTY || self.augment(matched[x], touch, seen, matched) {
                    matched[x] = region;
                    return true;
                }
            }
        }

        false
    }

    fn num_caps(&self) -> RangeInclusive<usize> {
        self.n / 3..=(self.n / 3 + 2).min(self.n)
    }

    fn neighbors(&self, y: usize, x: usize) -> impl Iterator<Item = Pos> {
        let n = self.n;

        [
            (y.wrapping_sub(1), x),
            (y + 1, x),
            (y, x.wrapping_sub(1)),
            (y, x + 1),
        ]
        .into_iter()
        .filter(move |&(y, x)| y < n && x < n)
    }

    fn rand_pos(&mut self) -> (usize, usize) {
        (self.rng.usize(0..self.n), self.rng.usize(0..self.n))
    }

    fn clear(&mut self) {
        for row in &mut self.grid {
            row.fill(EMPTY);
        }
    }

    fn init_grid(&mut self) {
        self.grid = vec![vec![EMPTY; self.n]; self.n];
    }

    /// Would region connect if p=(y, x) was removed?
    /// Method: look at anchors (neighbors of p) and floodfill from one
    /// If reach all other anchors, then is connected!
    fn stays_connected(&mut self, y: usize, x: usize) -> bool {
        let region = self.grid[y][x];
        let target = (y, x);

        let anchors: Vec<Pos> = self
            .neighbors(y, x)
            .filter(|&(y, x)| self.grid[y][x] == region)
            .collect();

        if anchors.is_empty() {
            return false; // cannot delete region
        }

        if anchors.len() == 1 {
            return true;
        }

        let mut seen = vec![vec![false; self.n]; self.n];
        let mut stack = vec![anchors[0]];
        seen[anchors[0].0][anchors[0].1] = true;

        while let Some((y, x)) = stack.pop() {
            for (ny, nx) in self.neighbors(y, x) {
                if (ny, nx) != target && !seen[ny][nx] && self.grid[ny][nx] == region {
                    seen[ny][nx] = true;
                    stack.push((ny, nx));
                }
            }
        }

        anchors.iter().all(|&(y, x)| seen[y][x])
    }
}
