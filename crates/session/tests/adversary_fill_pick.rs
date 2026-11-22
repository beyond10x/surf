//! Adversary cases of `story:agent-fill-pick`: the select after a `pick` of a later option, the
//! value sanitization of a text field, the radio button group rule at parse time across forms,
//! `form=`, names and missing names, a handle after `go`, the state after a refused `pick`, a
//! closed session, values with control characters, the `now` bound with long values, the order of
//! the refusals, and where a filled password must not appear.

#![forbid(unsafe_code)]

use crate::core::{Fetch, HTML, Site, holder, open_at};

use surf_answer::{Did, RefusalReason, Target, Value, parse, render};
use surf_session::{NOW_BYTES_MAX, Session};

/// A session opened at `/form.html` of `site` and its token.
fn opened(site: &Site) -> (Session<Fetch>, String) {
    let (session, token, _) = open_at(&site.url("/form.html"), &[]);
    (session, token)
}

fn site(page: &str) -> Site {
    Site::start(vec![("/form.html", HTML, page.as_bytes().to_vec())])
}

/// Status, lines and typed answer of `command` with `args`.
fn ask(
    session: &mut Session<Fetch>,
    token: &str,
    command: &str,
    args: &[(&str, &str)],
) -> (u8, String, surf_answer::Answer) {
    let reply = session.answer(&holder(command, token, args));
    let link = reply.to_link();
    (link.status, link.text, reply.answer)
}

/// The `now` line of `handle` in `text`.
fn line<'a>(text: &'a str, handle: &str) -> &'a str {
    let start = format!("  {handle} ");
    text.lines()
        .find(|line| line.starts_with(&start))
        .unwrap_or_else(|| panic!("no now line {handle}: {text}"))
}

/// `parse` of `text` reads back and renders the same bytes.
fn round_trip(text: &str) {
    let parsed = parse(text).unwrap_or_else(|error| panic!("{error}: {text}"));
    assert_eq!(render(&parsed).unwrap(), text);
}

/// A `pick` of an option after the selected one selects it: the `now` line shows its label. The
/// story's cases 5 and 6 pick `Red`, the option before the selected `Green`, so a `Green` left
/// selected beside it would not show.
#[test]
fn pick_of_an_option_after_the_selected_one_shows_it() {
    let page = "<!doctype html><form><label>Size <select name=s><option>Small</option>\
                <option selected>Medium</option><option>Large</option></select></label></form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (status, text, _) = ask(
        &mut session,
        &token,
        "pick",
        &[("handle", "f1"), ("option", "Large")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(line(&text, "f1"), "  f1 select ? \"Size\" = \"Large\"");
}

/// HTML Standard 4.10.5.1.2 (Text state), value sanitization algorithm: "Strip newlines from the
/// value." A text field never holds a line feed or a carriage return, whatever sets its value; a
/// person who presses Enter in one submits the form. The story's case 14 fills a line feed into
/// the text field `f1` and expects it kept.
#[test]
fn fill_of_a_text_field_strips_its_newlines() {
    let page = "<!doctype html><form><label>Name <input name=n></label>\
                <label>Note <textarea name=t></textarea></label></form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (status, text, _) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", "a\r\nb\nc")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(line(&text, "f1"), "  f1 text ? \"Name\" = \"abc\"");
    // A textarea keeps its line breaks.
    let (_, text, _) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f2"), ("value", "a\nb")],
    );
    assert_eq!(line(&text, "f2"), "  f2 textarea ? \"Note\" = \"a\\x0ab\"");
}

