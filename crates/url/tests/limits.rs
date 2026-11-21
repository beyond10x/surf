//! What the crate documentation promises beside the test data: the input length limit, the
//! reason of its own for a host outside ASCII, and no panic on any input.

use surf_url::{Host, ParseError, Url, MAX_INPUT_LENGTH};

#[test]
fn input_longer_than_the_limit_is_too_long() {
    let path = "a".repeat(MAX_INPUT_LENGTH - "http://h/".len());
    let at_limit = format!("http://h/{path}");
    assert_eq!(at_limit.len(), MAX_INPUT_LENGTH);
    assert!(Url::parse(&at_limit, None).is_ok());
    let over = format!("{at_limit}a");
    assert_eq!(Url::parse(&over, None), Err(ParseError::TooLong));
}

#[test]
fn special_host_outside_ascii_is_non_ascii_host() {
    assert_eq!(
        Url::parse("http://\u{e9}xample.org/", None),
        Err(ParseError::NonAsciiHost)
    );
    assert_eq!(
        Url::parse("https://%C3%A9xample.org/", None),
        Err(ParseError::NonAsciiHost)
    );
}

#[test]
fn opaque_host_outside_ascii_is_percent_encoded() {
    let url = Url::parse("web+demo://\u{e9}xample.org/", None).expect("an opaque host parses");
    assert_eq!(
        url.host(),
        Some(&Host::Opaque("%C3%A9xample.org".to_string()))
    );
}

/// A fixed sweep of generated inputs, each against no base and three bases: none panics, and
/// every one that parses serializes to a string that parses again to the same serialization
/// (the URL Standard's goal of idempotence).
#[test]
fn generated_inputs_neither_panic_nor_change_when_parsed_again() {
    const PIECES: [&str; 40] = [
        "a",
        "Z",
        "0",
        "9",
        ".",
        "..",
        "%2e",
        "%",
        "%41",
        "%zz",
        "%C3%A9",
        "/",
        "\\",
        "//",
        ":",
        "@",
        "?",
        "#",
        "[",
        "]",
        "[::1]",
        "[1:2::3]",
        "1.2.3.4",
        "0x7f",
        "999999999999",
        " ",
        "\t",
        "\u{0}",
        "\u{7f}",
        "\u{e9}",
        "\u{fffd}",
        "\u{1F4A9}",
        "http:",
        "file:",
        "c:",
        "C|",
        "blob:",
        "web+demo:",
        "localhost",
        "65536",
    ];
    let bases: Vec<Url> = [
        "http://u:p@example.org:81/a/b?q#f",
        "file:///C:/x/y",
        "mailto:x",
    ]
    .iter()
    .map(|b| Url::parse(b, None).expect("the base parses"))
    .collect();
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut changed = Vec::new();
    let mut parsed = 0usize;
    for _ in 0..20_000 {
        let count = 1 + (next() % 8) as usize;
        let input: String = (0..count)
            .map(|_| PIECES[(next() % PIECES.len() as u64) as usize])
            .collect();
        for base in std::iter::once(None).chain(bases.iter().map(Some)) {
            let Ok(url) = Url::parse(&input, base) else {
                continue;
            };
            parsed += 1;
            let _ = url.origin().serialize();
            let href = url.href();
            match Url::parse(&href, None) {
                Ok(again) if again.href() == href => {}
                Ok(again) => changed.push(format!("<{input}>: <{href}> then <{}>", again.href())),
                Err(error) => changed.push(format!("<{input}>: <{href}> then {error}")),
            }
        }
    }
    assert!(changed.is_empty(), "{}", changed.join("\n"));
    // A sweep in which few inputs parse checks little; the bar is a quarter of the attempts.
    assert!(parsed > 20_000, "only {parsed} of 80000 parses succeeded");
}
