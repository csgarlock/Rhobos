use std::io::stdin;

use crate::{bitboard::Color, move_gen::MoveGenType, parsing::parse_fen_string, piece_info::movement_info_init, state::State, tests::init};

const PERFT_TEST_CASES: [(&str, i64, i64); 1] = [
    ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 3, 8_902),
];

#[test]
fn perft_test() {
    init();
    for (i, case) in PERFT_TEST_CASES.iter().enumerate() {
        let mut move_count = 0;
        let mut state = parse_fen_string(case.0.to_string()).unwrap();
        match state.turn {
            Color::White => perft::<{Color::White}>(&mut state, case.1, &mut move_count),
            Color::Black => perft::<{Color::Black}>(&mut state, case.1, &mut move_count),
        }
        assert_eq!(move_count, case.2);
    }
}

fn perft<const C: Color>(state: &mut State, depth: i64, move_count: &mut i64) {
    if depth == 0 {
        *move_count += 1;
    } else {
        state.gen_all_moves::<C, {MoveGenType::All}>();
        for i in 0..state.move_stack.get_current().total_moves() {
            let m = state.move_stack.get_current().current();
            state.make_move::<C>(m);
            match C {
                Color::White => perft::<{Color::Black}>(state, depth-1, move_count),
                Color::Black => perft::<{Color::White}>(state, depth-1, move_count),
            }
            state.unmake_move::<C>(m);
        }
    }
}

fn debug_quick_perft(state: &mut State, depth: i64, move_count: &mut i64) {
    match state.turn {
        Color::White => perft::<{Color::White}>(state, depth, move_count),
        Color::Black => perft::<{Color::Black}>(state, depth, move_count),
    }
}

fn perft_checker(state: &mut State, depth: i64) {
    let mut current_depth = depth;
    loop {
        state.debug_quick_gen_moves();
        let mut temp = state.move_stack.get_current().debug_string_moves();
        let mut moves: Vec<(usize, &String)> = temp.iter().enumerate().collect();
        moves.sort_by(|(_, string1), (_, string2)| string1.cmp(string2));
        for (i, m) in moves.iter() {
            let m= state.move_stack.get_current().vec[*i];
            state.debug_quick_make_move(m);
            let mut counter = 0;
            debug_quick_perft(state, current_depth, &mut counter);
            println!("{}, Move {}: {}. M", m, i, counter);
            state.debug_quick_unmake_move(m);
        }
        let move_num = (|| {
            loop {
                println!("Enter move number: ");
                let mut input = String::new();
                if stdin().read_line(&mut input).is_ok() {
                    match input.trim().parse::<usize>() {
                        Ok(num) if num > 0 => return num,
                        _ => {
                            println!("Invalid input. Try again.");
                        }
                    }
                } else {
                    println!("Failed to read input. Try again.");
                }
            }
        })();
        let chosen_move = state.move_stack.get_current().vec[move_num];
        state.debug_quick_make_move(chosen_move);
        current_depth -= 1;
    }
}