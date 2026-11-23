//! Cases 1 to 16 of `story:agent-change-and-log`: the `changed` block of every answer and the
//! command `log`. Cases 1 to 11, 13, 14 and 16 drive the session core against a page server on
//! `127.0.0.1` the test starts ([`Site`]), case 12 against a transport in memory that answers
//! every address as asked; case 15 is the walk of the epic acceptance through the built
//! `surf-agent`, `surfd` and `surf-session`, `surfd` started by the walk's `open`, against a
//! [`Login`] server.

#![forbid(unsafe_code)]

use crate::common;
use crate::core::{Fetch, HTML, ID, Site, holder, open_at};

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::{TestDir, agent, agent_call, agent_open, opened, request, stdout};
use surf_answer::{Answer, Changed, Page, parse, quote, quote_cut, render};
use surf_session::Session;

/// The form of cases 1 to 6, 11 and 13: `f1` a text field, `f2` a password, `f3` a checkbox,
/// `f4` a textarea holding a line feed, `"` and U+202E, `f5` a select, `f6` and `f7` a radio
/// group with `f6` checked, `b1` the submit button.
const FORM: &str = "<!doctype html><title>Form</title><form action=\"/sent\" method=\"post\">\
<label>User <input name=\"user\"></label>\
<label>Password <input type=\"password\" name=\"pw\"></label>\
<label><input type=\"checkbox\" name=\"news\"> News</label>\
<label>Note <textarea name=\"note\">a&#10;b&quot;c&#x202e;d</textarea></label>\
<label>Colour <select name=\"colour\"><option selected>Red</option><option>Green</option></select></label>\
<label><input type=\"radio\" name=\"size\" value=\"s\" checked> Small</label>\
<label><input type=\"radio\" name=\"size\" value=\"m\"> Medium</label>\
<button>Send</button></form>";

/// The value of the textarea `f4` of [`FORM`]: a line feed, a double quote and U+202E.
const NOTE: &str = "a\nb\"c\u{202e}d";

/// What case 11 fills into `f4`: a line feed, a double quote and U+202E again.
const NOTE_AFTER: &str = "e\nf\"g\u{202e}h";

/// The page the cases of a new document start on, with a link to each page they go to.
const START: &str = "<!doctype html><title>Start</title><p>Alpha</p>\
<p><a href=\"/next\">Next</a> <a href=\"/many\">Many</a> <a href=\"/long\">Long</a> \
<a href=\"/forged\">Forged</a></p>";

/// The page of case 7: one text the start page holds and two it does not.
const NEXT: &str = "<!doctype html><title>Next</title><p>Alpha</p><p>Beta</p><p>Gamma</p>";

/// The text of case 9: 200 characters.
fn long_text() -> String {
    "0123456789".repeat(20)
}

/// The page of case 8: fifteen texts the start page does not hold.
fn many() -> String {
    let texts: String = (1..=15).map(|n| format!("<p>Text {n}</p>")).collect();
    format!("<!doctype html><title>Many</title>{texts}")
}

/// The page of case 10: a text with a line feed, a double quote and U+202E.
const FORGED: &str = "<!doctype html><title>Forged</title><p>a&#10;b&quot;c&#x202e;d</p>";

fn pages_site() -> Site {
    let long = format!("<!doctype html><title>Long</title><p>{}</p>", long_text());
    Site::start(vec![
        ("/start", HTML, START.as_bytes().to_vec()),
        ("/next", HTML, NEXT.as_bytes().to_vec()),
        ("/many", HTML, many().into_bytes()),
        ("/long", HTML, long.into_bytes()),
        ("/forged", HTML, FORGED.as_bytes().to_vec()),
        ("/form", HTML, FORM.as_bytes().to_vec()),
    ])
}

/// The reply to `command` with `args` as the holder: its status, its lines, its typed answer.
fn ask<T: surf::Transport>(
    session: &mut Session<T>,
    token: &str,
    command: &str,
    args: &[(&str, &str)],
) -> (u8, String, Answer) {
    let reply = session.answer(&holder(command, token, args));
    let link = reply.to_link();
    (link.status, link.text, reply.answer)
}

