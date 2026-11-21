//! The input byte stream (13.2.3): the decoders' error handling, the windows-1252 index, the
//! label table, BOM sniffing, the prescan and the confidence.
//!
//! The literal values come from the Encoding Standard at snapshot
//! `7d2e3e08e11f57ba1b080390b490d53648c6f69c`: the code points of
//! `windows_1252_maps_0x80_to_0x9f_as_the_standard_table` from its `index-windows-1252.txt`
//! (pointers 0 to 31), the labels of
//! `every_one_of_the_228_labels_names_one_of_the_40_encodings` from its `encodings.json`.

use surf_html::encoding::{self, Confidence, ENCODINGS};

fn decode_as(bytes: &[u8], name: &str) -> String {
    encoding::decode_with(bytes, name)
        .unwrap_or_else(|| panic!("{name} is the name of an encoding"))
        .text
}

#[test]
fn utf8_truncated_sequence_is_replaced() {
    // A three-byte sequence cut after two bytes, at the end and before an ASCII byte.
    assert_eq!(decode_as(b"a\xE2\x82", "UTF-8"), "a\u{FFFD}");
    assert_eq!(decode_as(b"\xE2\x82a", "UTF-8"), "\u{FFFD}a");
    // A four-byte sequence cut after three bytes, and a lead byte alone.
    assert_eq!(decode_as(b"\xF0\x9F\x98", "UTF-8"), "\u{FFFD}");
    assert_eq!(decode_as(b"\xC3", "UTF-8"), "\u{FFFD}");
    // A continuation byte with no lead.
    assert_eq!(decode_as(b"\x80a", "UTF-8"), "\u{FFFD}a");
    // Complete sequences decode.
    assert_eq!(
        decode_as(b"\xC3\xA9\xE2\x82\xAC\xF0\x9F\x98\x80", "UTF-8"),
        "é€😀"
    );
}

#[test]
fn utf8_overlong_form_is_replaced() {
    // `/` (U+002F) in two and in three bytes: 0xC0 is no lead byte; after 0xE0 the lower
    // boundary is 0xA0, so 0x80 is an error and is read again.
    assert_eq!(decode_as(b"\xC0\xAF", "UTF-8"), "\u{FFFD}\u{FFFD}");
    assert_eq!(
        decode_as(b"\xE0\x80\xAF", "UTF-8"),
        "\u{FFFD}\u{FFFD}\u{FFFD}"
    );
    // U+FFFF in four bytes: after 0xF0 the lower boundary is 0x90.
    assert_eq!(
        decode_as(b"\xF0\x8F\xBF\xBF", "UTF-8"),
        "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}"
    );
    // Above U+10FFFF: after 0xF4 the upper boundary is 0x8F.
    assert_eq!(
        decode_as(b"\xF4\x90\x80\x80", "UTF-8"),
        "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}"
    );
}

#[test]
fn utf8_encoded_surrogate_is_replaced() {
    // U+D800 and U+DFFF in three bytes: after 0xED the upper boundary is 0x9F.
    assert_eq!(
        decode_as(b"\xED\xA0\x80", "UTF-8"),
        "\u{FFFD}\u{FFFD}\u{FFFD}"
    );
    assert_eq!(
        decode_as(b"\xED\xBF\xBFa", "UTF-8"),
        "\u{FFFD}\u{FFFD}\u{FFFD}a"
    );
    // U+D7FF, the last code point before the surrogates, decodes.
    assert_eq!(decode_as(b"\xED\x9F\xBF", "UTF-8"), "\u{D7FF}");
}