/// The radio button group rule at parse time, beyond case 20: two forms with the same name, a
/// `form=` owner outside its form, radio buttons with no form owner, names differing in case, no
/// name and an empty name.
#[test]
fn radio_groups_at_parse_time_follow_owner_and_name() {
    let page = "<!doctype html>\
        <form id=a><input type=radio name=g value=1 checked><input type=radio name=g value=2 checked></form>\
        <form id=b><input type=radio name=g value=3 checked></form>\
        <input type=radio name=g value=4 checked form=b>\
        <input type=radio name=g value=5 checked>\
        <input type=radio name=g value=6 checked>\
        <input type=radio name=G value=7 checked>\
        <input type=radio value=8 checked><input type=radio value=9 checked>\
        <input type=radio name=\"\" value=10 checked><input type=radio name=\"\" value=11 checked>\
        <input type=radio name=g value=12 checked form=nowhere>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (_, text, _) = ask(&mut session, &token, "look", &[]);
    let checked: Vec<bool> = (1..=12)
        .map(|n| line(&text, &format!("f{n}")).ends_with(" checked"))
        .collect();
    assert_eq!(
        checked,
        [
            false, true, // form a
            false, true, // form b, the second by form=b
            false, false, // no form owner, name g: f12 is the last of the group
            true,  // G is another name
            true, true, // no name: each alone
            true, true, // an empty name: each alone
            true, // form=nowhere: no form owner, the last of the group g
        ],
        "{text}"
    );
}

/// A handle names the element of the current page: after `go`, `f1` is the new page's control,
/// and the value filled on the page before is gone when that page is loaded again.
#[test]
fn a_handle_after_go_names_the_new_page_and_go_drops_a_filled_value() {
    let one = "<!doctype html><form><label>Name <input name=n value=ada></label></form>";
    let two = "<!doctype html><form><label><input type=checkbox name=c> Box</label></form>";
    let site = Site::start(vec![
        ("/form.html", HTML, one.as_bytes().to_vec()),
        ("/two.html", HTML, two.as_bytes().to_vec()),
    ]);
    let (mut session, token) = opened(&site);
    let (status, text, _) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", "grace")],
    );
    assert_eq!(status, 0, "{text}");
    let (status, text, _) = ask(
        &mut session,
        &token,
        "go",
        &[("url", &site.url("/two.html"))],
    );
    assert_eq!(status, 0, "{text}");
    let (status, text, typed) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", "x")],
    );
    assert_eq!(status, 1, "{text}");
    assert_eq!(typed.did, Some(Did::Refused(RefusalReason::WrongKind)));
    let (_, text, _) = ask(
        &mut session,
        &token,
        "go",
        &[("url", &site.url("/form.html"))],
    );
    assert_eq!(line(&text, "f1"), "  f1 text ? \"Name\" = \"ada\"");
}

/// A `fill`, then a refused `pick`: the value stays, the path is the one after the `fill`.
#[test]
fn a_refused_pick_after_a_fill_changes_nothing() {
    let page = "<!doctype html><form><label>Name <input name=n></label>\
                <label>Colour <select name=c><option>Red</option><option disabled>Blue</option>\
                </select></label></form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (_, filled, after_fill) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", "grace")],
    );
    for option in ["Purple", "Blue"] {
        let (status, text, typed) = ask(
            &mut session,
            &token,
            "pick",
            &[("handle", "f2"), ("option", option)],
        );
        assert_eq!(status, 1, "{text}");
        assert_eq!(typed.path, after_fill.path, "{text}");
        assert_eq!(typed.now, after_fill.now, "{filled}");
    }
}

/// `fill`, `pick` and `follow` on a closed session: `closed`, status 1, before any other check.
#[test]
fn fill_and_pick_on_a_closed_session_are_refused_closed() {
    let page = "<!doctype html><form><label>Name <input name=n></label></form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (status, text, _) = ask(&mut session, &token, "close", &[]);
    assert_eq!(status, 0, "{text}");
    for (command, args) in [
        ("fill", [("handle", "f1"), ("value", "x")]),
        ("fill", [("handle", "f99"), ("value", "x")]),
        ("pick", [("handle", "f1"), ("option", "on")]),
    ] {
        let (status, text, typed) = ask(&mut session, &token, command, &args);
        assert_eq!(status, 1, "{command}: {text}");
        assert_eq!(
            typed.did,
            Some(Did::Refused(RefusalReason::Closed)),
            "{text}"
        );
    }
}

