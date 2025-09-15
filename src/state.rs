use core::fmt;
use std::{fmt::Display, hint::unreachable_unchecked, marker::ConstParamTy, mem::transmute};
use crate::{bitboard::{board_from_square, file, get_lsb, is_valid_square, pop_lsb, rank, Bitboard, Board, Color, Square, EMPTY_BITBOARD, NULL_SQUARE}, hash::{BLACK_HASH, CASTLE_HASHES, EN_PASSANT_HASHES, SQUARE_HASHES}, histories::{CaptureEntry, CastleHistoryEntry, EnPassantEntry, FiftyMoveHistory, History, HistoryEntry}, r#move::{move_destination_square, move_origin_square, move_special_info, move_special_type, Move, CASTLE_SPECIAL_MOVE, EN_PASSANT_SPECIAL_MOVE, NOT_SPECIAL_MOVE, NULL_MOVE, PROMOTION_SPECIAL_MOVE}, move_list::MoveStack, piece_info::{make_step, move_bitboard, PieceType, Step, PAWN_ATTACK_BOARDS}, transposition::prefetch_tt_address};

#[repr(u8)]
#[derive(Clone, Copy, ConstParamTy, PartialEq, Eq, Debug)]
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
    pub check_history:       History<bool>,
}

impl State {

