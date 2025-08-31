use std::thread::current;

use crate::r#move::Move;

pub const MAX_QUIET_MOVES: usize = 50;
pub const MAX_CAPTURE_MOVES: usize = 40;

pub struct MoveStack {
    vec: Vec<MoveList>,
    current: usize,
}

pub struct MoveList {
    vec: Vec<Move>,
    current_get: usize,
    last: usize,
}

impl MoveStack {
    #[inline(always)]
    fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline(always)]
    pub fn next(&mut self) {
        self.current += 1;
        if self.current >= self.len() {
            self.vec.push(MoveList::new());
        } else {
            self.vec[self.current].reset();
        }
    }

    #[inline(always)]
    pub fn previous(&mut self) {
        self.current -= 1;
    }

    #[inline(always)]
    pub fn get_current(&mut self) -> &mut MoveList {
        &mut self.vec[self.current]
    }

    #[inline(always)]
    pub fn push_current(&mut self, m: Move) {
        self.get_current().push(m);
    }
}

impl MoveList {
    #[inline(always)]
    fn new() -> MoveList {
        MoveList { vec: vec![0; MAX_CAPTURE_MOVES + MAX_QUIET_MOVES], current_get: 0, last: 0 }
    }

    #[inline(always)]
    fn reset(&mut self) {
        self.current_get = 0;
        self.last = 0;
    }

    #[inline(always)]
    pub fn push(&mut self, m: Move) {
        self.vec[self.last] = m;
        self.last += 1;
    }

    #[inline(always)]
    pub fn has_next(&self) -> bool {
        self.current_get + 1 <= self.last
    }

    #[inline(always)]
    pub fn next(&mut self) {
        self.current_get += 1;
    }

    #[inline(always)]
    pub fn current(&self) -> Move {
        self.vec[self.current_get]
    }
}