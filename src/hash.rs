use rand::{rng, RngCore};

use crate::{bitboard::{file, pop_lsb, Color, NULL_SQUARE}, state::{CastleAvailability, State}};


pub static mut SQUARE_HASHES: [[u64; 64]; 12] = [[0; 64]; 12];
pub static mut EN_PASSANT_HASHES: [u64; 8] = [0; 8];
pub static mut CASTLE_HASHES: [u64; 4] = [0; 4];
pub static mut BLACK_HASH: u64 = 0;

pub fn setup_hashes() {
    let mut rng = rng();
    for piece_type in 0..12 {
        for square in 0..64 {
            unsafe { SQUARE_HASHES[piece_type][square] = rng.next_u64() }
        }
    }
    for file in 0..8 {
        unsafe { EN_PASSANT_HASHES[file] = rng.next_u64() }
    }
    for castle in 0..4 {
        unsafe { CASTLE_HASHES[castle] = rng.next_u64() }
    }
    unsafe { BLACK_HASH = rng.next_u64() }
}

impl State {
    pub fn get_hash(&self) -> u64 {
        let mut result = 0;
        for piece_type in 0..12 {
            let mut board = self.board[piece_type];
            while board != 0 {
                result ^= unsafe { SQUARE_HASHES[piece_type][pop_lsb(&mut board) as usize] };
            }
        }
        for color in 0..2 {
            match self.castle_availability[color] {
                CastleAvailability::Both => {
                    result ^= unsafe { CASTLE_HASHES[color*2] };
                    result ^= unsafe { CASTLE_HASHES[color*2 + 1] };
                },
                CastleAvailability::King => result ^= unsafe { CASTLE_HASHES[color*2] },
                CastleAvailability::Queen => result ^= unsafe { CASTLE_HASHES[color*2 + 1] },
                CastleAvailability::None => (),
            }
        }
        if self.en_passant_square != NULL_SQUARE {
            result ^= unsafe { EN_PASSANT_HASHES[file(self.en_passant_square) as usize] };
        }
        if self.turn == Color::Black {
            result ^= unsafe { BLACK_HASH }
        }
        result
    }
}