//! Adversary cases of `story:agent-fill-pick` on the client: a value or an option word that
//! starts with `-`, and the bound of a value read from standard input.

use crate::common;

use common::*;

fn env(dir: &TestDir) -> surf_agent::cli::Environment {
    surf_agent::cli::Environment {
        runtime_dir: Some(dir.runtime().into()),
        session: Some(SESSION.into()),
        token: Some(TOKEN.into()),
        program_dir: None,
    }
}

/// `fill <handle> <value>`: a negative number for a number field is a value. Since decision 3
/// after the adversary's pass `--` ends the options and the word after it is the value; without
/// `--` the word is still read as an option and refused.
#[test]
fn fill_takes_a_value_that_starts_with_a_dash() {
    let dir = TestDir::new();
    let call = surf_agent::cli::parse(
        &["fill".into(), "f1".into(), "--".into(), "-5".into()],
        &env(&dir),
    )
    .unwrap_or_else(|error| panic!("{}", error.0));
    assert_eq!(call.value.as_deref(), Some("-5"));
    assert!(!call.stdin);
    let refused = surf_agent::cli::parse(&["fill".into(), "f1".into(), "-5".into()], &env(&dir))
        .expect_err("-5 without -- is an option");
    assert!(
        refused.0.contains("is not an option of surf-agent"),
        "{}",
        refused.0
    );
}

/// `pick <handle> <option>` takes an option's label: `-- choose --` is the label of many a
/// placeholder option; after `--` it is the option word.
#[test]
fn pick_takes_an_option_label_that_starts_with_dashes() {
    let dir = TestDir::new();
    let call = surf_agent::cli::parse(
        &[
            "pick".into(),
            "f1".into(),
            "--".into(),
            "-- choose --".into(),
        ],
        &env(&dir),
    )
    .unwrap_or_else(|error| panic!("{}", error.0));
    assert_eq!(call.option.as_deref(), Some("-- choose --"));
}

/// Decision 3: `fill f1 -` reads standard input; `fill f1 -- -` fills one dash; `--` ends the
/// options for every command, so an option after it is a word (and too many words are usage).
#[test]
fn a_dash_after_the_end_of_options_is_a_value() {
    let dir = TestDir::new();
    let parse = |words: &[&str]| {
        let words: Vec<std::ffi::OsString> = words.iter().map(Into::into).collect();
        surf_agent::cli::parse(&words, &env(&dir))
    };
    let stdin = parse(&["fill", "f1", "-"]).expect("a call");
    assert!(stdin.stdin);
    let dash = parse(&["fill", "f1", "--", "-"]).expect("a call");
    assert_eq!((dash.value.as_deref(), dash.stdin), (Some("-"), false));
    let handle = parse(&["follow", "--", "l1"]).expect("a call");
    assert_eq!(handle.handle.as_deref(), Some("l1"));
    let error = parse(&["look", "--", "--as", "tree"]).expect_err("two words for look");
    assert!(error.0.contains("look takes at most 0"), "{}", error.0);
}

/// Decision 4: a value of [`surf_agent::client::STDIN_VALUE_FITS`] bytes, each quoted to 4
/// bytes, fits a `fill` request beside a handle of `FILL_HANDLE_FITS` bytes, to the byte; one
/// byte more of handle does not.
#[test]
fn a_value_of_16000_bytes_always_fits_its_request() {
    use surf_agent::client::{FILL_HANDLE_FITS, STDIN_VALUE_FITS};
    use surf_agent::wire::{Arg, REQUEST_MAX, Request, RequestError};

    assert_eq!(STDIN_VALUE_FITS, 16_000);
    let request = |handle: usize| Request {
        command: "fill".into(),
        session: Some(SESSION.into()),
        token: surf_secret::Token::from_hex(TOKEN),
        args: vec![
            Arg {
                name: "handle".into(),
                value: "f".repeat(handle),
            },
            Arg {
                name: "value".into(),
                value: "\"".repeat(STDIN_VALUE_FITS),
            },
        ],
    };
    let bytes = request(FILL_HANDLE_FITS).encode().expect("it fits");
    assert_eq!(bytes.len(), REQUEST_MAX);
    assert!(matches!(
        request(FILL_HANDLE_FITS + 1).encode(),
        Err(RequestError::TooLarge)
    ));
}

/// Decision 5: the `Debug` of a wire request writes each argument's value as its length.
#[test]
fn a_request_debug_writes_a_value_length_not_its_bytes() {
    let request = surf_agent::wire::Request {
        command: "fill".into(),
        session: Some(SESSION.into()),
        token: surf_secret::Token::from_hex(TOKEN),
        args: vec![surf_agent::wire::Arg {
            name: "value".into(),
            value: "s3cret-wire".into(),
        }],
    };
    let printed = format!("{request:?}");
    assert!(!printed.contains("s3cret"), "{printed}");
    assert!(!printed.contains(TOKEN), "{printed}");
    assert!(printed.contains("11 bytes"), "{printed}");
}

/// A value read from standard input that the client accepts (at most `STDIN_VALUE_MAX` bytes) is
/// sent; one it cannot send is refused with the bound of the value, not of the request. Here no
/// `surfd` runs, so a value that is sent ends with status 3. A value of `STDIN_VALUE_MAX` bytes
/// of `Q`, and one of 20,000 `€` (60,000 bytes, 240,000 bytes quoted on the wire), are refused
/// as "the request is longer than 65536 bytes".
#[test]
fn a_value_from_standard_input_within_its_bound_is_sent_or_refused_by_that_bound() {
    let dir = TestDir::new();
    let config = surf_agent::client::Config::default();
    let max = surf_agent::client::STDIN_VALUE_MAX;
    for input in [vec![b'Q'; max], "€".repeat(20_000).into_bytes()] {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = surf_agent::client::run_with_input(
            &["fill".into(), "f2".into(), "-".into()],
            &env(&dir),
            &config,
            &mut input.as_slice(),
            &mut out,
            &mut err,
        );
        let printed = String::from_utf8_lossy(&err);
        assert!(
            status == 3 || printed.contains("the value read from standard input is longer than"),
            "{} bytes: status {status}: {printed}",
            input.len()
        );
    }
}
