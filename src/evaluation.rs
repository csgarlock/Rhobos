use std::cmp::min;

use crate::{bitboard::{bit_count, get_lsb, pop_lsb, Bitboard, Color, COLORS, EMPTY_BITBOARD}, piece_info::{move_bitboard, PieceType, BLACK_KING, KING, MOVE_BOARDS, WHITE_BISHOP, WHITE_KING, WHITE_KNIGHT, WHITE_PAWN, WHITE_QUEEN, WHITE_ROOK}, search::Depth, state::State};

pub type Evaluation = i32;

pub const CENTI_PAWN: Evaluation = 65536;

pub const KING_EVAL: Evaluation = 0; 
pub const QUEEN_EVAL: Evaluation = CENTI_PAWN * 900;
pub const ROOK_EVAL: Evaluation = CENTI_PAWN * 500;
pub const BISHOP_EVAL: Evaluation = CENTI_PAWN * 325;
pub const KNIGHT_EVAL: Evaluation = CENTI_PAWN * 300;
pub const PAWN_EVAL: Evaluation = CENTI_PAWN * 100;
pub const PIECE_EVAL_TABLE: [Evaluation; 6] = [KING_EVAL, QUEEN_EVAL, ROOK_EVAL, BISHOP_EVAL, KNIGHT_EVAL, PAWN_EVAL];

pub const KING_PHASE_VALUE: u8 = 0;
pub const QUEEN_PHASE_VALUE: u8 = 4;
pub const ROOK_PHASE_VALUE: u8 = 2;
pub const BISHOP_PHASE_VALUE: u8 = 1;
pub const KNIGHT_PHASE_VALUE: u8 = 1;
pub const PAWN_PHASE_VALUE: u8 = 0;
pub const PIECE_PHASE_TABLE: [u8; 6] = [KING_PHASE_VALUE, QUEEN_PHASE_VALUE, ROOK_PHASE_VALUE, BISHOP_PHASE_VALUE, KNIGHT_PHASE_VALUE, PAWN_PHASE_VALUE];
pub const TOTAL_PHASE_VALUE: u8 = (KING_PHASE_VALUE + QUEEN_PHASE_VALUE + ROOK_PHASE_VALUE*2 + BISHOP_PHASE_VALUE*2 + KNIGHT_PHASE_VALUE*2 + PAWN_PHASE_VALUE*8) * 2;

pub const MOBILITY_VALUE: Evaluation = 1 * CENTI_PAWN;

pub const KING_KING_SAFETY_VALUE: u32 = 1;
pub const QUEEN_KING_SAFETY_VALUE: u32 = 2;
pub const ROOK_KING_SAFETY_VALUE: u32 = 2;
pub const BISHOP_KING_SAFETY_VALUE: u32 = 3;
pub const KNIGHT_KING_SAFETY_VALUE: u32 = 5;
pub const PAWN_KING_SAFETY_VALUE: u32 = 1;
pub const PIECE_KING_SAFETY_VALUES: [u32; 6] = [KING_KING_SAFETY_VALUE, QUEEN_KING_SAFETY_VALUE, ROOK_KING_SAFETY_VALUE, BISHOP_KING_SAFETY_VALUE, KNIGHT_KING_SAFETY_VALUE, PAWN_KING_SAFETY_VALUE];

pub const KING_SAFETY_TABLE: [Evaluation; 100] = king_safety_table();

pub const LOWEST_EVAL: Evaluation = -2147483646 + CENTI_PAWN - 2; // The lowest 32 bit value such that the 16 least significant bits are all 0
pub const HIGHEST_EVAL: Evaluation = 2147483646 - CENTI_PAWN + 2; // The lowest 32 bit value such that the 16 least significant bits are all 0

pub const NEGATIVE_MATE_ZERO: Evaluation = LOWEST_EVAL + CENTI_PAWN;
pub const POSITIVE_MATE_ZERO: Evaluation = HIGHEST_EVAL - CENTI_PAWN;

pub const MATE_VALUE_CUTOFF: Evaluation = CENTI_PAWN * 30_000;