#[test]
fn utf16le_unpaired_surrogate_is_replaced() {
    // A lead surrogate before `a`: the code unit after it is read again.
    assert_eq!(
        decode_as(&[0x00, 0xD8, 0x61, 0x00], "UTF-16LE"),
        "\u{FFFD}a"
    );
    // A lead surrogate before another lead surrogate and a trail surrogate.
    assert_eq!(
        decode_as(&[0x00, 0xD8, 0x3D, 0xD8, 0x00, 0xDE], "UTF-16LE"),
        "\u{FFFD}😀"
    );
    // A trail surrogate alone, and a lead surrogate at the end.
    assert_eq!(
        decode_as(&[0x00, 0xDC, 0x61, 0x00], "UTF-16LE"),
        "\u{FFFD}a"
    );
    assert_eq!(
        decode_as(&[0x61, 0x00, 0x00, 0xD8], "UTF-16LE"),
        "a\u{FFFD}"
    );
    // A lead surrogate and one byte more at the end: one error.
    assert_eq!(decode_as(&[0x00, 0xD8, 0x61], "UTF-16LE"), "\u{FFFD}");
    // An odd byte at the end.
    assert_eq!(decode_as(&[0x61, 0x00, 0x62], "UTF-16LE"), "a\u{FFFD}");
    // A pair decodes.
    assert_eq!(decode_as(&[0x3D, 0xD8, 0x00, 0xDE], "UTF-16LE"), "😀");
}

#[test]
fn utf16be_unpaired_surrogate_is_replaced() {
    assert_eq!(
        decode_as(&[0xD8, 0x00, 0x00, 0x61], "UTF-16BE"),
        "\u{FFFD}a"
    );
    assert_eq!(
        decode_as(&[0xD8, 0x00, 0xD8, 0x3D, 0xDE, 0x00], "UTF-16BE"),
        "\u{FFFD}😀"
    );
    assert_eq!(
        decode_as(&[0xDC, 0x00, 0x00, 0x61], "UTF-16BE"),
        "\u{FFFD}a"
    );
    assert_eq!(
        decode_as(&[0x00, 0x61, 0xD8, 0x00], "UTF-16BE"),
        "a\u{FFFD}"
    );
    assert_eq!(decode_as(&[0xD8, 0x00, 0x00], "UTF-16BE"), "\u{FFFD}");
    assert_eq!(decode_as(&[0x00, 0x61, 0x00], "UTF-16BE"), "a\u{FFFD}");
    assert_eq!(decode_as(&[0xD8, 0x3D, 0xDE, 0x00], "UTF-16BE"), "😀");
}

#[test]
fn windows_1252_maps_0x80_to_0x9f_as_the_standard_table() {
    // Byte, code point: pointer = byte - 0x80 in index-windows-1252.txt at snapshot
    // 7d2e3e08e11f57ba1b080390b490d53648c6f69c.
    let table: [(u8, u32); 32] = [
        (0x80, 0x20AC),
        (0x81, 0x0081),
        (0x82, 0x201A),
        (0x83, 0x0192),
        (0x84, 0x201E),
        (0x85, 0x2026),
        (0x86, 0x2020),
        (0x87, 0x2021),
        (0x88, 0x02C6),
        (0x89, 0x2030),
        (0x8A, 0x0160),
        (0x8B, 0x2039),
        (0x8C, 0x0152),
        (0x8D, 0x008D),
        (0x8E, 0x017D),
        (0x8F, 0x008F),
        (0x90, 0x0090),
        (0x91, 0x2018),
        (0x92, 0x2019),
        (0x93, 0x201C),
        (0x94, 0x201D),
        (0x95, 0x2022),
        (0x96, 0x2013),
        (0x97, 0x2014),
        (0x98, 0x02DC),
        (0x99, 0x2122),
        (0x9A, 0x0161),
        (0x9B, 0x203A),
        (0x9C, 0x0153),
        (0x9D, 0x009D),
        (0x9E, 0x017E),
        (0x9F, 0x0178),
    ];
    for (byte, code_point) in table {
        let text = decode_as(&[byte], "windows-1252");
        let got: Vec<u32> = text.chars().map(u32::from).collect();
        assert_eq!(got, vec![code_point], "byte 0x{byte:02X}");
    }
    // All 32 together, in one stream.
    let bytes: Vec<u8> = table.iter().map(|(b, _)| *b).collect();
    let got: Vec<u32> = decode_as(&bytes, "windows-1252")
        .chars()
        .map(u32::from)
        .collect();
    let want: Vec<u32> = table.iter().map(|(_, c)| *c).collect();
    assert_eq!(got, want);
}

