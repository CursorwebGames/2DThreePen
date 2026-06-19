use std::ops;

use crate::cell::Cell;

/// Stores 'pointers' to all the nodes
/// A lookup table of sorts, containing multiple circular linked lists
#[derive(Default, Debug)]
pub struct LList {
    pub data: Vec<Link>,
}

#[derive(Debug)]
pub struct Link {
    pub prev: Cell,
    pub next: Cell,
}

impl ops::Index<Cell> for LList {
    type Output = Link;

    fn index(&self, index: Cell) -> &Self::Output {
        &self.data[index.0]
    }
}

impl ops::IndexMut<Cell> for LList {
    fn index_mut(&mut self, index: Cell) -> &mut Self::Output {
        &mut self.data[index.0]
    }
}

impl LList {
    pub fn with_capacity(cap: usize) -> LList {
        Self {
            data: Vec::with_capacity(cap),
        }
    }

    /// Make new cell
    pub fn alloc(&mut self) -> Cell {
        let cell = Cell(self.data.len());
        self.data.push(Link {
            prev: cell,
            next: cell,
        });
        cell
    }

    /// Inserts `b` into `a <-> c` to get `a <-> b <-> c`
    pub fn insert(&mut self, a: Cell, b: Cell) {
        let c = self[a].next;

        self[a].next = b;
        self[b].prev = a;

        self[b].next = c;
        self[c].prev = b;
    }

    /// Removes `b` from `a <-> b <-> c` to get `a <-> c`
    pub fn remove(&mut self, b: Cell) {
        let a = self[b].prev;
        let c = self[b].next;

        self[a].next = c;
        self[c].prev = a;
    }

    /// Restores previously removed `b` to get `a <-> b <-> c`
    pub fn restore(&mut self, b: Cell) {
        let a = self[b].prev;
        let c = self[b].next;

        self[a].next = b;
        self[c].prev = b;
    }
}

// Iter
impl LList {
    /// Create iterator to iterate through Circular Linked List
    pub fn cursor(&self, head: Cell) -> Cursor {
        Cursor { head, curr: head }
    }
}

pub struct Cursor {
    head: Cell,
    curr: Cell,
}

impl Cursor {
    pub fn next(&mut self, list: &LList) -> Option<Cell> {
        self.curr = list[self.curr].next;

        if self.curr == self.head {
            return None; // looped back to beginning
        }

        Some(self.curr)
    }

    pub fn prev(&mut self, list: &LList) -> Option<Cell> {
        self.curr = list[self.curr].prev;

        if self.curr == self.head {
            return None;
        }

        Some(self.curr)
    }
}
