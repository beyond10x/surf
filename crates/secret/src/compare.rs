//! The comparison of two byte slices that reads every byte.
//!
//! `tests/secret.rs` compiles this file into its own crate as well, where `cfg(test)` holds, to
//! read the count of bytes read that is compiled in tests only.

use std::hint::black_box;

/// True when `a` and `b` hold the same bytes.
///
/// Slices of different lengths are unequal, and that answer comes at once: the length is not
/// treated as secret. For two slices of one length the function reads every byte of both,
/// whatever they hold and wherever they first differ: it ORs together the XOR of each pair and
/// decides once, after the last pair. Nothing in the loop branches on the bytes.
///
/// What this promises is the shape of the source: no early return, no branch on a byte. It does
/// not promise a running time measured on a given machine. An optimizer is free to rewrite the
/// loop, and `std::hint::black_box`, which hides the running result from it after each pair, is
/// documented by the standard library as a best effort only. The claim that is checked is the
/// story's cases: compiled in tests, a count of the bytes read shows 32 of 32 read for slices that
/// differ in their first byte and for slices that differ in their last.
pub fn equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut difference = 0u8;
    for (x, y) in a.iter().zip(b) {
        difference = black_box(difference | xor(*x, *y));
    }
    difference == 0
}

/// The XOR of one pair. In tests it also counts the byte position read.
#[inline(always)]
fn xor(x: u8, y: u8) -> u8 {
    #[cfg(test)]
    BYTES_READ.with(|count| count.set(count.get() + 1));
    x ^ y
}

#[cfg(test)]
thread_local! {
    /// Byte positions `equal` has read on this thread.
    static BYTES_READ: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The number of byte positions `equal` has read on this thread since the last call, which sets
/// it back to zero. One position is one byte of each slice. Compiled in tests only; the library's
/// own test build does not call it, the cases in `tests/` do.
#[cfg(test)]
#[allow(dead_code)]
pub fn take_bytes_read() -> usize {
    BYTES_READ.with(|count| count.replace(0))
}
