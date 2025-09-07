#![feature(adt_const_params)]
#![allow(static_mut_refs)]

pub mod bitboard;
pub mod debugging;
pub mod hash;
pub mod histories;
pub mod magic;
pub mod move_gen;
pub mod move_list;
pub mod r#move;
pub mod parsing;
pub mod piece_info;
pub mod state;

pub mod tests;

use crate::{debugging::perft_checker, parsing::starting_fen, piece_info::movement_info_init};

fn main() {
    movement_info_init();
    // perft_test();
    // let mut state = parse_fen_string("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/1R2K2R b Kkq - 1 1".to_string()).unwrap();
    // state.gen_all_moves::<{Color::Black},{MoveGenType::All}>();
    let mut state = starting_fen();
    perft_checker(&mut state, 7);
}