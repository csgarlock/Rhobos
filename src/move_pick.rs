use std::{marker::ConstParamTy, mem::transmute};
use crate::{bitboard::Color, evaluation::{PAWN_EVAL, PIECE_EVAL_TABLE}, r#move::{move_destination_square, move_origin_square, move_special_info, move_special_type, Move, EN_PASSANT_SPECIAL_MOVE, NULL_MOVE, PROMOTION_SPECIAL_MOVE}, move_gen::MoveGenType, move_list::NUM_KILLERS, piece_info::PieceType, state::State};

// Set to 1 so that there will always be a move better than null move for quiet move ordering.
pub static mut HISTORY_TABLE: [[u64; 64]; 12] = [[1; 64]; 12];

#[derive(Clone, Copy, PartialEq, Eq, ConstParamTy)]
#[repr(u8)]
pub enum MovePickType {
    Negamax,
    Quiescence,
}

#[derive(Clone, Copy, PartialEq, Eq, ConstParamTy, Debug)]
#[repr(u8)]
pub enum MovePickStage {
    Start,
    TTMove,
    CaptureMoves,
    KillerMoves,
    QuietMoves,
    Done,
}

impl State {
    pub fn pick_next_move<const T: MovePickType>(&mut self) -> bool {
        match self.current_move_list().move_pick_stage {
            MovePickStage::Start => {
                self.next_stage::<T, {MovePickStage::Start}>();
                self.pick_next_move::<T>()
            },
            MovePickStage::TTMove => {
                if self.current_move_list().tt_move != NULL_MOVE {
                    self.current_move_list().current = self.current_move_list().tt_move;
                    for i in 0..self.current_move_list().last {
                        if self.current_move_list().move_vec[i] == self.current_move_list().tt_move {
                            self.current_move_list().move_vec[i] = NULL_MOVE;
                            break;
                        }
                    }
                    self.current_move_list().tt_move = NULL_MOVE;
                    true
                } else {
                    self.next_stage::<T, {MovePickStage::TTMove}>();
                    self.pick_next_move::<T>()
                }
            },
            MovePickStage::CaptureMoves => {
                let mut best_move = NULL_MOVE;
                let mut best_index = 0;
                let mut best_move_score = i32::MIN;
                for i in 0..self.current_move_list().last {
                    let contending_move = self.current_move_list().move_vec[i];
                    if contending_move == NULL_MOVE {
                        continue;
                    }
                    let contending_score = unsafe { self.current_move_list().value_vec[i].attack_val };
                    if contending_score > best_move_score {
                        best_move = contending_move;
                        best_index = i;
                        best_move_score = contending_score;
                    }
                }
                if best_move == NULL_MOVE {
                    self.next_stage::<T, {MovePickStage::CaptureMoves}>();
                    self.pick_next_move::<T>()
                } else {
                    self.current_move_list().current = best_move;
                    self.current_move_list().move_vec[best_index] = NULL_MOVE;
                    true
                }
            },
            MovePickStage::KillerMoves => {
                for i in 0..self.current_move_list().last {
                    let m = self.current_move_list().move_vec[i];
                    for j in 0..NUM_KILLERS {
                        if self.current_move_list().killer_moves[j] == m && self.current_move_list().killer_moves[j] != NULL_MOVE {
                            self.current_move_list().move_vec[i] = NULL_MOVE;
                            self.current_move_list().current = m;
                            return true;
                        }
                    }
                }
                self.next_stage::<T, {MovePickStage::KillerMoves}>();
                self.pick_next_move::<T>()
            },
            MovePickStage::QuietMoves => {
                let mut best_move = NULL_MOVE;
                let mut best_index = 0;
                let mut best_move_score = u64::MIN;
                for i in 0..self.current_move_list().last {
                    let contending_move = self.current_move_list().move_vec[i];
                    if contending_move == NULL_MOVE {
                        continue;
                    }
                    let contending_score = unsafe { self.current_move_list().value_vec[i].quiet_val };
                    if contending_score > best_move_score {
                        best_move = contending_move;
                        best_index = i;
                        best_move_score = contending_score;
                    }
                }
                if best_move == NULL_MOVE {
                    self.next_stage::<T, {MovePickStage::QuietMoves}>();
                    self.pick_next_move::<T>()
                } else {
                    self.current_move_list().current = best_move;
                    self.current_move_list().move_vec[best_index] = NULL_MOVE;
                    true
                }
            },
            MovePickStage::Done => false,
        }
    }

    #[inline(always)]
    fn next_stage<const T: MovePickType, const S: MovePickStage>(&mut self) {
        self.current_move_list().move_pick_stage = match T {
            MovePickType::Negamax => {
                match S {
                    MovePickStage::Start => MovePickStage::TTMove,
                    MovePickStage::TTMove => {
                        match self.turn {
                            Color::White => self.gen_all_moves::<{Color::White}, {MoveGenType::Capture}>(),
                            Color::Black => self.gen_all_moves::<{Color::Black}, {MoveGenType::Capture}>(),
                        }
                        self.assign_capture_scores();
                        MovePickStage::CaptureMoves
                    },
                    MovePickStage::CaptureMoves => {
                        if self.current_move_list().is_futile {
                            MovePickStage::Done
                        } else {
                            self.current_move_list().last = 0;
                            match self.turn {
                                Color::White => self.gen_all_moves::<{Color::White}, {MoveGenType::Quiet}>(),
                                Color::Black => self.gen_all_moves::<{Color::Black}, {MoveGenType::Quiet}>(),
                            }
                            self.assign_quiet_scores();
                            MovePickStage::KillerMoves
                        }
                    },
                    MovePickStage::KillerMoves => MovePickStage::QuietMoves,
                    MovePickStage::QuietMoves => MovePickStage::Done,
                    MovePickStage::Done => MovePickStage::Done,
                }
            },
            MovePickType::Quiescence => {
                match S {
                    MovePickStage::Start => {
                        match self.turn {
                            Color::White => self.gen_all_moves::<{Color::White}, {MoveGenType::Capture}>(),
                            Color::Black => self.gen_all_moves::<{Color::Black}, {MoveGenType::Capture}>(),
                        }
                        self.assign_capture_scores();
                        MovePickStage::CaptureMoves
                    },
                    MovePickStage::CaptureMoves => MovePickStage::Done,
                    MovePickStage::Done => MovePickStage::Done,
                    _ => unreachable!(),
                }
            }
        }
    }

    #[inline(always)]
    pub fn assign_quiet_scores(&mut self) {
        for i in 0..self.current_move_list().last {
            let m = self.current_move_list().move_vec[i];
            self.current_move_list().value_vec[i].quiet_val = self.move_quiet_score(m)
        }
    }

    #[inline(always)]
    pub fn assign_capture_scores(&mut self) {
        for i in 0..self.current_move_list().last {
            let m = self.current_move_list().move_vec[i];
            let (lva_mvv, futility_score) = self.move_capture_score(m);
            if self.current_move_list().is_futile && futility_score < self.current_move_list().futility_margin {
                self.current_move_list().move_vec[i] = NULL_MOVE;
            } else {
                self.current_move_list().value_vec[i].attack_val = lva_mvv;
            }
        }
    }

    #[inline(always)]
    pub fn move_quiet_score(&self, m: Move) -> u64 {
        let src_piece_type = match self.turn {
            Color::White => self.force_get_colored_piece_at_square::<{Color::White}>(move_origin_square(m)),
            Color::Black => self.force_get_colored_piece_at_square::<{Color::Black}>(move_origin_square(m)),
        };
        unsafe { HISTORY_TABLE[src_piece_type as usize][move_destination_square(m) as usize] }
    }

    // First return is lva-mva second is victim value/pawn_promotion value
    #[inline(always)]
    pub fn move_capture_score(&self, m: Move) -> (i32, i32) {
        if move_special_type(m) == PROMOTION_SPECIAL_MOVE {
            let promotion_type: PieceType = unsafe {transmute(move_special_info(m) + 1)};
            if let Some(des) = match self.turn {
                Color::White => self.get_colored_piece_at_square::<{Color::Black}>(move_destination_square(m)),
                Color::Black => self.get_colored_piece_at_square::<{Color::White}>(move_destination_square(m)),
            } {
                return (PIECE_EVAL_TABLE[promotion_type as usize], PIECE_EVAL_TABLE[promotion_type as usize] + PIECE_EVAL_TABLE[des as usize])
            } else {
                return (PIECE_EVAL_TABLE[promotion_type as usize], PIECE_EVAL_TABLE[promotion_type as usize]);
            }
        }
        // Currently this check is technically not needed since force_get_colored_piece_at_square returns PieceType::Pawn
        // if there is no piece present, but it is currently set to panic in debug if no piece is present. So this
        // may want to be changed in future, but since relying on that is asking for a subtle bug in the future it may want
        // to be kept this way for now.
        if move_special_type(m) == EN_PASSANT_SPECIAL_MOVE {
            return (0, PAWN_EVAL);
        }
        let (src, des) = match self.turn {
            Color::White => {
                (self.force_get_colored_piece_at_square::<{Color::White}>(move_origin_square(m)),
                self.force_get_colored_piece_at_square::<{Color::Black}>(move_destination_square(m)))
            },
            Color::Black => {
                (self.force_get_colored_piece_at_square::<{Color::Black}>(move_origin_square(m)),
                self.force_get_colored_piece_at_square::<{Color::White}>(move_destination_square(m)))
            }
        };
        (PIECE_EVAL_TABLE[des as usize] - PIECE_EVAL_TABLE[src as usize], PIECE_EVAL_TABLE[des as usize])
    }
}
