#![feature(adt_const_params)]
#![allow(static_mut_refs)]
#![allow(dead_code)]
#![allow(unused)]

use crate::{bitboard::{pretty_string_bitboard, pretty_string_square, square_from_string, Bitboard, UNIVERSAL_BITBOARD}, magic::SubsetIterator, r#move::{build_move, simple_move_from_string}, piece_info::movement_info_init};

mod bitboard;
mod piece_info;
mod state;
mod magic;
mod r#move;

fn main() {
    movement_info_init();
    let e2 = square_from_string("e2".to_string());
    println!("{}", pretty_string_square(e2));
}

fn bools_to_u64(bits: [bool; 64]) -> Bitboard {
    bits.iter().enumerate().fold(0u64, |acc, (i, &b)| {
        if b {
            acc | (1u64 << i)
        } else {
            acc
        }
    })
}