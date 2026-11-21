//! The input byte stream (13.2.3): the Encoding Standard's labels and the decoders, BOM
//! sniffing and the prescan. Written by `story:html-input-stream`.
//!
//! The standards are those `docs/design/html-parser.md` names: the HTML Standard at snapshot
//! `0e292f402d0c3dd17fe00c1bb2ef3a3c94cd8bce` and the Encoding Standard at snapshot
//! `7d2e3e08e11f57ba1b080390b490d53648c6f69c`. The encodings taken on are those of the note's
//! item 4: every label, decoders for UTF-8, UTF-16LE, UTF-16BE, windows-1252 and replacement,
//! windows-1252 as the default.

/// How sure the parser is of the encoding (13.2.3, its *confidence*). *Irrelevant*, the third
/// value of the standard, is for a stream of code points, which [`decode`] never reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    /// *Tentative*: a `meta` met while parsing may change the encoding (13.2.3.4).
    Tentative,
    /// *Certain*: the encoding is not changed.
    Certain,
}

/// What [`decode`] and [`decode_with`] return.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    /// The decoded text.
    pub text: String,
    /// The name, as the Encoding Standard writes it, of the encoding chosen: by the encoding
    /// sniffing algorithm for [`decode`], the one given (or a byte order mark's) for
    /// [`decode_with`]; also when it has no decoder here and [`Decoded::decoded_with`] is
    /// windows-1252. Both functions always choose one, so it is never `None`.
    pub encoding: Option<&'static str>,
    /// The name of the encoding the text was decoded with: [`Decoded::encoding`] when it has a
    /// decoder here, otherwise windows-1252.
    pub decoded_with: &'static str,
    /// The confidence in [`Decoded::encoding`].
    pub confidence: Confidence,
}

const UTF_8: &str = "UTF-8";
const UTF_16BE: &str = "UTF-16BE";
const UTF_16LE: &str = "UTF-16LE";
const WINDOWS_1252: &str = "windows-1252";
const REPLACEMENT: &str = "replacement";
const X_USER_DEFINED: &str = "x-user-defined";