/// Values with a NUL, a tab, U+0085, U+2028, U+2066 and the empty value:
/// each stays in its field on one line, printable ASCII, and reads back.
#[test]
fn values_with_control_characters_stay_in_their_field() {
    let page = "<!doctype html><form><label>Note <textarea name=t>x</textarea></label></form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    for value in ["\0", "\t", "\u{85}", "\u{2028}", "\u{2066}x", "", "\"\\"] {
        let (status, text, typed) = ask(
            &mut session,
            &token,
            "fill",
            &[("handle", "f1"), ("value", value)],
        );
        assert_eq!(status, 0, "{text}");
        assert!(text.is_ascii(), "{value:?}: {text}");
        let lines: Vec<&str> = text.lines().collect();
        let at = lines
            .iter()
            .position(|line| line.starts_with("now "))
            .expect("a now block");
        let expected = format!("  f1 textarea ? \"Note\" = {}", surf_answer::quote(value));
        assert_eq!(
            lines[at..],
            [
                "now 1 of 1",
                expected.as_str(),
                "note visibility: hidden attribute only"
            ],
            "{value:?}: {text}"
        );
        round_trip(&text);
        let Some(Target::State(state)) = typed
            .now
            .as_ref()
            .and_then(|now| now.elements.first())
            .map(|element| element.target.clone())
        else {
            panic!("no state: {text}");
        };
        assert_eq!(
            state.value,
            Some(Value::Text(surf_answer::Cut::of(
                value,
                surf_answer::VALUE_CHARS_MAX
            )))
        );
    }
}

/// The `now` block of a page of 70 textareas whose values quote to 16 bytes a character (U+10348,
/// 4 bytes of UTF-8) stays within [`NOW_BYTES_MAX`] and shows fewer lines than its total. The
/// fewest characters that fill the block: page text costs per character.
#[test]
fn a_now_block_of_long_values_stays_within_its_bound() {
    let value: String = std::iter::repeat_n('\u{10348}', 1_030).collect();
    let mut page = String::from("<!doctype html><meta charset=utf-8><form>");
    for _ in 0..70 {
        page.push_str(&format!("<textarea name=t>{value}</textarea>"));
    }
    page.push_str("</form>");
    let site = site(&page);
    let (mut session, token) = opened(&site);
    let (status, text, typed) = ask(&mut session, &token, "look", &[]);
    assert_eq!(status, 0);
    let now = typed.now.as_ref().expect("a now block");
    assert_eq!(now.total, 70);
    assert!(
        (40..70).contains(&now.elements.len()),
        "the bound cut the block: {} lines",
        now.elements.len()
    );
    let start = text.find("\nnow ").expect("a now line") + 1;
    let end = text.find("\nnote visibility").expect("the note") + 1;
    assert!(
        end - start <= NOW_BYTES_MAX,
        "{} bytes over {NOW_BYTES_MAX}",
        end - start
    );
}

/// The order of the refusals: `wrong-kind` before `disabled` and `hidden`, `disabled` before
/// `hidden`, `hidden` before `readonly` and `unknown-option`; a `file` control and a button are
/// `wrong-kind` for `fill`, a control is `wrong-kind` for `follow`, an option in a disabled
/// `optgroup` is `disabled`.
#[test]
fn refusals_come_in_their_order() {
    let page = "<!doctype html><form>\
        <input type=checkbox name=a disabled hidden>\
        <input name=b disabled hidden readonly>\
        <input name=c hidden readonly>\
        <select name=d hidden><option>One</option></select>\
        <input type=file name=e>\
        <select name=f><optgroup label=G disabled><option>In</option></optgroup>\
        <option>Out</option></select>\
        <input type=checkbox name=h>\
        </form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (_, look, before) = ask(&mut session, &token, "look", &[]);
    for (command, args, reason) in [
        (
            "fill",
            vec![("handle", "f1"), ("value", "x")],
            RefusalReason::WrongKind,
        ),
        (
            "fill",
            vec![("handle", "f2"), ("value", "x")],
            RefusalReason::Disabled,
        ),
        (
            "fill",
            vec![("handle", "f3"), ("value", "x")],
            RefusalReason::Hidden,
        ),
        (
            "pick",
            vec![("handle", "f4"), ("option", "None")],
            RefusalReason::Hidden,
        ),
        (
            "fill",
            vec![("handle", "f5"), ("value", "x")],
            RefusalReason::WrongKind,
        ),
        (
            "pick",
            vec![("handle", "f6"), ("option", "In")],
            RefusalReason::Disabled,
        ),
        ("follow", vec![("handle", "f7")], RefusalReason::WrongKind),
        (
            "pick",
            vec![("handle", "f7"), ("option", "On")],
            RefusalReason::UnknownOption,
        ),
    ] {
        let (status, text, typed) = ask(&mut session, &token, command, &args);
        assert_eq!(status, 1, "{command} {args:?}: {text}");
        assert_eq!(
            typed.did,
            Some(Did::Refused(reason)),
            "{command} {args:?}: {look}"
        );
        assert_eq!(typed.now, before.now, "{command} {args:?}");
        assert_eq!(typed.path, before.path, "{command} {args:?}");
    }
}

