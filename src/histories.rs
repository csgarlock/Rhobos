use crate::{bitboard::{Square, NULL_SQUARE}, piece_info::{PieceType, KING, NO_PIECE}};

pub const NO_LAST_PLY: u16 = 65530;

pub trait HistoryEntry : Copy {
    type Value: Sized + Copy;
    fn ply(self) -> u16;
    fn value(self) -> Self::Value;
    fn empty() -> Self;
    fn new(val: Self::Value, ply: u16) -> Self;
}

#[derive(Clone, Copy)]
pub struct CaptureEntry {
    piece: PieceType,
    ply: u16,
}

impl HistoryEntry for CaptureEntry {
    type Value = PieceType;
    fn ply(self) -> u16 { self.ply }
    fn value(self) -> Self::Value { self.piece }
    fn empty() -> Self { Self { piece: PieceType::King, ply: 0 } }
    fn new(val: Self::Value, ply: u16) -> Self { Self { piece: val, ply: ply } }
}

#[derive(Clone, Copy)]
pub struct EnPassantEntry {
    square: Square,
    ply: u16,
}

impl HistoryEntry for EnPassantEntry {
    type Value = Square;

    fn ply(self) -> u16 { self.ply }
    fn value(self) -> Self::Value { self.square }
    fn empty() -> Self { Self { square: NULL_SQUARE, ply: 0 } }
    fn new(val: Self::Value, ply: u16) -> Self { Self { square: val, ply: ply } }
}

#[derive(Clone, Copy)]
pub struct CastleHistoryEntry {
    castle: u8,
    ply: u16,
}

impl HistoryEntry for CastleHistoryEntry {
    type Value = u8;

    fn ply(self) -> u16 { self.ply }
    fn value(self) -> Self::Value { self.castle }
    fn empty() -> Self { Self { castle: 255, ply: 0 }}
    fn new(val: Self::Value, ply: u16) -> Self { Self { castle: val, ply: ply } }
}

#[derive(Clone, Copy)]
pub struct FiftyMoveHistory {
    last_count: u8,
    ply: u16,
}

impl HistoryEntry for FiftyMoveHistory {
    type Value = u8;

    fn ply(self) -> u16 { self.ply }
    fn value(self) -> Self::Value { self.last_count }
    fn empty() -> Self { Self { last_count: 255, ply: 0 }}
    fn new(val: Self::Value, ply: u16) -> Self { Self { last_count: val, ply: ply } }
}

impl HistoryEntry for u64 {
    type Value = u64;

    fn ply(self) -> u16 { unimplemented!() }
    fn value(self) -> Self::Value { self }
    fn empty() -> Self { 0 }
    fn new(val: Self::Value, ply: u16) -> Self { val }
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

    pub fn most_recent_ply(&self) -> u16 {
        if self.current_index == 0 {
            NO_LAST_PLY
        } else {
            self.vector[self.current_index - 1].ply()
        }
    }

    pub fn pop(&mut self) -> T {
        self.current_index -= 1;
        self.vector[self.current_index]
    }

    pub fn peek(&self) -> T {
        self.vector[self.current_index-1]
    }

    pub fn push(&mut self, value: T::Value, ply: u16) {
        if self.current_index >= self.vector.len() {
            self.vector.push(T::new(value, ply));
        } else {
            self.vector[self.current_index] = T::new(value, ply);
        }
        self.current_index += 1;
    } 
}