static mut MIDGAME_PIECE_SQUARE_TABLE: [[Evaluation; 64]; 12] = [[0; 64]; 12];
static mut ENDGAME_PIECE_SQUARE_TABLE: [[Evaluation; 64]; 12] = [[0; 64]; 12];

impl State {
    pub fn eval_state(&self, perspective: Color) -> Evaluation {
        let mut eval: Evaluation = 0;

        let mut phase_value = 0;
        let mut midgame_table_eval = 0;
        let mut endgame_table_eval = 0;

        for piece_type in 1..6 {
            let white_count = bit_count(unsafe { self.get_piece_board_raw(Color::White as u8, piece_type) });
            let black_count = bit_count(unsafe { self.get_piece_board_raw(Color::Black as u8, piece_type) });
            eval += white_count as i32 * PIECE_EVAL_TABLE[piece_type as usize];
            eval -= black_count as i32 * PIECE_EVAL_TABLE[piece_type as usize];
            phase_value += (white_count + black_count) as u8 * PIECE_PHASE_TABLE[piece_type as usize];
            let mut white_board = unsafe { self.get_piece_board_raw(Color::White as u8, piece_type) };
            while white_board != EMPTY_BITBOARD {
                let square = pop_lsb(&mut white_board);
                midgame_table_eval += unsafe { MIDGAME_PIECE_SQUARE_TABLE[piece_type as usize][square as usize] };
                endgame_table_eval += unsafe { ENDGAME_PIECE_SQUARE_TABLE[piece_type as usize][square as usize] };
            }
            let mut black_board = unsafe { self.get_piece_board_raw(Color::Black as u8, piece_type) };
            while black_board != EMPTY_BITBOARD {
                let square = pop_lsb(&mut black_board);
                midgame_table_eval -= unsafe { MIDGAME_PIECE_SQUARE_TABLE[piece_type as usize + 6][square as usize] };
                endgame_table_eval -= unsafe { ENDGAME_PIECE_SQUARE_TABLE[piece_type as usize + 6][square as usize] };
            }
        }

        let white_king_square = get_lsb(self.get_piece_board(Color::White, PieceType::King));
        midgame_table_eval += unsafe { MIDGAME_PIECE_SQUARE_TABLE[WHITE_KING as usize][white_king_square as usize] };
        endgame_table_eval += unsafe { ENDGAME_PIECE_SQUARE_TABLE[WHITE_KING as usize][white_king_square as usize] };

        let black_king_square = get_lsb(self.get_piece_board(Color::Black, PieceType::King));
        midgame_table_eval -= unsafe { MIDGAME_PIECE_SQUARE_TABLE[BLACK_KING as usize][black_king_square as usize] };
        endgame_table_eval -= unsafe { ENDGAME_PIECE_SQUARE_TABLE[BLACK_KING as usize][black_king_square as usize] };

        let midgame_phase_val = u8::min(phase_value, TOTAL_PHASE_VALUE);
        let endgame_phase_val = TOTAL_PHASE_VALUE - midgame_phase_val;
        eval += (midgame_table_eval * midgame_phase_val as i32 + endgame_table_eval * endgame_phase_val as i32) / TOTAL_PHASE_VALUE as i32;

        for color in COLORS {
            let king_square = get_lsb(self.get_piece_board(color, PieceType::King));
            let king_neighbors = unsafe {MOVE_BOARDS[KING as usize][king_square as usize]};
            let mut running_mobility_count = 0;
            let mut running_king_attacks = 0;

            let (mobility_count, king_attacks) = self.mobility_and_king_attacks::<{PieceType::King}>(color, king_neighbors);
            running_mobility_count += mobility_count; running_king_attacks += king_attacks;
            let (mobility_score, king_attacks) = self.mobility_and_king_attacks::<{PieceType::Queen}>(color, king_neighbors);
            running_mobility_count += mobility_score; running_king_attacks += king_attacks;
            let (mobility_score, king_attacks) = self.mobility_and_king_attacks::<{PieceType::Rook}>(color, king_neighbors);
            running_mobility_count += mobility_score; running_king_attacks += king_attacks;
            let (mobility_score, king_attacks) = self.mobility_and_king_attacks::<{PieceType::Bishop}>(color, king_neighbors);
            running_mobility_count += mobility_score; running_king_attacks += king_attacks;
            let (mobility_score, king_attacks) = self.mobility_and_king_attacks::<{PieceType::Knight}>(color, king_neighbors);
            running_mobility_count += mobility_score; running_king_attacks += king_attacks;
            let (mobility_score, king_attacks) = self.mobility_and_king_attacks::<{PieceType::Pawn}>(color, king_neighbors);
            running_mobility_count += mobility_score; running_king_attacks += king_attacks;

            match color {
                Color::White => {
                    eval += running_mobility_count as Evaluation * MOBILITY_VALUE;
                    eval += KING_SAFETY_TABLE[min(running_king_attacks, 99) as usize] * CENTI_PAWN;
                },
                Color::Black => {
                    eval -= running_mobility_count as Evaluation * MOBILITY_VALUE;
                    eval -= KING_SAFETY_TABLE[min(running_king_attacks, 99) as usize] * CENTI_PAWN;
                },
            }
        }

        match perspective {
            Color::White => eval,
            Color::Black => -eval,
        }.clamp(-MATE_VALUE_CUTOFF, MATE_VALUE_CUTOFF)

    }

