use std::{hint::unreachable_unchecked};
use std::marker::ConstParamTy;
use crate::bitboard::{board_from_square, file, is_valid_square, rank, Bitboard, Color, Square, BLACK_OFFSET, BLACK_VAL, EMPTY_BITBOARD, WHITE_OFFSET, WHITE_VAL};
use crate::magic::{get_magic_index, magic_init, BISHOP_MAGICS, BISHOP_TABLE, ROOK_MAGICS, ROOK_TABLE};

pub type Step = i8;

#[derive(ConstParamTy, PartialEq, Eq)]

pub enum PieceType {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
}

pub const RIGHT_STEP:      Step = 1;
pub const UP_RIGHT_STEP:   Step = 9;
pub const UP_STEP:         Step = 8;
pub const UP_LEFT_STEP:    Step = 7;
pub const LEFT_STEP:       Step = -1;
pub const DOWN_LEFT_STEP:  Step = -9;
pub const DOWN_STEP:       Step = -8;
pub const DOWN_RIGHT_STEP: Step = -7;

pub const KNIGHT_RIGHT_UP_STEP:   Step = 10;
pub const KNIGHT_UP_RIGHT_STEP:   Step = 17;
pub const KNIGHT_UP_LEFT_STEP:    Step = 15;
pub const KNIGHT_LEFT_UP_STEP:    Step = 6;
pub const KNIGHT_LEFT_DOWN_STEP:  Step = -10;
pub const KNIGHT_DOWN_LEFT_STEP:  Step = -17;
pub const KNIGHT_DOWN_RIGHT_STEP: Step = -15;
pub const KNIGHT_RIGHT_DOWN_STEP: Step = -6;

pub const KING:   u8 = 0;
pub const QUEEN:  u8 = 1;
pub const ROOK:   u8 = 2;
pub const BISHOP: u8 = 3;
pub const KNIGHT: u8 = 4;
pub const PAWN:   u8 = 5;

pub const WHITE_KING:   u8 = WHITE_OFFSET + KING;
pub const WHITE_QUEEN:  u8 = WHITE_OFFSET + QUEEN;
pub const WHITE_ROOK:   u8 = WHITE_OFFSET + ROOK;
pub const WHITE_BISHOP: u8 = WHITE_OFFSET + BISHOP;
pub const WHITE_KNIGHT: u8 = WHITE_OFFSET + KNIGHT;
pub const WHITE_PAWN:   u8 = WHITE_OFFSET + PAWN;

pub const BLACK_KING:   u8 = BLACK_OFFSET + KING;
pub const BLACK_QUEEN:  u8 = BLACK_OFFSET + QUEEN;
pub const BLACK_ROOK:   u8 = BLACK_OFFSET + ROOK;
pub const BLACK_BISHOP: u8 = BLACK_OFFSET + BISHOP;
pub const BLACK_KNIGHT: u8 = BLACK_OFFSET + KNIGHT;
pub const BLACK_PAWN:   u8 = BLACK_OFFSET + PAWN;

pub const NO_PIECE:     u8 = BLACK_PAWN + 1;

pub const CARDINAL_STEPS: [Step; 8] = [RIGHT_STEP, UP_RIGHT_STEP, UP_STEP, UP_LEFT_STEP, LEFT_STEP, DOWN_LEFT_STEP, DOWN_STEP, DOWN_RIGHT_STEP];
pub const ALL_STEPS:      [Step; 16] = [RIGHT_STEP, UP_RIGHT_STEP, UP_STEP, UP_LEFT_STEP, LEFT_STEP, DOWN_LEFT_STEP, DOWN_STEP, DOWN_RIGHT_STEP, KNIGHT_RIGHT_UP_STEP, KNIGHT_UP_RIGHT_STEP, KNIGHT_UP_LEFT_STEP, KNIGHT_LEFT_UP_STEP, KNIGHT_LEFT_DOWN_STEP, KNIGHT_DOWN_LEFT_STEP, KNIGHT_DOWN_RIGHT_STEP, KNIGHT_RIGHT_DOWN_STEP];