/// Every encoding of the Encoding Standard with its labels, in the order of its table
/// (4.2 "Names and labels"). Source: `encodings.json` of the Encoding Standard at snapshot
/// `7d2e3e08e11f57ba1b080390b490d53648c6f69c`, written out with
/// `jq -r '.[].encodings[] | "(\"\(.name)\", &[\(.labels|map("\"\(.)\"")|join(", "))]),"'`:
/// 40 encodings, 228 labels.
pub const ENCODINGS: &[(&str, &[&str])] = &[
    (
        "UTF-8",
        &[
            "unicode-1-1-utf-8",
            "unicode11utf8",
            "unicode20utf8",
            "utf-8",
            "utf8",
            "x-unicode20utf8",
        ],
    ),
    ("IBM866", &["866", "cp866", "csibm866", "ibm866"]),
    (
        "ISO-8859-2",
        &[
            "csisolatin2",
            "iso-8859-2",
            "iso-ir-101",
            "iso8859-2",
            "iso88592",
            "iso_8859-2",
            "iso_8859-2:1987",
            "l2",
            "latin2",
        ],
    ),
    (
        "ISO-8859-3",
        &[
            "csisolatin3",
            "iso-8859-3",
            "iso-ir-109",
            "iso8859-3",
            "iso88593",
            "iso_8859-3",
            "iso_8859-3:1988",
            "l3",
            "latin3",
        ],
    ),
    (
        "ISO-8859-4",
        &[
            "csisolatin4",
            "iso-8859-4",
            "iso-ir-110",
            "iso8859-4",
            "iso88594",
            "iso_8859-4",
            "iso_8859-4:1988",
            "l4",
            "latin4",
        ],
    ),
    (
        "ISO-8859-5",
        &[
            "csisolatincyrillic",
            "cyrillic",
            "iso-8859-5",
            "iso-ir-144",
            "iso8859-5",
            "iso88595",
            "iso_8859-5",
            "iso_8859-5:1988",
        ],
    ),
    (
        "ISO-8859-6",
        &[
            "arabic",
            "asmo-708",
            "csiso88596e",
            "csiso88596i",
            "csisolatinarabic",
            "ecma-114",
            "iso-8859-6",
            "iso-8859-6-e",
            "iso-8859-6-i",
            "iso-ir-127",
            "iso8859-6",
            "iso88596",
            "iso_8859-6",
            "iso_8859-6:1987",
        ],
    ),
    (
        "ISO-8859-7",
        &[
            "csisolatingreek",
            "ecma-118",
            "elot_928",
            "greek",
            "greek8",
            "iso-8859-7",
            "iso-ir-126",
            "iso8859-7",
            "iso88597",
            "iso_8859-7",
            "iso_8859-7:1987",
            "sun_eu_greek",
        ],
    ),
    (
        "ISO-8859-8",
        &[
            "csiso88598e",
            "csisolatinhebrew",
            "hebrew",
            "iso-8859-8",
            "iso-8859-8-e",
            "iso-ir-138",
            "iso8859-8",
            "iso88598",
            "iso_8859-8",
            "iso_8859-8:1988",
            "visual",
        ],
    ),
    ("ISO-8859-8-I", &["csiso88598i", "iso-8859-8-i", "logical"]),
    (
        "ISO-8859-10",
        &[
            "csisolatin6",
            "iso-8859-10",
            "iso-ir-157",
            "iso8859-10",
            "iso885910",
            "l6",
            "latin6",
        ],
    ),
    ("ISO-8859-13", &["iso-8859-13", "iso8859-13", "iso885913"]),
    ("ISO-8859-14", &["iso-8859-14", "iso8859-14", "iso885914"]),
    (
        "ISO-8859-15",
        &[
            "csisolatin9",
            "iso-8859-15",
            "iso8859-15",
            "iso885915",
            "iso_8859-15",
            "l9",
        ],
    ),
    ("ISO-8859-16", &["iso-8859-16"]),
    ("KOI8-R", &["cskoi8r", "koi", "koi8", "koi8-r", "koi8_r"]),
    ("KOI8-U", &["koi8-ru", "koi8-u"]),
    (
        "macintosh",
        &["csmacintosh", "mac", "macintosh", "x-mac-roman"],
    ),
    (
        "windows-874",
        &[
            "dos-874",
            "iso-8859-11",
            "iso8859-11",
            "iso885911",
            "tis-620",
            "windows-874",
        ],
    ),
    ("windows-1250", &["cp1250", "windows-1250", "x-cp1250"]),
    ("windows-1251", &["cp1251", "windows-1251", "x-cp1251"]),
    (
        "windows-1252",
        &[
            "ansi_x3.4-1968",
            "ascii",
            "cp1252",
            "cp819",
            "csisolatin1",
            "ibm819",
            "iso-8859-1",
            "iso-ir-100",
            "iso8859-1",
            "iso88591",
            "iso_8859-1",
            "iso_8859-1:1987",
            "l1",
            "latin1",
            "us-ascii",
            "windows-1252",
            "x-cp1252",
        ],
    ),
    ("windows-1253", &["cp1253", "windows-1253", "x-cp1253"]),
    (
        "windows-1254",
        &[
            "cp1254",
            "csisolatin5",
            "iso-8859-9",
            "iso-ir-148",
            "iso8859-9",
            "iso88599",
            "iso_8859-9",
            "iso_8859-9:1989",
            "l5",
            "latin5",
            "windows-1254",
            "x-cp1254",
        ],
    ),
    ("windows-1255", &["cp1255", "windows-1255", "x-cp1255"]),
    ("windows-1256", &["cp1256", "windows-1256", "x-cp1256"]),
    ("windows-1257", &["cp1257", "windows-1257", "x-cp1257"]),
    ("windows-1258", &["cp1258", "windows-1258", "x-cp1258"]),
    ("x-mac-cyrillic", &["x-mac-cyrillic", "x-mac-ukrainian"]),
    (
        "GBK",
        &[
            "chinese",
            "csgb2312",
            "csiso58gb231280",
            "gb2312",
            "gb_2312",
            "gb_2312-80",
            "gbk",
            "iso-ir-58",
            "x-gbk",
        ],
    ),
    ("gb18030", &["gb18030"]),
    (
        "Big5",
        &["big5", "big5-hkscs", "cn-big5", "csbig5", "x-x-big5"],
    ),
    ("EUC-JP", &["cseucpkdfmtjapanese", "euc-jp", "x-euc-jp"]),
    ("ISO-2022-JP", &["csiso2022jp", "iso-2022-jp"]),
    (
        "Shift_JIS",
        &[
            "csshiftjis",
            "ms932",
            "ms_kanji",
            "shift-jis",
            "shift_jis",
            "sjis",
            "windows-31j",
            "x-sjis",
        ],
    ),
    (
        "EUC-KR",
        &[
            "cseuckr",
            "csksc56011987",
            "euc-kr",
            "iso-ir-149",
            "korean",
            "ks_c_5601-1987",
            "ks_c_5601-1989",
            "ksc5601",
            "ksc_5601",
            "windows-949",
        ],
    ),
    (
        "replacement",
        &[
            "csiso2022kr",
            "hz-gb-2312",
            "iso-2022-cn",
            "iso-2022-cn-ext",
            "iso-2022-kr",
            "replacement",
        ],
    ),
    ("UTF-16BE", &["unicodefffe", "utf-16be"]),
    (
        "UTF-16LE",
        &[
            "csunicode",
            "iso-10646-ucs-2",
            "ucs-2",
            "unicode",
            "unicodefeff",
            "utf-16",
            "utf-16le",
        ],
    ),
    ("x-user-defined", &["x-user-defined"]),
];

