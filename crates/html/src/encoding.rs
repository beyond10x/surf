//! The input byte stream (13.2.3): the Encoding Standard's labels and the decoders, BOM
//! sniffing and the prescan. Written by `story:html-input-stream`.

/// What [`decode`] returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    /// The decoded text.
    pub text: String,
    /// The name, as the Encoding Standard writes it, of the encoding used; `None` when none was
    /// chosen.
    pub encoding: Option<&'static str>,
}

/// Runs the encoding sniffing algorithm with no transport-layer encoding over `bytes` and
/// decodes them with the encoding it chose.
///
/// Not written yet: returns no text and no encoding.
pub fn decode(bytes: &[u8]) -> Decoded {
    let _ = bytes;
    Decoded {
        text: String::new(),
        encoding: None,
    }
}
