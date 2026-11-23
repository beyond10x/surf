//! Cases 1 to 20 of `story:agent-fill-pick`: the control and button lines of the `now` block,
//! `fill <handle> <value>` and `pick <handle> <option>`, their refusals, the forging of a value,
//! a label and an option label, the entries `surfd` keeps of a fill, and the radio button group
//! rule at parse time. Most cases drive the session core against a page server on `127.0.0.1`
//! the test starts ([`Site`]); cases 3 and 17 run `surfd` in the test process (decision D29) with
//! the built `surf-session`, case 3 with the built `surf-agent` reading the value from standard
//! input.

#![forbid(unsafe_code)]

use crate::common;
use crate::core::{Fetch, HTML, ID, Site, holder, open_at};

use std::io::Write;
use std::process::Stdio;

use common::{
    PageServer, TestDir, agent, agent_open, golden, placeholders, stdout, surfd_in_process,
    wire_as, wire_open,
};
use surf_agent::wire::Request;
use surf_answer::{Answer, Did, Element, RefusalReason, State, Target, Value, parse, render};
use surf_session::Session;
use surfd::link::EntryKind;

/// The form of case 1: a text field, a password, a checkbox, a radio group, a select, a
/// textarea, a file control, a disabled field, a required field, a read-only field (case 19), a
/// field in a `hidden` element (case 9) and a submit button.
const FORM: &str = "<!doctype html><html><head><title>Sign up</title></head><body>\
<form action=\"/save\" method=\"post\">\
<label>User name <input name=\"user\" value=\"ada\"></label>\
<label>Password <input type=\"password\" name=\"pw\"></label>\
<label><input type=\"checkbox\" name=\"news\" value=\"yes\"> News</label>\
<label><input type=\"radio\" name=\"size\" value=\"s\" checked> Small</label>\
<label><input type=\"radio\" name=\"size\" value=\"m\"> Medium</label>\
<label>Colour <select name=\"colour\"><option value=\"r\">Red</option>\
<option value=\"g\" selected>Green</option><option value=\"b\" disabled>Blue</option></select></label>\
<label>Note <textarea name=\"note\">hello</textarea></label>\
<label>Photo <input type=\"file\" name=\"photo\"></label>\
<label>Coupon <input name=\"coupon\" value=\"SAVE10\" disabled></label>\
<label>Email <input type=\"email\" name=\"email\" required></label>\
<label>Code <input name=\"code\" value=\"X1\" readonly></label>\
<div hidden><label>Secret <input name=\"secret\"></label></div>\
<button>Save</button>\
</form></body></html>";

/// The value cases 14 to 16 put into a page or a fill: a line feed, a double quote and U+202E.
const FORGING: &str = "a\nb\"c\u{202e}d";

/// [`FORGING`] as page text: the line feed and the quote as character references.
const FORGING_HTML: &str = "a&#10;b&quot;c&#x202e;d";

/// The password of cases 3, 4 and 17.
const PASSWORD: &str = "hunter2-correct-horse";

fn form_site(page: &str) -> Site {
    Site::start(vec![("/form.html", HTML, page.as_bytes().to_vec())])
}

/// A session opened at the page of `site`, its token and its port.
fn opened(site: &Site) -> (Session<Fetch>, String, u16) {
    let url = site.url("/form.html");
    let port = url
        .rsplit(':')
        .next()
        .and_then(|rest| rest.split('/').next())
        .and_then(|port| port.parse().ok())
        .expect("a port");
    let (session, token, _) = open_at(&url, &[]);
    (session, token, port)
}

/// The reply to `request`: its exit status, its lines and its typed answer.
fn ask(session: &mut Session<Fetch>, request: &Request) -> (u8, String, Answer) {
    let reply = session.answer(request);
    let link = reply.to_link();
    (link.status, link.text, reply.answer)
}

fn call(
    session: &mut Session<Fetch>,
    token: &str,
    command: &str,
    args: &[(&str, &str)],
) -> (u8, String, Answer) {
    ask(session, &holder(command, token, args))
}

/// The `now` line of `handle` in `text`.
fn line<'a>(text: &'a str, handle: &str) -> &'a str {
    let start = format!("  {handle} ");
    text.lines()
        .find(|line| line.starts_with(&start))
        .unwrap_or_else(|| panic!("no now line {handle}: {text}"))
}

