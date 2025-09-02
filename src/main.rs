#![feature(adt_const_params)]
#![allow(static_mut_refs)]
#![allow(unused)]

use crate::{bitboard::{pop_lsb, pretty_string_bitboard, pretty_string_square, Bitboard, Color, EMPTY_BITBOARD}, parsing::{parse_fen_string, starting_fen}, piece_info::{movement_info_init, KNIGHT, MOVE_BOARDS}, state::MoveGenType};

mod bitboard;
mod histories;
mod magic;
mod move_list;
mod r#move;
mod parsing;
mod piece_info;
mod state;

fn main() {
    movement_info_init();
    let mut state = parse_fen_string("rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3".to_string()).unwrap();
    println!("{}", state);
    println!("{}", pretty_string_square(state.en_passant_square));
    state.gen_all_moves::<{Color::White}, {MoveGenType::All}>();
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