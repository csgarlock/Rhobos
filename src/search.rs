use crate::{bitboard::Color, evaluation::{Evaluation, CENTI_PAWN}, r#move::{Move, NULL_MOVE}, move_pick::MovePickType, state::State};

const FUTILITY_MARGIN: Evaluation = CENTI_PAWN * 90;

type Depth = u32;

pub struct Worker {
    pub true_depth: u16,
    pub nodes_search: u64,
}

impl Worker {
    pub fn negamax(&mut self, state: &mut State, depth: Depth, alpha: Evaluation, beta: Evaluation) {
        self.nodes_search += 1;
        self.true_depth += 1;
    }

    pub fn quiescence_search<const C: Color>(&mut self, state: &mut State, mut alpha: Evaluation, beta: Evaluation) -> (Evaluation, Move) {
        self.nodes_search += 1;
        let current_eval = state.eval_state();
        if current_eval >= beta {
            return (beta, NULL_MOVE);
        }
        if alpha < current_eval {
            alpha = current_eval;
        }
        
        while state.pick_next_move::<{MovePickType::Quiescence}>() {

        }

        todo!()
    }
}