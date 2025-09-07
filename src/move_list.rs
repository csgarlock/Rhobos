use crate::r#move::{pretty_string_move, Move, NULL_MOVE};

pub const MAX_QUIET_MOVES: usize = 50;
pub const MAX_CAPTURE_MOVES: usize = 40;
pub const NUM_KILLERS: usize = 2;

#[derive(Clone)]
pub struct MoveStack {
    vec: Vec<MoveList>,
    current: usize,
}

#[derive(Clone)]
pub struct MoveList {
    pub vec: Vec<Move>,
    pub current_get: usize,
    pub last: usize,
    pub move_fetch_stage: MoveFetchStage,
    pub tt_move: Move,
    pub killer_moves: [Move; NUM_KILLERS],
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MoveFetchStage {
    Start,
    TTMove,
    CaptureMoves,
    KillerMoves,
    QuietMoves,
    Done,
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
            vec: vec![0; MAX_CAPTURE_MOVES + MAX_QUIET_MOVES],
            current_get: 0,
            last: 0,
            move_fetch_stage: MoveFetchStage::Start,
            tt_move: NULL_MOVE,
            killer_moves: [NULL_MOVE; NUM_KILLERS],
        }
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

    #[inline(always)]
    pub fn add_killer(&mut self, m: Move) {
        self.killer_moves[NUM_KILLERS - 1] = m;
        self.killer_moves.rotate_right(1);
    } 

    #[inline(always)]
    pub fn total_moves(&self) -> usize {
        self.last
    }

    pub fn debug_string_moves(&mut self) -> Vec<String> {
        let mut result = Vec::new();
        for i in 0..self.last {
            result.push(pretty_string_move(self.vec[i]));
        }
        result
    }

}

impl MoveFetchStage {
    #[allow(dead_code)]
    fn next(self) -> MoveFetchStage {
        match self {
            MoveFetchStage::Start => MoveFetchStage::TTMove,
            MoveFetchStage::TTMove => MoveFetchStage::CaptureMoves,
            MoveFetchStage::CaptureMoves => MoveFetchStage::KillerMoves,
            MoveFetchStage::KillerMoves => MoveFetchStage::QuietMoves,
            MoveFetchStage::QuietMoves => MoveFetchStage::Done,
            MoveFetchStage::Done => MoveFetchStage::Done,
        }
    }
}