/// A password filled from the session is in no form of the page text (`look --as tree`,
/// `--as markdown`), in no answer of a refused command after it, and a line feed in it does
/// not show; after `go` the field is empty again.
#[test]
fn a_filled_password_is_in_no_page_text_and_go_drops_it() {
    const SECRET: &str = "s3cr3t-Pa55";
    let page = "<!doctype html><form><label>Pin <input type=password name=p></label>\
                <label>Code <input type=password name=q readonly></label></form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let mut texts = Vec::new();
    let (status, text, _) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", &format!("{SECRET}\n{SECRET}"))],
    );
    assert_eq!(status, 0, "{text}");
    assert!(line(&text, "f1").ends_with(" = (set)"), "{text}");
    texts.push(text);
    for (command, args) in [
        ("look", vec![("form", "tree")]),
        ("look", vec![("form", "markdown")]),
        ("look", vec![]),
        ("fill", vec![("handle", "f2"), ("value", SECRET)]),
        ("pick", vec![("handle", "f1"), ("option", SECRET)]),
        ("follow", vec![("handle", "f1")]),
    ] {
        let (_, text, _) = ask(&mut session, &token, command, &args);
        texts.push(text);
    }
    for text in &texts {
        assert!(!text.contains("s3cr3t"), "{text}");
    }
    let (_, text, _) = ask(
        &mut session,
        &token,
        "go",
        &[("url", &site.url("/form.html"))],
    );
    assert!(line(&text, "f1").ends_with(" = (empty)"), "{text}");
}

/// A password filled with the empty value shows `(empty)` again.
#[test]
fn a_password_filled_empty_shows_empty() {
    let page =
        "<!doctype html><form><label>Pin <input type=password name=p value=x></label></form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (_, text, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(line(&text, "f1"), "  f1 password ? \"Pin\" = (set)");
    let (status, text, _) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", "")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(line(&text, "f1"), "  f1 password ? \"Pin\" = (empty)");
}

/// `pick <checkbox> on` of a checked checkbox leaves it checked, and `off` of an unchecked one
/// leaves it unchecked: status 0, no action recorded.
#[test]
fn pick_of_a_checkbox_in_the_state_asked_leaves_it() {
    let page = "<!doctype html><form><label><input type=checkbox name=a checked> A</label>\
                <label><input type=checkbox name=b> B</label></form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (_, _, before) = ask(&mut session, &token, "look", &[]);
    for (handle, word, shown) in [
        ("f1", "on", "  f1 checkbox ? \"A\" checked"),
        ("f2", "off", "  f2 checkbox ? \"B\""),
    ] {
        let (status, text, typed) = ask(
            &mut session,
            &token,
            "pick",
            &[("handle", handle), ("option", word)],
        );
        assert_eq!(status, 0, "{text}");
        assert!(
            text.contains(&format!("\ndid pick \"{handle}\"\n")),
            "{text}"
        );
        assert_eq!(line(&text, handle), shown);
        assert_eq!(typed.path, before.path, "no action: {text}");
    }
}