/// The typed `now` element of `handle`.
fn element<'a>(answer: &'a Answer, handle: &str) -> &'a Element {
    answer
        .now
        .as_ref()
        .expect("a now block")
        .elements
        .iter()
        .find(|element| element.handle == handle)
        .unwrap_or_else(|| panic!("no now element {handle}"))
}

/// The state of the control `handle`.
fn state<'a>(answer: &'a Answer, handle: &str) -> &'a State {
    match &element(answer, handle).target {
        Target::State(state) => state,
        other => panic!("{handle} is not a control: {other:?}"),
    }
}

/// `parse` of `text` gives `typed`, and `render` of it gives `text` byte for byte (case 18).
fn round_trip(text: &str, typed: &Answer) {
    let parsed = parse(text).unwrap_or_else(|error| panic!("{error}: {text}"));
    assert_eq!(&parsed, typed, "{text}");
    assert_eq!(render(&parsed).unwrap(), text);
}

/// A refusal of `command` with `args`: status 1, `did refused <reason>`, the `now` line of
/// `handle` as it was, and no action recorded.
fn refused(command: &str, args: &[(&str, &str)], reason: RefusalReason) {
    let site = form_site(FORM);
    let (mut session, token, _) = opened(&site);
    let (_, before, before_typed) = call(&mut session, &token, "look", &[]);
    let (status, text, typed) = call(&mut session, &token, command, args);
    assert_eq!(status, 1, "{text}");
    assert_eq!(typed.did, Some(Did::Refused(reason)), "{text}");
    assert!(
        text.contains(&format!("\ndid refused {}\n", reason.word())),
        "{text}"
    );
    assert_eq!(
        typed.path, before_typed.path,
        "no action is recorded: {text}"
    );
    assert_eq!(typed.now, before_typed.now, "nothing changed: {before}");
    round_trip(&text, &typed);
}

#[test]
fn case_01_now_lists_the_controls_as_its_golden_text() {
    let site = form_site(FORM);
    let (mut session, token, port) = opened(&site);
    let (status, text, typed) = call(&mut session, &token, "look", &[]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        placeholders(&text, ID, &token, port),
        golden("look-controls.txt")
    );
    round_trip(&text, &typed);
}

#[test]
fn case_02_fill_a_text_field_shows_its_value() {
    let site = form_site(FORM);
    let (mut session, token, _) = opened(&site);
    let (status, text, typed) = call(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", "grace")],
    );
    assert_eq!(status, 0, "{text}");
    assert!(text.contains("\ndid fill \"f1\"\n"), "{text}");
    assert_eq!(line(&text, "f1"), "  f1 text ? \"User name\" = \"grace\"");
    assert!(text.contains("\n  3 fill f1\n"), "{text}");
    round_trip(&text, &typed);
}

#[test]
fn case_03_fill_a_password_from_standard_input_is_in_no_answer() {
    let dir = TestDir::new();
    let pages = PageServer::with_body(FORM.as_bytes().to_vec());
    let daemon = surfd_in_process(&dir, |_| {});
    let session = agent_open(&dir, Some(&pages.url("/form.html")));
    let mut child = agent(&dir)
        .args(["fill", "f2", "-"])
        .env("SURF_SESSION", &session.id)
        .env("SURF_TOKEN", &session.token)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("surf-agent runs");
    child
        .stdin
        .take()
        .expect("a piped standard input")
        .write_all(format!("{PASSWORD}\n").as_bytes())
        .expect("the password is written");
    let output = child.wait_with_output().expect("surf-agent ends");
    let filled = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{filled}");
    assert!(filled.contains("\ndid fill \"f2\"\n"), "{filled}");
    assert!(
        filled.contains("\n  f2 password ? \"Password\" = (set)\n"),
        "{filled}"
    );
    assert!(filled.contains("\n  3 fill f2\n"), "{filled}");
    let look = common::run(
        agent(&dir)
            .arg("look")
            .env("SURF_SESSION", &session.id)
            .env("SURF_TOKEN", &session.token),
    );
    let looked = stdout(&look);
    for text in [
        &filled,
        &looked,
        &String::from_utf8_lossy(&output.stderr).into_owned(),
    ] {
        assert!(!text.contains(PASSWORD), "{text}");
    }
    let entries = daemon
        .session_entries(&session.id)
        .expect("the session is listed")
        .entries;
    for entry in &entries {
        assert!(!entry.text.contains(PASSWORD), "{}", entry.text);
    }
    let fill = entries
        .iter()
        .find(|entry| entry.text.starts_with("event surf.agent.FillRequested "))
        .expect("the FillRequested entry");
    assert!(
        fill.text
            .contains(&format!(" value=\"{} bytes\"", PASSWORD.len())),
        "the value read from standard input without its line feed: {}",
        fill.text
    );
}

