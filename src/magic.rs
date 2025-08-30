use std::{ops::Range, u64};

use rand::{rng, RngCore, SeedableRng};
use rand_xoshiro::Xoroshiro128PlusPlus;

use crate::{bitboard::{bit_count, board_from_square, file, pretty_string_bitboard, rank, Bitboard, Square, EMPTY_BITBOARD, FILE0, FILE7, FILES, RANK0, RANK7, RANKS}, piece_info::{can_step, make_step, PieceType, MOVE_BOARDS}};

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
    const ROOK_MAGIC_SEED: [[u8; 16]; 4] = [[121, 127, 184, 174, 169,  98, 124, 185,   1,  34, 155, 108,  28,   9,   4, 186],
                                            [ 97, 244, 185,  57, 161, 116, 207,  93, 144, 129, 147, 248, 158,  75, 255, 136],
                                            [ 55, 193, 163, 105, 105, 119,  44, 224,  35, 188, 202,  90, 209,  15,  22, 101],
                                            [ 15,  85, 222,  70, 131,  49, 230, 209,  34, 253, 188,  12,   0,  99,   9,  14]];
    const BISHOP_MAGIC_SEED: [u8; 16] =     [119,  90, 149, 190, 209, 208, 116,  53,  27,  24, 211, 162, 183, 162, 239,   7];
    const EXPECTED_ROOK_PASSES: u64 = 128333 + 113880 + 98157 + 87764;
    const EXPECTED_BISHOP_PASSES: u64 = 97384;
    let mut rook_rng = Xoroshiro128PlusPlus::from_seed(ROOK_MAGIC_SEED[0]);
    let mut bishop_rng = Xoroshiro128PlusPlus::from_seed(BISHOP_MAGIC_SEED);
    let mut running_rook_offset: usize = 0;
    let mut running_bishop_offset: usize = 0;
    let mut running_total = 0u64;
    for square in 0..64 {
        unsafe {
            if square % 16 == 0 {
                rook_rng = Xoroshiro128PlusPlus::from_seed(ROOK_MAGIC_SEED[(square / 16) as usize]);
            }
            let rook_result= find_magic::<{ PieceType::Rook }>(square, &mut ROOK_MAGICS[square as usize], &mut ROOK_TABLE[running_rook_offset..], &mut rook_rng);
            let bishop_result = find_magic::<{ PieceType::Bishop }>(square, &mut BISHOP_MAGICS[square as usize], &mut BISHOP_TABLE[running_bishop_offset..], &mut bishop_rng);
            unsafe { ROOK_MAGICS[square as usize].offset = running_rook_offset as u32 };
            unsafe { BISHOP_MAGICS[square as usize].offset = running_bishop_offset as u32 };
            running_rook_offset += rook_result.0;
            running_bishop_offset += bishop_result.0;
            running_total += rook_result.1 + bishop_result.1;
        }
    }
    println!("Magics generated in {} passes", running_total);
    assert_eq!(EXPECTED_BISHOP_PASSES + EXPECTED_ROOK_PASSES, running_total);
}

fn find_magic<const P: PieceType>(square: Square, magic: &mut Magic, magic_table: &mut [Bitboard], rng: &mut Xoroshiro128PlusPlus) -> (usize, u64) {
    match P {
        PieceType::King | PieceType::Queen | PieceType::Pawn | PieceType::Knight => return (0, 0),
        _ => (),
    }
    let index = P.index();
    let attacks_board = unsafe { MOVE_BOARDS[index as usize][square as usize] };
    let mask = attacks_board & (!(RANK0 | RANK7) | RANKS[rank(square) as usize]) & (!(FILE0 | FILE7) | FILES[file(square) as usize]);
    let bitcount = bit_count(mask);
    let table_size = 1usize << bitcount;
    magic.mask = mask;
    magic.index = bitcount as u8;
    let mut test_table: Vec<u64> = vec![0; table_size];
    let mut found_table = vec![false; table_size];
    let mut total_passes = 0u64;
    for (i, blockers) in SubsetIterator::new(mask).enumerate() {
        test_table[i] = find_blocked_sliding_attacks::<P>(square, blockers).unwrap();
    }
    loop {
        total_passes += 1;
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
    (table_size, total_passes)
} 

pub fn find_blocked_sliding_attacks<const P: PieceType>(square: Square, mut blockers: Bitboard) -> Option<Bitboard> {
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

pub fn find_magic_seed<const P: PieceType>(n: usize, r: Range<u8>) -> ([u8; 16], u64) {
    let mut best = [0u8; 16];
    let mut best_count = u64::MAX;
    let mut meta_rng = rand::rng();
    for i in 0..n {
        let contending = &mut [0u8; 16];
        meta_rng.fill_bytes(contending);
        let mut rng = Xoroshiro128PlusPlus::from_seed(*contending);
        let mut running_offset: usize = 0;
        let mut running_total = 0u64;
        for square in r.clone() {
            unsafe {
                let magic = match P {
                    PieceType::Rook => { ROOK_MAGICS.get_mut(square as usize) },
                    PieceType::Bishop => { BISHOP_MAGICS.get_mut(square as usize) },
                    _ => unimplemented!(),
                }.unwrap();
                let result_table = match P {
                    PieceType::Rook => { &mut ROOK_TABLE[running_offset..] },
                    PieceType::Bishop => { &mut BISHOP_TABLE[running_offset..] },
                    _ => unimplemented!(),
                };
                magic.offset = 0;
                let result= find_magic::<P>(square, magic, result_table, &mut rng);
                running_offset += result.0;
                running_total += result.1;
                if running_total > best_count {
                    break;
                }
            }
        }
        if running_total < best_count {
            best_count = running_total;
            best = *contending;
        }
        if i % 100 == 0 {
            println!("Seeds Searched: {}, Best Found: {:?}, Best Count: {}", i + 1, best, best_count);
        }
    }
    (best, best_count)
}