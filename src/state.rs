use core::fmt;
use std::{fmt::Display, marker::ConstParamTy, mem::transmute};
use crate::{bitboard::{board_from_square, get_lsb, pop_lsb, pretty_string_bitboard, shift_bitboard, Bitboard, Board, Color, Square, EMPTY_BITBOARD, NULL_SQUARE}, histories::{CaptureEntry, CastleHistoryEntry, EnPassantEntry, FiftyMoveHistory, History}, r#move::{build_move, build_simple_move, BISHOP_PROMOTION, CASTLE_SPECIAL_MOVE, EN_PASSANT_SPECIAL_MOVE, KNIGHT_PROMOTION, PROMOTION_SPECIAL_MOVE, QUEEN_PROMOTION, ROOK_PROMOTION}, move_list::MoveStack, piece_info::{make_step, move_bitboard, Direction, PieceType, Step, PAWN_ATTACK_BOARDS}};

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
    None  = 0b0,
    King  = 0b1,
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
    pub half_move_clock:     u16,
    pub move_stack:          MoveStack,
    pub castle_availability: [CastleAvailability; 2],
    pub capture_history:     History<CaptureEntry>,
    pub en_passant_history:  History<EnPassantEntry>,
    pub castle_history:      History<CastleHistoryEntry>,
    pub fifty_move_history:  History<FiftyMoveHistory>,
    pub hash_history:        History<u64>,
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
        self.gen_king_moves::<C, G>(mask);
        self.gen_pawn_moves::<C, G>(mask);
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
    pub fn gen_king_moves<const C: Color, const G: MoveGenType>(&mut self, mask: Bitboard) {
        let king_square = get_lsb(self.get_piece_board(C, PieceType::King));
        let mut move_board = move_bitboard::<{ PieceType::King}>(king_square, self.occupied) & mask;
        while move_board != EMPTY_BITBOARD {
            let des_square = pop_lsb(&mut move_board);
            if self.is_square_safe::<C, true>(des_square, king_square) {
                self.move_stack.push_current(build_simple_move(king_square, des_square));
            }
        }
        if G.should_gen_quiets() && king_square == State::king_square::<C>(){
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
    pub fn gen_pawn_moves<const C: Color, const G: MoveGenType>(&mut self, mask: Bitboard) {

        let enemy_board = self.side_occupied[C.other() as usize];
        let third_rank_mask = C.color_rel_rank_mask::<2>();
        let last_rank_mask = C.color_rel_rank_mask::<6>();
        let pawns_on_last = self.get_piece_board(C, PieceType::Pawn) & last_rank_mask;
        let pawns_not_on_last = self.get_piece_board(C, PieceType::Pawn) & !last_rank_mask;

        let up = C.up();
        let down = C.down();
        let down_right = C.down_right();
        let down_left = C.down_left();

        if G == MoveGenType::All || G == MoveGenType::Quiet {
            let mut single_push;
            let mut double_push;
            match C {
                Color::White => {
                    single_push = shift_bitboard::<{Direction::Up}>(pawns_not_on_last) & self.not_occupied;
                    double_push = shift_bitboard::<{Direction::Up}>(single_push & third_rank_mask) & self.not_occupied & mask;
                },
                Color::Black => {
                    single_push = shift_bitboard::<{Direction::Down}>(pawns_not_on_last) & self.not_occupied;
                    double_push = shift_bitboard::<{Direction::Down}>(single_push & third_rank_mask) & self.not_occupied & mask;
                }
            }
            single_push &= mask;
            
            while single_push != EMPTY_BITBOARD {
                let des_square = pop_lsb(&mut single_push);
                self.move_stack.push_current(build_simple_move(make_step(des_square, down as Step), des_square));
            }
            while double_push != EMPTY_BITBOARD {
                let des_square = pop_lsb(&mut double_push);
                self.move_stack.push_current(build_simple_move(make_step(make_step(des_square, down as Step), down as Step), des_square));
            }
        }

        if pawns_on_last != EMPTY_BITBOARD {
            let mut up_board;
            let mut up_right_board;
            let mut up_left_board;
            match C {
                Color::White => {
                    up_board = shift_bitboard::<{Direction::Up}>(pawns_on_last) & self.not_occupied;
                    up_right_board = shift_bitboard::<{Direction::UpRight}>(pawns_on_last) & enemy_board;
                    up_left_board = shift_bitboard::<{Direction::UpLeft}>(pawns_on_last) & enemy_board;
                },
                Color::Black => {
                    up_board = shift_bitboard::<{Direction::Down}>(pawns_on_last) & self.not_occupied;
                    up_right_board = shift_bitboard::<{Direction::DownRight}>(pawns_on_last) & enemy_board;
                    up_left_board = shift_bitboard::<{Direction::DownLeft}>(pawns_on_last) & enemy_board;
                }
            }
            
            up_board &= !self.side_occupied[C as usize];

            while up_board != EMPTY_BITBOARD {
                let des_square = pop_lsb(&mut up_board);
                self.build_pawn_promotions::<G, false>(make_step(des_square, down as Step), des_square);
            }

            while up_right_board != EMPTY_BITBOARD {
                let des_square = pop_lsb(&mut up_right_board);
                self.build_pawn_promotions::<G, true>(make_step(des_square, down_left as Step), des_square);
            }

            while up_left_board != EMPTY_BITBOARD {
                let des_square = pop_lsb(&mut up_left_board);
                self.build_pawn_promotions::<G, true>(make_step(des_square, down_right as Step), des_square);
            }
        }

        if G.should_gen_captures() {
            let mut up_right_board;
            let mut up_left_board;
            match C {
                Color::White => {
                    up_right_board = shift_bitboard::<{Direction::UpRight}>(pawns_not_on_last) & enemy_board;
                    up_left_board = shift_bitboard::<{Direction::UpLeft}>(pawns_not_on_last) & enemy_board;
                },
                Color::Black => {
                    up_right_board = shift_bitboard::<{Direction::DownRight}>(pawns_not_on_last) & enemy_board;
                    up_left_board = shift_bitboard::<{Direction::DownLeft}>(pawns_not_on_last) & enemy_board;
                }
            }

            while up_right_board != EMPTY_BITBOARD {
                let des_square = pop_lsb(&mut up_right_board);
                let pawn_square = make_step(des_square, down_left as Step);
                self.move_stack.push_current(build_simple_move(pawn_square, des_square));
            }

            while up_left_board != EMPTY_BITBOARD {
                let des_square = pop_lsb(&mut up_left_board);
                let pawn_square = make_step(des_square, down_right as Step);
                self.move_stack.push_current(build_simple_move(pawn_square, des_square));
            }

            if self.can_en_passant {
                let mut en_passant_board = unsafe { PAWN_ATTACK_BOARDS[C.other().value() as usize][self.en_passant_square as usize] } & pawns_not_on_last;
                while en_passant_board != EMPTY_BITBOARD {
                    let src_square = pop_lsb(&mut en_passant_board);
                    self.move_stack.push_current(build_move(src_square, self.en_passant_square, 0, EN_PASSANT_SPECIAL_MOVE));
                }
            }
        }
    }

    #[inline(always)]
    pub fn build_pawn_promotions<const G: MoveGenType, const C: bool>(&mut self, src_square: Square, des_square: Square) {
        if G == MoveGenType::All || G == MoveGenType::Capture {
            self.move_stack.push_current(build_move(src_square, des_square, QUEEN_PROMOTION, PROMOTION_SPECIAL_MOVE));
        }
        if (G == MoveGenType::Quiet && !C) || (G == MoveGenType::Capture && C) || G == MoveGenType::All {
            self.move_stack.push_current(build_move(src_square, des_square, ROOK_PROMOTION, PROMOTION_SPECIAL_MOVE));
            self.move_stack.push_current(build_move(src_square, des_square, BISHOP_PROMOTION, PROMOTION_SPECIAL_MOVE));
            self.move_stack.push_current(build_move(src_square, des_square, KNIGHT_PROMOTION, PROMOTION_SPECIAL_MOVE));
        }
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

impl CastleAvailability {
    #[inline(always)]
    pub const fn through_squares<const C: Color, const A: CastleAvailability>() -> Bitboard {
        (match A {
            CastleAvailability::King => 0x60,
            CastleAvailability::Queen => 0xE,
            _ => EMPTY_BITBOARD,
        }) >> C.castle_shift()
    }

    #[inline(always)]
    pub const fn rook_square<const C: Color, const A: CastleAvailability>() -> Square {
        (match A {
            CastleAvailability::King => 7,
            CastleAvailability::Queen => 0,
            _ => NULL_SQUARE,
        }) + C.castle_shift()
    }

    #[inline(always)]
    pub const fn des_square<const C: Color, const A: CastleAvailability>() -> Square {
        (match A {
            CastleAvailability::King => 6,
            CastleAvailability::Queen => 2,
            _ => NULL_SQUARE,
        }) + C.castle_shift()
    }
}

impl MoveGenType {

    #[inline(always)]
    pub const fn should_gen_quiets(self) -> bool {
        match self {
            MoveGenType::All | MoveGenType::Quiet => true,
            _ => false,
        }
    }

    #[inline(always)]
    pub const fn should_gen_captures(self) -> bool {
        match self {
            MoveGenType::All | MoveGenType::Capture => true,
            _ => false,
        }
    }
}