//! Base64 of RFC 4648 section 4, the standard alphabet, for PEM: line breaks and other white
//! space between the characters are skipped; the padding is checked.

/// Why base64 did not decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Invalid;

fn sextet(c: u8) -> Option<u32> {
    let v = match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    };
    Some(u32::from(v))
}

/// Decodes `text`, skipping CR, LF, space and tab; the count of bytes looked at is added to
/// `work`. The characters, white space left out, are a multiple of four; `=` stands only as
/// the last one or two of them; the bits a padded group leaves over are zero.
pub(crate) fn decode(text: &[u8], work: &mut usize) -> Result<Vec<u8>, Invalid> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut group = 0u32;
    let mut chars = 0; // characters of the group being read, 0 to 3
    let mut pads = 0;
    for &c in text {
        *work += 1;
        if matches!(c, b'\r' | b'\n' | b' ' | b'\t') {
            continue;
        }
        if c == b'=' {
            if chars < 2 || chars + pads >= 4 {
                return Err(Invalid);
            }
            pads += 1;
            continue;
        }
        if pads > 0 {
            return Err(Invalid);
        }
        group = (group << 6) | sextet(c).ok_or(Invalid)?;
        chars += 1;
        if chars == 4 {
            out.extend_from_slice(&[(group >> 16) as u8, (group >> 8) as u8, group as u8]);
            group = 0;
            chars = 0;
        }
    }
    match (chars, pads) {
        (0, 0) => Ok(out),
        // Two characters and two pads: one byte, 4 bits left over.
        (2, 2) if group & 0x0F == 0 => {
            out.push((group >> 4) as u8);
            Ok(out)
        }
        // Three characters and one pad: two bytes, 2 bits left over.
        (3, 1) if group & 0x03 == 0 => {
            out.extend_from_slice(&[(group >> 10) as u8, (group >> 2) as u8]);
            Ok(out)
        }
        _ => Err(Invalid),
    }
}
