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

/// `fill <handle> <value>`: a value is any word but `-` (`cli/surf-agent.yaml`, the usage
/// `fill <handle> <value>|-`). A negative number for a number field is a value; the parser
/// refuses it as "not an option of surf-agent".
#[test]
fn fill_takes_a_value_that_starts_with_a_dash() {
    let dir = TestDir::new();
    let call = surf_agent::cli::parse(&["fill".into(), "f1".into(), "-5".into()], &env(&dir))
        .unwrap_or_else(|error| panic!("{}", error.0));
    assert_eq!(call.value.as_deref(), Some("-5"));
}

/// `pick <handle> <option>` takes an option's label: `-- choose --` is the label of many a
/// placeholder option, and the parser reads it as an unknown long option.
#[test]
fn pick_takes_an_option_label_that_starts_with_dashes() {
    let dir = TestDir::new();
    let call = surf_agent::cli::parse(
        &["pick".into(), "f1".into(), "-- choose --".into()],
        &env(&dir),
    )
    .unwrap_or_else(|error| panic!("{}", error.0));
    assert_eq!(call.option.as_deref(), Some("-- choose --"));
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