/// The lines of the `changed` block of `text`: the `changed` line and the indented lines after
/// it.
fn changed_block(text: &str) -> Vec<&str> {
    let mut lines = text
        .lines()
        .skip_while(|line| !line.starts_with("changed "));
    let Some(first) = lines.next() else {
        panic!("no changed block: {text}");
    };
    let mut block = vec![first];
    block.extend(lines.take_while(|line| line.starts_with("  ~ ") || line.starts_with("  + ")));
    block
}

/// `parse` of `text` gives `typed`, and `render` of it gives `text` byte for byte.
fn round_trip(text: &str, typed: &Answer) {
    let parsed = parse(text).unwrap_or_else(|error| panic!("{error}: {text}"));
    assert_eq!(&parsed, typed, "{text}");
    assert_eq!(render(&parsed).unwrap(), text);
}

/// Every byte of `text` is printable ASCII or a line feed.
fn printable(text: &str) {
    assert!(
        text.bytes()
            .all(|b| b == b'\n' || (b' '..=b'~').contains(&b)),
        "{text}"
    );
}

/// A session at the form of `site`, its token and the answer of a first `look`.
fn at_form(site: &Site) -> (Session<Fetch>, String) {
    let (mut session, token, _) = open_at(&site.url("/form"), &[]);
    let (status, text, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(status, 0, "{text}");
    (session, token)
}

/// A session at the start page of `site` that followed `handle`: the answer of the `follow`.
fn followed(site: &Site, handle: &str) -> (Session<Fetch>, String, String, Answer) {
    let (mut session, token, _) = open_at(&site.url("/start"), &[]);
    let (status, text, typed) = ask(&mut session, &token, "follow", &[("handle", handle)]);
    assert_eq!(status, 0, "{text}");
    (session, token, text, typed)
}

#[test]
fn case_01_fill_a_field_changes_its_value() {
    let site = pages_site();
    let (mut session, token) = at_form(&site);
    let (status, text, _) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", "ada")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        changed_block(&text),
        ["changed 1", "  ~ f1 value \"\" -> \"ada\""]
    );
}

#[test]
fn case_02_fill_a_password_changes_empty_to_set() {
    let site = pages_site();
    let (mut session, token) = at_form(&site);
    let (status, text, _) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f2"), ("value", "hunter2")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        changed_block(&text),
        ["changed 1", "  ~ f2 value (empty) -> (set)"]
    );
    assert!(!text.contains("hunter2"), "{text}");
}

