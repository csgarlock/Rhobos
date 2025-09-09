use crate::{bitboard::{bit_count, Color}, state::State};

pub type Evaluation = i32;

pub const CENTI_PAWN: Evaluation = 66536;
pub const QUEEN_EVAL: Evaluation = CENTI_PAWN * 900;
pub const ROOK_EVAL: Evaluation = CENTI_PAWN * 500;
pub const BISHOP_EVAL: Evaluation = CENTI_PAWN * 325;
pub const KNIGHT_EVAL: Evaluation = CENTI_PAWN * 300;
pub const PAWN_EVAL: Evaluation = CENTI_PAWN * 100;
pub const PIECE_EVAL_TABLE: [Evaluation; 6] = [0, QUEEN_EVAL, ROOK_EVAL, BISHOP_EVAL, KNIGHT_EVAL, PAWN_EVAL];

pub const LOWEST_EVAL: Evaluation = -2147483646 + CENTI_PAWN - 2; // The lowest 32 bit value such that the 16 least significant bits are all 0
pub const HIGHEST_EVAL: Evaluation = 2147483646 - CENTI_PAWN + 2; // The lowest 32 bit value such that the 16 least significant bits are all 0


impl State {
    pub fn eval_state(&self) -> Evaluation {
        let mut eval: Evaluation = 0;

        for piece_type in 1..6 {
            let white_count = bit_count(unsafe { self.get_piece_board_raw(Color::White as u8, piece_type) });
            let black_count = bit_count(unsafe { self.get_piece_board_raw(Color::Black as u8, piece_type) });
            eval += white_count as i32 * PIECE_EVAL_TABLE[piece_type as usize];
            eval -= black_count as i32 * PIECE_EVAL_TABLE[piece_type as usize];
        }

        match self.turn {
            Color::White => eval,
            Color::Black => -eval,
        }
    }
}