#[test]
fn replacement_decodes_to_one_u_fffd() {
    assert_eq!(decode_as(b"<p>abc\xFF\x00def", "replacement"), "\u{FFFD}");
    assert_eq!(decode_as(b"a", "replacement"), "\u{FFFD}");
    assert_eq!(decode_as(b"", "replacement"), "");
    // A meta that names a label of replacement chooses it.
    let decoded = encoding::decode(b"<meta charset=\"iso-2022-kr\"><p>x");
    assert_eq!(decoded.encoding, Some("replacement"));
    assert_eq!(decoded.decoded_with, "replacement");
    assert_eq!(decoded.text, "\u{FFFD}");
}

#[test]
fn bom_wins_over_meta() {
    let decoded = encoding::decode(b"\xEF\xBB\xBF<meta charset=\"windows-1252\">\xC3\xA9");
    assert_eq!(decoded.encoding, Some("UTF-8"));
    assert_eq!(decoded.decoded_with, "UTF-8");
    assert_eq!(decoded.text, "<meta charset=\"windows-1252\">é");

    let mut le = vec![0xFF, 0xFE];
    for b in "<meta charset=\"utf-8\">é".encode_utf16() {
        le.extend_from_slice(&b.to_le_bytes());
    }
    let decoded = encoding::decode(&le);
    assert_eq!(decoded.encoding, Some("UTF-16LE"));
    assert_eq!(decoded.text, "<meta charset=\"utf-8\">é");

    let mut be = vec![0xFE, 0xFF];
    for b in "<meta charset=\"utf-8\">é".encode_utf16() {
        be.extend_from_slice(&b.to_be_bytes());
    }
    let decoded = encoding::decode(&be);
    assert_eq!(decoded.encoding, Some("UTF-16BE"));
    assert_eq!(decoded.text, "<meta charset=\"utf-8\">é");
}

#[test]
fn unsupported_encoding_is_decoded_as_windows_1252_and_named() {
    // 0xA9 is U+0160 in ISO-8859-2 and U+00A9 in windows-1252.
    let decoded = encoding::decode(b"<meta charset=\"latin2\">\xA9");
    assert_eq!(decoded.encoding, Some("ISO-8859-2"));
    assert_eq!(decoded.decoded_with, "windows-1252");
    assert_eq!(decoded.text, "<meta charset=\"latin2\">\u{A9}");

    let decoded = encoding::decode(
        b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=euc-jp\">\x80",
    );
    assert_eq!(decoded.encoding, Some("EUC-JP"));
    assert_eq!(decoded.decoded_with, "windows-1252");
    assert!(decoded.text.ends_with('\u{20AC}'));

    let decoded = encoding::decode_with(b"\xA9", "ISO-8859-2").expect("an encoding's name");
    assert_eq!(decoded.encoding, Some("ISO-8859-2"));
    assert_eq!(decoded.decoded_with, "windows-1252");
    assert_eq!(decoded.text, "\u{A9}");
}

#[test]
fn prescan_reads_the_encoding_of_an_xml_declaration() {
    let decoded = encoding::decode(b"<?xml version=\"1.0\" encoding=\"utf-8\"?><p>\xC3\xA9");
    assert_eq!(decoded.encoding, Some("UTF-8"));
    assert_eq!(decoded.confidence, Confidence::Tentative);
    assert_eq!(
        decoded.text,
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><p>é"
    );

    // Spaces and controls around `=`, single quotes, a label in upper case.
    let decoded = encoding::decode(b"<?xml version='1.0' encoding \t= 'ISO-8859-2'?>");
    assert_eq!(decoded.encoding, Some("ISO-8859-2"));

    // UTF-16BE/LE named by the declaration becomes UTF-8.
    let decoded = encoding::decode(b"<?xml version=\"1.0\" encoding=\"utf-16\"?>");
    assert_eq!(decoded.encoding, Some("UTF-8"));

    // A space inside the value, no `<?xml` at the start, and an `encoding` after the `>` are
    // failures: the default.
    for bytes in [
        &b"<?xml version=\"1.0\" encoding=\"utf -8\"?>"[..],
        &b" <?xml version=\"1.0\" encoding=\"utf-8\"?>"[..],
        &b"<?xml version=\"1.0\"?> encoding=\"utf-8\""[..],
    ] {
        let decoded = encoding::decode(bytes);
        assert_eq!(decoded.encoding, Some("windows-1252"), "{bytes:?}");
    }
}

