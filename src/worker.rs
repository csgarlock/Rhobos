use crate::evaluation::Evaluation;


pub struct Worker {
    pub main_thread: bool,
    pub true_depth: i16,
    pub nodes_searched: u64,
    pub last_ids_score: Evaluation,
}

impl Worker {
    pub fn new() -> Worker {
        Worker {
            main_thread: false,
            true_depth: 0,
            nodes_searched: 0,
            last_ids_score: 0,
        }
    }
}