use std::marker::ConstParamTy;
use crate::{bitboard::{board_from_square, get_lsb, pop_lsb, Bitboard, Board, Color, Square, EMPTY_BITBOARD, NULL_SQUARE, UNIVERSAL_BITBOARD}, histories::{CaptureEntry, CastleHistoryEntry, EnPassantEntry, FiftyMoveHistory, History}, r#move::{build_move, build_simple_move, CASTLE_SPECIAL_MOVE}, move_list::{MoveList, MoveStack}, piece_info::{move_bitboard, PieceType}};

#[repr(u8)]
#[derive(Clone, Copy, ConstParamTy, PartialEq, Eq)]
pub enum MoveGenType {
    All,
    Quiet,
    Capture,
}

#[repr(u8)]
#[derive(Clone, Copy, ConstParamTy, PartialEq, Eq)]
pub enum CastleAvailability {
    None  = 0b00,
    King  = 0b01,
    Queen = 0b10,
    Both  = 0b11,
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
    move_stack:          MoveStack,
    castle_availability: [CastleAvailability; 2],
    capture_history:     History<CaptureEntry>,
    en_passant_history:  History<EnPassantEntry>,
    castle_history:      History<CastleHistoryEntry>,
    fifty_move_history:  History<FiftyMoveHistory>,
    hash_history:        History<u64>,
}

impl State {
    pub fn gen_all_moves<const C: Color, const G: MoveGenType>(&mut self) {
        debug_assert!(!self.check);
        let mask = match G {
            MoveGenType::Capture => self.side_occupied[C.other() as usize],
            MoveGenType::Quiet => self.not_occupied,
            MoveGenType::All => !self.side_occupied[C as usize],
        };
        self.gen_piece_moves::<C, {PieceType::Queen}>(mask);
        self.gen_piece_moves::<C, {PieceType::Rook}>(mask);
        self.gen_piece_moves::<C, {PieceType::Bishop}>(mask);
        self.gen_piece_moves::<C, {PieceType::Knight}>(mask);
        self.gen_king_moves::<C>(mask);
        self.gen_pawn_moves::<C>(mask);
    }

    #[inline]
    pub fn gen_piece_moves<const C: Color, const P: PieceType>(&mut self, mask: Bitboard) {
        debug_assert!(P != PieceType::King && P != PieceType::Pawn);
        let mut piece_board = self.get_piece_board(C, P);
        while piece_board != EMPTY_BITBOARD {
            let src_square = pop_lsb(&mut piece_board);
            let mut move_board = move_bitboard::<P>(src_square, self.occupied) & mask;
            while move_board != EMPTY_BITBOARD {
                let des_square = pop_lsb(&mut move_board);
                self.move_stack.push_current(build_simple_move(src_square, des_square));
            }
        }
    }

    #[inline]
    pub fn gen_king_moves<const C: Color>(&mut self, mask: Bitboard) {
        let king_sqaure = get_lsb(self.get_piece_board(C, PieceType::King));
        let mut move_board = move_bitboard::<{ PieceType::King}>(king_sqaure, self.occupied) & mask;
        while move_board != EMPTY_BITBOARD {
            let des_square = pop_lsb(&mut move_board);
            if self.is_square_safe::<C>(des_square) {
                self.move_stack.push_current(build_simple_move(king_sqaure, des_square));
            }
        }
        if king_sqaure == State::king_square::<C>(){
            match self.castle_availability[C as usize] {
                CastleAvailability::Both => {
                    self.gen_king_castle::<C, { CastleAvailability::King }>();
                    self.gen_king_castle::<C, { CastleAvailability::Queen }>();
                },
                CastleAvailability::King => self.gen_king_castle::<C, { CastleAvailability::King }>(),
                CastleAvailability::Queen => self.gen_king_castle::<C, { CastleAvailability::Queen }>(),
                CastleAvailability::None => (),
            }
        }
    }

    #[inline(always)]
    pub fn gen_king_castle<const C: Color, const A: CastleAvailability>(&mut self) {
        debug_assert!(A != CastleAvailability::Both && A != CastleAvailability::None);
        let color_shift = C.castle_shift();
        let king_square = 4 + color_shift;
        if (
            // Check if rook is present on correct square
            board_from_square(CastleAvailability::rook_square::<C, A>()) & self.get_piece_board(C, PieceType::Rook) != EMPTY_BITBOARD &&
            // Check if squares in between king and rook are free
            CastleAvailability::through_squares::<C, A>() & self.occupied == EMPTY_BITBOARD &&
            // Check if all squares in between king and rook are not attacked by another piece
            self.are_castle_through_squares_safe::<C, A>()
        ) {
            self.move_stack.push_current(build_move(
                king_square,
                CastleAvailability::des_square::<C, A>(),
                0,
                CASTLE_SPECIAL_MOVE)
            );
        }
    }

    #[inline]
    pub fn gen_pawn_moves<const C: Color>(&mut self, mask: Bitboard) {
        todo!()
    }

    #[inline(always)]
    pub fn is_square_safe<const C: Color>(&self, square: Square) -> bool {
        todo!()
    }

    #[inline(always)]
    pub const fn get_piece_board(&self, color: Color, piece_type: PieceType) -> Bitboard {
        self.board[piece_type.colored_value(color) as usize]
    }

    #[inline(always)]
    pub fn set_piece_board(&mut self, bitboard: Bitboard, color: Color, piece_type: PieceType) {
        self.board[piece_type.colored_value(color) as usize] = bitboard;
    }

    #[inline(always)]
    pub const fn king_square<const C: Color>() -> Square {
        match C {
            Color::White => 4,
            Color::Black => 60,
        }
    }

    #[inline(always)]
    pub fn are_castle_through_squares_safe<const C: Color, const A: CastleAvailability>(&self) -> bool {
        let color_shift = C.castle_shift();
        match A {
            CastleAvailability::King => {
                self.is_square_safe::<C>(4 >> color_shift) &&
                self.is_square_safe::<C>(5 >> color_shift)
            },
            CastleAvailability::Queen => {
                self.is_square_safe::<C>(2 >> color_shift) &&
                self.is_square_safe::<C>(3 >> color_shift)
            },
            _ => false,
        }
    }
}

impl CastleAvailability {
    #[inline(always)]
    pub const fn through_squares<const C: Color, const A: CastleAvailability>() -> Bitboard {
        (match A {
            CastleAvailability::King => 0x60,
            CastleAvailability::None => 0xE,
            _ => EMPTY_BITBOARD,
        }) >> C.castle_shift()
    }

    #[inline(always)]
    pub const fn rook_square<const C: Color, const A: CastleAvailability>() -> Square {
        (match A {
            CastleAvailability::King => 8,
            CastleAvailability::Queen => 0,
            _ => NULL_SQUARE,
        }) + C.castle_shift()
    }

    #[inline(always)]
    pub const fn des_square<const C: Color, const A: CastleAvailability>() -> Square {
        (match A {
            CastleAvailability::King => 7,
            CastleAvailability::Queen => 2,
            _ => NULL_SQUARE,
        }) + C.castle_shift()
    }
}