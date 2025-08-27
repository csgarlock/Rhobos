use crate::bitboard::{Bitboard, Board, Color, Square};

struct State {
    board:             Board,
    side_occupied:     [Bitboard; 2],
    occupied:          Bitboard,
    not_occupied:      Bitboard,
    turn:              Color,
    en_passant_square: Square,
    check:             bool,
}