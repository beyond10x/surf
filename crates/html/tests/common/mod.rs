//! What the limit tests share (`tests/limits.rs`, `tests/adversary_limits.rs`): a counting
//! global allocator, so that a test asserts the bytes a parse allocates and holds instead of its
//! time, and the bound those tests hold a parse's allocations to.
//!
//! The counts are kept for each thread: a parse runs on the thread of its test, so a test reads
//! only its own allocations also when `cargo test` runs tests side by side in one process.

#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use surf_html::Tree;
use surf_html::limits::NODE_BYTES;

thread_local! {
    /// The bytes allocated and not yet freed by this thread.
    static LIVE: Cell<isize> = const { Cell::new(0) };
    /// The most of [`LIVE`] since the last [`peak_while`] began.
    static PEAK: Cell<isize> = const { Cell::new(0) };
    /// Every byte this thread allocated, and every byte a reallocation grew by, summed.
    static ALLOCATED: Cell<usize> = const { Cell::new(0) };
}

fn grew(n: usize) {
    let _ = LIVE.try_with(|live| {
        let now = live.get() + n as isize;
        live.set(now);
        let _ = PEAK.try_with(|peak| peak.set(peak.get().max(now)));
    });
    let _ = ALLOCATED.try_with(|allocated| allocated.set(allocated.get().saturating_add(n)));
}

fn shrank(n: usize) {
    let _ = LIVE.try_with(|live| live.set(live.get() - n as isize));
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            grew(layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            grew(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        shrank(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            if new_size >= layout.size() {
                grew(new_size - layout.size());
            } else {
                shrank(layout.size() - new_size);
            }
        }
        p
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The bytes this thread holds now.
pub fn live() -> isize {
    LIVE.with(Cell::get)
}

/// The most bytes this thread held while `f` ran, above those it held before.
pub fn peak_while<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = live();
    PEAK.with(|peak| peak.set(before));
    let out = f();
    (out, (PEAK.with(Cell::get) - before).max(0) as usize)
}

/// The bytes this thread allocated while `f` ran: every allocation and every growth of one,
/// whether freed again or not.
pub fn allocated_while<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATED.with(Cell::get);
    let out = f();
    (out, ALLOCATED.with(Cell::get) - before)
}

/// The most bytes these tests let a parse of `input_bytes` bytes allocate, summed over its run
/// ([`allocated_while`]), given the tree it left: 64 for each input byte, 4 for each stored
/// byte, 4 × [`NODE_BYTES`] for each node, and 1 MiB. The bound is these tests' own, not a
/// promise of the crate. What a parse allocates: the input as code points, twice (8 bytes a
/// character); each token's strings and lists, and a tag's set of attribute names (about 250
/// bytes for an attribute of 6 input bytes); and the tree, whose heap the stored-byte limit
/// bounds by 2 × its stored bytes and 3 × `NODE_BYTES` for each node, here doubled for the
/// lists that grew and were freed again. It catches the copy of the kind the step limit cannot
/// see: work that a token repeats without a step or a stored byte, such as the start tag token
/// of a formatting element copied for each of its end tags (about 61 KB for 4 input bytes with
/// 1,024 attributes).
pub fn allocation_bound(input_bytes: usize, tree: &Tree) -> usize {
    64 * input_bytes + 4 * tree.stored_bytes() + 4 * NODE_BYTES * tree.node_count() + (1 << 20)
}
