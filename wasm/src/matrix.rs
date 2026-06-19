use std::fmt;

use crate::{cell::Cell, llist::LList};

pub struct Matrix {
    /// Links across horizontal direction (lookup table)
    /// o-o-o
    pub x: LList,

    /// Links across vertical direction
    /// ```txt
    /// o
    /// |
    /// o
    /// |
    /// o
    /// ```
    pub y: LList,

    /// Lookup table, maps each cell to its header
    pub c: Vec<Cell>,

    /// Map each cell to a row_id (for soln)
    pub row_id: Vec<usize>,
    row_count: usize,

    /// Num of 1s for each column
    pub size: Vec<usize>,
}

/// Root header node, sentinel
pub const H: Cell = Cell(0);

impl Matrix {
    pub fn new(n_cols: usize) -> Matrix {
        // +1 for sentinel
        let mut res = Matrix {
            x: LList::with_capacity(n_cols + 1),
            y: LList::with_capacity(n_cols + 1),
            c: Vec::with_capacity(n_cols + 1),
            row_id: Vec::with_capacity(n_cols + 1),
            row_count: 0,
            size: Vec::with_capacity(n_cols + 1),
        };

        // side effect must stay out of debug_assert!, which release strips
        let sentinel = res.alloc_column();
        debug_assert_eq!(sentinel, H);

        for _ in 0..n_cols {
            res.add_column();
        }

        // at this point, x and y look like:
        // x: H <-> C1 <-> C2 <-> C3 where C1, C2, C3 are the headers
        // y: H C1 C2 C3. Four disjoint circular linked lists
        res
    }

    fn add_column(&mut self) {
        // takes new column, and connects it horizontally
        // C1 C2 => C1 <-> C2
        let new_col = self.alloc_column();
        // note self.x[H].prev is the last element in LList
        self.x.insert(self.x[H].prev, new_col);
    }

    fn alloc_column(&mut self) -> Cell {
        // use a placeholder here that immediately gets overwritten
        // call alloc just to add a node rather than associate with a header
        let cell = self.alloc(H);
        self.c[cell] = cell;
        self.size.push(0);
        self.row_id.push(0);
        cell
    }

    /// Allocates a new node belonging to header c
    fn alloc(&mut self, c: Cell) -> Cell {
        self.c.push(c);
        let cell = self.x.alloc();
        // make sure both x and y refer to the same cell
        // both lookup tables need to have same num of nodes
        let y_cell = self.y.alloc();
        debug_assert_eq!(y_cell, cell);
        cell
    }

    pub fn add_row(&mut self, row: &[bool]) {
        assert_eq!(row.len(), self.size.len() - 1);

        let row_id = self.row_count;
        self.row_count += 1;

        // keep track of header position with c
        // start at sentinel!
        let mut c = H;
        // keep track of most recent inserted node
        // to keep the xs aligned
        let mut prev = None;
        for &is_filled in row {
            c = self.x[c].next;
            if is_filled {
                self.size[c] += 1;
                let new_cell = self.alloc(c);
                debug_assert_eq!(new_cell.0, self.row_id.len());
                self.row_id.push(row_id);
                // self.y[c] gets the header node, and .prev takes you to the end of list
                self.y.insert(self.y[c].prev, new_cell);
                if let Some(prev) = prev {
                    self.x.insert(prev, new_cell);
                }
                prev = Some(new_cell);
            }
        }
    }
}

impl Matrix {
    /// 'Removes' column, as well as all rows that contain the column
    ///     Think: if the constraint is satisfied, then none of the rows
    ///     that satisfy constraint can be in the solution anymore
    pub fn cover(&mut self, c: Cell) {
        self.x.remove(c);
        let mut i = self.y.cursor(c);
        // go through each row
        while let Some(i) = i.next(&self.y) {
            let mut j = self.x.cursor(i);
            // remove row
            while let Some(j) = j.next(&self.x) {
                self.y.remove(j);
                self.size[self.c[j]] -= 1;
            }
        }
    }

    pub fn uncover(&mut self, c: Cell) {
        let mut i = self.y.cursor(c);
        // i.next would have been the first thing removed
        // so it needs to be the last thing restored since it links to so much before it
        while let Some(i) = i.prev(&self.y) {
            let mut j = self.x.cursor(i);
            while let Some(j) = j.next(&self.x) {
                self.y.restore(j);
                self.size[self.c[j]] += 1;
            }
        }
        self.x.restore(c);
    }
}

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "s: ")?;
        for s in &self.size {
            write!(f, "{:^5}", s)?;
        }
        writeln!(f)?;

        write!(f, "c: ")?;
        for &Cell(c) in &self.c {
            write!(f, "{:^5}", c.saturating_sub(1))?;
        }
        writeln!(f)?;

        write!(f, "x: ")?;
        for link in &self.x.data {
            write!(f, " {:>1}|{:<1} ", link.prev.0, link.next.0)?
        }
        writeln!(f)?;

        write!(f, "y: ")?;
        for link in &self.y.data {
            write!(f, " {:>1}|{:<1} ", link.prev.0, link.next.0)?
        }
        writeln!(f)?;

        write!(f, "i: ")?;
        for i in 0..self.x.data.len() {
            write!(f, "{:^5}", i)?;
        }
        writeln!(f)?;

        Ok(())
    }
}
