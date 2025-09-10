use std::time::{Duration, Instant};

use crate::{bitboard::Color, evaluation::{mate_in, pretty_string_eval, Evaluation, CENTI_PAWN, NEGATIVE_MATE_ZERO, POSITIVE_MATE_ZERO}, r#move::{pretty_string_move, Move, NULL_MOVE}, move_pick::MovePickType, state::State, worker::Worker};

pub type Depth = i32;

const MAX_ASPIRATION_OFFSET_INDEX: usize = 32;
const ASPIRATION_OFFSET: [Evaluation; MAX_ASPIRATION_OFFSET_INDEX] = aspiration_window_offsets();

const ASPIRATION_MATE_CUTOFF: Evaluation = 2000 * CENTI_PAWN;

impl Worker {
    pub fn iterative_deepening_search(&mut self, state: &mut State, search_time: Duration, info_print: bool) -> Move {
        let start = Instant::now();
        self.true_depth = 0;
        let start_node_count = self.nodes_searched;

        let mut eval_guess = self.last_ids_score;
        let mut aspiration_delta = ASPIRATION_OFFSET[0];
        let mut aspiration_window_low = eval_guess - aspiration_delta;
        let mut aspiration_window_high = eval_guess + aspiration_delta;
        let mut best_move = NULL_MOVE;
        let mut current_depth = 1;

        while start.elapsed() < search_time {
            if aspiration_window_high > ASPIRATION_MATE_CUTOFF {
                aspiration_window_low = ASPIRATION_MATE_CUTOFF - (5 * CENTI_PAWN);
                aspiration_window_high = POSITIVE_MATE_ZERO;
            } else if aspiration_window_low < -ASPIRATION_MATE_CUTOFF {
                aspiration_window_low = NEGATIVE_MATE_ZERO;
                aspiration_window_high = -ASPIRATION_MATE_CUTOFF + (5 * CENTI_PAWN);
            }
            if info_print {
                println!("Search time left: {:?}", search_time - start.elapsed());
                println!("Searching next depth with window [{}, {}]", pretty_string_eval(aspiration_window_low), pretty_string_eval(aspiration_window_high));
            }
            let (new_score, new_move) = match state.turn {
                Color::White => self.negamax::<{Color::White}>(state, current_depth, aspiration_window_low, aspiration_window_high),
                Color::Black => self.negamax::<{Color::Black}>(state, current_depth, aspiration_window_low, aspiration_window_high),
            };
            if info_print {
                println!("Searched to depth: {}, Best move: {}, Move eval: {}", current_depth, pretty_string_move(new_move), pretty_string_eval(new_score));
            }
            debug_assert!(new_score >= aspiration_window_low && new_score <= aspiration_window_high);
            if new_score == aspiration_window_low {
                // Fail low
                if info_print {
                    println!("Search failed low");
                }
                aspiration_window_low -= aspiration_delta;
                aspiration_window_high -= aspiration_delta / 3;
                aspiration_delta *= 2;
            } else if new_score == aspiration_window_high {
                // Fail high
                if info_print {
                    println!("Search failed high");
                }
                aspiration_window_low += aspiration_delta / 3;
                aspiration_window_high += aspiration_delta;
                aspiration_delta *= 2;
            } else {
                if info_print {
                    println!("Search in bounds");
                }
                current_depth += 1;
                best_move = new_move;
                eval_guess = new_score;
                aspiration_delta = ASPIRATION_OFFSET[usize::min(current_depth as usize, MAX_ASPIRATION_OFFSET_INDEX - 1)];
                aspiration_window_low = eval_guess - aspiration_delta;
                aspiration_window_high = eval_guess + aspiration_delta;
            }
            state.current_move_list().reset();
        }
        self.last_ids_score = eval_guess;
        if info_print {
            println!("Best move: {}", pretty_string_move(best_move));
            println!("Move evaluation: {}", pretty_string_eval(eval_guess));
            println!("Total moves searched: {}", self.nodes_searched - start_node_count);
            println!("Total search time: {:?}", start.elapsed());
            println!("Search rate (MNPS): {:.2}", (self.nodes_searched - start_node_count) as f64 / start.elapsed().as_secs_f64() / 1_000_000.0);
        }
        best_move
    }

    pub fn negamax<const C: Color>(&mut self, state: &mut State, mut depth: Depth, mut alpha: Evaluation, beta: Evaluation) -> (Evaluation, Move) {
        debug_assert_eq!(C, state.turn);
        debug_assert!(alpha < beta);
        depth = depth.max(0);
        
        self.nodes_searched += 1;
        self.true_depth += 1;
        if depth == 0 {
            let result = match C { 
                Color::White => self.quiescence_search::<{Color::White}>(state, alpha, beta),
                Color::Black => self.quiescence_search::<{Color::Black}>(state, alpha, beta),
            };
            self.true_depth -= 1;
            return result;
        }

        let mut best_move = NULL_MOVE;
        let mut any_searched = false;
        while state.pick_next_move::<{MovePickType::Negamax}>() {
            any_searched = true;
            let current_move = state.current_move_list().current;
            if state.make_move::<C>(current_move) {
                let score = match C {
                    Color::White => -self.negamax::<{Color::Black}>(state, depth-1, -beta, -alpha).0,
                    Color::Black => -self.negamax::<{Color::White}>(state, depth-1, -beta, -alpha).0,
                };
                if score >= beta {
                    self.true_depth -= 1;
                    state.unmake_move::<C>(current_move);
                    return (score, current_move);
                }
                if score > alpha {
                    best_move = current_move;
                    alpha = score;
                }
            }
            state.unmake_move::<C>(current_move);
        }

        // If no moves were search it means either mate or stalemate
        if !any_searched {
            self.true_depth -= 1;
            if state.check {
                return (mate_in(self.true_depth, true), NULL_MOVE);
            } else {
                return (0, NULL_MOVE);
            }
        }

        self.true_depth -= 1;
        (alpha, best_move)
    }

    pub fn quiescence_search<const C: Color>(&mut self, state: &mut State, mut alpha: Evaluation, beta: Evaluation) -> (Evaluation, Move) {
        debug_assert_eq!(C, state.turn);
        debug_assert!(alpha < beta);
        self.nodes_searched += 1;
        self.true_depth += 1;
        let current_eval = state.eval_state(C);
        if current_eval >= beta {
            self.true_depth -= 1;
            return (beta, NULL_MOVE);
        }
        if alpha < current_eval {
            alpha = current_eval;
        }

        let mut best_move = NULL_MOVE;
        while state.pick_next_move::<{MovePickType::Quiescence}>() {
            let current_move = state.current_move_list().current;
            if state.make_move::<C>(current_move) {
                let score = match C {
                    Color::White => -self.quiescence_search::<{Color::Black}>(state, -beta, -alpha).0,
                    Color::Black => -self.quiescence_search::<{Color::White}>(state, -beta, -alpha).0,
                };
                if score >= beta {
                    state.unmake_move::<C>(current_move);
                    self.true_depth -= 1;
                    return (beta, current_move);
                }
                if score > alpha {
                    alpha = score;
                    best_move = current_move;
                }
            }
            state.unmake_move::<C>(current_move);
        }
        self.true_depth -= 1;
        (alpha, best_move)
    }
}

const fn aspiration_window_offsets() -> [Evaluation; MAX_ASPIRATION_OFFSET_INDEX] {
    [
        CENTI_PAWN * 30, CENTI_PAWN * 25, CENTI_PAWN * 20, CENTI_PAWN * 17, CENTI_PAWN * 14, CENTI_PAWN * 12, CENTI_PAWN * 10, CENTI_PAWN * 8,
        CENTI_PAWN *  7, CENTI_PAWN *  7, CENTI_PAWN *  6, CENTI_PAWN *  6, CENTI_PAWN *  5, CENTI_PAWN *  5, CENTI_PAWN *  5, CENTI_PAWN *  5,
        CENTI_PAWN *  5, CENTI_PAWN *  5, CENTI_PAWN *  5, CENTI_PAWN *  4, CENTI_PAWN *  4, CENTI_PAWN *  4, CENTI_PAWN *  4, CENTI_PAWN *  4,
        CENTI_PAWN *  4, CENTI_PAWN *  4, CENTI_PAWN *  4, CENTI_PAWN *  3, CENTI_PAWN *  3, CENTI_PAWN *  3, CENTI_PAWN *  3, CENTI_PAWN *  2,
    ]
}