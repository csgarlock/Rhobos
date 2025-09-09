#![feature(adt_const_params)]
#![allow(static_mut_refs)]

pub mod bitboard;
pub mod debugging;
pub mod evaluation;
pub mod hash;
pub mod histories;
pub mod magic;
pub mod move_gen;
pub mod move_list;
pub mod move_pick;
pub mod r#move;
pub mod parsing;
pub mod piece_info;
pub mod search;
pub mod state;

pub mod tests;

use std::io::{stdin, stdout, Write};

use crate::{bitboard::Color, evaluation::{HIGHEST_EVAL, LOWEST_EVAL}, r#move::{debug_same_src_des, pretty_string_move, Move}, parsing::{parse_fen_string, simple_move_from_string, starting_fen}, piece_info::movement_info_init, search::Worker};

fn main() {
    movement_info_init();
    // perft_test();
    // let mut state = parse_fen_string("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/1R2K2R b Kkq - 1 1".to_string()).unwrap();
    // state.gen_all_moves::<{Color::Black},{MoveGenType::All}>();
    let mut state = parse_fen_string("8/1b2k3/8/8/8/pR1nK3/8/8 w - - 0 1".to_string()).unwrap();
    let mut worker = Worker { true_depth: 0, nodes_search: 0 };
    println!("{}", pretty_string_move(worker.negamax::<{Color::White}>(&mut state, 3, LOWEST_EVAL, HIGHEST_EVAL).1));
}

fn ui_game() {
    let player_side = prompt_until("What color do you want (white/black)", |str| {
        let lower_case = str.to_lowercase();
        if lower_case == "white" || lower_case == "w" { return Some(Color::White) }
        else if lower_case == "black" || lower_case == "b" { return Some(Color::Black) }
        else { return None; }
    });
    let mut state = starting_fen();
    let mut game_over = false;
    let mut play_turn = if player_side == state.turn { true } else { false };
    while !game_over {
        println!("{}", state);
        if play_turn {
            state.debug_quick_gen_moves();
            let non_validated_moves = state.debug_move_vec();
            let valid_moves = state.debug_validate_moves(&non_validated_moves);
            let user_move = prompt_until("Enter a move", |str| {
                if str.len() != 4 { return None }
                let m = match simple_move_from_string(str.to_string()) {
                    Some(val) => val,
                    None => return None
                };
                for valid_m in valid_moves.iter() {
                    if debug_same_src_des(m, *valid_m) {
                        return Some(m)
                    }
                }
                None
            });
            
        }
    }
}

fn prompt_until<T, F>(prompt: &str, parser: F) -> T
where
    F: Fn(&str) -> Option<T>,
{
    loop {
        print!("{}", prompt);
        stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        if stdin().read_line(&mut input).is_err() {
            eprintln!("Error reading input. Try again.");
            continue;
        }

        let input = input.trim();
        if let Some(value) = parser(input) {
            return value;
        } else {
            println!("Invalid input. Please try again.");
        }
    }
}