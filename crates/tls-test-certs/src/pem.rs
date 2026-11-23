//! PEM of certificates (RFC 7468): base64 of 64 characters a line between the `CERTIFICATE`
//! lines, written and read back.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
const END: &str = "-----END CERTIFICATE-----";

/// Base64 of `bytes` (RFC 4648 section 4), with padding.
pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - 6 * i)) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The bytes of base64 `text` (no whitespace), or why not.
pub fn unbase64(text: &str) -> Result<Vec<u8>, String> {
    let text = text.as_bytes();
    if !text.len().is_multiple_of(4) {
        return Err(format!(
            "{} characters of base64, not a multiple of 4",
            text.len()
        ));
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    for (i, quad) in text.chunks(4).enumerate() {
        let last = i + 1 == text.len() / 4;
        let pad = quad.iter().rev().take_while(|c| **c == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return Err("misplaced padding".into());
        }
        let mut n = 0u32;
        for c in &quad[..4 - pad] {
            let v = ALPHABET
                .iter()
                .position(|a| a == c)
                .ok_or_else(|| format!("{:?} is no base64", *c as char))?;
            n = n << 6 | v as u32;
        }
        n <<= 6 * pad as u32;
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..3 - pad]);
    }
    Ok(out)
}

/// One certificate as PEM, ending with a newline.
pub fn certificate(der: &[u8]) -> String {
    let text = base64(der);
    let mut out = String::new();
    out.push_str(BEGIN);
    out.push('\n');
    for line in text.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(line).expect("base64 is ASCII"));
        out.push('\n');
    }
    out.push_str(END);
    out.push('\n');
    out
}

/// The certificates of a PEM text, DER, in order. Any other block, or text outside a block
/// that is not blank, is refused.
pub fn decode_certificates(text: &str) -> Result<Vec<Vec<u8>>, String> {
    let mut out = Vec::new();
    let mut body: Option<String> = None;
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        match (&mut body, line) {
            (None, BEGIN) => body = Some(String::new()),
            (None, l) if l.trim().is_empty() => {}
            (None, l) => return Err(format!("line {n}: {l:?} outside a CERTIFICATE block")),
            (Some(b), END) => {
                out.push(unbase64(b).map_err(|e| format!("line {n}: {e}"))?);
                body = None;
            }
            (Some(b), l) => b.push_str(l.trim()),
        }
    }
    if body.is_some() {
        return Err("a block without its END line".into());
    }
    Ok(out)
}