    #[inline(always)]
    fn mobility_and_king_attacks<const P: PieceType>(&self, color: Color, king_neighbors: Bitboard) -> (u32, u32) {
        let mut mobility_count = 0;
        let mut king_attacks = 0;
        let mut piece_board = self.get_piece_board(color, P);
        if P != PieceType::Pawn {
            while piece_board != EMPTY_BITBOARD {
                let piece_square = pop_lsb(&mut piece_board);
                let moves = move_bitboard::<P>(piece_square, self.occupied);
                mobility_count += bit_count(moves);
                king_attacks += bit_count(moves & king_neighbors) * PIECE_KING_SAFETY_VALUES[P as usize];
            }
        } else {
            
        }
        (mobility_count, king_attacks)
    }
}

#[cold]
pub fn eval_info_init() {
    piece_square_table_init();
}

#[cold]
fn piece_square_table_init() {
    unsafe {
        MIDGAME_PIECE_SQUARE_TABLE[WHITE_KING as usize] = [
            20, 30, 10, 0, 0, 10, 30, 20,
            20, 20, 0, 0, 0, 0, 20, 20,
            -10, -20, -20, -20, -20, -20, -20, -10,
            -20, -30, -30, -40, -40, -30, -30, -20,
            -30, -40, -40, -50, -50, -40, -40, -30,
            -30, -40, -40, -50, -50, -40, -40, -30,
            -30, -40, -40, -50, -50, -40, -40, -30,
            -30, -40, -40, -50, -50, -40, -40, -30,
        ];
        ENDGAME_PIECE_SQUARE_TABLE[WHITE_KING as usize] = [
            -50, -30, -30, -30, -30, -30, -30, -50,
            -30, -30, 0, 0, 0, 0, -30, -30,
            -30, -10, 20, 30, 30, 20, -10, -30,
            -30, -10, 30, 40, 40, 30, -10, -30,
            -30, -10, 30, 40, 40, 30, -10, -30,
            -30, -10, 20, 30, 30, 20, -10, -30,
            -30, -20, -10, 0, 0, -10, -20, -30,
            -50, -40, -30, -20, -20, -30, -40, -50,
        ];
        MIDGAME_PIECE_SQUARE_TABLE[WHITE_QUEEN as usize] = [
            -20, -10, -10, -5, -5, -10, -10, -20,
            -10, 0, 5, 0, 0, 0, 0, -10,
            -10, 5, 5, 5, 5, 5, 0, -10,
            0, 0, 5, 5, 5, 5, 0, -5,
            -5, 0, 5, 5, 5, 5, 0, -5,
            -10, 0, 5, 5, 5, 5, 0, -10,
            -10, 0, 0, 0, 0, 0, 0, -10,
            -20, -10, -10, -5, -5, -10, -10, -20,
        ];
        ENDGAME_PIECE_SQUARE_TABLE[WHITE_QUEEN as usize] = [
            -20, -10, -10, -5, -5, -10, -10, -20,
            -10, 0, 5, 0, 0, 0, 0, -10,
            -10, 5, 5, 5, 5, 5, 0, -10,
            0, 0, 5, 5, 5, 5, 0, -5,
            -5, 0, 5, 5, 5, 5, 0, -5,
            -10, 0, 5, 5, 5, 5, 0, -10,
            -10, 0, 0, 0, 0, 0, 0, -10,
            -20, -10, -10, -5, -5, -10, -10, -20,
        ];
        MIDGAME_PIECE_SQUARE_TABLE[WHITE_ROOK as usize] = [
            0, 0, 0, 5, 5, 0, 0, 0,
            -5, 0, 0, 0, 0, 0, 0, -5,
            -5, 0, 0, 0, 0, 0, 0, -5,
            -5, 0, 0, 0, 0, 0, 0, -5,
            -5, 0, 0, 0, 0, 0, 0, -5,
            -5, 0, 0, 0, 0, 0, 0, -5,
            5, 10, 10, 10, 10, 10, 10, 5,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        ENDGAME_PIECE_SQUARE_TABLE[WHITE_ROOK as usize] = [
            0, 0, 0, 5, 5, 0, 0, 0,
            -5, 0, 0, 0, 0, 0, 0, -5,
            -5, 0, 0, 0, 0, 0, 0, -5,
            -5, 0, 0, 0, 0, 0, 0, -5,
            -5, 0, 0, 0, 0, 0, 0, -5,
            -5, 0, 0, 0, 0, 0, 0, -5,
            5, 10, 10, 10, 10, 10, 10, 5,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        MIDGAME_PIECE_SQUARE_TABLE[WHITE_BISHOP as usize] = [
            -20, -10, -10, -10, -10, -10, -10, -20,
            -10, 5, 0, 0, 0, 0, 5, -10,
            -10, 10, 10, 10, 10, 10, 10, -10,
            -10, 0, 10, 10, 10, 10, 0, -10,
            -10, 5, 5, 10, 10, 5, 5, -10,
            -10, 0, 5, 10, 10, 5, 0, -10,
            -10, 0, 0, 0, 0, 0, 0, -10,
            -20, -10, -10, -10, -10, -10, -10, -20,
        ];
        ENDGAME_PIECE_SQUARE_TABLE[WHITE_BISHOP as usize] = [
            -20, -10, -10, -10, -10, -10, -10, -20,
            -10, 5, 0, 0, 0, 0, 5, -10,
            -10, 10, 10, 10, 10, 10, 10, -10,
            -10, 0, 10, 10, 10, 10, 0, -10,
            -10, 5, 5, 10, 10, 5, 5, -10,
            -10, 0, 5, 10, 10, 5, 0, -10,
            -10, 0, 0, 0, 0, 0, 0, -10,
            -20, -10, -10, -10, -10, -10, -10, -20,
        ];
        MIDGAME_PIECE_SQUARE_TABLE[WHITE_KNIGHT as usize] = [
            -50, -40, -30, -30, -30, -30, -40, -50,
            -40, -20, 0, 5, 5, 0, -20, -40,
            -30, 5, 10, 15, 15, 10, 5, -30,
            -30, 0, 15, 20, 20, 15, 0, -30,
            -30, 5, 15, 20, 20, 15, 5, -30,
            -30, 0, 10, 15, 15, 10, 0, -30,
            -40, -20, 0, 0, 0, 0, -20, -40,
            -50, -40, -30, -30, -30, -30, -40, -50,
        ];
        ENDGAME_PIECE_SQUARE_TABLE[WHITE_KNIGHT as usize] = [
            -50, -40, -30, -30, -30, -30, -40, -50,
            -40, -20, 0, 5, 5, 0, -20, -40,
            -30, 5, 10, 15, 15, 10, 5, -30,
            -30, 0, 15, 20, 20, 15, 0, -30,
            -30, 5, 15, 20, 20, 15, 5, -30,
            -30, 0, 10, 15, 15, 10, 0, -30,
            -40, -20, 0, 0, 0, 0, -20, -40,
            -50, -40, -30, -30, -30, -30, -40, -50,
        ];
        MIDGAME_PIECE_SQUARE_TABLE[WHITE_PAWN as usize] = [
            0, 0, 0, 0, 0, 0, 0, 0,
            5, 10, 10, -20, -20, 10, 10, 5,
            5, -5, -10, 0, 0, -10, -5, 5,
            0, 0, 0, 20, 20, 0, 0, 0,
            5, 5, 10, 25, 25, 10, 5, 5,
            10, 10, 20, 30, 30, 20, 10, 10,
            50, 50, 50, 50, 50, 50, 50, 50,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        ENDGAME_PIECE_SQUARE_TABLE[WHITE_PAWN as usize] = [
            0, 0, 0, 0, 0, 0, 0, 0,
            -30, -30, -30, -30, -30, -30, -30, -30,
            -10, -10, -10, -10, -10, -10, -10, -10,
            0, 0, 0, 0, 0, 0, 0, 0,
            20, 20, 20, 20, 20, 20, 20, 20,
            40, 40, 40, 40, 40, 40, 40, 40,
            60, 60, 60, 60, 60, 60, 60, 60,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        for piece_type in 0..6 {
            for square in 0..64 {
                MIDGAME_PIECE_SQUARE_TABLE[piece_type][square] *= CENTI_PAWN;
                ENDGAME_PIECE_SQUARE_TABLE[piece_type][square] *= CENTI_PAWN;
            }
        }
        for piece_type in 0..6 {
            for square in 0..64 {
                MIDGAME_PIECE_SQUARE_TABLE[piece_type + 6][square ^ 56] = MIDGAME_PIECE_SQUARE_TABLE[piece_type][square];
                ENDGAME_PIECE_SQUARE_TABLE[piece_type + 6][square ^ 56] = ENDGAME_PIECE_SQUARE_TABLE[piece_type][square];
            }
        }
    }
}

const fn king_safety_table() -> [Evaluation; 100] {
    [0,  0,   1,   2,   3,   5,   7,   9,  12,  15,
    18,  22,  26,  30,  35,  39,  44,  50,  56,  62,
    68,  75,  82,  85,  89,  97, 105, 113, 122, 131,
    140, 150, 169, 180, 191, 202, 213, 225, 237, 248,
    260, 272, 283, 295, 307, 319, 330, 342, 354, 366,
    377, 389, 401, 412, 424, 436, 448, 459, 471, 483,
    494, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500,]
}

#[inline(always)]
pub fn unchecked_eval_clamp(val: Evaluation, min: Evaluation, max: Evaluation) -> Evaluation {
    debug_assert!(min <= max);
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

#[inline(always)]
pub fn mate_in(mate_in: Depth, negative: bool) -> Evaluation {
    debug_assert!(mate_in >= 0 && mate_in < 128);
    if negative {
        NEGATIVE_MATE_ZERO + (mate_in * CENTI_PAWN)
    } else {
        POSITIVE_MATE_ZERO - (mate_in * CENTI_PAWN)
    }
}

#[inline(always)]
pub fn mate_depth(eval: Evaluation) -> Depth {
    debug_assert!(eval.abs() > MATE_VALUE_CUTOFF);
    if eval > MATE_VALUE_CUTOFF {
        (POSITIVE_MATE_ZERO - eval) / CENTI_PAWN
    } else {
        (NEGATIVE_MATE_ZERO - eval) / CENTI_PAWN
    }
}

pub fn pretty_string_eval(raw_eval: Evaluation) -> String {
    if raw_eval == HIGHEST_EVAL {
        "MAX".to_string()
    }
    else if raw_eval == LOWEST_EVAL {
        "MIN".to_string()
    } else if raw_eval.abs() > MATE_VALUE_CUTOFF {
        format!("M{}", mate_depth(raw_eval))
    } else {
        let centi_eval = (raw_eval / CENTI_PAWN) as f64;
        format!("{:.2}", centi_eval / 100.0)
    }
}