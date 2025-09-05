use std::sync::Once;

use crate::piece_info::movement_info_init;

pub mod perft;

static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| movement_info_init());
}