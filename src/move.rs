use crate::bitboard::{pretty_string_square, Square};

pub type Move = u16;

pub const NULL_MOVE:    Move = 0x0000;
pub const PASSING_MOVE: Move = 0xdfff;

const BIT_MASK_12:  u16 = 0xfff;
const BIT_MASK_6:   u16 = 0x3f;
const BIT_MASK_2:   u16 = 0x3;

pub const NOT_SPECIAL_MOVE:        u8 = 0;
pub const CASTLE_SPECIAL_MOVE:     u8 = 1;
pub const PROMOTION_SPECIAL_MOVE:  u8 = 2;
pub const EN_PASSANT_SPECIAL_MOVE: u8 = 3;

pub const QUEEN_PROMOTION:   u8 = 0;
pub const ROOK_PROMOTION:    u8 = 1;
pub const BISHOP_PROMOTION:  u8 = 2;
pub const KNIGHT_PROMOTION:  u8 = 3;

pub const KING_CASTLE: u8 = 0b01;
pub const QUEEN_CASTLE: u8 = 0b10;

#[inline(always)]
pub const fn move_origin_square(m: Move) -> Square {
    (m & BIT_MASK_6) as u8
}

#[inline(always)]
pub const fn move_destination_square(m: Move) -> Square {
    ((m >> 6) & BIT_MASK_6) as u8
}

#[inline(always)]
pub const fn move_special_info(m: Move) -> u8 {
    ((m >> 12) & BIT_MASK_2) as u8
}

#[inline(always)]
pub const fn move_special_type(m: Move) -> u8 {
    ((m >> 14) & BIT_MASK_2) as u8
}

#[inline(always)]
pub const fn build_move(origin: Square, destination: Square, special_info: u8, special_type: u8) -> Move {
    (origin as Move) | ((destination as Move) << 6) | ((special_info as Move) << 12) | ((special_type as Move) << 14)
}

#[inline(always)]
pub const fn build_simple_move(origin: Square, destination: Square) -> Move {
    (origin as Move) | ((destination as Move) << 6)
}

pub fn debug_same_src_des(m1: Move, m2: Move) -> bool {
    return (m1 & BIT_MASK_12) == (m2 & BIT_MASK_12)
}

pub fn pretty_string_move(m: Move) -> String {
    if m == NULL_MOVE {
        return "Null Move".to_string();
    }
    if move_special_type(m) == CASTLE_SPECIAL_MOVE {
        if move_destination_square(m) == 2 || move_destination_square(m) == 58 {
            return "O-O-O".to_string();
        } else {
            return "O-O".to_string();
        }
    }
    let promotion_string = if move_special_type(m) == PROMOTION_SPECIAL_MOVE {
        match move_special_info(m) {
            QUEEN_PROMOTION => "=Q",
            ROOK_PROMOTION => "=R",
            BISHOP_PROMOTION => "=B",
            KNIGHT_PROMOTION => "=N",
            _ => unreachable!(),
        }} else {""};
        pretty_string_square(move_origin_square(m)) +
            &pretty_string_square(move_destination_square(m)) +
            promotion_string
}