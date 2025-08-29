use rand::{rng, rngs, RngCore};

use crate::{bitboard::{bit_count, board_from_square, file, rank, Bitboard, Square, EMPTY_BITBOARD, FILE0, FILE7, FILES, RANK0, RANK7, RANKS}, piece_info::{can_step, make_step, PieceType, MOVE_BOARDS}};

#[derive(Clone, Copy)]
pub struct Magic {
    mask: Bitboard,
    magic_number: u64,
    index: u8,
    offset: u32,
}

#[derive(Clone, Copy)]
pub struct SubsetIterator {
    n: Bitboard,
    d: Bitboard,
    done: bool,
}

impl SubsetIterator {
    pub fn new(board: Bitboard) -> SubsetIterator {
        SubsetIterator { n: 0, d: board, done: false}
    }
}

impl Iterator for SubsetIterator {
    type Item = Bitboard;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let return_val = self.n;
        self.n = (self.n.wrapping_sub(self.d)) & self.d;
        if self.n == 0 {
           self.done = true;
        }
        Some(return_val)
    }
}

pub static mut BISHOP_MAGICS: [Magic; 64] = [Magic {mask: 0, magic_number: 0, index: 0, offset: 0}; 64];
pub static mut ROOK_MAGICS: [Magic; 64] = [Magic {mask: 0, magic_number: 0, index: 0, offset: 0}; 64];

pub static mut BISHOP_TABLE: [Bitboard; 0x1480] = [EMPTY_BITBOARD; 0x1480];
pub static mut ROOK_TABLE: [Bitboard; 0x19000] = [EMPTY_BITBOARD; 0x19000];

pub fn magic_init() {
    let mut running_rook_offset: usize = 0;
    let mut running_bishop_offset: usize = 0;
    for square in 0..64 {
        unsafe {
            let rook_offset= find_magic::<{ PieceType::Rook }>(square, &mut ROOK_MAGICS[square as usize], &mut ROOK_TABLE[running_rook_offset..]);
            let bishop_offset = find_magic::<{ PieceType::Bishop }>(square, &mut BISHOP_MAGICS[square as usize], &mut BISHOP_TABLE[running_bishop_offset..]);
            unsafe { ROOK_MAGICS[square as usize].offset = running_rook_offset as u32 };
            unsafe { BISHOP_MAGICS[square as usize].offset = running_bishop_offset as u32 };
            running_rook_offset += rook_offset;
            running_bishop_offset += bishop_offset;
        }
    }
}

fn find_magic<const P: PieceType>(square: Square, magic: &mut Magic, magic_table: &mut [Bitboard]) -> usize {
    match P {
        PieceType::King | PieceType::Queen | PieceType::Pawn | PieceType::Knight => return 0,
        _ => (),
    }
    let index = P.value();
    let attacks_board = unsafe { MOVE_BOARDS[index as usize][square as usize] };
    let mask = attacks_board & (!(RANK0 | RANK7) | RANKS[rank(square) as usize]) & (!(FILE0 | FILE7) | FILES[file(square) as usize]);
    let bitcount = bit_count(mask);
    let table_size = 1usize << bitcount;
    magic.mask = mask;
    magic.index = bitcount as u8;
    let mut test_table: Vec<u64> = vec![0; table_size];
    let mut found_table = vec![false; table_size];
    let mut rng = rng();
    for (i, blockers) in SubsetIterator::new(mask).enumerate() {
        test_table[i] = find_blocked_sliding_attacks::<P>(square, blockers).unwrap();
    }
    loop {
        let test_magic = rng.next_u64() & rng.next_u64() & rng.next_u64();
        magic.magic_number = test_magic;
        found_table.fill(false);
        let mut good_table = true;
        for (i, blockers) in SubsetIterator::new(mask).enumerate() {
            let moves = test_table[i];
            let table_index = get_magic_index(magic, blockers);
            if found_table[table_index] {
                good_table = false;
                break;
            } else {
                magic_table[table_index] = moves;
                found_table[table_index] = true;
            }
        }
        if good_table {
            break;
        }
    }
    table_size
} 

fn find_blocked_sliding_attacks<const P: PieceType>(square: Square, mut blockers: Bitboard) -> Option<Bitboard> {
    if !P.is_slider() {return None;}
    let mut result = EMPTY_BITBOARD;
    if board_from_square(square) & blockers != EMPTY_BITBOARD {
        blockers ^= board_from_square(square);
    }
    for step in P.steps().iter() {
        let mut step_square = square;
        while unsafe { can_step(step_square, *step) } && (board_from_square(step_square) & blockers == EMPTY_BITBOARD) {
            step_square = make_step(step_square, *step);
            result |= board_from_square(step_square)
        }
    }
    Some(result)
} 

#[inline(always)]
pub fn get_magic_index(magic: &Magic, occupied: Bitboard) -> usize {
    let blockers = occupied & magic.mask;
    let hash = blockers.wrapping_mul(magic.magic_number);
    ((hash >> (64 - magic.index)) + (magic.offset as u64)) as usize
}