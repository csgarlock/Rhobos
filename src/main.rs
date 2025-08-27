#![feature(adt_const_params)]
use crate::{bitboard::{pretty_string_bitboard, UNIVERSAL_BITBOARD}, r#move::build_move};

mod bitboard;
mod state;
mod r#move;

fn main() {
    println!("{}", build_move(5, 12, 2, 1));
    foo();
}

#[inline(never)]
fn foo() {
    println!("Hello, World!");
    println!("Freaky World!");
}