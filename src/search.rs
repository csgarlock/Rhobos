use crate::{bitboard::Color, evaluation::Evaluation, r#move::{Move, NULL_MOVE}, move_pick::MovePickType, state::State};

type Depth = u32;

pub struct Worker {
    pub true_depth: u16,
    pub nodes_search: u64,
}

impl Worker {
    pub fn negamax<const C: Color>(&mut self, state: &mut State, depth: Depth, mut alpha: Evaluation, beta: Evaluation) -> (Evaluation, Move) {
        debug_assert_eq!(C, state.turn);
        self.nodes_search += 1;
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
        while state.pick_next_move::<{MovePickType::Negamax}>() {
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
        self.true_depth -= 1;
        (alpha, best_move)
    }

    pub fn quiescence_search<const C: Color>(&mut self, state: &mut State, mut alpha: Evaluation, beta: Evaluation) -> (Evaluation, Move) {
        debug_assert_eq!(C, state.turn);
        self.nodes_search += 1;
        self.true_depth += 1;
        let current_eval = state.eval_state();
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