#[test]
fn case_03_pick_a_checkbox_changes_checked() {
    let site = pages_site();
    let (mut session, token) = at_form(&site);
    let (status, text, _) = ask(
        &mut session,
        &token,
        "pick",
        &[("handle", "f3"), ("option", "on")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        changed_block(&text),
        ["changed 1", "  ~ f3 checked false -> true"]
    );
}

#[test]
fn case_04_pick_a_radio_button_changes_each_control_of_its_group() {
    let site = pages_site();
    let (mut session, token) = at_form(&site);
    let (status, text, _) = ask(
        &mut session,
        &token,
        "pick",
        &[("handle", "f7"), ("option", "on")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        changed_block(&text),
        [
            "changed 2",
            "  ~ f6 checked true -> false",
            "  ~ f7 checked false -> true"
        ]
    );
}

#[test]
fn case_05_pick_a_select_option_changes_selected() {
    let site = pages_site();
    let (mut session, token) = at_form(&site);
    let (status, text, _) = ask(
        &mut session,
        &token,
        "pick",
        &[("handle", "f5"), ("option", "Green")],
    );
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        changed_block(&text),
        ["changed 1", "  ~ f5 selected \"Red\" -> \"Green\""]
    );
}

#[test]
fn case_06_look_after_look_changes_nothing() {
    let site = pages_site();
    let (mut session, token) = at_form(&site);
    let (status, text, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(changed_block(&text), ["changed 0"]);
}

#[test]
fn case_07_a_new_document_shows_its_address_and_its_new_texts() {
    let site = pages_site();
    let (_, _, text, _) = followed(&site, "l1");
    assert_eq!(
        changed_block(&text),
        [
            format!(
                "changed page {} -> {}",
                quote(&site.url("/start")),
                quote(&site.url("/next"))
            )
            .as_str(),
            "  + text \"Beta\"",
            "  + text \"Gamma\"",
        ]
    );
}

#[test]
fn case_08_fifteen_new_texts_show_ten_and_the_count_of_the_rest() {
    let site = pages_site();
    let (_, _, text, _) = followed(&site, "l2");
    let block = changed_block(&text);
    let mut expected = vec![format!(
        "changed page {} -> {}",
        quote(&site.url("/start")),
        quote(&site.url("/many"))
    )];
    expected.extend((1..=10).map(|n| format!("  + text \"Text {n}\"")));
    expected.push("  + text more 5".to_string());
    assert_eq!(block, expected);
}

#[test]
fn case_09_a_text_of_200_characters_is_cut_to_120() {
    let site = pages_site();
    let (_, _, text, _) = followed(&site, "l3");
    let block = changed_block(&text);
    assert_eq!(block.len(), 2, "{text}");
    let expected = format!("  + text {}", quote_cut(&long_text(), 120));
    assert!(expected.ends_with("\"..."), "{expected}");
    assert_eq!(block[1], expected);
}

/// The body line of `look --as markdown` that holds `needle`, without its prefix: the line of
/// `surf-page-text` that a `+ text` line quotes.
fn markdown_line<T: surf::Transport>(
    session: &mut Session<T>,
    token: &str,
    needle: &str,
) -> String {
    let (status, text, _) = ask(session, token, "look", &[("form", "markdown")]);
    assert_eq!(status, 0, "{text}");
    text.lines()
        .filter_map(|line| line.strip_prefix("> "))
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no markdown line with {needle}: {text}"))
        .to_string()
}

#[test]
fn case_10_a_forged_text_stays_one_quoted_line() {
    let site = pages_site();
    let (mut session, token, text, typed) = followed(&site, "l4");
    printable(&text);
    let block = changed_block(&text);
    assert_eq!(block.len(), 2, "one text line, no other: {text}");
    let line = markdown_line(&mut session, &token, "c");
    assert_eq!(block[1], format!("  + text {}", quote(&line)));
    assert!(!block[1][10..block[1].len() - 1].contains('"'), "{text}");
    match &typed.changed {
        Some(Changed::Page { texts, .. }) => assert_eq!(texts[0].kept, line),
        other => panic!("not a changed page block: {other:?}"),
    }
    round_trip(&text, &typed);
}

/// The answer of case 11: `fill` of the textarea `f4` with [`NOTE_AFTER`].
fn forged_value(site: &Site) -> (String, Answer) {
    let (mut session, token) = at_form(site);
    let (status, text, typed) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f4"), ("value", NOTE_AFTER)],
    );
    assert_eq!(status, 0, "{text}");
    (text, typed)
}

#[test]
fn case_11_a_forged_textarea_value_stays_one_quoted_line() {
    let site = pages_site();
    let (text, typed) = forged_value(&site);
    printable(&text);
    assert_eq!(
        changed_block(&text),
        [
            "changed 1".to_string(),
            format!("  ~ f4 value {} -> {}", quote(NOTE), quote(NOTE_AFTER)),
        ]
    );
    round_trip(&text, &typed);
}

/// A transport in memory: every address answers status 200, `text/html` and one text, at the
/// address as asked.
fn as_asked(request: &surf::RequestStarted) -> surf::Answer {
    surf::Answer::Response {
        url: request.url.0.clone(),
        method: request.method,
        status: 200,
        media_type: "text/html".to_string(),
        body: b"<p>a page</p>".to_vec(),
    }
}

/// The address of the `page` line of `typed`.
fn page_address(typed: &Answer) -> &str {
    match &typed.page {
        Some(Page::Loaded { address, .. } | Page::Failed { address, .. }) => address,
        other => panic!("no address on the page line: {other:?}"),
    }
}

/// The two forged addresses of case 12.
const FIRST_ADDRESS: &str = "http://127.0.0.1:8000/a\nb\"c\u{202e}d";
const SECOND_ADDRESS: &str = "http://127.0.0.1:8000/e\nf\"g\u{202e}h";

/// The answer of case 12: `go` from the page of [`FIRST_ADDRESS`] to [`SECOND_ADDRESS`], and
/// the addresses of the two page lines.
fn forged_addresses() -> (String, String, String, Answer) {
    let transport: Fetch = as_asked;
    let mut session = Session::new(ID, transport).unwrap();
    let reply = session.answer(&request("open", None, None, &[("url", FIRST_ADDRESS)]));
    let link = reply.to_link();
    assert_eq!(link.status, 0, "{}", link.text);
    let token = opened(&link.text).token;
    let before = page_address(&reply.answer).to_string();
    let (status, text, typed) = ask(&mut session, &token, "go", &[("url", SECOND_ADDRESS)]);
    assert_eq!(status, 0, "{text}");
    let after = page_address(&typed).to_string();
    (before, after, text, typed)
}

#[test]
fn case_12_forged_addresses_stay_one_quoted_line() {
    let (before, after, text, typed) = forged_addresses();
    printable(&text);
    for (address, typed_address) in [(FIRST_ADDRESS, &before), (SECOND_ADDRESS, &after)] {
        assert!(
            typed_address.contains('"') && typed_address.contains('\u{202e}'),
            "the page line holds the address {address:?} as asked: {typed_address:?}"
        );
    }
    assert_eq!(
        changed_block(&text)[0],
        format!("changed page {} -> {}", quote(&before), quote(&after))
    );
    round_trip(&text, &typed);
}

/// The answers of one run of case 13 at `site`, the token replaced by `<token>`.
fn run_of_calls(site: &Site) -> Vec<String> {
    let (mut session, token, open) = open_at(&site.url("/start"), &[]);
    let mut texts = vec![open];
    let form = site.url("/form");
    for (command, args) in [
        ("follow", vec![("handle", "l1")]),
        ("back", vec![]),
        ("go", vec![("url", form.as_str())]),
        ("fill", vec![("handle", "f1"), ("value", "ada")]),
        ("pick", vec![("handle", "f7"), ("option", "on")]),
        ("pick", vec![("handle", "f5"), ("option", "Green")]),
        ("look", vec![]),
        ("log", vec![]),
    ] {
        let (status, text, _) = ask(&mut session, &token, command, &args);
        assert_eq!(status, 0, "{command}: {text}");
        texts.push(text);
    }
    texts
        .into_iter()
        .map(|text| text.replace(&token, "<token>"))
        .collect()
}

#[test]
fn case_13_the_same_calls_in_two_runs_give_the_same_output() {
    let site = pages_site();
    let first = run_of_calls(&site);
    let second = run_of_calls(&site);
    assert_eq!(first, second);
    assert!(
        first.iter().all(|text| text.contains("\nchanged ")),
        "{first:?}"
    );
}

/// The `path` block of `text`: its `path` line and its indented lines.
fn path_lines(text: &str) -> Vec<&str> {
    let mut lines = text.lines().skip_while(|line| !line.starts_with("path "));
    let first = lines.next().unwrap_or_else(|| panic!("no path: {text}"));
    let mut block = vec![first];
    block.extend(lines.take_while(|line| {
        line.starts_with("  ") && !line.starts_with("  ~ ") && !line.starts_with("  + ")
    }));
    block
}

/// A session at the start page of `site` with seven actions: `open`, `go`, `follow`, `back`,
/// `forward`, `go`, `follow`.
fn seven_actions(site: &Site) -> (Session<Fetch>, String) {
    let (mut session, token, _) = open_at(&site.url("/start"), &[]);
    let start = site.url("/start");
    for (command, args) in [
        ("follow", vec![("handle", "l1")]),
        ("back", vec![]),
        ("forward", vec![]),
        ("go", vec![("url", start.as_str())]),
        ("follow", vec![("handle", "l1")]),
    ] {
        let (status, text, _) = ask(&mut session, &token, command, &args);
        assert_eq!(status, 0, "{command}: {text}");
    }
    (session, token)
}

/// The answers of case 14: `log`, then `log 2`, after [`seven_actions`].
fn logs(site: &Site) -> (Session<Fetch>, String, [(String, Answer); 2]) {
    let (mut session, token) = seven_actions(site);
    let mut answers = [None, None];
    for (at, args) in [(0, vec![]), (1, vec![("count", "2")])] {
        let (status, text, typed) = ask(&mut session, &token, "log", &args);
        assert_eq!(status, 0, "{text}");
        answers[at] = Some((text, typed));
    }
    let [Some(all), Some(two)] = answers else {
        unreachable!("both answers are set")
    };
    (session, token, [all, two])
}

#[test]
fn case_14_log_lists_every_action_in_order_and_log_2_the_last_two() {
    let site = pages_site();
    let (mut session, token, [(text, typed), (two, two_typed)]) = logs(&site);
    let start = quote(&site.url("/start"));
    let all = [
        "path 7 of 7".to_string(),
        "  1 open".to_string(),
        format!("  2 go {start}"),
        "  3 follow l1 \"Next\"".to_string(),
        "  4 back".to_string(),
        "  5 forward".to_string(),
        format!("  6 go {start}"),
        "  7 follow l1 \"Next\"".to_string(),
    ];
    assert_eq!(path_lines(&text), all, "{text}");
    for text in [&text, &two] {
        assert!(!text.lines().any(|line| line.starts_with("did ")), "{text}");
        assert_eq!(changed_block(text), ["changed 0"], "{text}");
    }
    assert_eq!(
        path_lines(&two),
        ["path 2 of 7", all[6].as_str(), all[7].as_str()],
        "{two}"
    );
    round_trip(&text, &typed);
    round_trip(&two, &two_typed);

    let (status, text, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        path_lines(&text)[0],
        "path 5 of 7",
        "log records no action: {text}"
    );
    for count in ["0", "x", ""] {
        let (status, text, _) = ask(&mut session, &token, "log", &[("count", count)]);
        assert_eq!(status, 2, "log {count:?}: {text}");
        assert!(text.contains("\ndid refused usage\n"), "{text}");
    }
}

// --- Case 15: the walk through the built programs ---------------------------------------------

/// The page of links of the walk.
const HELP: &str = "<!doctype html><title>Help</title><p>How to sign in.</p>\
<p><a href=\"/login\">Sign in</a> <a href=\"/about\">About</a></p>";

/// The login form of the walk, with a link to the page of links.
const LOGIN: &str = "<!doctype html><title>Sign in</title><p><a href=\"/help\">Help</a></p>\
<form action=\"/session\" method=\"post\">\
<label>User <input name=\"user\"></label>\
<label>Password <input type=\"password\" name=\"pw\"></label>\
<button>Sign in</button></form>";

/// The password of the walk.
const PASSWORD: &str = "correct-horse-7";

/// An HTTP server on `127.0.0.1` with the walk's pages: `/login`, `/help`, and a `POST` to
/// `/session` answered with a page whose message names the user of the body, when the body
/// holds the walk's password. Stopped when dropped.
struct Login {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Login {
    fn start() -> Login {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    serve_login(stream);
                }
            }
        });
        Login {
            port,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for Login {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve_login(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 64 * 1024 {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            _ => return,
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let length: usize = head
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())?
        })
        .unwrap_or(0);
    let mut body = vec![0u8; length.min(64 * 1024)];
    if stream.read_exact(&mut body).is_err() {
        return;
    }
    let body = String::from_utf8_lossy(&body).into_owned();
    let line = head.lines().next().unwrap_or("");
    let page = match line.split(' ').take(2).collect::<Vec<_>>().as_slice() {
        ["GET", "/login"] => LOGIN.to_string(),
        ["GET", "/help"] => HELP.to_string(),
        ["POST", "/session"] if body.contains(&format!("pw={PASSWORD}")) => {
            let user = body
                .split('&')
                .find_map(|pair| pair.strip_prefix("user="))
                .unwrap_or("");
            format!("<!doctype html><title>Welcome</title><p>Signed in as {user}.</p>")
        }
        _ => "<!doctype html><title>No</title><p>Not here.</p>".to_string(),
    };
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: \
         close\r\n\r\n{page}",
        page.len()
    );
    let _ = stream.write_all(reply.as_bytes());
}

