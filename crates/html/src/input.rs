//! Preprocessing the input stream (13.2.3.5). Written by `story:html-tokenizer-markup`.
//!
//! The input stream is a sequence of code points, surrogates included (a surrogate is a parse
//! error, not an impossible value), so it is held as `u32` and not as `char`. Newlines are
//! normalized before tokenization: every U+000D CR followed by U+000A LF, and every U+000D CR on
//! its own, becomes one U+000A LF ("normalize newlines", Infra).
//!
//! The parse errors this section defines, `surrogate-in-input-stream`,
//! `noncharacter-in-input-stream` and `control-character-in-input-stream`, are reported by the
//! tokenizer ([`parse_error`]) when the code point becomes the next input character: when it is
//! consumed, or before a state that looks ahead without consuming (the markup declaration open
//! state) decides. A code point further ahead is not reported by being compared. The standard
//! does not order these errors among the tokenizer's; the tokenizer data does:
//! `html5lib-tests/tokenizer/test3.test`, `<!\u000B`, gives `control-character-in-input-stream`
//! before `incorrectly-opened-comment`, and `test4.test`, `<!doc\u0080`, gives it after.

/// The input stream after preprocessing, with the position of the next input character.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputStream {
    code_points: Vec<u32>,
    position: usize,
}

impl InputStream {
    /// The input stream of `input`, newlines normalized, positioned on its first code point.
    pub fn new(input: &[u32]) -> InputStream {
        let mut code_points = Vec::with_capacity(input.len());
        let mut after_cr = false;
        for &c in input {
            match c {
                0x0D => {
                    code_points.push(0x0A);
                    after_cr = true;
                }
                0x0A if after_cr => after_cr = false,
                _ => {
                    code_points.push(c);
                    after_cr = false;
                }
            }
        }
        InputStream {
            code_points,
            position: 0,
        }
    }

    /// Consumes the next input character; `None` is the "EOF" character, the end of the stream.
    pub fn consume(&mut self) -> Option<u32> {
        let c = self.code_points.get(self.position).copied();
        if c.is_some() {
            self.position += 1;
        }
        c
    }

    /// The number of code points consumed so far.
    pub fn position(&self) -> usize {
        self.position
    }

    /// The code point at `index` from the start of the stream, if the stream has one there.
    pub fn get(&self, index: usize) -> Option<u32> {
        self.code_points.get(index).copied()
    }

    /// Whether the next input characters are `word`, which is ASCII, exactly or as an ASCII
    /// case-insensitive match. Consumes nothing.
    pub fn next_characters_are(&self, word: &str, case_insensitive: bool) -> bool {
        word.bytes().enumerate().all(|(i, w)| {
            self.get(self.position + i).is_some_and(|c| {
                u8::try_from(c).is_ok_and(|c| {
                    if case_insensitive {
                        c.eq_ignore_ascii_case(&w)
                    } else {
                        c == w
                    }
                })
            })
        })
    }
}

/// Whether `c` is a surrogate: U+D800 to U+DFFF (Infra).
pub fn is_surrogate(c: u32) -> bool {
    (0xD800..=0xDFFF).contains(&c)
}

/// Whether `c` is a noncharacter: U+FDD0 to U+FDEF, or the last two code points of a plane,
/// U+FFFE and U+FFFF up to U+10FFFE and U+10FFFF (Infra).
pub fn is_noncharacter(c: u32) -> bool {
    (0xFDD0..=0xFDEF).contains(&c) || (c <= 0x10FFFF && c & 0xFFFE == 0xFFFE)
}

/// Whether `c` is a control: a C0 control (U+0000 to U+001F) or U+007F to U+009F (Infra).
pub fn is_control(c: u32) -> bool {
    c <= 0x1F || (0x7F..=0x9F).contains(&c)
}

/// The parse error 13.2.3.5 defines for an occurrence of `c` in the input stream, if any:
/// `surrogate-in-input-stream` for a surrogate, `noncharacter-in-input-stream` for a
/// noncharacter, `control-character-in-input-stream` for a control other than ASCII whitespace
/// (tab, LF, FF, CR, space) and U+0000 NULL.
pub fn parse_error(c: u32) -> Option<&'static str> {
    if is_surrogate(c) {
        Some("surrogate-in-input-stream")
    } else if is_noncharacter(c) {
        Some("noncharacter-in-input-stream")
    } else if is_control(c) && !matches!(c, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20) {
        Some("control-character-in-input-stream")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(s: &str) -> Vec<u32> {
        let mut input = InputStream::new(&s.chars().map(u32::from).collect::<Vec<_>>());
        std::iter::from_fn(|| input.consume()).collect()
    }

    #[test]
    fn newlines_are_normalized() {
        assert_eq!(stream("a\r\nb\rc\n\r\r\nd"), stream("a\nb\nc\n\n\nd"));
        assert_eq!(stream("\r"), vec![0x0A]);
        assert_eq!(stream("\r\n\n"), vec![0x0A, 0x0A]);
    }

    #[test]
    fn parse_errors_name_the_code_point_classes() {
        assert_eq!(parse_error(0xD800), Some("surrogate-in-input-stream"));
        assert_eq!(parse_error(0xDFFF), Some("surrogate-in-input-stream"));
        assert_eq!(parse_error(0xFDD0), Some("noncharacter-in-input-stream"));
        assert_eq!(parse_error(0x10FFFF), Some("noncharacter-in-input-stream"));
        assert_eq!(parse_error(0x1FFFE), Some("noncharacter-in-input-stream"));
        assert_eq!(parse_error(0x01), Some("control-character-in-input-stream"));
        assert_eq!(parse_error(0x0B), Some("control-character-in-input-stream"));
        assert_eq!(parse_error(0x7F), Some("control-character-in-input-stream"));
        assert_eq!(parse_error(0x9F), Some("control-character-in-input-stream"));
        for c in [
            0x00,
            0x09,
            0x0A,
            0x0C,
            0x0D,
            0x20,
            0x41,
            0xA0,
            0xFFFD,
            0xFDEF + 1,
        ] {
            assert_eq!(parse_error(c), None, "U+{c:04X}");
        }
    }

    #[test]
    fn lookahead_does_not_consume() {
        let mut input = InputStream::new(&"DocType x".chars().map(u32::from).collect::<Vec<_>>());
        assert!(input.next_characters_are("doctype", true));
        assert!(!input.next_characters_are("DOCTYPE", false));
        assert!(input.next_characters_are("DocType", false));
        assert!(!input.next_characters_are("DocType x and more", true));
        assert_eq!(input.consume(), Some(u32::from('D')));
        assert_eq!(input.position(), 1);
    }
}