/// The index of windows-1252: the code point of each pointer 0 to 127, the byte being the
/// pointer plus 0x80. Source: `index-windows-1252.txt` of the Encoding Standard at snapshot
/// `7d2e3e08e11f57ba1b080390b490d53648c6f69c` (its "Identifier: e56d49d9…"), written out with
/// awk from its second column. Every pointer has a code point, so the decoder has no error.
const INDEX_WINDOWS_1252: [u16; 128] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, //
    0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x008D, 0x017D, 0x008F, //
    0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, //
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178, //
    0x00A0, 0x00A1, 0x00A2, 0x00A3, 0x00A4, 0x00A5, 0x00A6, 0x00A7, //
    0x00A8, 0x00A9, 0x00AA, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x00AF, //
    0x00B0, 0x00B1, 0x00B2, 0x00B3, 0x00B4, 0x00B5, 0x00B6, 0x00B7, //
    0x00B8, 0x00B9, 0x00BA, 0x00BB, 0x00BC, 0x00BD, 0x00BE, 0x00BF, //
    0x00C0, 0x00C1, 0x00C2, 0x00C3, 0x00C4, 0x00C5, 0x00C6, 0x00C7, //
    0x00C8, 0x00C9, 0x00CA, 0x00CB, 0x00CC, 0x00CD, 0x00CE, 0x00CF, //
    0x00D0, 0x00D1, 0x00D2, 0x00D3, 0x00D4, 0x00D5, 0x00D6, 0x00D7, //
    0x00D8, 0x00D9, 0x00DA, 0x00DB, 0x00DC, 0x00DD, 0x00DE, 0x00DF, //
    0x00E0, 0x00E1, 0x00E2, 0x00E3, 0x00E4, 0x00E5, 0x00E6, 0x00E7, //
    0x00E8, 0x00E9, 0x00EA, 0x00EB, 0x00EC, 0x00ED, 0x00EE, 0x00EF, //
    0x00F0, 0x00F1, 0x00F2, 0x00F3, 0x00F4, 0x00F5, 0x00F6, 0x00F7, //
    0x00F8, 0x00F9, 0x00FA, 0x00FB, 0x00FC, 0x00FD, 0x00FE, 0x00FF, //
];

