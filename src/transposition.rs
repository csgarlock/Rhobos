use std::{alloc::{alloc_zeroed, dealloc, Layout}, mem::transmute, ptr::null_mut};

use crate::{evaluation::{Evaluation, CENTI_PAWN}, hash::setup_hashes, r#move::Move, search::Depth, state::State};

const TABLE_ENTRY_SIZE: usize = size_of::<TTableEntry>();
const TABLE_ENTRY_ALIGN: usize = 64;
const MEGABYTE_TO_BYTE: usize = 1024 * 1024;

const BIT_MASK_2:  u16 = 0x3;
const BIT_MASK_14: u16 = 0x3FFF;

static mut TRANSPOSITION_TABLE: TranspositionTable = TranspositionTable {
    data_pointer: null_mut(),
    entries: 0,
    mod_and_mask: 0,
    use_and: false,
    layout: Layout::new::<TTableEntry>(),
};

type TTEval = i16;
type PackedDepthAndNode = u16;

#[derive(Clone, Copy)]
pub struct TTableData {
    pub eval: TTEval,
    pub best_move: Move,
    pub ply: u16,
    pub packed_depth_and_node: PackedDepthAndNode,
} 

pub struct TTableEntry {
    hash: u64,
    data: TTableData,
}

pub struct TranspositionTable {
    data_pointer: *mut TTableEntry,
    entries: u64,
    mod_and_mask: u64,
    use_and: bool,
    layout: Layout,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    PVNode,
    CutNode,
    AllNode,
    TerminalNode,
}

pub unsafe fn ttable_init(size_in_mb: usize) {
    setup_hashes();
    let layout = Layout::from_size_align(size_in_mb * MEGABYTE_TO_BYTE, TABLE_ENTRY_ALIGN).unwrap();
    unsafe { TRANSPOSITION_TABLE.data_pointer = alloc_zeroed(layout) as *mut TTableEntry }
    unsafe { TRANSPOSITION_TABLE.layout = layout };
    let entries = ((size_in_mb * MEGABYTE_TO_BYTE) / TABLE_ENTRY_SIZE) as u64;
    unsafe { TRANSPOSITION_TABLE.entries = entries };
    // is power of two
    if entries.count_ones() == 1 {
        unsafe { TRANSPOSITION_TABLE.use_and = true }
        unsafe { TRANSPOSITION_TABLE.mod_and_mask = entries as u64 - 1}
    }
}

pub unsafe fn free_ttable() {
    unsafe { dealloc(TRANSPOSITION_TABLE.data_pointer as *mut u8, TRANSPOSITION_TABLE.layout); }
    unsafe { TRANSPOSITION_TABLE.data_pointer = null_mut(); }
}

#[inline(always)]
pub fn tt_index(hash: u64) -> usize {
    unsafe {
        (if TRANSPOSITION_TABLE.use_and {
            hash & TRANSPOSITION_TABLE.mod_and_mask
        } else {
            hash % TRANSPOSITION_TABLE.entries
        }) as usize
    }   
}

#[inline(always)]
pub fn add_tt_state(state: &State, eval: Evaluation, best_move: Move, depth: Depth, node_type: NodeType) {
    let hash = state.hashcode;
    let index = tt_index(hash);
    debug_assert!(index < unsafe { TRANSPOSITION_TABLE.entries as usize });
    unsafe { *TRANSPOSITION_TABLE.data_pointer.add(index) = TTableEntry {
        hash: hash,
        data: TTableData {
            eval: eval_convert_precision_high_to_low(eval),
            best_move,
            ply: state.ply,
            packed_depth_and_node: ((node_type as u16) << 14) | depth as u16
        }
    }};
}

#[inline(always)]
pub fn search_tt_state(state: &State) -> Option<TTableData> {
    let hash = state.hashcode;
    let index = tt_index(hash);
    debug_assert!(index < unsafe { TRANSPOSITION_TABLE.entries as usize });
    unsafe {
        if (*TRANSPOSITION_TABLE.data_pointer.add(index)).hash == hash {
            Some((*TRANSPOSITION_TABLE.data_pointer.add(index)).data)
        } else {
            None
        }
    }
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn prefetch_tt_address(state: &State) {
    let hash = state.hashcode;
    let index = tt_index(hash);
    debug_assert!(index < unsafe { TRANSPOSITION_TABLE.entries as usize });
    unsafe { std::arch::x86_64::_mm_prefetch::<{std::arch::x86_64::_MM_HINT_T0}>(TRANSPOSITION_TABLE.data_pointer.add(index) as *mut i8) };
}

#[inline(always)]
#[cfg(not(target_arch = "x86_64"))]
pub fn prefetch_tt_address(state: &State) {}

#[inline(always)]
pub const fn parse_packed_depth_and_node(packed_data: PackedDepthAndNode) -> (Depth, NodeType) {
    ((packed_data & BIT_MASK_14) as Depth, unsafe { transmute(((packed_data >> 14) & BIT_MASK_2) as u8) })
}

#[inline(always)]
pub const fn eval_convert_precision_high_to_low(high_eval: Evaluation) -> TTEval {
    (high_eval / CENTI_PAWN) as TTEval
}

#[inline(always)]
pub const fn eval_convert_precision_low_to_high(low_eval: TTEval) -> Evaluation {
    low_eval as Evaluation * CENTI_PAWN
}