use crate::state::State;

pub type Evaluation = i32;

pub const CENTI_PAWN: Evaluation = 66536;
pub const QUEEN_EVAL: Evaluation = CENTI_PAWN * 900;
pub const ROOK_EVAL: Evaluation = CENTI_PAWN * 500;
pub const BISHOP_EVAL: Evaluation = CENTI_PAWN * 325;
pub const KNIGHT_EVAL: Evaluation = CENTI_PAWN * 300;
pub const PAWN_EVAL: Evaluation = CENTI_PAWN * 100;
pub const EVAL_TABLE: [Evaluation; 6] = [0, QUEEN_EVAL, ROOK_EVAL, BISHOP_EVAL, KNIGHT_EVAL, PAWN_EVAL];

impl State {
    pub fn eval_state(&self) -> Evaluation {
        todo!()
    }
}