#[test]
fn every_one_of_the_228_labels_names_one_of_the_40_encodings() {
    assert_eq!(ENCODINGS.len(), 40);
    let mut names: Vec<&str> = ENCODINGS.iter().map(|(name, _)| *name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 40, "the names are distinct");

    let mut labels: Vec<&str> = Vec::new();
    for (name, own) in ENCODINGS {
        for label in *own {
            assert_eq!(encoding::get_an_encoding(label), Some(*name), "{label}");
            labels.push(label);
        }
    }
    assert_eq!(labels.len(), 228);
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), 228, "the labels are distinct");

    // One label each, picked from encodings.json at snapshot
    // 7d2e3e08e11f57ba1b080390b490d53648c6f69c, with the name it maps to.
    assert_eq!(encoding::get_an_encoding("latin2"), Some("ISO-8859-2"));
    assert_eq!(encoding::get_an_encoding("us-ascii"), Some("windows-1252"));
    assert_eq!(encoding::get_an_encoding("unicodefeff"), Some("UTF-16LE"));
    assert_eq!(
        encoding::get_an_encoding("iso-2022-kr"),
        Some("replacement")
    );
    assert_eq!(
        encoding::get_an_encoding("cseucpkdfmtjapanese"),
        Some("EUC-JP")
    );

    // Get an encoding: ASCII whitespace stripped, ASCII case-insensitive, nothing else.
    assert_eq!(
        encoding::get_an_encoding(" \tLATIN2\n\x0C\r"),
        Some("ISO-8859-2")
    );
    assert_eq!(encoding::get_an_encoding("latin 2"), None);
    assert_eq!(encoding::get_an_encoding("\u{A0}latin2"), None);
    assert_eq!(encoding::get_an_encoding(""), None);
}

#[test]
fn a_bom_is_certain_and_a_meta_or_the_default_is_tentative() {
    let bom = encoding::decode(b"\xEF\xBB\xBFabc");
    assert_eq!(bom.confidence, Confidence::Certain);
    assert_eq!(bom.text, "abc");

    let meta = encoding::decode(b"<meta charset=utf-8>abc");
    assert_eq!(meta.encoding, Some("UTF-8"));
    assert_eq!(meta.confidence, Confidence::Tentative);

    let default = encoding::decode(b"<p>abc");
    assert_eq!(default.encoding, Some("windows-1252"));
    assert_eq!(default.decoded_with, "windows-1252");
    assert_eq!(default.confidence, Confidence::Tentative);

    let empty = encoding::decode(b"");
    assert_eq!(empty.encoding, Some("windows-1252"));
    assert_eq!(empty.confidence, Confidence::Tentative);
    assert_eq!(empty.text, "");
}

#[test]
fn decoding_again_with_a_given_encoding_is_certain() {
    let bytes = b"<p>\xC3\xA9";
    let first = encoding::decode(bytes);
    assert_eq!(first.encoding, Some("windows-1252"));
    assert_eq!(first.text, "<p>\u{C3}\u{A9}");
    assert_eq!(first.confidence, Confidence::Tentative);

    let again = encoding::decode_with(bytes, "UTF-8").expect("an encoding's name");
    assert_eq!(again.encoding, Some("UTF-8"));
    assert_eq!(again.decoded_with, "UTF-8");
    assert_eq!(again.text, "<p>é");
    assert_eq!(again.confidence, Confidence::Certain);

    // A label that is not a name, and no encoding at all.
    assert!(encoding::decode_with(bytes, "utf8").is_none());
    assert!(encoding::decode_with(bytes, "UTF-7").is_none());
}
