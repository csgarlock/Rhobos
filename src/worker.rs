use crate::{evaluation::Evaluation, search::Depth};


pub struct Worker {
    pub main_thread: bool,
    pub root_ply:    u16,
    pub nodes_searched: u64,
    pub last_ids_score: Evaluation,
    pub current_iids_depth: Depth
}

impl Worker {
    pub fn new() -> Worker {
        Worker {
            main_thread: false,
            root_ply: 0,
            nodes_searched: 0,
            last_ids_score: 0,
            current_iids_depth: -1,
        }
    }

    #[inline(always)]
    pub fn true_depth(&self, current_ply: u16) -> Depth {
        (self.root_ply - current_ply) as Depth
    }
}