pub const ROOK_STEPS:   [Step; 4] = [RIGHT_STEP, UP_STEP, LEFT_STEP, DOWN_RIGHT_STEP];
pub const BISHOP_STEPS: [Step; 4] = [UP_RIGHT_STEP, UP_LEFT_STEP, DOWN_LEFT_STEP, DOWN_RIGHT_STEP];
pub const KNIGHT_STEPS: [Step; 8] = [KNIGHT_RIGHT_UP_STEP, KNIGHT_UP_RIGHT_STEP, KNIGHT_UP_LEFT_STEP, KNIGHT_LEFT_UP_STEP, KNIGHT_LEFT_DOWN_STEP, KNIGHT_DOWN_LEFT_STEP, KNIGHT_DOWN_RIGHT_STEP, KNIGHT_RIGHT_DOWN_STEP];

pub static mut CAN_STEP_TABLE: [[bool; 64]; 16] = [[false; 64]; 16];
pub static mut MOVE_BOARDS: [[Bitboard; 64]; 5] = [[EMPTY_BITBOARD; 64]; 5];
pub static mut PAWN_ATTACK_BOARDS: [[Bitboard; 64]; 2] = [[EMPTY_BITBOARD; 64]; 2];

impl PieceType {
    #[inline(always)]
    pub const fn value(self) -> u8 {
        match self {
            PieceType::King => {KING},
            PieceType::Queen => {QUEEN},
            PieceType::Rook => {ROOK},
            PieceType::Bishop => {BISHOP},
            PieceType::Knight => {KNIGHT},
            PieceType::Pawn => {PAWN},
        }
    }

    #[inline(always)]
    pub const fn colored_value(self, color: Color) -> u8 {
        self.value() + color.board_offset()
    }

    #[inline(always)]
    pub const fn is_slider(self) -> bool {
        match self {
            PieceType::Queen | PieceType::Rook | PieceType::Bishop => {true},
            _ => {false},
        }
    }

    #[inline(always)]
    pub const fn steps(self) -> &'static[Step] {
        match self {
           PieceType::King | PieceType::Queen => {&CARDINAL_STEPS},
           PieceType::Rook => {&ROOK_STEPS},
           PieceType::Bishop => {&BISHOP_STEPS},
           PieceType::Knight => {&KNIGHT_STEPS},
           PieceType::Pawn => {&[]}
        }
    }
}

pub fn movement_info_init() {
    step_info_init();
    fill_moves_boards::<{ PieceType::King }>();
    fill_moves_boards::<{ PieceType::Queen }>();
    fill_moves_boards::<{ PieceType::Rook }>();
    fill_moves_boards::<{ PieceType::Bishop }>();
    fill_moves_boards::<{ PieceType::Knight }>();
    fill_moves_boards::<{ PieceType::Pawn }>();
    magic_init();
}

fn step_info_init() {
    const CENTER_SQUARE: Square = 35;
    const CENTER_RANK: u8 = rank(CENTER_SQUARE);
    const CENTER_FILE: u8 = file(CENTER_SQUARE);
    for (i, step) in ALL_STEPS.iter().enumerate() {
        let center_step_square = make_step(CENTER_SQUARE, *step);
        let rank_diff = rank(center_step_square).wrapping_sub(CENTER_RANK);
        let file_diff = file(center_step_square).wrapping_sub(CENTER_FILE);
        for square in 0..64 {
            let step_square = make_step(square, *step);
            if rank(step_square) - rank(square) == rank_diff && file(step_square) - file(square) == file_diff {
                unsafe { CAN_STEP_TABLE[i][square as usize] = true };
            } else {
                unsafe { CAN_STEP_TABLE[i][square as usize] = false };
            }
        }
    }
}

fn fill_moves_boards<const P: PieceType>() {
    match P {
        PieceType::Queen | PieceType::Rook | PieceType::Bishop => {
            for step in P.steps() {
                for square in 0..64 {
                    let mut step_square = square;
                    while unsafe { can_step(step_square, *step) } {
                        step_square = make_step(step_square, *step);
                        unsafe { MOVE_BOARDS[P.value() as usize][square as usize] |= board_from_square(square) };
                    }
                }
            }
        },
        PieceType::King | PieceType::Knight => {
            for step in P.steps() {
                for square in 0..64 {
                    if unsafe { can_step(square, *step) } {
                        unsafe { MOVE_BOARDS[P.value() as usize][square as usize] |= board_from_square(square)}
                    }
                }
            }
        },
        PieceType::Pawn => {
            for square in 0..64 {
                let mut white_board = EMPTY_BITBOARD;
                let mut black_board = EMPTY_BITBOARD;
                if unsafe { can_step(square, UP_RIGHT_STEP) } {
                    white_board |= board_from_square(make_step(square, UP_RIGHT_STEP));
                }
                if unsafe { can_step(square, UP_LEFT_STEP) } {
                    white_board |= board_from_square(make_step(square, UP_LEFT_STEP));
                }
                if unsafe { can_step(square, DOWN_RIGHT_STEP) } {
                    black_board |= board_from_square(make_step(square, DOWN_RIGHT_STEP));
                }
                if unsafe { can_step(square, DOWN_LEFT_STEP) } {
                    black_board |= board_from_square(make_step(square, DOWN_LEFT_STEP));
                }
                unsafe {
                    PAWN_ATTACK_BOARDS[WHITE_VAL as usize][square as usize] = white_board;
                    PAWN_ATTACK_BOARDS[BLACK_VAL as usize][square as usize] = black_board;
                }
            }
        }
    }
}

