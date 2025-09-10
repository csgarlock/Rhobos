use std::{io::stdin, time::Instant};

use crate::{bitboard::{Color, Square}, r#move::{pretty_string_move, Move}, move_gen::MoveGenType, piece_info::PieceType, state::State, tests::perft::perft};


pub fn perft_checker(state: &mut State, depth: i64) {
    let mut current_depth = depth - 1;
    loop {
        let start = Instant::now();
        state.debug_quick_gen_moves();
        let mut total = 0;
        let mut results = Vec::new();
        for (i, m) in state.debug_move_vec().iter().enumerate() {
            if state.debug_quick_make_move(*m) {
                let mut counter = 0;
                debug_quick_perft(state, current_depth, &mut counter);
                results.push(format!("{}, Move {}: {}", pretty_string_move(*m), i, counter));
                total += counter;
            }
            state.debug_quick_unmake_move(*m);
        }
        results.sort();
        println!("Ply Moves: {}", state.current_move_list().total_moves());
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
        let chosen_move = state.current_move_list().move_vec[move_num];
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

    pub fn debug_quick_gen_moves(&mut self) {
        match self.turn {
            Color::White => self.gen_all_moves::<{Color::White}, {MoveGenType::All}>(),
            Color::Black => self.gen_all_moves::<{Color::Black}, {MoveGenType::All}>(),
        }
    }

    pub fn debug_validate_moves(&mut self, moves: &Vec<Move>) -> Vec<Move> {
        let mut result = Vec::new();
        for m in moves.iter() {
            if self.debug_quick_make_move(*m) {
                result.push(*m);
            }
            self.debug_quick_unmake_move(*m);
        }
        result
    }

    pub fn debug_get_piece_at_square(&mut self, square: Square, color: Color) -> Option<PieceType> {
        match color {
            Color::White => self.get_colored_piece_at_square::<{Color::White}>(square),
            Color::Black => self.get_colored_piece_at_square::<{Color::Black}>(square),
        }
    }

    pub fn debug_move_vec(&mut self) -> Vec<Move> {
        let mut result = Vec::new();
        for i in 0..self.current_move_list().last {
            result.push(self.current_move_list().move_vec[i]);
        }
        result
    }
}