/// `surf-agent <words>` as the holder of `session`: its exit status and standard output.
fn walk(dir: &TestDir, session: &common::Opened, words: &[&str]) -> (Option<i32>, String) {
    let output = agent_call(dir, words, &session.id, &session.token);
    (output.status.code(), stdout(&output))
}

#[test]
fn case_15_the_walk_answers_press_with_the_new_page_and_its_message() {
    let dir = TestDir::new();
    let server = Login::start();
    let session = agent_open(&dir, None);
    let login = server.url("/login");
    for (words, did) in [
        (
            vec!["go", login.as_str()],
            format!("did go {}", quote(&login)),
        ),
        (vec!["follow", "l1"], "did follow \"l1\"".to_string()),
        (vec!["back"], "did back".to_string()),
        (vec!["fill", "f1", "ada"], "did fill \"f1\"".to_string()),
    ] {
        let (status, text) = walk(&dir, &session, &words);
        assert_eq!(status, Some(0), "{words:?}: {text}");
        assert!(text.lines().any(|line| line == did), "{words:?}: {text}");
    }
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
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    assert_eq!(
        changed_block(&stdout(&output)),
        ["changed 1", "  ~ f2 value (empty) -> (set)"]
    );

    let (status, pressed) = walk(&dir, &session, &["press", "b1"]);
    assert_eq!(status, Some(0), "{pressed}");
    assert_eq!(
        changed_block(&pressed),
        [
            format!(
                "changed page {} -> {}",
                quote(&login),
                quote(&server.url("/session"))
            )
            .as_str(),
            "  + text \"Signed in as ada.\"",
        ]
    );
    assert_eq!(
        path_lines(&pressed),
        [
            "path 5 of 7",
            "  3 follow l1 \"Help\"",
            "  4 back",
            "  5 fill f1",
            "  6 fill f2",
            "  7 press b1",
        ]
    );
    // Case 16 for the answer of `press`.
    let parsed = parse(&pressed).unwrap_or_else(|error| panic!("{error}: {pressed}"));
    assert_eq!(render(&parsed).unwrap(), pressed);

    let (status, log) = walk(&dir, &session, &["log"]);
    assert_eq!(status, Some(0), "{log}");
    assert_eq!(
        path_lines(&log),
        [
            "path 7 of 7".to_string(),
            "  1 open".to_string(),
            format!("  2 go {}", quote(&login)),
            "  3 follow l1 \"Help\"".to_string(),
            "  4 back".to_string(),
            "  5 fill f1".to_string(),
            "  6 fill f2".to_string(),
            "  7 press b1".to_string(),
        ]
    );
    for text in [&pressed, &log] {
        assert!(!text.contains(PASSWORD), "{text}");
    }

    let other = agent_open(&dir, None);
    let output = agent_call(&dir, &["look"], &session.id, &other.token);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(
        stdout(&output)
            .lines()
            .any(|line| line == "did refused token"),
        "{}",
        stdout(&output)
    );

    let (status, closed) = walk(&dir, &session, &["close"]);
    assert_eq!(status, Some(0), "{closed}");
    assert!(closed.lines().any(|line| line == "did close"), "{closed}");
    let (status, after) = walk(&dir, &session, &["look"]);
    assert_eq!(status, Some(1), "{after}");
    assert!(
        after.lines().any(|line| line == "did refused closed"),
        "{after}"
    );
}

