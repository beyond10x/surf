//! `escape` over every Unicode scalar value: what it writes is printable ASCII only, so no
//! character of an address can end, split or reorder an output line, and it can be read back.

use surf::escape;

/// Undo `escape`: `\xHH` sequences back to bytes, everything else as itself.
fn unescape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            assert_eq!(bytes.get(i + 1), Some(&b'x'), "{text:?}");
            let hex = std::str::from_utf8(&bytes[i + 2..i + 4]).expect("hex digits");
            assert_eq!(hex, hex.to_ascii_lowercase(), "{text:?}");
            out.push(u8::from_str_radix(hex, 16).expect("hex"));
            i += 4;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).expect("UTF-8")
}

#[test]
fn every_character_is_written_as_printable_ascii_and_reads_back() {
    for code in 0..=0x10_ffffu32 {
        let Some(character) = char::from_u32(code) else {
            continue;
        };
        let text = character.to_string();
        let escaped = escape(&text);
        assert!(
            escaped.bytes().all(|b| (0x20..=0x7e).contains(&b)),
            "U+{code:04X} is written as {escaped:?}"
        );
        assert_eq!(unescape(&escaped), text, "U+{code:04X}");
        let printable = (0x20..=0x7e).contains(&code) && character != '\\';
        assert_eq!(
            escaped == text,
            printable,
            "U+{code:04X} is written as {escaped:?}"
        );
    }
}

/// The characters that motivated the rule, by name: line and paragraph separators, the
/// bidirectional marks, embeddings, overrides and isolates, the byte order mark and a tag.
#[test]
fn separators_and_format_characters_are_escaped() {
    for (character, expected) in [
        ('\u{2028}', "\\xe2\\x80\\xa8"),
        ('\u{2029}', "\\xe2\\x80\\xa9"),
        ('\u{200e}', "\\xe2\\x80\\x8e"),
        ('\u{200f}', "\\xe2\\x80\\x8f"),
        ('\u{061c}', "\\xd8\\x9c"),
        ('\u{202a}', "\\xe2\\x80\\xaa"),
        ('\u{202e}', "\\xe2\\x80\\xae"),
        ('\u{2066}', "\\xe2\\x81\\xa6"),
        ('\u{2069}', "\\xe2\\x81\\xa9"),
        ('\u{feff}', "\\xef\\xbb\\xbf"),
        ('\u{e0041}', "\\xf3\\xa0\\x81\\x81"),
    ] {
        assert_eq!(escape(&format!("a{character}b")), format!("a{expected}b"));
    }
}
