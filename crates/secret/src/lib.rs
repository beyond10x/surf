//! The one home of the secrets of the agent and the view (decision D25 of
//! `docs/plan/agent-plan-revision-2.md`): the operating system random source, a comparison of
//! byte slices that reads every byte, and the 32-byte token with its form of 64 lower-case
//! hexadecimal digits. Standard library only.
//!
//! What it does not do: it holds no other cryptographic primitive, no hash of a token, and no
//! fallback when the random source cannot be read. It does not promise a measured running time:
//! see [`equal`].

#![forbid(unsafe_code)]

mod compare;

use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub use compare::equal;

/// The device [`fill_random`] reads.
const DEVICE: &str = "/dev/urandom";

/// The lower-case hexadecimal digits, by value.
const HEX: &[u8; 16] = b"0123456789abcdef";

/// Fills all of `buffer` from the operating system's random source, `/dev/urandom`, read through
/// the standard library.
///
/// On `Err`, the error of opening or reading the device, the buffer's contents are unspecified
/// and are not to be used. No value of this crate's own ever takes the place of the device's.
pub fn fill_random(buffer: &mut [u8]) -> io::Result<()> {
    fill_random_from(DEVICE, buffer)
}

/// [`fill_random`] over the file at `path`: opened for this one read, read until `buffer` is
/// full, and closed.
pub fn fill_random_from(path: impl AsRef<Path>, buffer: &mut [u8]) -> io::Result<()> {
    File::open(path)?.read_exact(buffer)
}

/// A 32-byte secret drawn from the operating system's random source.
///
/// Its text form is 64 lower-case hexadecimal digits. Two tokens are compared with
/// [`Token::matches`], which goes through [`equal`]; there is no `PartialEq`. Its `Debug` writes
/// `Token(..)` and no digit, and it has no `Display`.
#[derive(Clone)]
pub struct Token([u8; 32]);

impl Token {
    /// A token of 32 bytes from [`fill_random`], or the error of reading the random source.
    pub fn draw() -> io::Result<Self> {
        let mut bytes = [0u8; 32];
        fill_random(&mut bytes)?;
        Ok(Self(bytes))
    }

    /// The 64 lower-case hexadecimal digits of the token, the high half of each byte first.
    pub fn to_hex(&self) -> String {
        let mut text = String::with_capacity(64);
        for byte in &self.0 {
            text.push(char::from(HEX[usize::from(byte >> 4)]));
            text.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        text
    }

    /// The token whose form is `text`, which is exactly 64 digits of `0-9a-f`; `None` for
    /// anything else, upper-case digits included.
    pub fn from_hex(text: &str) -> Option<Self> {
        let digits = text.as_bytes();
        if digits.len() != 64 {
            return None;
        }
        let mut bytes = [0u8; 32];
        let (pairs, _) = digits.as_chunks::<2>();
        for (byte, [high, low]) in bytes.iter_mut().zip(pairs) {
            *byte = (digit(*high)? << 4) | digit(*low)?;
        }
        Some(Self(bytes))
    }

    /// True when `other` is the same token, compared through [`equal`].
    pub fn matches(&self, other: &Token) -> bool {
        equal(&self.0, &other.0)
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Token(..)")
    }
}

/// The value of one lower-case hexadecimal digit.
fn digit(character: u8) -> Option<u8> {
    match character {
        b'0'..=b'9' => Some(character - b'0'),
        b'a'..=b'f' => Some(character - b'a' + 10),
        _ => None,
    }
}