#[inline(always)]
pub fn move_bitboard<const P: PieceType>(square: Square, occupied: Bitboard) -> Bitboard{
    debug_assert!(is_valid_square(square));
    match P {
        PieceType::King => {
            return unsafe { *MOVE_BOARDS[KING as usize].get_unchecked(square as usize) }
        },
        PieceType::Queen => {
            move_bitboard::< {PieceType::Bishop}>(square, occupied) | move_bitboard::< {PieceType::Rook}>(square, occupied)
        },
        PieceType::Rook => {
            let magic = unsafe { ROOK_MAGICS.get_unchecked(square as usize) };
            unsafe { ROOK_TABLE[get_magic_index(magic, occupied)] }
        },
        PieceType::Bishop => {
            let magic = unsafe { BISHOP_MAGICS.get_unchecked(square as usize) };
            unsafe { BISHOP_TABLE[get_magic_index(magic, occupied)] }
        },
        PieceType::Knight => {
            unsafe { *MOVE_BOARDS[KNIGHT as usize].get_unchecked(square as usize) }
        }
        PieceType::Pawn => {
            debug_assert!(false);
            unsafe { unreachable_unchecked() };
        },
    }
}

pub const unsafe fn get_step_id(step: Step) -> usize {
    debug_assert!(is_valid_step(step));
    match step {
        RIGHT_STEP => {0},
        UP_RIGHT_STEP => {1},
        UP_STEP => {2},
        UP_LEFT_STEP => {3},
        LEFT_STEP => {4},
        DOWN_LEFT_STEP => {5},
        DOWN_STEP => {6},
        DOWN_RIGHT_STEP => {7},
        KNIGHT_RIGHT_UP_STEP => {8},
        KNIGHT_UP_RIGHT_STEP => {9},
        KNIGHT_UP_LEFT_STEP => {10},
        KNIGHT_LEFT_UP_STEP => {11},
        KNIGHT_LEFT_DOWN_STEP => {12},
        KNIGHT_DOWN_LEFT_STEP => {13},
        KNIGHT_DOWN_RIGHT_STEP => {14},
        KNIGHT_RIGHT_DOWN_STEP => {15},
        _ => {
            unsafe { unreachable_unchecked() }
        }
    }
}  

pub const fn is_valid_step(step: Step) -> bool {
    match step {
        RIGHT_STEP |
        UP_RIGHT_STEP |
        UP_STEP |
        UP_LEFT_STEP |
        LEFT_STEP |
        DOWN_LEFT_STEP |
        DOWN_STEP |
        DOWN_RIGHT_STEP |
        KNIGHT_RIGHT_UP_STEP | 
        KNIGHT_UP_RIGHT_STEP |
        KNIGHT_UP_LEFT_STEP |
        KNIGHT_LEFT_UP_STEP |
        KNIGHT_LEFT_DOWN_STEP |
        KNIGHT_DOWN_LEFT_STEP | 
        KNIGHT_DOWN_RIGHT_STEP |
        KNIGHT_RIGHT_DOWN_STEP => {true},
        _ => false,
    }
}
pub fn make_step(square: Square, step: Step) -> Square {
    square + (step as Square) % 64
}

pub unsafe fn can_step(square: Square, step: Step) -> bool {
    debug_assert!(is_valid_step(step));
    debug_assert!(is_valid_square(square));
    unsafe { *CAN_STEP_TABLE.get_unchecked(get_step_id(step)).get_unchecked(square as usize) }
}