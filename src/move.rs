use crate::bitboard::{square_from_string, Square};


type Move = u16;

const NIL_MOVE:     Move = 0xffff;
const PASSING_MOVE: Move = 0xdfff;

const BIT_MASK_12:  u16 = 0xfff;
const BIT_MASK_6:   u16 = 0x3f;
const BIT_MASK_2:   u16 = 0x3;

const NOT_SPECIAL_MOVE:        u8 = 0;
const CASTLE_SPECIAL_MOVE:     u8 = 1;
const PROMOTION_SPECIAL_MOVE:  u8 = 2;
const EN_PASSANT_SPECIAL_MOVE: u8 = 3;

const QUEEN_PROMOTION:   u8 = 0;
const ROOK_PROMOTION:    u8 = 1;
const KNIGHT_PROMOTION:  u8 = 2;
const BISHOP_PROMOTION:  u8 = 3;

#[inline(always)]
pub const fn move_origin_square(m: Move) -> Square {
    (m & BIT_MASK_6) as u8
}

#[inline(always)]
pub const fn move_destination_square(m: Move) -> Square {
    ((m >> 6) & BIT_MASK_6) as u8
}

#[inline(always)]
pub const fn move_promotion_type(m: Move) -> u8 {
    ((m >> 12) & BIT_MASK_2) as u8
}

#[inline(always)]
pub const fn move_special_type(m: Move) -> u8 {
    ((m >> 14) & BIT_MASK_2) as u8
}

#[inline(always)]
pub const fn build_move(origin: Square, destination: Square, promotion: u8, special_type: u8) -> Move {
    (origin as Move) | ((destination as Move) << 6) | ((promotion as Move) << 12) | ((special_type as Move) << 14)
}

#[inline(always)]
pub const fn build_simple_move(origin: Square, destination: Square) -> Move {
    (origin as Move) | ((destination as Move) << 6)
}

pub fn simple_move_from_string(move_string: String) -> Move {
    build_simple_move(square_from_string(move_string[0..2].to_string()), square_from_string(move_string[2..4].to_string()))
}