/// The answer of `press` in the walk of case 15, on the session core: `go` to the login form,
/// `follow`, `back`, `fill` of the user and the password, `press`.
fn core_press(server: &Login) -> (String, Answer) {
    let (mut session, token, _) = open_at(&server.url("/login"), &[]);
    for (command, args) in [
        ("follow", vec![("handle", "l1")]),
        ("back", vec![]),
        ("fill", vec![("handle", "f1"), ("value", "ada")]),
        ("fill", vec![("handle", "f2"), ("value", PASSWORD)]),
    ] {
        let (status, text, _) = ask(&mut session, &token, command, &args);
        assert_eq!(status, 0, "{command}: {text}");
    }
    let (status, text, typed) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    (text, typed)
}

#[test]
fn case_16_the_answers_of_cases_10_to_12_14_and_15_round_trip() {
    let site = pages_site();
    let (_, _, forged_text, forged_text_typed) = followed(&site, "l4");
    let (forged_value, forged_value_typed) = forged_value(&site);
    let (_, _, forged_addresses, forged_addresses_typed) = forged_addresses();
    let (_, _, [(log, log_typed), (two, two_typed)]) = logs(&site);
    // The walk of case 15 parses and renders its own `press` answer through the built programs;
    // here the same steps on the session core give the typed answer to compare with.
    let server = Login::start();
    let (pressed, pressed_typed) = core_press(&server);
    assert!(
        pressed.contains("\n  + text \"Signed in as ada.\"\n"),
        "{pressed}"
    );
    for (text, typed) in [
        (forged_text, forged_text_typed),
        (forged_value, forged_value_typed),
        (forged_addresses, forged_addresses_typed),
        (log, log_typed),
        (two, two_typed),
        (pressed, pressed_typed),
    ] {
        assert!(typed.changed.is_some(), "{text}");
        round_trip(&text, &typed);
    }
}