    pub fn make_move<const C: Color>(&mut self, m: Move) -> bool {
        debug_assert_eq!(C, self.turn);
        debug_assert_ne!(m, NULL_MOVE);

        let src_square = move_origin_square(m);
        let des_square = move_destination_square(m);
        debug_assert!(src_square < 64 && des_square < 64);
        
        self.en_passant_history.push(self.en_passant_square);
        self.castle_history.push(self.castle_availability[C as usize]);
        self.fifty_move_history.push(self.half_move_clock);
        self.hash_history.push(self.hashcode);
        self.check_history.push(self.check);
        let mut capture_entry = CaptureEntry::empty();

        self.clear_en_passant::<true>();

        let src_piece_type = self.force_get_colored_piece_at_square::<C>(src_square);
        self.clear_square::<true>(src_square, C, src_piece_type);
        self.set_square::<true>(des_square, C, src_piece_type);

        let des_piece_type_option = match C {
            Color::White => self.get_colored_piece_at_square::<{Color::Black}>(des_square),
            Color::Black => self.get_colored_piece_at_square::<{Color::White}>(des_square),
        };

        if let Some(des_piece_type) = des_piece_type_option {
            capture_entry = CaptureEntry{ piece: Some(des_piece_type), bitboard: self.get_piece_board(C.other(), des_piece_type) };
            self.clear_square::<true>(des_square, C.other(), des_piece_type);
            self.half_move_clock = 0;
        }

        match move_special_type(m) {
            NOT_SPECIAL_MOVE => {
                match src_piece_type {
                    PieceType::King => {
                        if src_square == State::king_square::<C>() {
                            match self.castle_availability[C as usize] {
                                CastleAvailability::Both => self.toggle_castle_availability::<C, {CastleAvailability::Both}, true>(),
                                CastleAvailability::King => self.toggle_castle_availability::<C, {CastleAvailability::King}, true>(),
                                CastleAvailability::Queen => self.toggle_castle_availability::<C, {CastleAvailability::Queen}, true>(),
                                CastleAvailability::None => (),
                            }
                        }
                    },
                    PieceType::Rook => {
                        if src_square == CastleAvailability::rook_square::<C, {CastleAvailability::King}>() {
                            self.clear_castle_availability::<C, {CastleAvailability::King}, true>();
                        } else if src_square == CastleAvailability::rook_square::<C, {CastleAvailability::Queen}>() {
                            self.clear_castle_availability::<C, {CastleAvailability::Queen}, true>();
                        }
                    }
                    PieceType::Pawn => {
                        let rank_diff = rank(src_square) as i8 - rank(des_square) as i8;
                        if rank_diff == 2 || rank_diff == -2 {
                            self.en_passant_square = make_step(src_square, C.up() as i8);
                            self.set_en_passant::<true>(self.en_passant_square);
                        }
                        self.half_move_clock = 0;
                    }
                    _ => (),
                }
            }
            CASTLE_SPECIAL_MOVE => {
                debug_assert!(move_special_info(m) == 1 || move_special_info(m) == 2);
                let castle_type: CastleAvailability = unsafe { transmute(move_special_info(m)) };
                let (old_rook_square, new_rook_square) = match castle_type {
                    CastleAvailability::King => { 
                        self.toggle_castle_availability::<C, {CastleAvailability::King}, true>();
                        (7 + C.castle_shift(), 5 + C.castle_shift())
                    },
                    CastleAvailability::Queen => {
                        self.toggle_castle_availability::<C, {CastleAvailability::Queen}, true>();
                        (C.castle_shift(), 3 + C.castle_shift())
                    },
                    _ => { debug_assert!(false); unsafe { unreachable_unchecked() }; },
                };
                self.clear_square::<true>(old_rook_square, C, PieceType::Rook);
                self.set_square::<true>(new_rook_square, C, PieceType::Rook);
            },
            PROMOTION_SPECIAL_MOVE => {
                let promotion_type = move_special_info(m);
                debug_assert!(promotion_type < 4);
                self.clear_square::<true>(des_square, C, PieceType::Pawn);
                unsafe { self.set_square_raw::<true>(des_square, C as u8, promotion_type + 1) };
            },
            EN_PASSANT_SPECIAL_MOVE => {
                let down_step = C.down() as Step;
                let en_passant_square = make_step(des_square, down_step);
                capture_entry = CaptureEntry { piece: Some(PieceType::Pawn), bitboard: self.get_piece_board(C.other(), PieceType::Pawn) };
                self.clear_square::<true>(en_passant_square, C.other(), PieceType::Pawn);
            }
            _ => { debug_assert!(false); unsafe { unreachable_unchecked() }; },
        }

        prefetch_tt_address(self);

        self.capture_history.push((capture_entry.piece, capture_entry.bitboard));
        self.update_occupied();

        self.turn = C.other();
        self.ply += 1;
        self.hashcode ^= unsafe { BLACK_HASH };
        debug_assert_eq!(self.hashcode, self.get_hash());

        self.move_stack.next();
        match C {
            Color::White => {
                self.check = !self.is_square_safe::<{Color::Black}, false>(
                    get_lsb(self.get_piece_board(Color::Black, PieceType::King)),
                    NULL_SQUARE
                );
            }
            Color::Black => {
                self.check = !self.is_square_safe::<{Color::White}, false>(
                    get_lsb(self.get_piece_board(Color::White, PieceType::King)),
                    NULL_SQUARE
                );
            }
        }

        // Quick legality for now
        match C {
            Color::White => self.is_square_safe::<{Color::White}, false>(
                get_lsb(self.get_piece_board(Color::White, PieceType::King)),
                NULL_SQUARE
            ),
            Color::Black => self.is_square_safe::<{Color::Black}, false>(
                get_lsb(self.get_piece_board(Color::Black, PieceType::King)),
                NULL_SQUARE
            )
        }
    }

    // C is the color that originally made the move
    pub fn unmake_move<const C: Color>(&mut self, m: Move) {
        debug_assert_eq!(C, self.turn.other());
        debug_assert_ne!(m, NULL_MOVE);

        let src_square = move_origin_square(m);
        let des_square = move_destination_square(m);
        debug_assert!(src_square < 64 && des_square < 64);

        self.en_passant_square = self.en_passant_history.pop().value();
        self.castle_availability[C as usize] = self.castle_history.pop().value();
        self.half_move_clock = self.fifty_move_history.pop().value();
        self.hashcode = self.hash_history.pop().value();
        self.check = self.check_history.pop().value();
        
        let src_piece_type = self.force_get_colored_piece_at_square::<C>(des_square);
        self.clear_square::<false>(des_square, C, src_piece_type);
        self.set_square::<false>(src_square, C, src_piece_type);
        
        let capture_entry = self.capture_history.pop().value();
        if let Some(piece_type) = capture_entry.0 {
            self.set_piece_board(capture_entry.1, C.other(), piece_type);
        }

        match move_special_type(m) {
            CASTLE_SPECIAL_MOVE => {
                debug_assert!(move_special_info(m) == 1 || move_special_info(m) == 2);
                let castle_type: CastleAvailability = unsafe { transmute(move_special_info(m)) };
                let (old_rook_square, new_rook_square) = match castle_type {
                    CastleAvailability::King => (7 + C.castle_shift(), 5 + C.castle_shift()),
                    CastleAvailability::Queen => (C.castle_shift(), 3 + C.castle_shift()),
                    _ => unreachable!(),
                };
                self.clear_square::<false>(new_rook_square, C, PieceType::Rook);
                self.set_square::<false>(old_rook_square, C, PieceType::Rook);
            },
            PROMOTION_SPECIAL_MOVE => {
                let promotion_type = move_special_info(m);
                debug_assert!(promotion_type < 4);
                self.set_square::<false>(src_square, C, PieceType::Pawn);
                unsafe { self.clear_square_raw::<false>(src_square, C as u8, promotion_type + 1) };
            },
            _ => (),
        }

        self.turn = C;
        self.ply -= 1;
        self.update_occupied();

        debug_assert_eq!(self.hashcode, self.get_hash());
        
        self.move_stack.previous();
    }

