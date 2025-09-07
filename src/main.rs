#![feature(adt_const_params)]
#![allow(static_mut_refs)]
#![allow(unused)]

pub mod bitboard;
pub mod histories;
pub mod magic;
pub mod move_gen;
pub mod move_list;
pub mod r#move;
pub mod parsing;
pub mod piece_info;
pub mod state;

pub mod tests;

use std::{io::stdin, time::Instant};

use crate::{bitboard::{pop_lsb, pretty_string_bitboard, pretty_string_square, Bitboard, Color, EMPTY_BITBOARD}, r#move::{build_move, pretty_string_move, CASTLE_SPECIAL_MOVE}, move_gen::MoveGenType, parsing::{parse_fen_string, simple_move_from_string, starting_fen}, piece_info::{movement_info_init, KNIGHT, MOVE_BOARDS}, state::State};

fn main() {
    movement_info_init();
    perft_test();
    // let mut state = parse_fen_string("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/1R2K2R b Kkq - 1 1".to_string()).unwrap();
    // state.gen_all_moves::<{Color::Black},{MoveGenType::All}>();
    // let mut state = parse_fen_string("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1".to_string()).unwrap();
    // perft_checker(&mut state, 7);
}


const PERFT_TEST_CASES: [(&str, i64, i64); 6] = [
    ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 6, 119060324),
    ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 5, 193690690),
    ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 7, 178633661),
    ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 5, 89941194),
    ("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", 5, 164075551),
    ("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1", 5, 3605103),
];

fn perft_test() {
    for (i, case) in PERFT_TEST_CASES.iter().enumerate() {
        let mut move_count = 0;
        let mut state = parse_fen_string(case.0.to_string()).unwrap();
        match state.turn {
            Color::White => perft::<{Color::White}>(&mut state, case.1, &mut move_count),
            Color::Black => perft::<{Color::Black}>(&mut state, case.1, &mut move_count),
        }
        assert_eq!(move_count, case.2);
    }
}

fn perft<const C: Color>(state: &mut State, depth: i64, move_count: &mut i64) {
    if depth == 0 {
        *move_count += 1;
    } else {
        state.gen_all_moves::<C, {MoveGenType::All}>();
        for i in 0..state.move_stack.get_current().total_moves() {
            let temp = state.move_stack.get_current();
            let m = state.move_stack.get_current().current();
            if state.make_move::<C>(m) {
                match C {
                    Color::White => perft::<{Color::Black}>(state, depth-1, move_count),
                    Color::Black => perft::<{Color::White}>(state, depth-1, move_count),
                }
            }
            state.unmake_move::<C>(m);
            state.move_stack.get_current().next();
        }
    }
}

fn debug_quick_perft(state: &mut State, depth: i64, move_count: &mut i64) {
    match state.turn {
        Color::White => perft::<{Color::White}>(state, depth, move_count),
        Color::Black => perft::<{Color::Black}>(state, depth, move_count),
    }
}

fn perft_checker(state: &mut State, depth: i64) {
    let mut current_depth = depth - 1;
    loop {
        let start = Instant::now();
        state.debug_quick_gen_moves();
        let mut total = 0;
        let mut results = Vec::new();
        for i in 0..state.move_stack.get_current().total_moves() {
            let m = state.move_stack.get_current().current();
            if state.debug_quick_make_move(m) {
                let mut counter = 0;
                debug_quick_perft(state, current_depth, &mut counter);
                results.push(format!("{}, Move {}: {}", pretty_string_move(m), i, counter));
                total += counter;
            }
            state.debug_quick_unmake_move(m);
            state.move_stack.get_current().next();
        }
        results.sort();
        println!("Ply Moves: {}", state.move_stack.get_current().total_moves());
        println!("Total Time: {:?}", start.elapsed());
        println!("Total Moves: {}", total);
        results.iter().for_each(|s| println!("{}", s));
        let move_num = (|| {
            loop {
                println!("Enter move number:");
                let mut input = String::new();
                if stdin().read_line(&mut input).is_ok() {
                    match input.trim().parse::<usize>() {
                        Ok(num) => return num,
                        _ => {
                            println!("Invalid input. Try again.");
                        }
                    }
                } else {
                    println!("Failed to read input. Try again.");
                }
            }
        })();
        let chosen_move = state.move_stack.get_current().vec[move_num];
        state.debug_quick_make_move(chosen_move);
        current_depth -= 1;
    }
}