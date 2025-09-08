use crate::{bitboard::{Bitboard, Square, EMPTY_BITBOARD, NULL_SQUARE}, piece_info::PieceType, state::CastleAvailability};

pub trait HistoryEntry : Copy {
    type Value: Sized + Copy;

    fn value(self) -> Self::Value;
    fn empty() -> Self;
    fn new(val: Self::Value) -> Self;
}

#[derive(Clone, Copy)]
pub struct CaptureEntry {
    pub piece: Option<PieceType>,
    pub bitboard: Bitboard,
}

impl HistoryEntry for CaptureEntry {
    type Value = (Option<PieceType>, Bitboard);

    fn value(self) -> Self::Value { (self.piece, self.bitboard) }
    fn empty() -> Self { Self { piece: None, bitboard: EMPTY_BITBOARD } }
    fn new(val: Self::Value) -> Self { Self { piece: val.0, bitboard: val.1 } }
}

#[derive(Clone, Copy)]
pub struct EnPassantEntry {
    square: Square,
}

impl HistoryEntry for EnPassantEntry {
    type Value = Square;

    fn value(self) -> Self::Value { self.square }
    fn empty() -> Self { Self { square: NULL_SQUARE } }
    fn new(val: Self::Value) -> Self { Self { square: val } }
}

#[derive(Clone, Copy)]
pub struct CastleHistoryEntry {
    castle: CastleAvailability,
}

impl HistoryEntry for CastleHistoryEntry {
    type Value = CastleAvailability;

    fn value(self) -> Self::Value { self.castle }
    fn empty() -> Self { Self { castle: CastleAvailability::None }}
    fn new(val: Self::Value ) -> Self { Self { castle: val } }
}

#[derive(Clone, Copy)]
pub struct FiftyMoveHistory {
    last_count: u8,
}

impl HistoryEntry for FiftyMoveHistory {
    type Value = u8;

    fn value(self) -> Self::Value { self.last_count }
    fn empty() -> Self { Self { last_count: 255 }}
    fn new(val: Self::Value) -> Self { Self { last_count: val } }
}

impl HistoryEntry for u64 {
    type Value = u64;

    fn value(self) -> Self::Value { self }
    fn empty() -> Self { 0 }
    fn new(val: Self::Value ) -> Self { val }
}

impl HistoryEntry for bool {
    type Value = bool;

    fn value(self) -> Self::Value { self }
    fn empty() -> Self { false }
    fn new(val: Self::Value) -> Self { val }
}

pub struct History<T: HistoryEntry> {
    vector: Vec<T>,
    current_index: usize,
}


impl<T: HistoryEntry> History<T>  {
    pub fn new(starting_length: usize) -> History<T> {
        let vector = vec![T::empty(); starting_length as usize];
        History { vector: vector, current_index: 0 }
    }

    pub fn pop(&mut self) -> T {
        self.current_index -= 1;
        self.vector[self.current_index]
    }

    pub fn peek(&self) -> T {
        self.vector[self.current_index-1]
    }

    pub fn push(&mut self, value: T::Value) {
        if self.current_index >= self.vector.len() {
            self.vector.push(T::new(value));
        } else {
            self.vector[self.current_index] = T::new(value);
        }
        self.current_index += 1;
    } 
}