#[test]
fn case_04_fill_a_password_as_a_word_shows_set() {
    let site = form_site(FORM);
    let (mut session, token, _) = opened(&site);
    let (_, before, _) = call(&mut session, &token, "look", &[]);
    assert_eq!(
        line(&before, "f2"),
        "  f2 password ? \"Password\" = (empty)"
    );
    let (status, text, typed) = call(
        &mut session,
        &token,
        "fill",
        &[("handle", "f2"), ("value", PASSWORD)],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(line(&text, "f2"), "  f2 password ? \"Password\" = (set)");
    assert!(!text.contains(PASSWORD), "{text}");
    assert_eq!(state(&typed, "f2").value, Some(Value::Set));
}

#[test]
fn case_05_pick_a_select_option_by_label() {
    let site = form_site(FORM);
    let (mut session, token, _) = opened(&site);
    let (status, text, _) = call(
        &mut session,
        &token,
        "pick",
        &[("handle", "f6"), ("option", "Red")],
    );
    assert_eq!(status, 0, "{text}");
    assert!(text.contains("\ndid pick \"f6\"\n"), "{text}");
    assert_eq!(line(&text, "f6"), "  f6 select ? \"Colour\" = \"Red\"");
    assert!(text.contains("\n  3 pick f6\n"), "{text}");
}

#[test]
fn case_06_pick_a_select_option_by_value() {
    let site = form_site(FORM);
    let (mut session, token, _) = opened(&site);
    let (status, text, _) = call(
        &mut session,
        &token,
        "pick",
        &[("handle", "f6"), ("option", "r")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(line(&text, "f6"), "  f6 select ? \"Colour\" = \"Red\"");
}

#[test]
fn case_07_pick_a_checkbox_on_then_off() {
    let site = form_site(FORM);
    let (mut session, token, _) = opened(&site);
    let (status, text, _) = call(
        &mut session,
        &token,
        "pick",
        &[("handle", "f3"), ("option", "on")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(line(&text, "f3"), "  f3 checkbox ? \"News\" checked");
    let (status, text, _) = call(
        &mut session,
        &token,
        "pick",
        &[("handle", "f3"), ("option", "off")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(line(&text, "f3"), "  f3 checkbox ? \"News\"");
    assert!(text.contains("\n  3 pick f3\n  4 pick f3\n"), "{text}");
}

#[test]
fn case_08_pick_a_radio_unchecks_the_others_of_its_group() {
    let site = form_site(FORM);
    let (mut session, token, _) = opened(&site);
    let (_, before, _) = call(&mut session, &token, "look", &[]);
    assert_eq!(line(&before, "f4"), "  f4 radio ? \"Small\" checked");
    let (status, text, _) = call(
        &mut session,
        &token,
        "pick",
        &[("handle", "f5"), ("option", "on")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(line(&text, "f5"), "  f5 radio ? \"Medium\" checked");
    assert_eq!(line(&text, "f4"), "  f4 radio ? \"Small\"");
}

#[test]
fn case_09_fill_a_hidden_control_is_refused_hidden() {
    refused(
        "fill",
        &[("handle", "f12"), ("value", "x")],
        RefusalReason::Hidden,
    );
}

#[test]
fn case_10_fill_a_disabled_control_is_refused_disabled() {
    refused(
        "fill",
        &[("handle", "f9"), ("value", "FREE")],
        RefusalReason::Disabled,
    );
}

#[test]
fn case_11_fill_a_checkbox_is_refused_wrong_kind() {
    refused(
        "fill",
        &[("handle", "f3"), ("value", "yes")],
        RefusalReason::WrongKind,
    );
}

#[test]
fn case_12_pick_an_unknown_option_is_refused_unknown_option() {
    refused(
        "pick",
        &[("handle", "f6"), ("option", "Purple")],
        RefusalReason::UnknownOption,
    );
}

#[test]
fn case_13_fill_f99_is_refused_unknown_handle() {
    refused(
        "fill",
        &[("handle", "f99"), ("value", "x")],
        RefusalReason::UnknownHandle,
    );
}

/// The lines of `text` and the one line of `handle`, which must hold `quoted` in its field.
fn stays_in_its_field(text: &str, lines: usize, handle: &str, quoted: &str) {
    assert_eq!(text.lines().count(), lines, "no extra line: {text}");
    assert!(line(text, handle).contains(quoted), "{text}");
    assert!(text.is_ascii(), "{text}");
}

#[test]
fn case_14_a_forged_value_stays_in_its_field() {
    let site = form_site(FORM);
    let (mut session, token, _) = opened(&site);
    let (_, plain, _) = call(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", "plain")],
    );
    let (status, text, typed) = call(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", FORGING)],
    );
    assert_eq!(status, 0, "{text}");
    // A text field strips its line breaks (decision 1 of "As decided after the adversary's
    // pass"): the line feed is gone, `"` and the bidirectional control are escaped.
    let stripped = FORGING.replace('\n', "");
    stays_in_its_field(
        &text,
        plain.lines().count() + 1,
        "f1",
        &format!("= {}", surf_answer::quote(&stripped)),
    );
    assert_eq!(
        state(&typed, "f1").value,
        Some(Value::Text(surf_answer::Cut::of(
            &stripped,
            surf_answer::VALUE_CHARS_MAX
        )))
    );
    round_trip(&text, &typed);
    // A textarea keeps it, escaped as `\x0a`.
    let (status, text, typed) = call(
        &mut session,
        &token,
        "fill",
        &[("handle", "f7"), ("value", FORGING)],
    );
    assert_eq!(status, 0, "{text}");
    let quoted = surf_answer::quote(FORGING);
    assert!(quoted.contains("\\x0a"), "{quoted}");
    stays_in_its_field(
        &text,
        plain.lines().count() + 2,
        "f7",
        &format!("= {quoted}"),
    );
    assert_eq!(
        state(&typed, "f7").value,
        Some(Value::Text(surf_answer::Cut::of(
            FORGING,
            surf_answer::VALUE_CHARS_MAX
        )))
    );
    round_trip(&text, &typed);
}

#[test]
fn case_15_a_forged_label_stays_in_its_field() {
    let forged = FORM.replace("User name", FORGING_HTML);
    let site = form_site(&forged);
    let (mut session, token, _) = opened(&site);
    let (_, text, typed) = call(&mut session, &token, "look", &[]);
    let plain_site = form_site(FORM);
    let (mut plain_session, plain_token, _) = opened(&plain_site);
    let (_, plain, _) = call(&mut plain_session, &plain_token, "look", &[]);
    // The accessible name strips and collapses ASCII whitespace: the line feed is a space.
    stays_in_its_field(
        &text,
        plain.lines().count(),
        "f1",
        &surf_answer::quote("a b\"c\u{202e}d"),
    );
    round_trip(&text, &typed);
}

#[test]
fn case_16_a_forged_option_label_stays_in_its_field() {
    let forged = FORM.replace(">Green<", &format!(">{FORGING_HTML}<"));
    let site = form_site(&forged);
    let (mut session, token, _) = opened(&site);
    let (_, text, typed) = call(&mut session, &token, "look", &[]);
    let plain_site = form_site(FORM);
    let (mut plain_session, plain_token, _) = opened(&plain_site);
    let (_, plain, _) = call(&mut plain_session, &plain_token, "look", &[]);
    // An option's label is its text with ASCII whitespace stripped and collapsed.
    stays_in_its_field(
        &text,
        plain.lines().count(),
        "f6",
        &format!("= {}", surf_answer::quote("a b\"c\u{202e}d")),
    );
    round_trip(&text, &typed);
}

#[test]
fn case_17_fill_events_map_with_their_fields() {
    let dir = TestDir::new();
    let pages = PageServer::with_body(FORM.as_bytes().to_vec());
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, Some(&pages.url("/form.html")));
    let answer = wire_as(
        &dir,
        &session,
        "fill",
        &[("handle", "f2"), ("value", PASSWORD)],
    );
    assert_eq!(answer.status, 0, "{answer:?}");
    let answer = wire_as(
        &dir,
        &session,
        "fill",
        &[("handle", "f1"), ("value", "grace")],
    );
    assert_eq!(answer.status, 0, "{answer:?}");
    let entries = daemon
        .session_entries(&session.id)
        .expect("the session is listed")
        .entries;
    for entry in &entries {
        assert!(!entry.text.contains(PASSWORD), "{}", entry.text);
    }
    let fills: Vec<&str> = entries
        .iter()
        .filter(|entry| entry.kind != EntryKind::Answer)
        .map(|entry| entry.text.as_str())
        .filter(|text| text.starts_with("event surf.agent.FillRequested "))
        .collect();
    assert_eq!(fills.len(), 2, "{fills:#?}");
    for (fill, (handle, length)) in fills.iter().zip([("f2", PASSWORD.len()), ("f1", 5)]) {
        assert!(!fill.contains(" unmapped"), "{fill}");
        assert!(fill.contains(&format!(" handle=\"{handle}\"")), "{fill}");
        assert!(
            fill.contains(&format!(" value=\"{length} bytes\"")),
            "{fill}"
        );
    }
    let set = entries
        .iter()
        .filter(|entry| entry.text.starts_with("event surf.document.ValueSet "))
        .count();
    assert_eq!(set, 2, "one ValueSet per fill");
    let actions: Vec<&str> = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Action)
        .map(|entry| entry.text.as_str())
        .filter(|text| text.trim_end().split(' ').nth(2) == Some("fill"))
        .collect();
    for action in &actions {
        assert_eq!(
            action.trim_end().split(' ').count(),
            3,
            "no argument: {action}"
        );
    }
    assert_eq!(actions.len(), 2, "{actions:#?}");
}

#[test]
fn case_18_answers_read_back_to_their_typed_value() {
    let site = form_site(FORM);
    let (mut session, token, _) = opened(&site);
    for args in [
        vec![],
        vec![("handle", "f1"), ("value", FORGING)],
        vec![("handle", "f2"), ("value", PASSWORD)],
        vec![("handle", "f6"), ("option", "Red")],
        vec![("handle", "f3"), ("option", "on")],
    ] {
        let command = match args.get(1).map(|(name, _)| *name) {
            None => "look",
            Some("value") => "fill",
            _ => "pick",
        };
        let (status, text, typed) = call(&mut session, &token, command, &args);
        assert_eq!(status, 0, "{text}");
        round_trip(&text, &typed);
    }
    for page in [
        FORM.replace("User name", FORGING_HTML),
        FORM.replace(">Green<", &format!(">{FORGING_HTML}<")),
    ] {
        let site = form_site(&page);
        let (mut session, token, _) = opened(&site);
        let (_, text, typed) = call(&mut session, &token, "look", &[]);
        round_trip(&text, &typed);
    }
}

#[test]
fn case_19_fill_a_readonly_field_is_refused_readonly() {
    refused(
        "fill",
        &[("handle", "f11"), ("value", "X2")],
        RefusalReason::Readonly,
    );
}

#[test]
fn case_20_radio_group_two_checked_shows_the_last() {
    let page = "<!doctype html><form><label><input type=radio name=size value=s checked> \
                Small</label><label><input type=radio name=size value=m checked> \
                Medium</label></form>";
    let site = form_site(page);
    let (mut session, token, _) = opened(&site);
    let (_, text, _) = call(&mut session, &token, "look", &[]);
    assert_eq!(line(&text, "f1"), "  f1 radio ? \"Small\"");
    assert_eq!(line(&text, "f2"), "  f2 radio ? \"Medium\" checked");
}

/// `fill` and `pick` without their value or option, or with an argument they do not take, are
/// usage refusals.
#[test]
fn fill_and_pick_without_their_words_are_usage() {
    let site = form_site(FORM);
    let (mut session, token, _) = opened(&site);
    for (command, args) in [
        ("fill", vec![("handle", "f1")]),
        ("pick", vec![("handle", "f6")]),
        ("fill", vec![("value", "x")]),
        (
            "fill",
            vec![("handle", "f1"), ("value", "x"), ("option", "y")],
        ),
    ] {
        let (status, text, typed) = call(&mut session, &token, command, &args);
        assert_eq!(status, 2, "{command} {args:?}: {text}");
        assert_eq!(typed.did, Some(Did::Refused(RefusalReason::Usage)));
    }
}

/// `pick` refuses a word a checkbox or a radio button does not take, a disabled option and a
/// control it does not act on.
#[test]
fn pick_refuses_what_its_control_does_not_take() {
    refused(
        "pick",
        &[("handle", "f3"), ("option", "maybe")],
        RefusalReason::UnknownOption,
    );
    refused(
        "pick",
        &[("handle", "f5"), ("option", "off")],
        RefusalReason::UnknownOption,
    );
    refused(
        "pick",
        &[("handle", "f6"), ("option", "Blue")],
        RefusalReason::Disabled,
    );
    refused(
        "pick",
        &[("handle", "f1"), ("option", "on")],
        RefusalReason::WrongKind,
    );
    refused(
        "fill",
        &[("handle", "b1"), ("value", "x")],
        RefusalReason::WrongKind,
    );
}