/// The number of bytes the prescan reads: "User agents are encouraged to only prescan the first
/// 1024 bytes" (13.2.3.2, the encoding sniffing algorithm, step 5).
pub const PRESCAN_BYTES: usize = 1024;

/// Encoding Standard, 4.2 "get an encoding": `label` with leading and trailing ASCII whitespace
/// removed, matched ASCII case-insensitively against the labels of [`ENCODINGS`]; the name of
/// the encoding it is a label of, or `None` (the standard's failure).
pub fn get_an_encoding(label: &str) -> Option<&'static str> {
    let label = label.trim_matches(is_ascii_whitespace_char);
    ENCODINGS
        .iter()
        .find(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case(label)))
        .map(|(name, _)| *name)
}

/// The name of [`ENCODINGS`] equal to `name`, as a `'static` string.
fn encoding_named(name: &str) -> Option<&'static str> {
    ENCODINGS.iter().map(|(n, _)| *n).find(|n| *n == name)
}

/// Get an encoding from bytes "isomorphic decoded" (each byte the code point of its value).
fn get_an_encoding_from_bytes(bytes: &[u8]) -> Option<&'static str> {
    let label: String = bytes.iter().map(|&b| char::from(b)).collect();
    get_an_encoding(&label)
}

fn is_ascii_whitespace_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' ')
}

