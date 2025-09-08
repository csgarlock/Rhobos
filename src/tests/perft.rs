#[allow(unused_imports)]
use crate::{bitboard::Color, move_gen::MoveGenType, parsing::parse_fen_string, state::State, tests::init};

#[allow(dead_code)]
const PERFT_TEST_CASES: [(&str, i64, i64); 6] = [
    ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 6, 119060324),
    ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 5, 193690690),
    ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 7, 178633661),
    ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 5, 89941194),
    ("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", 5, 164075551),
    ("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1", 5, 3605103),
];

#[test]
#[allow(dead_code)]
fn perft_test() {
    init();
    for case in PERFT_TEST_CASES.iter() {
        let mut move_count = 0;
        let mut state = parse_fen_string(case.0.to_string()).unwrap();
        match state.turn {
            Color::White => perft::<{Color::White}>(&mut state, case.1, &mut move_count),
            Color::Black => perft::<{Color::Black}>(&mut state, case.1, &mut move_count),
        }
        assert_eq!(move_count, case.2);
    }
}

#[allow(dead_code)]
pub fn perft<const C: Color>(state: &mut State, depth: i64, move_count: &mut i64) {
    assert_eq!(state.hashcode, state.get_hash());
    if depth == 0 {
        *move_count += 1;
    } else {
        state.gen_all_moves::<C, {MoveGenType::All}>();
        for m in state.debug_move_vec() {
            if state.make_move::<C>(m) {
                match C {
                    Color::White => perft::<{Color::Black}>(state, depth-1, move_count),
                    Color::Black => perft::<{Color::White}>(state, depth-1, move_count),
                }
            }
            state.unmake_move::<C>(m);
        }
    }
}