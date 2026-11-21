//! The two readers of the test formats: JSON (the tokenizer data) and `.dat` (tree
//! construction and encoding).

use surf_html_conformance::{dat, json};

#[test]
fn json_reader_rejects_malformed_input() {
    let malformed = [
        "",
        "   ",
        "{",
        "[1, 2",
        "[1, 2,]",
        "{\"a\": 1,}",
        "{\"a\" 1}",
        "{a: 1}",
        "{\"a\": 1 \"b\": 2}",
        "\"unterminated",
        "\"bad escape \\x\"",
        "\"short unicode \\u12\"",
        "\"raw control \u{1} character\"",
        "\"raw line\nfeed\"",
        "01",
        "1.",
        ".5",
        "-",
        "1e",
        "+1",
        "tru",
        "nul",
        "True",
        "[1] [2]",
        "{} x",
        "'single'",
        "[\"a\" \"b\"]",
    ];
    for input in malformed {
        assert!(json::parse(input).is_err(), "accepted {input:?}");
    }
    let mut deep = String::new();
    for _ in 0..100_000 {
        deep.push('[');
    }
    assert!(
        json::parse(&deep).is_err(),
        "accepted 100,000 open brackets"
    );

    let well_formed = [
        "null",
        " true ",
        "false",
        "0",
        "-0.5e+10",
        "\"\"",
        "[]",
        "{}",
        "[1, \"a\", {\"b\": [null]}]",
        "\"\\ud83d\\ude00 \\uD800 \\\" \\\\ \\/ \\b \\f \\n \\r \\t\"",
    ];
    for input in well_formed {
        assert!(json::parse(input).is_ok(), "refused {input:?}");
    }
    let json::Value::String(s) = json::parse("\"\\ud83d\\ude00\\uD800x\"").unwrap() else {
        panic!("not a string");
    };
    assert_eq!(
        s,
        vec![0x1F600, 0xD800, 'x' as u32],
        "a pair joins, a lone surrogate stays"
    );
}

#[test]
fn a_description_section_starts_a_case() {
    let file = b"#description\nfirst\n#data\none\n#errors\n#document\n| <a>\n\n\
#data\ntwo\nlines\n#errors\n(1,1): e\n#document\n| <b>\n#description\nthird\n#data\nthree\n#errors\n#document\n| <c>\n";
    let cases = dat::read(file).expect("the file reads");
    assert_eq!(cases.len(), 3, "{cases:?}");

    assert_eq!(cases[0].section("description"), Some(&b"first"[..]));
    assert_eq!(cases[0].section("data"), Some(&b"one"[..]));
    assert_eq!(cases[0].section("document"), Some(&b"| <a>"[..]));

    assert_eq!(cases[1].section("description"), None);
    assert_eq!(cases[1].section("data"), Some(&b"two\nlines"[..]));
    assert_eq!(cases[1].section("errors"), Some(&b"(1,1): e"[..]));
    assert_eq!(
        cases[1].section("document"),
        Some(&b"| <b>"[..]),
        "a case without a blank line before the next `#description` keeps its last line"
    );

    assert_eq!(cases[2].section("description"), Some(&b"third"[..]));
    assert_eq!(
        cases[2].section("data"),
        Some(&b"three"[..]),
        "the `#data` after `#description` belongs to the same case"
    );
    assert_eq!(cases[2].section("document"), Some(&b"| <c>"[..]));

    let lone = dat::read(b"#data\n#errors\n#document\n").expect("the file reads");
    assert_eq!(lone.len(), 1);
    assert_eq!(lone[0].section("data"), Some(&b""[..]));
    assert_eq!(lone[0].section("document"), Some(&b""[..]));
}
