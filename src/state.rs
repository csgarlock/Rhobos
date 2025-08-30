use std::marker::ConstParamTy;
use crate::{bitboard::{Bitboard, Board, Color, Square}, histories::{CaptureEntry, CastleHistoryEntry, EnPassantEntry, FiftyMoveHistory, History}};

type CastleAvailability = [bool; 4];


#[repr(u8)]
#[derive(Clone, Copy, ConstParamTy, PartialEq, Eq)]
pub enum MoveGenType {
    All,
    Quiet,
    Capture,
}

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

impl State {
    pub fn gen_all_moves<const C: Color, const G: MoveGenType>(&self) {
        if G == MoveGenType::All {
            self.gen_all_moves::<C, {MoveGenType::Capture}>();
            self.gen_all_moves::<C, {MoveGenType::Quiet}>();
            return;
        }
    }
}