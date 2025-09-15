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
pub mod transposition;
pub mod worker;

pub mod tests;

use std::{io::{stdin, stdout, Write}, time::Duration};

use crate::{bitboard::Color, evaluation::eval_info_init, r#move::{build_move, debug_same_src_des, move_destination_square, move_origin_square, move_special_type, BISHOP_PROMOTION, KNIGHT_PROMOTION, PROMOTION_SPECIAL_MOVE, QUEEN_PROMOTION, ROOK_PROMOTION}, parsing::{simple_move_from_string, starting_fen}, piece_info::move_gen_init, search::search_init, transposition::{free_ttable, ttable_init}, worker::Worker};

fn main() {
    move_gen_init();
    eval_info_init();
    search_init();
    unsafe { ttable_init(2048) };
    ui_game();
    unsafe { free_ttable() };
}

#[allow(dead_code)]
fn ui_game() {
    let player_side = prompt_until("What color do you want (white/black): ", |str| {
        let lower_case = str.to_lowercase();
        if lower_case == "white" || lower_case == "w" { return Some(Color::White) }
        else if lower_case == "black" || lower_case == "b" { return Some(Color::Black) }
        else { return None; }
    });
    let mut state = starting_fen();
    let mut worker = Worker::new();
    let mut game_over = false;
    let mut player_turn = if player_side == state.turn { true } else { false };
    while !game_over {
        println!("{}", state);
        if player_turn {
            state.debug_quick_gen_moves();
            let non_validated_moves = state.debug_move_vec();
            state.current_move_list().reset();
            let valid_moves = state.debug_validate_moves(&non_validated_moves);
            let mut user_move = prompt_until("Enter a move: ", |str| {
                if str.len() != 4 { return None }
                let m = match simple_move_from_string(str.to_string()) {
                    Some(val) => val,
                    None => return None
                };
                for valid_m in valid_moves.iter() {
                    if debug_same_src_des(m, *valid_m) {
                        return Some(*valid_m)
                    }
                }
                None
            });
            if move_special_type(user_move) == PROMOTION_SPECIAL_MOVE {
                let promotion = prompt_until("What piece do you want to promote to (queen/rook/bishop/knight): ", |str| {
                    match str.to_lowercase() {
                        val if val == "queen".to_string() => Some(QUEEN_PROMOTION),
                        val if val == "rook".to_string() => Some(ROOK_PROMOTION),
                        val if val == "bishop".to_string() => Some(BISHOP_PROMOTION),
                        val if val == "knight".to_string() => Some(KNIGHT_PROMOTION),
                        _ => None
                    }
                });
                user_move = build_move(move_origin_square(user_move), move_destination_square(user_move), promotion, PROMOTION_SPECIAL_MOVE)
            }
            assert!(state.non_reversible_move(user_move))
        } else {
            let search_time = prompt_until("How long would you like to search: ", |str| {
                match str.parse::<f64>() {
                    Ok(val) => Some(val),
                    Err(_) => None,
                }
            });
            let best_move = worker.iterative_deepening_search(&mut state, Duration::from_secs_f64(search_time), true);
            state.non_reversible_move(best_move);
        }
        state.debug_quick_gen_moves();
        let mut moves = state.debug_move_vec();
        moves = state.debug_validate_moves(&moves);
        state.current_move_list().reset();
        if moves.len() == 0 {
            println!("{}", state);
            if state.check {
                if player_turn {
                    println!("You Win");
                } else {
                    println!("Rhobos Wins");
                }
            } else {
                println!("Stalemate");
            }
            game_over = true
        } else if state.half_move_clock >= 100 {
            println!("{}", state);
            println!("Draw by 50 move rule");
            game_over = true
        }
        player_turn = !player_turn
    }
}

#[allow(dead_code)]
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