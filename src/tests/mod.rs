use std::sync::Once;

use crate::piece_info::move_gen_init;

pub mod perft;

static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(move_gen_init);
}