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
    current_store: usize,
    last: usize,
}

impl MoveStack {

    #[inline(always)]
    pub fn get_list(&mut self) -> &mut MoveList {
        let move_list = self.vec.get_mut(self.current);
        match move_list {
            (Some(list)) => return list,
            (None) => {
                self.vec.push(MoveList::new());
                self.current += 1;
                return &mut self.vec[self.current];
            }
        }
    }

}

impl Iterator for MoveList {
    type Item = Move;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_get >= self.last {
            None
        } else {
            let result = self.vec[self.current_get];
            self.current_get += 1;
            Some(result)
        }
    }
}

impl MoveList {
    fn new() -> MoveList {
        MoveList { vec: vec![0; MAX_CAPTURE_MOVES + MAX_QUIET_MOVES], current_get: 0, current_store: 0, last: 0 }
    }

    #[inline(always)]
    pub fn push(&mut self, m: Move) {
        self.vec[self.current_store] = m;
        self.current_store += 1;
    }
}