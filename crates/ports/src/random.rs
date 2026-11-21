//! The random source identities are drawn from, and the version 4 UUID built from it.

use std::fs::File;
use std::io::Read;

use surf_types::behaviour::unmet_context;
use surf_types::obligation::UnmetObligation;
use surf_types::primitives::Uuid;

/// Where the random bits of an assigned identity come from.
///
/// A ports type is built with one and reads 16 bytes from it per identity. A source that cannot
/// fill the buffer says so; the ports type then answers the identity request with an unmet
/// obligation and never substitutes a value of its own.
pub trait RandomSource {
    /// Fills all of `buffer` with random bytes, or returns why it could not.
    ///
    /// On `Err` the buffer's contents are unspecified and are not used.
    fn fill(&mut self, buffer: &mut [u8]) -> std::io::Result<()>;
}

/// A borrowed source, so a caller can keep the one it hands a ports type.
impl<R: RandomSource + ?Sized> RandomSource for &mut R {
    fn fill(&mut self, buffer: &mut [u8]) -> std::io::Result<()> {
        (**self).fill(buffer)
    }
}

/// The operating system's random source, read through the standard library from `/dev/urandom`.
///
/// The file is opened on the first read and kept open. Where it cannot be opened or read (no
/// such device, as on a system without one), every read fails and no identity is assigned.
#[derive(Debug, Default)]
pub struct OsRandom {
    file: Option<File>,
}

/// The device [`OsRandom`] reads.
const DEVICE: &str = "/dev/urandom";

/// The lower-case hexadecimal digits, by value.
const HEX: &[u8; 16] = b"0123456789abcdef";

impl OsRandom {
    /// A source that has not opened the device yet.
    pub fn new() -> Self {
        Self::default()
    }
}

impl RandomSource for OsRandom {
    fn fill(&mut self, buffer: &mut [u8]) -> std::io::Result<()> {
        let file = match self.file.take() {
            Some(file) => file,
            None => File::open(DEVICE)?,
        };
        self.file.insert(file).read_exact(buffer)
    }
}

/// A version 4 UUID from `source`, or the unmet obligation naming `kind`, the identity asked for.
///
/// 16 bytes from the source, of which 122 bits are kept: the version nibble is set to `4` and the
/// two variant bits to `10`. The text is the canonical lower-case 8-4-4-4-12 form. The source's
/// error is not carried: `UnmetObligation` holds static text only.
pub(crate) fn assign<R: RandomSource>(
    source: &mut R,
    kind: &'static str,
) -> Result<Uuid, UnmetObligation> {
    let mut bytes = [0u8; 16];
    source.fill(&mut bytes).map_err(|_| unmet_context(kind))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let mut text = String::with_capacity(36);
    for (at, byte) in bytes.iter().enumerate() {
        if matches!(at, 4 | 6 | 8 | 10) {
            text.push('-');
        }
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(Uuid(text))
}
