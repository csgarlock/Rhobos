use core::fmt;
use std::{fmt::Display, marker::ConstParamTy, mem::transmute};
use crate::{bitboard::{board_from_square, get_lsb, is_valid_square, pop_lsb, pretty_string_bitboard, shift_bitboard, Bitboard, Board, Color, Square, EMPTY_BITBOARD, NULL_SQUARE}, histories::{CaptureEntry, CastleHistoryEntry, EnPassantEntry, FiftyMoveHistory, History}, r#move::{build_move, build_simple_move, move_destination_square, move_origin_square, move_special_info, move_special_type, Move, BISHOP_PROMOTION, CASTLE_SPECIAL_MOVE, EN_PASSANT_SPECIAL_MOVE, KNIGHT_PROMOTION, NULL_MOVE, PROMOTION_SPECIAL_MOVE, QUEEN_PROMOTION, ROOK_PROMOTION}, move_list::MoveStack, piece_info::{make_step, move_bitboard, Direction, PieceType, Step, PAWN_ATTACK_BOARDS}};

#[repr(u8)]
#[derive(Clone, Copy, ConstParamTy, PartialEq, Eq)]
pub enum CastleAvailability {
    None  = 0b00,
    King  = 0b01,
    Queen = 0b10,
    Both  = 0b11,
}

pub struct State {
    pub board:               Board,
    pub side_occupied:       [Bitboard; 2],
    pub occupied:            Bitboard,
    pub not_occupied:        Bitboard,
    pub turn:                Color,
    pub ply:                 u16,
    pub can_en_passant:      bool,
    pub en_passant_square:   Square,
    pub check:               bool,
    pub hashcode:            u64,
    pub half_move_clock:     u8,
    pub move_stack:          MoveStack,
    pub castle_availability: [CastleAvailability; 2],
    pub capture_history:     History<CaptureEntry>,
    pub en_passant_history:  History<EnPassantEntry>,
    pub castle_history:      History<CastleHistoryEntry>,
    pub fifty_move_history:  History<FiftyMoveHistory>,
    pub hash_history:        History<u64>,
}

impl State {

    pub fn make_move<const C: Color>(&mut self, m: Move) {
        debug_assert_ne!(m, NULL_MOVE);
        let src_square = move_origin_square(m);
        let des_square = move_destination_square(m);
        debug_assert!(src_square < 64 && des_square < 64);

        let src_piece_type = self.force_get_colored_piece_at_square::<C>(src_square);
        self.clear_square(src_square, C, src_piece_type);
        self.set_square(des_square, C, src_piece_type);

        let des_piece_type_option = match C {
            Color::White => self.get_colored_piece_at_square::<{Color::Black}>(des_square),
            Color::Black => self.get_colored_piece_at_square::<{Color::White}>(des_square),
        };

        let is_capture = match des_piece_type_option {
            Some(des_piece_type) => {
                self.clear_square(des_square, C.other(), des_piece_type);
                self.capture_history.push(des_piece_type, self.ply);
                self.fifty_move_history.push(self.half_move_clock, self.ply);
                self.half_move_clock = 0;
                true
            },
            None => false
        };
        match move_special_type(m) {
            CASTLE_SPECIAL_MOVE => {
                debug_assert!(move_special_info(m) == 1 || move_special_info(m) == 2);
                let castle_type: CastleAvailability = unsafe { transmute(move_special_info(m)) };
                // let rook_square = match castle_type {
                //     CastleAvailability::King => {
                //         self.castle_availability[C as usize].
                //     },
                //     CastleAvailability::Queen => {

                //     },
                //     _ => unreachable!(),
                // }
            },
            PROMOTION_SPECIAL_MOVE => {
                let promotion_type = move_special_info(m);
                debug_assert!(promotion_type < 4);
                self.clear_square(des_square, C, PieceType::Pawn);
                unsafe { self.set_square_raw(des_square, C as u8, promotion_type + 1) };
            },
            EN_PASSANT_SPECIAL_MOVE => {
                let down_step = C.down() as Step;
                let en_passant_square = make_step(des_square, down_step);
                self.clear_square(en_passant_square, C.other(), PieceType::Pawn);
                self.capture_history.push(PieceType::Pawn, self.ply);
            }
            _ => (),
        }
    }

    pub fn unmake_move<const C: Color>(m: Move) {

    }
    
    #[inline(always)]
    pub fn is_square_safe<const C: Color, const U: bool>(&self, square: Square, un_set_square: Square) -> bool {
        let mut occupied = self.occupied;
        if U {
            debug_assert_ne!(un_set_square, NULL_SQUARE);
            // Often times this function will be desirable to call with a piece virtually removed. For example when checking if a king is moving into check
            // you would like to treat the old king square as if it was empty.
            occupied &= !board_from_square(un_set_square);
        }
        let bishop_board = move_bitboard::<{ PieceType::Bishop }>(square, occupied);
        if bishop_board & (self.get_piece_board(C.other(), PieceType::Queen) | self.get_piece_board(C.other(), PieceType::Bishop)) != EMPTY_BITBOARD { return false }

        let rook_board = move_bitboard::<{ PieceType::Rook }>(square, occupied);
        if rook_board & (self.get_piece_board(C.other(), PieceType::Queen) | self.get_piece_board(C.other(), PieceType::Rook)) != EMPTY_BITBOARD { return false }

        let knight_board = move_bitboard::<{ PieceType::Knight }>(square, occupied);
        if knight_board & self.get_piece_board(C.other(), PieceType::Knight) != EMPTY_BITBOARD { return false }

        let pawn_board = unsafe{PAWN_ATTACK_BOARDS[C as usize][square as usize]};
        if pawn_board & self.get_piece_board(C.other(), PieceType::Pawn) != EMPTY_BITBOARD { return false }

        true
    }

    #[inline(always)]
    pub const fn get_piece_board(&self, color: Color, piece_type: PieceType) -> Bitboard {
        self.board[piece_type.colored_value(color) as usize]
    }

    #[inline(always)]
    pub unsafe fn get_piece_board_raw(&self, color: u8, piece_type: u8) -> Bitboard {
        debug_assert!(color < 2);
        debug_assert!(piece_type < 6);
        unsafe { *self.board.get_unchecked((color * 6 + piece_type) as usize) }
    }

    #[inline(always)]
    pub fn set_piece_board(&mut self, bitboard: Bitboard, color: Color, piece_type: PieceType) {
        self.board[piece_type.colored_value(color) as usize] = bitboard;
    }

    #[inline(always)]
    pub unsafe fn set_piece_board_raw(&mut self, bitboard: Bitboard, color: u8, piece_type: u8) {
        debug_assert!(color < 2);
        debug_assert!(piece_type < 6);
        unsafe { *self.board.get_unchecked_mut((color * 6 + piece_type) as usize) = bitboard }
    }

    #[inline(always)]
    pub fn set_square(&mut self, square: Square, color: Color, piece_type: PieceType) {
        debug_assert!(is_valid_square(square));
        self.board[piece_type.colored_value(color) as usize] |= board_from_square(square);
    }

    #[inline(always)]
    pub unsafe fn set_square_raw(&mut self, square: Square, color: u8, piece_type: u8) {
        debug_assert!(color < 2);
        debug_assert!(piece_type < 6);
        debug_assert!(is_valid_square(square));
        unsafe { *self.board.get_unchecked_mut((color * 6 + piece_type) as usize) |= board_from_square(square) };
    }

    #[inline(always)]
    pub fn clear_square(&mut self, square: Square, color: Color, piece_type: PieceType){
        debug_assert!(is_valid_square(square));
        self.board[piece_type.colored_value(color) as usize] &= !board_from_square(square);
    }

    #[inline(always)]
    pub unsafe fn clear_square_raw(&mut self, square: Square, color: u8, piece_type: u8) {
        debug_assert!(color < 2);
        debug_assert!(piece_type < 6);
        debug_assert!(is_valid_square(square));
        unsafe { *self.board.get_unchecked_mut((color * 6 + piece_type) as usize) &= !board_from_square(square) };
    }

    #[inline(always)]
    pub fn update_occupied(&mut self) {
        let mut white_board = EMPTY_BITBOARD;
        let mut black_board = EMPTY_BITBOARD;
        for i in 0..6 {
            white_board |= self.board[i];
            black_board |= self.board[6 + i];
        }
        self.side_occupied = [white_board, black_board];
        self.occupied = white_board | black_board;
        self.not_occupied = !(white_board | black_board);
    }

    #[inline(always)]
    pub fn get_colored_piece_at_square<const C: Color>(&self, square: Square) -> Option<PieceType> {
        let piece_board = board_from_square(square);
        if self.get_piece_board(C, PieceType::King) & piece_board != EMPTY_BITBOARD { Some(PieceType::King) }
        else if self.get_piece_board(C, PieceType::Queen) & piece_board != EMPTY_BITBOARD { Some(PieceType::Queen) }
        else if self.get_piece_board(C, PieceType::Rook) & piece_board != EMPTY_BITBOARD { Some(PieceType::Rook) }
        else if self.get_piece_board(C, PieceType::Bishop) & piece_board != EMPTY_BITBOARD { Some(PieceType::Bishop) }
        else if self.get_piece_board(C, PieceType::Knight) & piece_board != EMPTY_BITBOARD { Some(PieceType::Knight) }
        else if self.get_piece_board(C, PieceType::Pawn) & piece_board != EMPTY_BITBOARD { Some(PieceType::Pawn) }
        else { None }
    }

    #[inline(always)]
    pub fn force_get_colored_piece_at_square<const C: Color>(&self, square: Square) -> PieceType {
        let piece_board = board_from_square(square);
        if self.get_piece_board(C, PieceType::King) & piece_board != EMPTY_BITBOARD { PieceType::King }
        else if self.get_piece_board(C, PieceType::Queen) & piece_board != EMPTY_BITBOARD { PieceType::Queen }
        else if self.get_piece_board(C, PieceType::Rook) & piece_board != EMPTY_BITBOARD { PieceType::Rook }
        else if self.get_piece_board(C, PieceType::Bishop) & piece_board != EMPTY_BITBOARD { PieceType::Bishop }
        else if self.get_piece_board(C, PieceType::Knight) & piece_board != EMPTY_BITBOARD { PieceType::Knight }
        else {
            debug_assert!(self.get_piece_board(C, PieceType::Pawn) & piece_board != EMPTY_BITBOARD);
            PieceType::Pawn
        }
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
                self.is_square_safe::<C, false>(4 >> color_shift, NULL_SQUARE) &&
                self.is_square_safe::<C, false>(5 >> color_shift, NULL_SQUARE)
            },
            CastleAvailability::Queen => {
                self.is_square_safe::<C, false>(2 >> color_shift, NULL_SQUARE) &&
                self.is_square_safe::<C, false>(3 >> color_shift, NULL_SQUARE)
            },
            _ => false,
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        const PIECE_MAP: [char; 12] = ['K', 'Q', 'R', 'B', 'N', 'P', 'k', 'q', 'r', 'b', 'n', 'p'];
        let mut result = ['\0'; 64];
        for (i, c) in PIECE_MAP.iter().enumerate() {
            let mut bitboard = self.board[i];
            while bitboard != EMPTY_BITBOARD {
                let spot = pop_lsb(&mut bitboard);
                result[spot as usize] = *c;
            }
        }
        let bottom_line = "  -----------------".to_string();
        write!(f, "{}\n", bottom_line);
        for i in (0..8).rev() {
            let mut line = format!("{} ", i+1);
            for j in 0..8 {
                let spot = result[i*8 + j];
                line += "|";
                if spot == '\0' {
                    line += " ";
                } else {
                    line += &spot.to_string();
                }
            }
            write!(f, "{}|\n{}\n", line, bottom_line);
        }
        write!(f, " a b c d e f g h\n");
        write!(f, "Turn: {}\n", if self.turn == Color::White {"White"} else {"Black"});
        if self.check {
            write!(f, "In Check\n");
        }
        Ok(())
    }
}
