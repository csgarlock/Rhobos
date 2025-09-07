use std::marker::ConstParamTy;
use crate::{bitboard::{board_from_square, get_lsb, pop_lsb, shift_bitboard, Bitboard, Color, Square, EMPTY_BITBOARD, NULL_SQUARE}, r#move::{build_move, build_simple_move, BISHOP_PROMOTION, CASTLE_SPECIAL_MOVE, EN_PASSANT_SPECIAL_MOVE, KNIGHT_PROMOTION, PROMOTION_SPECIAL_MOVE, QUEEN_PROMOTION, ROOK_PROMOTION}, piece_info::{make_step, move_bitboard, Direction, PieceType, Step, PAWN_ATTACK_BOARDS}, state::{CastleAvailability, State}};

#[repr(u8)]
#[derive(Clone, Copy, ConstParamTy, PartialEq, Eq)]
pub enum MoveGenType {
    All,
    Quiet,
    Capture,
}

impl State {
    pub fn gen_all_moves<const C: Color, const G: MoveGenType>(&mut self) {
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
        if G.should_gen_quiets() && king_square == State::king_square::<C>() && !self.check {
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

    #[inline(never)]
    pub fn gen_king_castle<const C: Color, const A: CastleAvailability>(&mut self) {
        debug_assert!(A != CastleAvailability::Both && A != CastleAvailability::None);
        let color_shift = C.castle_shift();
        let king_square = 4 + color_shift;
        if board_from_square(CastleAvailability::rook_square::<C, A>()) & self.get_piece_board(C, PieceType::Rook) != EMPTY_BITBOARD &&
            // Check if squares in between king and rook are free
            CastleAvailability::through_squares::<C, A>() & self.occupied == EMPTY_BITBOARD &&
            // Check if all squares in between king and rook are not attacked by another piece
            self.are_castle_through_squares_safe::<C, A>()
        {
            self.move_stack.push_current(build_move(
                king_square,
                CastleAvailability::des_square::<C, A>(),
                A as u8,
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

            if self.en_passant_square != NULL_SQUARE {
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

    pub fn debug_quick_gen_moves(&mut self) {
        match self.turn {
            Color::White => self.gen_all_moves::<{Color::White}, {MoveGenType::All}>(),
            Color::Black => self.gen_all_moves::<{Color::Black}, {MoveGenType::All}>(),
        }
    }
}

impl CastleAvailability {
    #[inline(always)]
    pub const fn through_squares<const C: Color, const A: CastleAvailability>() -> Bitboard {
        (match A {
            CastleAvailability::King => 0x60,
            CastleAvailability::Queen => 0xE,
            _ => EMPTY_BITBOARD,
        }) << C.castle_shift()
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

    #[inline(always)]
    pub fn bit_mask<const A: CastleAvailability>() -> u8 {
        match A {
            CastleAvailability::Both => 0b11,
            CastleAvailability::King => 0b01,
            CastleAvailability::Queen => 0b10,
            CastleAvailability::None => 0b00,
        }
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