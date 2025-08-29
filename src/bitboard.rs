pub type Bitboard = u64;
pub type Square   = u8;
pub type Board = [Bitboard; 12];

#[derive(Clone, Copy)]
pub enum Color {
    White,
    Black,
}

pub const EMPTY_BITBOARD:     Bitboard = 0;
pub const UNIVERSAL_BITBOARD: Bitboard = !EMPTY_BITBOARD;

pub const RANK0: Bitboard = 0xff;
pub const RANK1: Bitboard = RANK0 << (8 * 1);
pub const RANK2: Bitboard = RANK0 << (8 * 2);
pub const RANK3: Bitboard = RANK0 << (8 * 3);
pub const RANK4: Bitboard = RANK0 << (8 * 4);
pub const RANK5: Bitboard = RANK0 << (8 * 5);
pub const RANK6: Bitboard = RANK0 << (8 * 6);
pub const RANK7: Bitboard = RANK0 << (8 * 7);

pub const FILE0: Bitboard = 0x0101010101010101;
pub const FILE1: Bitboard = FILE0 << 1;
pub const FILE2: Bitboard = FILE0 << 2;
pub const FILE3: Bitboard = FILE0 << 3;
pub const FILE4: Bitboard = FILE0 << 4;
pub const FILE5: Bitboard = FILE0 << 5;
pub const FILE6: Bitboard = FILE0 << 6;
pub const FILE7: Bitboard = FILE0 << 7;

pub const NULL_SQUARE: Square = 100;

pub const WHITE_VAL: u8 = 0;
pub const BLACK_VAL: u8 = 1;
pub const WHITE_OFFSET: u8 = 0;
pub const BLACK_OFFSET: u8 = 6;

pub const RANKS: [Bitboard; 8] = [RANK0, RANK1, RANK2, RANK3, RANK4, RANK5, RANK6, RANK7];
pub const FILES: [Bitboard; 8] = [FILE0, FILE1, FILE2, FILE3, FILE4, FILE5, FILE6, FILE7];
pub const FILE_MAP: [char; 8] =  ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];

impl Color {
    #[inline(always)]
    pub const fn other(self) -> Color {
        match self {
            Color::White => {Color::Black},
            Color::Black => {Color::White},
        }
    }

    #[inline(always)]
    pub const fn value(self) -> u8 {
        match self {
            Color::White => {WHITE_VAL},
            Color::Black => {BLACK_VAL},
        }
    }

    #[inline(always)]
    pub const fn board_offset(self) -> u8 {
        match self {
            Color::White => {WHITE_OFFSET},
            Color::Black => {BLACK_OFFSET},
        }
    }
}

#[inline(always)]
pub const fn rank(s: Square) -> u8 { (s / 8) as u8 }
#[inline(always)]
pub const fn file(s: Square) -> u8 { (s % 8) as u8 }

#[inline(always)]
pub const fn get_lsb(b: Bitboard) -> Square {
    b.trailing_zeros() as Square
}

#[inline(always)]
pub const fn pop_lsb(b: &mut Bitboard) -> Square {
    let lsb = get_lsb(*b);
    *b &= *b - 1;
    lsb 
}

#[inline(always)]
pub const fn bit_count(b: Bitboard) -> u32 {
    b.count_ones()
}

#[inline(always)]
pub const fn board_from_square(s: Square) -> Bitboard {
    1 << (s as Bitboard)
}

pub const fn is_valid_square(s: Square) -> bool {
    s < 64
}

pub fn square_from_string(string: String) -> Square {
    let rank = string[1..].parse::<Square>().unwrap() - 1;
    let mut file: u8 = 0;
    for (i, r) in FILE_MAP.iter().enumerate() {
        if string.chars().nth(0).unwrap() == *r {
            file = i as u8;
        }
    }
    rank * 8 + file
}

pub fn pretty_string_bitboard(b: Bitboard) -> String {
    let zeros = String::from("00000000");
    let mut output = String::with_capacity(64);
    for i in 0..8 {
        let mut line = format!("{:b}", (b>>(8*i))&0xff);
        line = String::from(&zeros[0..8-line.len()]) + &line;
        let reversed: String = line.chars().rev().collect();
        output = reversed + "\n" + &output;
    }
    output
}

pub fn pretty_string_square(s: Square) -> String {
    if s == NULL_SQUARE {
        return String::from("NS");
    }
    return String::from(FILE_MAP[file(s) as usize]) + &format!("{}", rank(s));
}