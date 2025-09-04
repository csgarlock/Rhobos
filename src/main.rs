#![feature(adt_const_params)]
#![allow(static_mut_refs)]
#![allow(unused)]

use crate::{bitboard::{pop_lsb, pretty_string_bitboard, pretty_string_square, Bitboard, Color, EMPTY_BITBOARD}, move_gen::MoveGenType, parsing::{parse_fen_string, starting_fen}, piece_info::{movement_info_init, KNIGHT, MOVE_BOARDS}};

mod bitboard;
mod histories;
mod magic;
mod move_gen;
mod move_list;
mod r#move;
mod parsing;
mod piece_info;
mod state;

fn main() {
    movement_info_init();
    let mut state = parse_fen_string("1Rn5/1P2k3/8/8/8/3K4/8/8 w - - 0 1".to_string()).unwrap();
    state.gen_all_moves::<{Color::White}, {MoveGenType::Capture}>();
    for (i, s) in state.move_stack.get_current().debug_string_moves().iter().enumerate() {
        println!("{}", s);
    }
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