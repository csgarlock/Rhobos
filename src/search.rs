use std::{hint::unreachable_unchecked, time::{Duration, Instant}};

use crate::{bitboard::{board_from_square, Color, EMPTY_BITBOARD}, evaluation::{mate_in, pretty_string_eval, unchecked_eval_clamp, Evaluation, CENTI_PAWN, HIGHEST_EVAL, LOWEST_EVAL, MATE_VALUE_CUTOFF, NEGATIVE_MATE_ZERO}, r#move::{move_destination_square, move_origin_square, pretty_string_move, Move, NULL_MOVE}, move_pick::{MovePickType, HISTORY_TABLE}, piece_info::{KING, PAWN}, state::State, transposition::{add_tt_state, eval_convert_precision_low_to_high, parse_packed_depth_and_node, search_tt_state, NodeType}, worker::Worker};

pub type Depth = i32;
pub type Reduction = i32;

const MAX_ASPIRATION_OFFSET_INDEX: usize = 32;
const ASPIRATION_OFFSET: [Evaluation; MAX_ASPIRATION_OFFSET_INDEX] = aspiration_window_offsets();

const ASPIRATION_MATE_CUTOFF: Evaluation = 2000 * CENTI_PAWN;

const INTERNAL_IDS_DEPTH: Depth = 5;

const REDUCTION_FACTOR: Reduction = 1024;

const NULL_MOVE_REDUCTION: Depth = 2;
static mut LATE_MOVE_REDUCTION_TABLE: [[Reduction; 64]; 64] = [[0; 64]; 64];

impl Worker {
    pub fn iterative_deepening_search(&mut self, state: &mut State, search_time: Duration, info_print: bool) -> Move {
        let start = Instant::now();
        let start_node_count = self.nodes_searched;
        self.root_ply = state.ply;

        let mut eval_guess = self.last_ids_score;
        let mut aspiration_delta = ASPIRATION_OFFSET[0];
        let mut aspiration_window_low = eval_guess - aspiration_delta;
        let mut aspiration_window_high = eval_guess + aspiration_delta;
        let mut best_move = NULL_MOVE;
        let mut current_depth = 1;

        while start.elapsed() < search_time {
            if aspiration_window_high > ASPIRATION_MATE_CUTOFF {
                aspiration_window_low = ASPIRATION_MATE_CUTOFF - (200 * CENTI_PAWN);
                aspiration_window_high = HIGHEST_EVAL;
            } else if aspiration_window_low < -ASPIRATION_MATE_CUTOFF {
                aspiration_window_low = LOWEST_EVAL;
                aspiration_window_high = -ASPIRATION_MATE_CUTOFF + (200 * CENTI_PAWN);
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
                if eval_guess.abs() < MATE_VALUE_CUTOFF { 
                    aspiration_window_low = eval_guess - aspiration_delta;
                    aspiration_window_high = eval_guess + aspiration_delta;
                }
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

        // Quick fix for now. This does not need to be called every time as make_move usually handles this, however
        // when a search is repeated at the same depth from LMR re searches, IIDS, ect.., the current move
        // list is left in a garbage state.
        state.current_move_list().reset();
        
        depth = depth.max(0);
        self.nodes_searched += 1;
        let is_root = state.ply == self.root_ply;
        
        if depth == 0 {
            let result = match C { 
                Color::White => self.quiescence_search::<{Color::White}>(state, alpha, beta),
                Color::Black => self.quiescence_search::<{Color::Black}>(state, alpha, beta),
            };
            return result;
        }

        let tt_result = search_tt_state(state);
        if let Some(result) = tt_result {
            let tt_eval = eval_convert_precision_low_to_high(result.eval);
            let (tt_depth, tt_node_type) = parse_packed_depth_and_node(result.packed_depth_and_node);
            let tt_best_move = result.best_move;
            if tt_node_type == NodeType::TerminalNode && !is_root {
                let return_eval = if tt_eval == 0 {
                        unchecked_eval_clamp(0, alpha, beta)
                    } else {
                        unchecked_eval_clamp(mate_in(
                            self.true_depth(state.ply),
                            true,
                        ), alpha, beta)
                    };
                return (return_eval, NULL_MOVE);
            }
            if tt_depth >= depth && !is_root {
                match tt_node_type {
                    NodeType::PVNode => { if tt_eval >= alpha && tt_eval <= beta { return (tt_eval, tt_best_move) } },
                    NodeType::CutNode => { if tt_eval >= beta { return (beta, tt_best_move) } },
                    NodeType::AllNode => { if tt_eval <= alpha { return (alpha, tt_best_move) } },
                    _ => {debug_assert!(false); unsafe {unreachable_unchecked()}},
                }
            }
            if tt_best_move != NULL_MOVE {
                state.current_move_list().add_tt_move(tt_best_move);
            }

        } else if depth >= INTERNAL_IDS_DEPTH {
            // Internal iterative deepening search for getting a good first move.
            let iids_suggested_move = self.negamax::<C>(state, depth / 2, alpha, beta).1;
            // Reset the move list as after search it is in a garbage state.
            state.current_move_list().reset();
            if iids_suggested_move != NULL_MOVE {
                state.current_move_list().add_tt_move(iids_suggested_move);
            }
        }

        // Null move pruning
        if depth > 2 && !state.check && !is_root {
            // don't check for null move pruning if only king and pawns to help avoid mistreating zugzwang positions
            if state.side_occupied[C as usize] != (state.board[(C.board_offset() + PAWN) as usize] | state.board[(C.board_offset() + KING) as usize]) {
                state.passing_move::<C>();
                let score = match C {
                    Color::White => -self.negamax::<{Color::Black}>(state, depth-NULL_MOVE_REDUCTION-1, -beta, -beta+1).0,
                    Color::Black => -self.negamax::<{Color::White}>(state, depth-NULL_MOVE_REDUCTION-1, -beta, -beta+1).0,
                };
                state.un_passing_move::<C>();
                if score >= beta {
                    return (beta, NULL_MOVE);
                }
            } 
        }

        let mut best_move = NULL_MOVE;
        let mut move_count = 0;
        while state.pick_next_move::<{MovePickType::Negamax}>() {
            let current_move = state.current_move_list().current;
            let mut reduction = 0;
            if state.make_move::<C>(current_move) {
                let mut score;
                let should_full_re_search;
                if move_count != 0 {
                    if depth > 2 {
                        // Late move reduction
                        reduction += unsafe {LATE_MOVE_REDUCTION_TABLE[depth.min(63) as usize][move_count.min(63) as usize]};
                    }

                    let adjusted_reduction = reduction / REDUCTION_FACTOR;
                    // search with null windows as we expect late moves to fail low
                    score = match C {
                        Color::White => -self.negamax::<{Color::Black}>(state, depth-adjusted_reduction-1, -(alpha + 1), -alpha).0,
                        Color::Black => -self.negamax::<{Color::White}>(state, depth-adjusted_reduction-1, -(alpha + 1), -alpha).0
                    };
                    // should_full_re_search = true;
                    should_full_re_search = score > alpha;
                } else {
                    let adjusted_reduction = reduction / REDUCTION_FACTOR;
                    score = match C {
                        Color::White => -self.negamax::<{Color::Black}>(state, depth-adjusted_reduction-1, -beta, -alpha).0,
                        Color::Black => -self.negamax::<{Color::White}>(state, depth-adjusted_reduction-1, -beta, -alpha).0,
                    };
                    should_full_re_search = score > alpha && adjusted_reduction > 0;
                }

                // re search at full depth if a reduced depth search failed high.
                if should_full_re_search {
                    score = match C {
                        Color::White => -self.negamax::<{Color::Black}>(state, depth-1, -beta, -alpha).0,
                        Color::Black => -self.negamax::<{Color::White}>(state, depth-1, -beta, -alpha).0,
                    };
                }

                if score >= beta {
                    state.unmake_move::<C>(current_move);
                    add_tt_state(state, score, current_move, depth, NodeType::CutNode);
                    // add to history table if quiet move
                    if state.side_occupied[C.other() as usize] & board_from_square(move_destination_square(current_move)) == EMPTY_BITBOARD {
                        let src_piece_type = match C {
                            Color::White => state.force_get_colored_piece_at_square::<{Color::White}>(move_origin_square(current_move)),
                            Color::Black => state.force_get_colored_piece_at_square::<{Color::Black}>(move_origin_square(current_move)),
                        };
                        unsafe { HISTORY_TABLE [src_piece_type as usize][move_destination_square(current_move) as usize] += (depth * depth) as u64};
                        state.current_move_list().add_killer(current_move);
                    }
                    return (score, current_move);
                }
                if score > alpha {
                    best_move = current_move;
                    alpha = score;
                }
                move_count += 1;
            }
            state.unmake_move::<C>(current_move);
        }

        // If no moves were search it means either mate or stalemate
        if move_count == 0 {
            if state.check {
                add_tt_state(state, NEGATIVE_MATE_ZERO, NULL_MOVE, depth, NodeType::TerminalNode);
                return (mate_in(self.true_depth(state.ply), true).clamp(alpha, beta), NULL_MOVE);
            } else {
                add_tt_state(state, 0, NULL_MOVE, depth, NodeType::TerminalNode);
                return (0.clamp(alpha, beta), NULL_MOVE);
            }
        }

        if best_move == NULL_MOVE {
            add_tt_state(state, alpha, NULL_MOVE, depth, NodeType::AllNode);
        } else {
            add_tt_state(state, alpha, best_move, depth, NodeType::PVNode);
        }

        (alpha, best_move)
    }

    pub fn quiescence_search<const C: Color>(&mut self, state: &mut State, mut alpha: Evaluation, beta: Evaluation) -> (Evaluation, Move) {
        debug_assert_eq!(C, state.turn);
        debug_assert!(alpha < beta);
        self.nodes_searched += 1;
        let current_eval = state.eval_state(C);
        if current_eval >= beta {
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
                    return (beta, current_move);
                }
                if score > alpha {
                    alpha = score;
                    best_move = current_move;
                }
            }
            state.unmake_move::<C>(current_move);
        }
        (alpha, best_move)
    }
}

pub fn search_init() {
    for depth in 1..64 {
        for moves in 1..64 {
            unsafe { LATE_MOVE_REDUCTION_TABLE[depth][moves] = (REDUCTION_FACTOR as f64 * (0.6 + 0.4 * f64::ln(depth as f64) * f64::ln(moves as f64))) as Reduction };
        }
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