fn is_ascii_whitespace(b: u8) -> bool {
    matches!(b, 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

/// Runs the encoding sniffing algorithm (13.2.3.2) with no transport-layer encoding over
/// `bytes` and decodes them with the encoding it chose.
///
/// The steps taken: the byte order mark (confidence certain); the prescan of the first
/// [`PRESCAN_BYTES`] bytes with its "get an XML encoding" step (tentative); windows-1252, the
/// default (tentative). Deviations, `docs/design/html-parser.md` item 4: the transport-layer
/// step is skipped, because `surf.net.MediaType` carries no `charset` parameter; the optional
/// steps (the user's override, the container document, an earlier visit, autodetection) are not
/// taken.
pub fn decode(bytes: &[u8]) -> Decoded {
    let (encoding, confidence) = match bom_sniff(bytes) {
        Some((encoding, _)) => (encoding, Confidence::Certain),
        None => {
            let window = &bytes[..bytes.len().min(PRESCAN_BYTES)];
            match prescan(window) {
                Some(encoding) => (encoding, Confidence::Tentative),
                None => (WINDOWS_1252, Confidence::Tentative),
            }
        }
    };
    let (encoding, decoded_with, text) = decode_bytes(bytes, encoding);
    Decoded {
        text,
        encoding: Some(encoding),
        decoded_with,
        confidence,
    }
}

/// Decodes `bytes` again with the encoding named `encoding` (a name of [`ENCODINGS`], as the
/// Encoding Standard writes it), with the confidence certain: the entry for 13.2.3.4
/// "Changing the encoding while parsing", whose last step parses the same bytes again.
///
/// As the Encoding Standard's "decode" does, a byte order mark wins over `encoding`, and
/// [`Decoded::encoding`] then names the encoding of the mark. `None` when `encoding` is no
/// encoding's name.
pub fn decode_with(bytes: &[u8], encoding: &str) -> Option<Decoded> {
    let encoding = encoding_named(encoding)?;
    let (encoding, decoded_with, text) = decode_bytes(bytes, encoding);
    Some(Decoded {
        text,
        encoding: Some(encoding),
        decoded_with,
        confidence: Confidence::Certain,
    })
}

/// Encoding Standard, "BOM sniff": the encoding of a byte order mark at the start of `bytes`,
/// with the mark's length.
fn bom_sniff(bytes: &[u8]) -> Option<(&'static str, usize)> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        Some((UTF_8, 3))
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        Some((UTF_16BE, 2))
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        Some((UTF_16LE, 2))
    } else {
        None
    }
}

/// Encoding Standard, "decode": a byte order mark wins over `encoding` and is skipped; then the
/// decoder with the error mode replacement. Returns the encoding used, the encoding decoded
/// with and the text.
///
/// Deviation, `docs/design/html-parser.md` item 4: an encoding with no decoder here (35 of the
/// 40) is decoded with windows-1252's, and its name is kept.
fn decode_bytes(bytes: &[u8], encoding: &'static str) -> (&'static str, &'static str, String) {
    let (encoding, rest) = match bom_sniff(bytes) {
        Some((bom, length)) => (bom, &bytes[length..]),
        None => (encoding, bytes),
    };
    let (decoded_with, text) = match encoding {
        UTF_8 => (UTF_8, utf_8_decode(rest)),
        UTF_16BE => (UTF_16BE, utf_16_decode(rest, true)),
        UTF_16LE => (UTF_16LE, utf_16_decode(rest, false)),
        REPLACEMENT => (REPLACEMENT, replacement_decode(rest)),
        _ => (WINDOWS_1252, windows_1252_decode(rest)),
    };
    (encoding, decoded_with, text)
}

/// Encoding Standard, 8.1.1 "UTF-8 decoder", error mode replacement: each error is one
/// U+FFFD, and the byte that ends a sequence early is read again.
fn utf_8_decode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    let mut code_point: u32 = 0;
    let mut bytes_seen = 0;
    let mut bytes_needed = 0;
    let mut lower_boundary: u8 = 0x80;
    let mut upper_boundary: u8 = 0xBF;
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if bytes_needed == 0 {
            match byte {
                0x00..=0x7F => out.push(char::from(byte)),
                0xC2..=0xDF => {
                    bytes_needed = 1;
                    code_point = u32::from(byte & 0x1F);
                }
                0xE0..=0xEF => {
                    if byte == 0xE0 {
                        lower_boundary = 0xA0;
                    }
                    if byte == 0xED {
                        upper_boundary = 0x9F;
                    }
                    bytes_needed = 2;
                    code_point = u32::from(byte & 0x0F);
                }
                0xF0..=0xF4 => {
                    if byte == 0xF0 {
                        lower_boundary = 0x90;
                    }
                    if byte == 0xF4 {
                        upper_boundary = 0x8F;
                    }
                    bytes_needed = 3;
                    code_point = u32::from(byte & 0x07);
                }
                _ => out.push('\u{FFFD}'),
            }
            i += 1;
            continue;
        }
        if !(lower_boundary..=upper_boundary).contains(&byte) {
            code_point = 0;
            bytes_needed = 0;
            bytes_seen = 0;
            lower_boundary = 0x80;
            upper_boundary = 0xBF;
            // The byte is restored to the queue: read again, not consumed.
            out.push('\u{FFFD}');
            continue;
        }
        lower_boundary = 0x80;
        upper_boundary = 0xBF;
        code_point = (code_point << 6) | u32::from(byte & 0x3F);
        bytes_seen += 1;
        i += 1;
        if bytes_seen == bytes_needed {
            // The boundaries keep the value a scalar value.
            out.push(char::from_u32(code_point).unwrap_or('\u{FFFD}'));
            code_point = 0;
            bytes_needed = 0;
            bytes_seen = 0;
        }
    }
    if bytes_needed != 0 {
        out.push('\u{FFFD}');
    }
    out
}

