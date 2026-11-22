//! Reading the structures of RFC 8446 section 3 from a handshake message the server sent:
//! integers, and vectors with a length prefix of 1, 2 or 3 bytes. Every read stays inside the
//! input and allocates nothing; a length past the end, a vector outside its bounds or bytes
//! left over are [`Malformed`], which the handshake answers with `decode_error` (section 6).

use crate::error::Violation;

/// A message that does not parse: the handshake sends `decode_error` and names the message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Malformed;

impl Malformed {
    /// The violation of `message`.
    pub(crate) fn of(message: &'static str) -> Violation {
        Violation::Malformed { message }
    }
}

/// A reader over the bytes of one structure.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reader<'a> {
    input: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A reader of `input`.
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Reader { input }
    }

    /// Whether every byte is read.
    pub(crate) fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// The next `n` bytes.
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], Malformed> {
        if n > self.input.len() {
            return Err(Malformed);
        }
        let (head, tail) = self.input.split_at(n);
        self.input = tail;
        Ok(head)
    }

    /// One byte.
    pub(crate) fn u8(&mut self) -> Result<u8, Malformed> {
        Ok(self.take(1)?[0])
    }

    /// A `uint16`.
    pub(crate) fn u16(&mut self) -> Result<u16, Malformed> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    /// A `uint24`.
    pub(crate) fn u24(&mut self) -> Result<usize, Malformed> {
        let b = self.take(3)?;
        Ok(usize::from(b[0]) << 16 | usize::from(b[1]) << 8 | usize::from(b[2]))
    }

    /// A `uint32`.
    pub(crate) fn u32(&mut self) -> Result<u32, Malformed> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// A vector of a 1-byte length, of `min..=max` bytes.
    pub(crate) fn vec8(&mut self, min: usize, max: usize) -> Result<&'a [u8], Malformed> {
        let len = usize::from(self.u8()?);
        self.bounded(len, min, max)
    }

    /// A vector of a 2-byte length, of `min..=max` bytes.
    pub(crate) fn vec16(&mut self, min: usize, max: usize) -> Result<&'a [u8], Malformed> {
        let len = usize::from(self.u16()?);
        self.bounded(len, min, max)
    }

    /// A vector of a 3-byte length, of `min..=max` bytes.
    pub(crate) fn vec24(&mut self, min: usize, max: usize) -> Result<&'a [u8], Malformed> {
        let len = self.u24()?;
        self.bounded(len, min, max)
    }

    fn bounded(&mut self, len: usize, min: usize, max: usize) -> Result<&'a [u8], Malformed> {
        if len < min || len > max {
            return Err(Malformed);
        }
        self.take(len)
    }

    /// Succeeds when every byte is read.
    pub(crate) fn finish(self) -> Result<(), Malformed> {
        if self.input.is_empty() {
            Ok(())
        } else {
            Err(Malformed)
        }
    }
}

/// A value longer than the length field it is written with can count: nothing is truncated,
/// the caller refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TooLong;

/// Appends `value` as a vector of a 1-byte length; [`TooLong`] past 255 bytes, `out` unchanged.
pub(crate) fn put_vec8(out: &mut Vec<u8>, value: &[u8]) -> Result<(), TooLong> {
    let len = u8::try_from(value.len()).map_err(|_| TooLong)?;
    out.push(len);
    out.extend_from_slice(value);
    Ok(())
}

/// Appends `value` as a vector of a 2-byte length; [`TooLong`] past 65,535 bytes, `out`
/// unchanged.
pub(crate) fn put_vec16(out: &mut Vec<u8>, value: &[u8]) -> Result<(), TooLong> {
    let len = u16::try_from(value.len()).map_err(|_| TooLong)?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(value);
    Ok(())
}

/// Appends `value` as a vector of a 3-byte length; [`TooLong`] past 2^24 - 1 bytes, `out`
/// unchanged.
pub(crate) fn put_vec24(out: &mut Vec<u8>, value: &[u8]) -> Result<(), TooLong> {
    let len = u32::try_from(value.len()).map_err(|_| TooLong)?;
    if len > 0x00ff_ffff {
        return Err(TooLong);
    }
    out.extend_from_slice(&len.to_be_bytes()[1..]);
    out.extend_from_slice(value);
    Ok(())
}

/// A handshake message of `msg_type` around `body`: the type and the 3-byte length first;
/// [`TooLong`] for a body of 2^24 bytes or more.
pub(crate) fn handshake_message(msg_type: u8, body: &[u8]) -> Result<Vec<u8>, TooLong> {
    let mut out = Vec::with_capacity(4 + body.len());
    out.push(msg_type);
    put_vec24(&mut out, body)?;
    Ok(out)
}
