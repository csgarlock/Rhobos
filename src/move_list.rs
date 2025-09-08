use crate::{r#move::{pretty_string_move, Move, NULL_MOVE}, move_pick::MovePickStage, state::State};

pub const MAX_QUIET_MOVES: usize = 50;
pub const MAX_CAPTURE_MOVES: usize = 40;
pub const NUM_KILLERS: usize = 2;

#[derive(Clone)]
pub struct MoveStack {
    vec: Vec<MoveList>,
    current: usize,
}

#[derive(Clone, Copy)]
pub union MoveValue {
    pub attack_val: i8,
    pub quiet_val: u64,
}

#[derive(Clone)]
pub struct MoveList {
    pub move_vec: Vec<Move>,
    pub value_vec: Vec<MoveValue>,
    pub current: Move,
    pub last: usize,
    pub move_pick_stage: MovePickStage,
    pub tt_move: Move,
    pub killer_moves: [Move; NUM_KILLERS],
}

impl MoveStack {
    pub fn new(size: usize) -> MoveStack {
        MoveStack { vec: vec![MoveList::new(); size], current: 0 }
    }

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
        MoveList {
            move_vec: vec![0; MAX_CAPTURE_MOVES + MAX_QUIET_MOVES],
            value_vec: vec![MoveValue {attack_val: 0}; MAX_CAPTURE_MOVES + MAX_QUIET_MOVES],
            current: NULL_MOVE,
            last: 0,
            move_pick_stage: MovePickStage::Start,
            tt_move: NULL_MOVE,
            killer_moves: [NULL_MOVE; NUM_KILLERS],
        }
    }

    #[inline(always)]
    fn reset(&mut self) {
        self.current = 0;
        self.last = 0;
        self.tt_move = NULL_MOVE;
    }

    #[inline(always)]
    pub fn push(&mut self, m: Move) {
        self.move_vec[self.last] = m;
        self.last += 1;
    }

    #[inline(always)]
    pub fn add_killer(&mut self, m: Move) {
        self.killer_moves[NUM_KILLERS - 1] = m;
        self.killer_moves.rotate_right(1);
    }

    #[inline(always)]
    pub fn add_tt_move(&mut self, m: Move) {
        self.tt_move = m;
    } 

    #[inline(always)]
    pub fn total_moves(&self) -> usize {
        self.last
    }

    pub fn debug_string_moves(&mut self) -> Vec<String> {
        let mut result = Vec::new();
        for i in 0..self.last {
            result.push(pretty_string_move(self.move_vec[i]));
        }
        result
    }

}

impl State {
    #[inline(always)]
    pub fn current_move_list(&mut self) -> &mut MoveList {
        self.move_stack.get_current()
    }
}