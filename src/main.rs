#![feature(adt_const_params)]
#![allow(static_mut_refs)]
#![allow(dead_code)]
#![allow(unused)]

use crate::{bitboard::{pretty_string_bitboard, pretty_string_square, square_from_string, Bitboard, Square, UNIVERSAL_BITBOARD}, magic::{find_blocked_sliding_attacks, find_magic_seed, SubsetIterator}, r#move::{build_move, simple_move_from_string}, piece_info::{move_bitboard, movement_info_init, PieceType}};

mod bitboard;
mod histories;
mod piece_info;
mod state;
mod magic;
mod r#move;

fn main() {
    movement_info_init();
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