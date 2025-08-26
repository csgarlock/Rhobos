#![feature(adt_const_params)]
use crate::bitboard::{pretty_string_bitboard, UNIVERSAL_BITBOARD};

mod bitboard;

fn main() {
    println!("{}", pretty_string_bitboard(36));
}