/// Encoding Standard, 14.2.1 "shared UTF-16 decoder", error mode replacement. A lead surrogate
/// not followed by a trail surrogate is one U+FFFD and the code unit after it is read again; a
/// trail surrogate alone is one U+FFFD; a lead surrogate or an odd byte at the end is one
/// U+FFFD.
fn utf_16_decode(bytes: &[u8], big_endian: bool) -> String {
    let (pairs, _) = bytes.as_chunks::<2>();
    let units: Vec<u16> = pairs
        .iter()
        .map(|&pair| {
            if big_endian {
                u16::from_be_bytes(pair)
            } else {
                u16::from_le_bytes(pair)
            }
        })
        .collect();
    let mut out = String::with_capacity(units.len());
    let mut lead_surrogate: Option<u16> = None;
    let mut k = 0;
    while k < units.len() {
        let unit = units[k];
        if let Some(lead) = lead_surrogate.take() {
            if (0xDC00..=0xDFFF).contains(&unit) {
                let code_point =
                    0x10000 + ((u32::from(lead) - 0xD800) << 10) + (u32::from(unit) - 0xDC00);
                out.push(char::from_u32(code_point).unwrap_or('\u{FFFD}'));
                k += 1;
            } else {
                // The code unit's bytes are restored to the queue: read again.
                out.push('\u{FFFD}');
            }
            continue;
        }
        match unit {
            0xD800..=0xDBFF => lead_surrogate = Some(unit),
            0xDC00..=0xDFFF => out.push('\u{FFFD}'),
            _ => out.push(char::from_u32(u32::from(unit)).unwrap_or('\u{FFFD}')),
        }
        k += 1;
    }
    if lead_surrogate.is_some() || !bytes.len().is_multiple_of(2) {
        out.push('\u{FFFD}');
    }
    out
}

/// Encoding Standard, 9.1.1 "single-byte decoder" with the index of windows-1252.
fn windows_1252_decode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| match byte {
            0x00..=0x7F => char::from(byte),
            _ => char::from_u32(u32::from(INDEX_WINDOWS_1252[usize::from(byte - 0x80)]))
                .unwrap_or('\u{FFFD}'),
        })
        .collect()
}

/// Encoding Standard, 14.1.1 "replacement decoder": one U+FFFD for an input that is not empty,
/// then nothing.
fn replacement_decode(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        String::new()
    } else {
        "\u{FFFD}".to_string()
    }
}

/// The prescan ran out of bytes: its `position` went past the end of the bytes it was given.
struct OutOfBytes;

/// Moves `position` to the next byte; out of bytes when there is none.
fn advance(bytes: &[u8], position: &mut usize) -> Result<(), OutOfBytes> {
    *position += 1;
    if *position < bytes.len() {
        Ok(())
    } else {
        Err(OutOfBytes)
    }
}

/// Moves `position` to the first byte at or after `from` for which `found` holds; out of bytes
/// when there is none.
fn advance_to(
    bytes: &[u8],
    position: &mut usize,
    from: usize,
    found: impl Fn(usize) -> bool,
) -> Result<(), OutOfBytes> {
    match (from..bytes.len()).find(|&i| found(i)) {
        Some(i) => {
            *position = i;
            Ok(())
        }
        None => Err(OutOfBytes),
    }
}

/// 13.2.3.2 "prescan a byte stream to determine its encoding" over `bytes`, whose end is the
/// end condition: running out of them returns "get an XML encoding" over the same bytes.
fn prescan(bytes: &[u8]) -> Option<&'static str> {
    match prescan_steps(bytes) {
        Ok(encoding) => Some(encoding),
        Err(OutOfBytes) => get_an_xml_encoding(bytes),
    }
}