/// `pick` looks for an option by its label first, then by its value: the word `A` is the label
/// of the first option and the value of the second.
#[test]
fn pick_takes_an_option_label_before_another_options_value() {
    let page = "<!doctype html><form><label>L <select name=s><option value=B>A</option>\
                <option value=A>B</option><option value=Z selected>C</option></select></label>\
                </form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (status, text, _) = ask(
        &mut session,
        &token,
        "pick",
        &[("handle", "f1"), ("option", "A")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(line(&text, "f1"), "  f1 select ? \"L\" = \"A\"");
}

/// A control hidden by `aria-hidden` or by a closed `details` has no `hid` on its line, and its
/// row is hidden: `fill` is refused `hidden`, the value unchanged.
#[test]
fn fill_of_a_control_hidden_without_the_hidden_attribute_is_refused_hidden() {
    let page = "<!doctype html><form>\
                <div aria-hidden=true><label>A <input name=a value=one></label></div>\
                <details><summary>More</summary><label>B <input name=b value=two></label></details>\
                </form>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (_, _, before) = ask(&mut session, &token, "look", &[]);
    for handle in ["f1", "f2"] {
        let (status, text, typed) = ask(
            &mut session,
            &token,
            "fill",
            &[("handle", handle), ("value", "x")],
        );
        assert_eq!(status, 1, "{handle}: {text}");
        assert_eq!(
            typed.did,
            Some(Did::Refused(RefusalReason::Hidden)),
            "{handle}: {text}"
        );
        assert_eq!(typed.now, before.now, "{handle}: {text}");
    }
}

/// `fill <handle> -` strips one line feed from standard input and only one: `ab` and two line
/// feeds is the value `ab` and a line feed (3 bytes); `ab`, a carriage return and a line feed is
/// `ab` and a carriage return (3 bytes).
#[test]
fn fill_from_standard_input_strips_exactly_one_line_feed() {
    use std::io::Write;
    use std::process::Stdio;

    let page = "<!doctype html><form><label>Note <textarea name=t></textarea></label></form>";
    let dir = crate::common::TestDir::new();
    let pages = crate::common::PageServer::with_body(page.as_bytes().to_vec());
    let daemon = crate::common::surfd_in_process(&dir, |_| {});
    let session = crate::common::agent_open(&dir, Some(&pages.url("/form.html")));
    for input in ["ab\n\n", "ab\r\n"] {
        let mut child = crate::common::agent(&dir)
            .args(["fill", "f1", "-"])
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
            .write_all(input.as_bytes())
            .expect("the value is written");
        let output = child.wait_with_output().expect("surf-agent ends");
        assert_eq!(output.status.code(), Some(0), "{input:?}");
    }
    let entries = daemon
        .session_entries(&session.id)
        .expect("the session is listed")
        .entries;
    let values: Vec<bool> = entries
        .iter()
        .filter(|entry| entry.text.starts_with("event surf.agent.FillRequested "))
        .map(|entry| entry.text.contains(" value=\"3 bytes\""))
        .collect();
    assert_eq!(values, [true, true], "{entries:#?}");
}

/// `docs/agent-answer.md`: `pick <handle> on` of a radio button "turns the others of its group
/// off". Two radio buttons with no form owner and one name are one group, as the parse-time rule
/// of this story groups them; a `pick` of the second leaves the first checked.
#[test]
fn pick_of_a_radio_without_a_form_unchecks_the_others_of_its_group() {
    let page = "<!doctype html><label><input type=radio name=g value=1 checked> One</label>\
                <label><input type=radio name=g value=2> Two</label>";
    let site = site(page);
    let (mut session, token) = opened(&site);
    let (status, text, _) = ask(
        &mut session,
        &token,
        "pick",
        &[("handle", "f2"), ("option", "on")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(line(&text, "f2"), "  f2 radio ? \"Two\" checked");
    assert_eq!(line(&text, "f1"), "  f1 radio ? \"One\"", "{text}");
}