    pub fn non_reversible_move(&mut self, m: Move) -> bool {
        let result = match self.turn {
            Color::White => self.make_move::<{Color::White}>(m),
            Color::Black => self.make_move::<{Color::Black}>(m),
        };
        self.move_stack.previous();
        self.en_passant_history.pop();
        self.castle_history.pop();
        self.fifty_move_history.pop();
        self.hash_history.pop();
        self.check_history.pop();
        self.capture_history.pop();
        result
    }
    
    pub fn passing_move<const C: Color>(&mut self) {
        debug_assert!(!self.check);
        debug_assert_eq!(C, self.turn);

        // Many of these things not needed but to lazy to optimize for now
        self.en_passant_history.push(self.en_passant_square);
        self.castle_history.push(self.castle_availability[C as usize]);
        self.fifty_move_history.push(self.half_move_clock);
        self.hash_history.push(self.hashcode);
        self.check_history.push(self.check);
        self.capture_history.push((None, EMPTY_BITBOARD));

        self.clear_en_passant::<true>();
        self.turn = C.other();

        match C {
            Color::White => {
                self.check = !self.is_square_safe::<{Color::Black}, false>(
                    get_lsb(self.get_piece_board(Color::Black, PieceType::King)),
                    NULL_SQUARE
                );
            }
            Color::Black => {
                self.check = !self.is_square_safe::<{Color::White}, false>(
                    get_lsb(self.get_piece_board(Color::White, PieceType::King)),
                    NULL_SQUARE
                );
            }
        }
        
    }

