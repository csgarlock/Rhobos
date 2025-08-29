use crate::{bitboard::{Bitboard, Board, Color, Square}, histories::{CaptureEntry, CastleHistoryEntry, EnPassantEntry, FiftyMoveHistory, History}};

type CastleAvailability = [bool; 4];

struct State {
    board:               Board,
    side_occupied:       [Bitboard; 2],
    occupied:            Bitboard,
    not_occupied:        Bitboard,
    turn:                Color,
    can_en_passant:      bool,
    en_passant_square:   Square,
    check:               bool,
    hashcode:            u64,
    castle_availability: CastleAvailability,
    capture_history:     History<CaptureEntry>,
    en_passant_history:  History<EnPassantEntry>,
    castle_history:      History<CastleHistoryEntry>,
    fifty_move_history:  History<FiftyMoveHistory>,
    hash_history:        History<u64>,
}