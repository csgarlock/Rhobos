use std::collections::HashMap;

use crate::{bitboard::{get_lsb, Bitboard, Board, Color, Square, EMPTY_BITBOARD, FILE_MAP, NULL_SQUARE}, histories::History, r#move::{build_simple_move, Move}, move_list::MoveStack, piece_info::PieceType, state::{CastleAvailability, State}};

pub fn square_from_string(string: String) -> Square {
    let rank = string[1..].parse::<Square>().unwrap() - 1;
    let mut file: u8 = 0;
    for (i, r) in FILE_MAP.iter().enumerate() {
        if string.chars().nth(0).unwrap() == *r {
            file = i as u8;
        }
    }
    rank * 8 + file
}

pub fn simple_move_from_string(move_string: String) -> Move {
    build_simple_move(square_from_string(move_string[0..2].to_string()), square_from_string(move_string[2..4].to_string()))
}

pub fn parse_fen_string(fen_string: String) -> Result<State, String> {
    let mut piece_map = HashMap::new();
    const PIECE_CHARS: [char; 12] = ['K', 'Q', 'R', 'B', 'N', 'P', 'k', 'q', 'r', 'b', 'n', 'p'];
    for (i, c) in PIECE_CHARS.iter().enumerate() { piece_map.insert(*c, i); }
    let split_fen_string: Vec<&str> = fen_string.split(' ').collect();

    // Board section
    let mut board = [EMPTY_BITBOARD; 12];
    let board_string: Vec<&str> = split_fen_string[0].split('/').collect();
    for i in 0..8 {
        let rank= board_string[7-i];
        let mut column = 0;
        for c in rank.chars() {
            match piece_map.get(&c) {
                Some(index) => board[*index] |= 1 << (i*8 + column),
                None => {
                    match c.to_digit(10) {
                        Some(num) => column += num as usize - 1,
                        None => return Err("Unable to parse Fen String. Bad board layout".into()),
                    }
                }
            }
            column += 1;
        }
    }
    let mut side_occupied = [EMPTY_BITBOARD; 2];
    for i in 0..6 {
        side_occupied[0] |= board[i];
        side_occupied[1] |= board[6+i];
    }

    // Castle Section
    let castle_string = split_fen_string[2];
    let mut castle_availability = [CastleAvailability::None; 2];
    if castle_string != "-" {
        if castle_string.contains("K") {
            castle_availability[0] = CastleAvailability::King;
        }
        if castle_string.contains("Q") {
            castle_availability[0] = if castle_availability[0] == CastleAvailability::King {CastleAvailability::Both} else {CastleAvailability::Queen};
        }
        if castle_string.contains("k") {
            castle_availability[1] = CastleAvailability::King;
        }
        if castle_string.contains("q") {
            castle_availability[1] = if castle_availability[1] == CastleAvailability::King {CastleAvailability::Both} else {CastleAvailability::Queen};
        }
    }

    // En Passant Section
    const RANKS_CHARS: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
    let en_passant_string = split_fen_string[3];
    let mut en_passant_square = NULL_SQUARE;
    if en_passant_string != "-" {
        let mut rank_map = HashMap::new();
        for (i, c) in RANKS_CHARS.iter().enumerate() {
            rank_map.insert(*c, i);
        }
        let file_char = en_passant_string.chars().nth(0).unwrap();
        match rank_map.get(&file_char) {
            Some(c) => en_passant_square = *c as u8,
            None => return Err("Invalid en passant substring".into()),
        }
        let rank_char = en_passant_string.chars().nth(1).unwrap();
        match rank_char.to_digit(10) {
            Some(d) => en_passant_square += ((d-1) * 8) as u8,
            None => return Err("Invalid en passant substring".into()),
        }
    }

    // Ply Section
    let mut ply;
    match split_fen_string[5].parse::<u16>() {
        Ok(val) => ply = val,
        Err(err) => return Err(err.to_string()),
    }
    let mut state = State {
        board: board,
        side_occupied: side_occupied,
        occupied: side_occupied[0] | side_occupied[1],
        not_occupied: !(side_occupied[0] | side_occupied[1]),
        turn: if split_fen_string[1] == "w" {Color::White} else {Color::Black},
        ply: ply,
        can_en_passant: if en_passant_square == NULL_SQUARE {false} else {true},
        en_passant_square: en_passant_square,
        check: false,
        hashcode: 0,
        half_move_clock: split_fen_string[4].parse().unwrap(),
        move_stack: MoveStack::new(25),
        castle_availability: castle_availability,
        capture_history: History::new(5),
        en_passant_history: History::new(5),
        castle_history: History::new(5),
        fifty_move_history: History::new(5),
        hash_history: History::new(5),
    };
    if state.turn == Color::White {
        state.check = !state.is_square_safe::<{ Color::White }, false>(get_lsb(state.get_piece_board(Color::White, PieceType::King)), NULL_SQUARE);
    } else {
        state.check = !state.is_square_safe::<{ Color::Black }, false>(get_lsb(state.get_piece_board(Color::Black, PieceType::King)), NULL_SQUARE);
        state.ply += 1;
    }
    Ok(state)
}

pub fn starting_fen() -> State {
    parse_fen_string("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string()).unwrap()
}