    pub fn un_passing_move<const C: Color>(&mut self) {
        debug_assert_eq!(C.other(), self.turn);

        self.en_passant_square = self.en_passant_history.pop().value();
        self.castle_availability[C as usize] = self.castle_history.pop().value();
        self.half_move_clock = self.fifty_move_history.pop().value();
        self.hashcode = self.hash_history.pop().value();
        self.check = self.check_history.pop().value();
        self.capture_history.pop();

        self.turn = C;
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

        let king_board = move_bitboard::<{ PieceType::King }>(square, occupied);
        if king_board & self.get_piece_board(C.other(), PieceType::King) != EMPTY_BITBOARD { return false }

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
    pub fn set_square<const H: bool>(&mut self, square: Square, color: Color, piece_type: PieceType) {
        debug_assert!(is_valid_square(square));
        self.board[piece_type.colored_value(color) as usize] |= board_from_square(square);
        if H {
            self.hashcode ^= unsafe { SQUARE_HASHES[piece_type.colored_value(color) as usize][square as usize] }
        }
    }

    #[inline(always)]
    pub unsafe fn set_square_raw<const H: bool>(&mut self, square: Square, color: u8, piece_type: u8) {
        debug_assert!(color < 2);
        debug_assert!(piece_type < 6);
        debug_assert!(is_valid_square(square));
        unsafe { *self.board.get_unchecked_mut((color * 6 + piece_type) as usize) |= board_from_square(square) };
        if H {
            self.hashcode ^= unsafe { SQUARE_HASHES[(color * 6 + piece_type) as usize][square as usize] }
        }
    }

    #[inline(always)]
    pub fn clear_square<const H: bool>(&mut self, square: Square, color: Color, piece_type: PieceType){
        debug_assert!(is_valid_square(square));
        self.board[piece_type.colored_value(color) as usize] &= !board_from_square(square);
        if H {
            self.hashcode ^= unsafe { SQUARE_HASHES[piece_type.colored_value(color) as usize][square as usize] }
        }
    }

    #[inline(always)]
    pub unsafe fn clear_square_raw<const H: bool>(&mut self, square: Square, color: u8, piece_type: u8) {
        debug_assert!(color < 2);
        debug_assert!(piece_type < 6);
        debug_assert!(is_valid_square(square));
        unsafe { *self.board.get_unchecked_mut((color * 6 + piece_type) as usize) &= !board_from_square(square) };
        if H {
            self.hashcode ^= unsafe { SQUARE_HASHES[(color * 6 + piece_type) as usize][square as usize] }
        }
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
    pub fn toggle_castle_availability<const C: Color, const A: CastleAvailability, const H: bool>(&mut self) {
        let bit_mask = CastleAvailability::bit_mask::<A>();
        self.castle_availability[C as usize] = unsafe { transmute(self.castle_availability[C as usize] as u8 ^ bit_mask) };
        if H {
            match A {
                CastleAvailability::Both => {
                    self.hashcode ^= unsafe { CASTLE_HASHES[C as usize * 2] };
                    self.hashcode ^= unsafe { CASTLE_HASHES[C as usize * 2 + 1] };
                },
                CastleAvailability::King => self.hashcode ^= unsafe { CASTLE_HASHES[C as usize * 2] },
                CastleAvailability::Queen => self.hashcode ^= unsafe { CASTLE_HASHES[C as usize * 2 + 1] },
                CastleAvailability::None => (),
            }
        }
    }

    #[inline(never)]
    pub fn clear_castle_availability<const C: Color, const A: CastleAvailability, const H: bool>(&mut self) {
        debug_assert!(A != CastleAvailability::Both && A != CastleAvailability::None);
        if self.castle_availability[C as usize] == CastleAvailability::Both || self.castle_availability[C as usize] == A {
            let bit_mask = CastleAvailability::bit_mask::<A>();
            self.castle_availability[C as usize] = unsafe { transmute(self.castle_availability[C as usize] as u8 & !bit_mask) };
            if H {
                match A {
                    CastleAvailability::King => self.hashcode ^= unsafe { CASTLE_HASHES[C as usize * 2] },
                    CastleAvailability::Queen => self.hashcode ^= unsafe { CASTLE_HASHES[C as usize * 2 + 1] },
                    _ => unreachable!(),
                }
            }
        }
    }

    #[inline(always)]
    pub fn set_en_passant<const H: bool>(&mut self, square: Square) {
        self.en_passant_square = square;
        if H {
            self.hashcode ^= unsafe { EN_PASSANT_HASHES[file(square) as usize] }
        }
    }

    #[inline(always)]
    pub fn clear_en_passant<const H: bool>(&mut self) {
        if H && self.en_passant_square != NULL_SQUARE {
            self.hashcode ^= unsafe { EN_PASSANT_HASHES[file(self.en_passant_square) as usize] }
        }
        self.en_passant_square = NULL_SQUARE;
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
                self.is_square_safe::<C, false>(4 + color_shift, NULL_SQUARE) &&
                self.is_square_safe::<C, false>(5 + color_shift, NULL_SQUARE)
            },
            CastleAvailability::Queen => {
                self.is_square_safe::<C, false>(2 + color_shift, NULL_SQUARE) &&
                self.is_square_safe::<C, false>(3 + color_shift, NULL_SQUARE)
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
        writeln!(f, "{}", bottom_line)?;
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
            writeln!(f, "{}|\n{}", line, bottom_line)?;
        }
        writeln!(f, "   a b c d e f g h")?;
        writeln!(f, "Turn: {}", if self.turn == Color::White {"White"} else {"Black"})?;
        if self.check {
            writeln!(f, "In Check")?;
        }
        Ok(())
    }
}
