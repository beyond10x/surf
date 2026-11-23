//! A counting global allocator for the test binary of `surf-tls`, so that a test asserts the
//! bytes an operation allocates and holds instead of its time; in the form of
//! `crates/html/tests/common/mod.rs`. The one `unsafe` under `crates/tls`, and test only.
//!
//! The counts are kept for each thread: an operation runs on the thread of its test, so a test
//! reads only its own allocations also when `cargo test` runs tests side by side in one
//! process.

#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::mpsc;
use std::thread;

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

#[allow(unsafe_code)]
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

const MIB: usize = 1 << 20;

/// A test that allocates 1 MiB and holds it reads a peak of at least 1 MiB, and allocated
/// bytes of at least as many.
#[test]
fn a_held_mebibyte_is_counted() {
    let ((), allocated) = allocated_while(|| {
        let ((), peak) = peak_while(|| {
            let block = std::hint::black_box(vec![1u8; MIB]);
            assert_eq!(block.len(), MIB);
        });
        assert!(peak >= MIB, "peak {peak} bytes, at least {MIB}");
    });
    assert!(
        allocated >= MIB,
        "allocated {allocated} bytes, at least {MIB}"
    );
}

/// A test that allocates nothing reads a peak under 64 KiB while another thread of the same
/// test allocates 4 MiB and holds it: the counts are per thread.
#[test]
fn another_threads_allocation_is_not_counted() {
    let (allocated_tx, allocated_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let other = thread::spawn(move || {
        let block = std::hint::black_box(vec![1u8; 4 * MIB]);
        allocated_tx.send(()).expect("tell the test");
        release_rx.recv().expect("wait for the test");
        block.len()
    });
    let ((), peak) = peak_while(|| {
        allocated_rx.recv().expect("the other thread allocated");
    });
    release_tx.send(()).expect("release the other thread");
    assert_eq!(other.join().expect("join"), 4 * MIB);
    assert!(peak < 64 * 1024, "peak {peak} bytes, under 64 KiB");
}