fn prescan_steps(bytes: &[u8]) -> Result<&'static str, OutOfBytes> {
    // Prescan for UTF-16 XML declarations.
    if bytes.starts_with(&[0x3C, 0x00, 0x3F, 0x00, 0x78, 0x00]) {
        return Ok(UTF_16LE);
    }
    if bytes.starts_with(&[0x00, 0x3C, 0x00, 0x3F, 0x00, 0x78]) {
        return Ok(UTF_16BE);
    }
    let mut position = 0;
    if bytes.is_empty() {
        return Err(OutOfBytes);
    }
    loop {
        let rest = &bytes[position..];
        if rest.starts_with(b"<!--") {
            // The first `>` preceded by two `-`, which may be those of `<!--`.
            let start = position;
            advance_to(bytes, &mut position, start + 4, |i| {
                bytes[i] == b'>' && bytes[i - 1] == b'-' && bytes[i - 2] == b'-'
            })?;
        } else if rest.len() >= 6
            && rest[..5].eq_ignore_ascii_case(b"<meta")
            && matches!(rest[5], 0x09 | 0x0A | 0x0C | 0x0D | 0x20 | 0x2F)
        {
            position += 5;
            if let Some(encoding) = prescan_meta(bytes, &mut position)? {
                return Ok(encoding);
            }
        } else if rest.len() >= 2
            && rest[0] == b'<'
            && (rest[1].is_ascii_alphabetic()
                || (rest[1] == b'/' && rest.len() >= 3 && rest[2].is_ascii_alphabetic()))
        {
            let start = position;
            advance_to(bytes, &mut position, start + 1, |i| {
                matches!(bytes[i], 0x09 | 0x0A | 0x0C | 0x0D | 0x20 | 0x3E)
            })?;
            while get_an_attribute(bytes, &mut position)?.is_some() {}
        } else if rest.starts_with(b"<!") || rest.starts_with(b"</") || rest.starts_with(b"<?") {
            let start = position;
            advance_to(bytes, &mut position, start + 1, |i| bytes[i] == b'>')?;
        }
        // Next byte.
        advance(bytes, &mut position)?;
    }
}

/// The steps of the prescan for a `meta` element, `position` at the space or slash after
/// `<meta`: the encoding to return, or `None` to go on with the next byte.
fn prescan_meta(bytes: &[u8], position: &mut usize) -> Result<Option<&'static str>, OutOfBytes> {
    let mut attribute_list: Vec<Vec<u8>> = Vec::new();
    let mut got_pragma = false;
    let mut need_pragma: Option<bool> = None;
    // `None` is the standard's null, `Some(None)` its failure.
    let mut charset: Option<Option<&'static str>> = None;
    while let Some((name, value)) = get_an_attribute(bytes, position)? {
        if attribute_list.contains(&name) {
            continue;
        }
        match name.as_slice() {
            b"http-equiv" => {
                if value == b"content-type" {
                    got_pragma = true;
                }
            }
            b"content" => {
                if let Some(encoding) = extract_from_meta_content(&value)
                    && charset.is_none()
                {
                    charset = Some(Some(encoding));
                    need_pragma = Some(true);
                }
            }
            b"charset" => {
                charset = Some(get_an_encoding_from_bytes(&value));
                need_pragma = Some(false);
            }
            _ => {}
        }
        attribute_list.push(name);
    }
    // Processing.
    match need_pragma {
        None => return Ok(None),
        Some(true) if !got_pragma => return Ok(None),
        _ => {}
    }
    let Some(Some(encoding)) = charset else {
        return Ok(None);
    };
    Ok(Some(match encoding {
        UTF_16BE | UTF_16LE => UTF_8,
        X_USER_DEFINED => WINDOWS_1252,
        other => other,
    }))
}

/// An attribute the prescan read: its name and its value, bytes lowered in ASCII.
type SniffedAttribute = (Vec<u8>, Vec<u8>);

