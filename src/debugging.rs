#![allow(dead_code)]
use std::{io::stdin, time::Instant};

use crate::{bitboard::Color, r#move::{pretty_string_move, Move}, state::State, tests::perft::perft};


pub fn perft_checker(state: &mut State, depth: i64) {
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


pub fn debug_quick_perft(state: &mut State, depth: i64, move_count: &mut i64) {
    match state.turn {
        Color::White => perft::<{Color::White}>(state, depth, move_count),
        Color::Black => perft::<{Color::Black}>(state, depth, move_count),
    }
}

impl State {
    pub fn debug_quick_make_move(&mut self, m: Move) -> bool {
        match self.turn {
            Color::White => self.make_move::<{Color::White}>(m),
            Color::Black => self.make_move::<{Color::Black}>(m),
        }
    }

    pub fn debug_quick_unmake_move(&mut self, m: Move) {
        match self.turn {
            Color::White => self.unmake_move::<{Color::Black}>(m),
            Color::Black => self.unmake_move::<{Color::White}>(m),
        }
    }
}