/// 13.2.3.2 "get an attribute": the attribute's name and value, both lowered in ASCII, or
/// `None` when there is none (`position` at a `>`).
fn get_an_attribute(
    bytes: &[u8],
    position: &mut usize,
) -> Result<Option<SniffedAttribute>, OutOfBytes> {
    let at = |p: usize| bytes.get(p).copied().ok_or(OutOfBytes);
    while matches!(at(*position)?, 0x09 | 0x0A | 0x0C | 0x0D | 0x20 | 0x2F) {
        advance(bytes, position)?;
    }
    if at(*position)? == b'>' {
        return Ok(None);
    }
    let mut name: Vec<u8> = Vec::new();
    let mut value: Vec<u8> = Vec::new();
    loop {
        let byte = at(*position)?;
        if byte == b'=' && !name.is_empty() {
            advance(bytes, position)?;
            break;
        }
        if is_ascii_whitespace(byte) {
            // Spaces.
            while is_ascii_whitespace(at(*position)?) {
                advance(bytes, position)?;
            }
            if at(*position)? != b'=' {
                return Ok(Some((name, value)));
            }
            advance(bytes, position)?;
            break;
        }
        if byte == b'/' || byte == b'>' {
            return Ok(Some((name, value)));
        }
        name.push(byte.to_ascii_lowercase());
        advance(bytes, position)?;
    }
    // Value.
    while is_ascii_whitespace(at(*position)?) {
        advance(bytes, position)?;
    }
    let byte = at(*position)?;
    if byte == b'"' || byte == b'\'' {
        loop {
            advance(bytes, position)?;
            let next = at(*position)?;
            if next == byte {
                advance(bytes, position)?;
                return Ok(Some((name, value)));
            }
            value.push(next.to_ascii_lowercase());
        }
    }
    if byte == b'>' {
        return Ok(Some((name, value)));
    }
    value.push(byte.to_ascii_lowercase());
    advance(bytes, position)?;
    loop {
        let byte = at(*position)?;
        if is_ascii_whitespace(byte) || byte == b'>' {
            return Ok(Some((name, value)));
        }
        value.push(byte.to_ascii_lowercase());
        advance(bytes, position)?;
    }
}

/// The HTML Standard's "algorithm for extracting a character encoding from a `meta` element",
/// over the bytes of a `content` value: the encoding, or `None` for nothing and for failure.
pub(crate) fn extract_from_meta_content(s: &[u8]) -> Option<&'static str> {
    let mut position = 0;
    loop {
        // Loop: the first `charset` after position, ASCII case-insensitive.
        let found = (position..s.len().saturating_sub(6))
            .find(|&i| s[i..i + 7].eq_ignore_ascii_case(b"charset"))?;
        let mut i = found + 7;
        while i < s.len() && is_ascii_whitespace(s[i]) {
            i += 1;
        }
        if s.get(i) != Some(&b'=') {
            position = i;
            continue;
        }
        i += 1;
        while i < s.len() && is_ascii_whitespace(s[i]) {
            i += 1;
        }
        let next = *s.get(i)?;
        if next == b'"' || next == b'\'' {
            let end = s[i + 1..].iter().position(|&b| b == next)?;
            return get_an_encoding_from_bytes(&s[i + 1..i + 1 + end]);
        }
        let end = s[i..]
            .iter()
            .position(|&b| is_ascii_whitespace(b) || b == b';')
            .map_or(s.len(), |n| i + n);
        return get_an_encoding_from_bytes(&s[i..end]);
    }
}

/// 13.2.3.2 "get an XML encoding": the encoding of a `<?xml … encoding="…"` at the start of
/// `bytes`, or `None` for failure.
fn get_an_xml_encoding(bytes: &[u8]) -> Option<&'static str> {
    if !bytes.starts_with(b"<?xml") {
        return None;
    }
    let declaration_end = bytes.iter().position(|&b| b == b'>')?;
    let found = bytes[..declaration_end]
        .windows(8)
        .position(|w| w == b"encoding")?;
    let mut position = found + 8;
    // The `>` at declaration_end stops both loops before the end of the bytes.
    while bytes[position] <= 0x20 {
        position += 1;
    }
    if bytes[position] != b'=' {
        return None;
    }
    position += 1;
    while bytes[position] <= 0x20 {
        position += 1;
    }
    let quote_mark = bytes[position];
    if quote_mark != b'"' && quote_mark != b'\'' {
        return None;
    }
    position += 1;
    let length = bytes[position..].iter().position(|&b| b == quote_mark)?;
    let potential_encoding = &bytes[position..position + length];
    if potential_encoding.iter().any(|&b| b <= 0x20) {
        return None;
    }
    match get_an_encoding_from_bytes(potential_encoding)? {
        UTF_16BE | UTF_16LE => Some(UTF_8),
        other => Some(other),
    }
}
