//! Cases 1 to 9 and 22 of `story:view-surfd-serves`, case 1 of `story:view-now-panel`
//! (`case_01_now_…`) and case 1 of `story:view-controls-and-changes` (`case_01_controls_…`): the
//! JSON writer of `surf-view` from model values, held to the fenced examples of
//! `docs/view-stream.md`, read from the document itself so that no copy can drift, and for the
//! controls to the golden files of `tests/golden/`, which the document's examples equal.

#![forbid(unsafe_code)]

use surf_view::model::{
    Action, Change, Changed, ChangedText, ControlChange, ControlState, ControlValue, Detail, Entry,
    EntryKind, Now, NowElement, PageLine, Path, PathAction, Reset, SessionList, SessionState,
    Submits, Summary,
};
use surf_view::to_json;

const A: &str = "5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69";
const B: &str = "0b7d4e21-9c3a-4f58-b6e2-7a1d9c8e5f30";
const WINDOW: &str = "2c4e6a80-1b3d-4f57-9a2c-4e6a8b0d2f41";
const TAB: &str = "7e9a1c3e-5a7c-4e91-b3d5-7f9b1d3f5a72";
const NAVIGATION: &str = "c1e3a5c7-e9b1-4d3f-8a5c-7e9b1d3f5a73";
const PAGE: &str = "http://127.0.0.1:41000/a.html";
const NOWHERE: &str = "http://127.0.0.1:9/";

// --- The document's fenced blocks -------------------------------------------------------------

fn document() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/view-stream.md");
    std::fs::read_to_string(path).expect("docs/view-stream.md")
}

/// Every fenced block of language `language`, in the document's order, each with the line feed
/// of its last line.
fn blocks(language: &str) -> Vec<String> {
    let text = document();
    let fence = format!("```{language}\n");
    let mut found = Vec::new();
    let mut rest = text.as_str();
    while let Some(start) = rest.find(&fence) {
        let after = &rest[start + fence.len()..];
        let end = after.find("\n```").expect("a closed fence");
        found.push(after[..end + 1].to_string());
        rest = &after[end + 4..];
    }
    found
}

/// The body of the answer example to `GET <target>`, cut at its `Content-Length`.
fn body_of(target: &str) -> Vec<u8> {
    let http = blocks("http");
    let at = http
        .iter()
        .position(|block| block.starts_with(&format!("GET {target} HTTP/1.1\n")))
        .expect("the request example");
    let answer = &http[at + 1];
    let (head, body) = answer.split_once("\n\n").expect("head and body");
    let length: usize = head
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .expect("Content-Length")
        .parse()
        .unwrap();
    body.as_bytes()[..length].to_vec()
}

/// The one-line `json` block that starts with `start`, without its line feed.
fn json_block(start: &str) -> Vec<u8> {
    let found: Vec<String> = blocks("json")
        .into_iter()
        .filter(|block| block.starts_with(start))
        .collect();
    assert_eq!(found.len(), 1, "one json block starting {start}");
    found[0].strip_suffix('\n').unwrap().as_bytes().to_vec()
}

/// The `data:` value of the `text` block of the stream event `event: <name>`.
fn stream_data(name: &str) -> Vec<u8> {
    let found: Vec<String> = blocks("text")
        .into_iter()
        .filter(|block| block.lines().nth(1) == Some(&format!("event: {name}")))
        .collect();
    assert!(!found.is_empty(), "a stream example of {name}");
    let data = found
        .last()
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .expect("a data line")
        .to_string();
    data.into_bytes()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("UTF-8")
}

// --- The run behind the examples --------------------------------------------------------------

fn loaded() -> PageLine {
    PageLine::Loaded {
        status: 200,
        media_type: "text/html".to_string(),
        address: PAGE.to_string(),
        title: None,
    }
}

fn unreachable() -> PageLine {
    PageLine::Failed {
        reason: "Unreachable".to_string(),
        address: NOWHERE.to_string(),
    }
}

fn summary_a() -> Summary {
    Summary {
        id: A.to_string(),
        state: SessionState::Open,
        address: Some(PAGE.to_string()),
        title: None,
        opened_at: 1_795_226_400_000,
        last_action_at: Some(1_795_226_401_500),
        ended_at: None,
        page: Some(loaded()),
    }
}

fn summary_b() -> Summary {
    Summary {
        id: B.to_string(),
        state: SessionState::Closed,
        address: Some(NOWHERE.to_string()),
        title: None,
        opened_at: 1_795_226_401_990,
        last_action_at: Some(1_795_226_403_000),
        ended_at: Some(1_795_226_403_010),
        page: Some(unreachable()),
    }
}

fn action(sequence: u64, command: &str, argument: Option<&str>) -> Action {
    Action {
        sequence: Some(sequence),
        command: Some(command.to_string()),
        argument: argument.map(str::to_string),
    }
}

/// One action of an answer's `path`, on no element.
fn path_action(sequence: u64, command: &str, argument: Option<&str>) -> PathAction {
    PathAction {
        sequence,
        command: command.to_string(),
        handle: None,
        argument: argument.map(str::to_string),
        argument_cut: false,
    }
}

fn entry(number: u64, session: &str, index: u64, kind: EntryKind) -> Entry {
    let base = if number <= 6 {
        1_795_226_400_009
    } else {
        1_795_226_401_493
    };
    Entry {
        number,
        session: session.to_string(),
        index,
        kept_at: base + number,
        kind,
    }
}

fn event(name: &str, fields: &[(&str, Option<&str>)]) -> EntryKind {
    EntryKind::Event {
        name: Some(name.to_string()),
        unmapped: false,
        fields: fields
            .iter()
            .map(|(name, value)| (name.to_string(), value.map(str::to_string)))
            .collect(),
    }
}

/// Entries 1 to 17 of the first session, as the example lists them.
fn entries_a() -> Vec<Entry> {
    let kinds = vec![
        EntryKind::Action(action(1, "open", None)),
        event("surf.shell.WindowOpened", &[("window_id", Some(WINDOW))]),
        event(
            "surf.shell.TabOpened",
            &[("tab_id", Some(TAB)), ("window_id", Some(WINDOW))],
        ),
        event(
            "surf.agent.SessionOpened",
            &[
                ("session_id", Some(A)),
                ("window_id", Some(WINDOW)),
                ("tab_id", Some(TAB)),
            ],
        ),
        event(
            "surf.agent.ActionRecorded",
            &[
                ("action_id", Some("e5a7c9e1-3b5d-4f71-9a3c-5e7a9c1e3b51")),
                ("session_id", Some(A)),
                ("sequence", Some("1")),
                ("command", Some("open")),
                ("url", None),
                ("handle", None),
                ("label", None),
            ],
        ),
        EntryKind::Answer {
            page: Some(PageLine::None),
            path: Some(Path {
                shown: 1,
                total: 1,
                actions: vec![path_action(1, "open", None)],
            }),
            now: None,
            changed: None,
        },
        EntryKind::Action(action(2, "go", Some(PAGE))),
        event(
            "surf.agent.NavigationRequested",
            &[
                ("session_id", Some(A)),
                ("tab_id", Some(TAB)),
                ("url", Some(PAGE)),
                ("handle", None),
                ("label", None),
                ("initiator", None),
            ],
        ),
        event(
            "surf.agent.ActionRecorded",
            &[
                ("action_id", Some("e5a7c9e1-3b5d-4f71-9a3c-5e7a9c1e3b52")),
                ("session_id", Some(A)),
                ("sequence", Some("2")),
                ("command", Some("go")),
                ("url", Some(PAGE)),
                ("handle", None),
                ("label", None),
            ],
        ),
        event(
            "surf.navigation.NavigationStarted",
            &[
                ("navigation_id", Some(NAVIGATION)),
                ("tab_id", Some(TAB)),
                ("url", Some(PAGE)),
                ("method", Some("Get")),
                ("body", None),
                ("body_type", None),
                ("initiator", None),
            ],
        ),
        event(
            "surf.agent.SessionNavigationChanged",
            &[
                ("navigation_id", Some(NAVIGATION)),
                ("state", Some("Started")),
                ("sessions", Some("1")),
            ],
        ),
        event(
            "surf.net.RequestStarted",
            &[
                ("request_id", Some(NAVIGATION)),
                ("url", Some(PAGE)),
                ("method", Some("Get")),
                ("body", None),
                ("body_type", None),
                ("initiator", None),
            ],
        ),
        response_received(),
        event(
            "surf.navigation.NavigationCommitted",
            &[
                ("navigation_id", Some(NAVIGATION)),
                ("tab_id", Some(TAB)),
                ("url", Some(PAGE)),
                ("status", Some("200")),
                ("media_type", Some("text/html")),
                ("body", Some("5000 bytes")),
            ],
        ),
        event(
            "surf.agent.SessionNavigationChanged",
            &[
                ("navigation_id", Some(NAVIGATION)),
                ("state", Some("Committed")),
                ("sessions", Some("1")),
            ],
        ),
        event(
            "surf.document.ParsingStarted",
            &[
                ("document_id", Some("d2f4b6d8-0a2c-4e4a-9b6d-8f0a2c4e6b84")),
                ("body", Some("5000 bytes")),
            ],
        ),
        answer_17(),
    ];
    kinds
        .into_iter()
        .enumerate()
        .map(|(at, kind)| entry(at as u64 + 1, A, at as u64 + 1, kind))
        .collect()
}

fn response_received() -> EntryKind {
    event(
        "surf.net.ResponseReceived",
        &[
            ("request_id", Some(NAVIGATION)),
            ("url", Some(PAGE)),
            ("method", Some("Get")),
            ("status", Some("200")),
            ("media_type", Some("text/html")),
            ("body", Some("5000 bytes")),
        ],
    )
}

fn answer_17() -> EntryKind {
    EntryKind::Answer {
        page: Some(loaded()),
        path: Some(Path {
            shown: 2,
            total: 2,
            actions: vec![
                path_action(1, "open", None),
                path_action(2, "go", Some(PAGE)),
            ],
        }),
        now: None,
        changed: None,
    }
}

// --- The cases --------------------------------------------------------------------------------

#[test]
fn case_01_the_session_list_is_the_documents_example() {
    let list = SessionList {
        newest: 38,
        sessions: vec![summary_a(), summary_b()],
    };
    let written = to_json(&list);
    let expected = body_of("/api/sessions");
    assert_eq!(expected.len(), 603, "the document's example");
    assert_eq!(text(&written), text(&expected));
}

#[test]
fn case_02_a_detail_whose_page_is_committed_is_the_documents_example() {
    let detail = Detail {
        session: summary_a(),
        dropped: 0,
        log_dropped: false,
        unreadable: 0,
        entries: entries_a(),
    };
    let written = to_json(&detail);
    let expected = body_of(&format!("/api/sessions/{A}"));
    assert_eq!(expected.len(), 5_629, "the document's example");
    assert_eq!(text(&written), text(&expected));
}

#[test]
fn case_03_a_detail_whose_page_failed_names_its_reason() {
    let answer = Entry {
        number: 38,
        session: B.to_string(),
        index: 21,
        kept_at: 1_795_226_403_004,
        kind: EntryKind::Answer {
            page: Some(unreachable()),
            path: Some(Path {
                shown: 3,
                total: 3,
                actions: vec![
                    path_action(1, "open", None),
                    path_action(2, "go", Some(NOWHERE)),
                    path_action(3, "close", None),
                ],
            }),
            now: None,
            changed: None,
        },
    };
    let detail = Detail {
        session: summary_b(),
        dropped: 20,
        log_dropped: false,
        unreadable: 0,
        entries: vec![answer],
    };
    let expected = concat!(
        r#"{"session":{"id":"0b7d4e21-9c3a-4f58-b6e2-7a1d9c8e5f30","state":"closed","#,
        r#""address":"http://127.0.0.1:9/","title":null,"opened_at":1795226401990,"#,
        r#""last_action_at":1795226403000,"ended_at":1795226403010,"#,
        r#""page":{"kind":"failed","reason":"Unreachable","address":"http://127.0.0.1:9/"}},"#,
        r#""dropped":20,"log_dropped":false,"unreadable":0,"entries":["#,
        r#"{"number":38,"session":"0b7d4e21-9c3a-4f58-b6e2-7a1d9c8e5f30","index":21,"#,
        r#""kept_at":1795226403004,"kind":"answer","#,
        r#""page":{"kind":"failed","reason":"Unreachable","address":"http://127.0.0.1:9/"},"#,
        r#""path":{"shown":3,"total":3,"actions":["#,
        r#"{"sequence":1,"command":"open","handle":null,"argument":null,"argument_cut":false},"#,
        r#"{"sequence":2,"command":"go","handle":null,"argument":"http://127.0.0.1:9/","#,
        r#""argument_cut":false},"#,
        r#"{"sequence":3,"command":"close","handle":null,"argument":null,"argument_cut":false}]},"#,
        r#""now":null,"changed":null}]}"#,
    );
    assert_eq!(text(&to_json(&detail)), expected);
}

#[test]
fn case_04_a_detail_before_the_first_page_has_the_page_none() {
    let mut session = summary_a();
    session.address = None;
    session.last_action_at = Some(1_795_226_400_010);
    session.page = Some(PageLine::None);
    let mut entries = entries_a();
    entries.retain(|entry| entry.number == 1 || entry.number == 6);
    let detail = Detail {
        session,
        dropped: 0,
        log_dropped: false,
        unreadable: 0,
        entries,
    };
    let expected = concat!(
        r#"{"session":{"id":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","state":"open","#,
        r#""address":null,"title":null,"opened_at":1795226400000,"#,
        r#""last_action_at":1795226400010,"ended_at":null,"page":{"kind":"none"}},"#,
        r#""dropped":0,"log_dropped":false,"unreadable":0,"entries":["#,
        r#"{"number":1,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":1,"#,
        r#""kept_at":1795226400010,"kind":"action","sequence":1,"command":"open","argument":null},"#,
        r#"{"number":6,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":6,"#,
        r#""kept_at":1795226400015,"kind":"answer","page":{"kind":"none"},"#,
        r#""path":{"shown":1,"total":1,"actions":[{"sequence":1,"command":"open","handle":null,"#,
        r#""argument":null,"argument_cut":false}]},"now":null,"changed":null}]}"#,
    );
    assert_eq!(text(&to_json(&detail)), expected);
}

#[test]
fn case_05_the_action_entry_7() {
    let written = to_json(&entries_a()[6]);
    let expected = json_block(r#"{"number":7,"#);
    assert_eq!(expected.len(), 182);
    assert_eq!(text(&written), text(&expected));
}

#[test]
fn case_06_the_event_entry_13_has_every_value_a_string() {
    let written = to_json(&entries_a()[12]);
    let expected = json_block(r#"{"number":13,"#);
    assert_eq!(expected.len(), 340);
    assert_eq!(text(&written), text(&expected));
    let written = text(&written);
    assert!(written.contains(r#""method":"Get""#));
    assert!(written.contains(r#""status":"200""#));
    assert!(written.contains(r#""body":"5000 bytes""#));
}

#[test]
fn case_07_the_answer_entry_17_has_its_status_a_number() {
    let written = to_json(&entries_a()[16]);
    let expected = json_block(r#"{"number":17,"#);
    assert_eq!(expected.len(), 489);
    assert_eq!(text(&written), text(&expected));
    assert!(text(&written).contains(r#""status":200,"#));
}

#[test]
fn case_08_the_detail_whose_log_was_dropped() {
    let mut session = summary_b();
    session.last_action_at = None;
    session.page = None;
    let detail = Detail {
        session,
        dropped: 21,
        log_dropped: true,
        unreadable: 0,
        entries: Vec::new(),
    };
    let written = to_json(&detail);
    let expected = json_block(r#"{"session":{"id":"0b7d4e21"#);
    assert_eq!(expected.len(), 264, "the example as corrected");
    assert_eq!(text(&written), text(&expected));
}

/// The characters of case 9, each with the escape the document gives it (`None`: written as
/// itself).
const MATRIX: [(char, Option<&str>); 13] = [
    ('"', Some("\\u0022")),
    ('\\', Some("\\u005c")),
    ('\u{0}', Some("\\u0000")),
    ('\u{1f}', Some("\\u001f")),
    ('\u{7f}', Some("\\u007f")),
    ('\u{80}', Some("\\u0080")),
    ('\u{9f}', Some("\\u009f")),
    ('\u{2028}', Some("\\u2028")),
    ('\u{2029}', Some("\\u2029")),
    ('<', Some("\\u003c")),
    ('>', Some("\\u003e")),
    ('&', Some("\\u0026")),
    ('é', None),
];

#[test]
fn case_09_the_string_matrix_reads_back_exactly() {
    let all: String = MATRIX.iter().map(|(character, _)| *character).collect();
    let mut values: Vec<String> = MATRIX
        .iter()
        .map(|(character, _)| format!("a{character}b"))
        .collect();
    values.push(all);
    for value in &values {
        let mut summary = summary_a();
        summary.title = Some(value.clone());
        summary.address = Some(value.clone());
        let event = Entry {
            number: 1,
            session: A.to_string(),
            index: 1,
            kept_at: 1,
            kind: event("surf.net.RequestStarted", &[("url", Some(value.as_str()))]),
        };
        for written in [to_json(&summary), to_json(&event)] {
            let written = text(&written);
            assert!(
                !written.chars().any(|c| matches!(
                    c,
                    '<' | '>' | '&' | '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}'
                )),
                "a raw character in {written:?}"
            );
            let read = json::parse(&written);
            let strings: Vec<&str> = if written.contains(r#""kind":"event""#) {
                vec![read.get("fields").get("url").str()]
            } else {
                vec![read.get("title").str(), read.get("address").str()]
            };
            for read_back in strings {
                assert_eq!(read_back, value, "in {written}");
            }
        }
    }
    for (character, escape) in MATRIX {
        let written = text(&to_json(character.to_string().as_str()));
        match escape {
            Some(escape) => assert_eq!(written, format!("\"{escape}\"")),
            None => assert_eq!(written, format!("\"{character}\"")),
        }
    }
    let one_value = json_block("\"a");
    assert_eq!(
        text(&to_json("a\"b\\c<d>&e\n\u{2028}é")),
        text(&one_value),
        "the one-value example, as corrected"
    );
    assert_eq!(
        text(&one_value),
        "\"a\\u0022b\\u005cc\\u003cd\\u003e\\u0026e\\u000a\\u2028é\""
    );
}

#[test]
fn case_22_the_data_of_the_session_and_reset_events() {
    let session = stream_data("session");
    assert_eq!(session.len(), 269);
    assert_eq!(text(&to_json(&summary_b())), text(&session));
    let reset = stream_data("reset");
    assert_eq!(text(&reset), r#"{"newest":38}"#);
    assert_eq!(text(&to_json(&Reset { newest: 38 })), text(&reset));
}

// --- story:view-now-panel, case 1 -------------------------------------------------------------

const LINKS: &str = "http://127.0.0.1:41000/now/links.html";
const C: &str = "3d5f7b91-2a4c-4e6f-8b0d-1f3a5c7e9b20";

fn link(handle: &str, visibility: &str, label: &str, target: &str, unresolved: bool) -> NowElement {
    NowElement {
        handle: handle.to_string(),
        kind: "link".to_string(),
        visibility: visibility.to_string(),
        label: Some(label.to_string()),
        label_cut: Some(false),
        target: Some(target.to_string()),
        target_cut: Some(false),
        unresolved: Some(unresolved),
        text: None,
        state: None,
    }
}

/// A line of a kind the view does not know, with its kind and its text only.
fn other(handle: &str, kind: &str, line: &str) -> NowElement {
    NowElement {
        handle: handle.to_string(),
        kind: kind.to_string(),
        visibility: "?".to_string(),
        label: None,
        label_cut: None,
        target: None,
        target_cut: None,
        unresolved: None,
        text: Some(line.to_string()),
        state: None,
    }
}

fn three_links() -> PageLine {
    PageLine::Loaded {
        status: 200,
        media_type: "text/html".to_string(),
        address: LINKS.to_string(),
        title: Some("Three links".to_string()),
    }
}

#[test]
fn case_01_now_the_answer_of_the_three_link_page_is_the_documents_example() {
    let answer = Entry {
        number: 14,
        session: C.to_string(),
        index: 14,
        kept_at: 1_795_226_500_014,
        kind: EntryKind::Answer {
            page: Some(three_links()),
            path: Some(Path {
                shown: 2,
                total: 2,
                actions: vec![
                    path_action(1, "open", None),
                    path_action(2, "go", Some(LINKS)),
                ],
            }),
            now: Some(Now {
                shown: 3,
                total: 3,
                elements: vec![
                    link("l1", "?", "Next page", "/now/next.html", false),
                    link("l2", "hid", "Members", "/now/members.html", false),
                    link("l3", "?", "Broken", "http://[", true),
                ],
                visibility_note: Some("hidden attribute only".to_string()),
            }),
            changed: None,
        },
    };
    let expected = json_block(r#"{"number":14,"#);
    assert_eq!(expected.len(), 1_101, "the document's example");
    assert_eq!(text(&to_json(&answer)), text(&expected));
}

#[test]
fn case_01_now_the_page_line_holds_the_title() {
    let expected = json_block(r#"{"kind":"loaded","#);
    assert_eq!(text(&to_json(&three_links())), text(&expected));
}

#[test]
fn case_01_now_a_path_action_on_an_element_has_its_handle() {
    let follow = PathAction {
        sequence: 3,
        command: "follow".to_string(),
        handle: Some("l1".to_string()),
        argument: Some("Next page".to_string()),
        argument_cut: false,
    };
    let expected = json_block(r#"{"sequence":3,"#);
    assert_eq!(text(&to_json(&follow)), text(&expected));
}

#[test]
fn case_01_now_a_control_line_and_a_button_line_go_out_with_kind_and_text() {
    let now = Now {
        shown: 2,
        total: 2,
        elements: vec![
            other("f1", "text", r#"f1 text ? "Email" "<b>x</b>" required"#),
            other("b1", "button", r#"b1 button ? "Sign in""#),
        ],
        visibility_note: None,
    };
    let expected = concat!(
        r#"{"shown":2,"total":2,"elements":["#,
        r#"{"handle":"f1","kind":"text","visibility":"?","label":null,"label_cut":null,"#,
        r#""target":null,"target_cut":null,"unresolved":null,"#,
        "\"text\":\"f1 text ? \\u0022Email\\u0022 \\u0022\\u003cb\\u003ex\\u003c/b\\u003e\\u0022 required\",\"state\":null},",
        r#"{"handle":"b1","kind":"button","visibility":"?","label":null,"label_cut":null,"#,
        r#""target":null,"target_cut":null,"unresolved":null,"#,
        "\"text\":\"b1 button ? \\u0022Sign in\\u0022\",\"state\":null}],",
        r#""visibility_note":null}"#,
    );
    assert_eq!(text(&to_json(&now)), expected);
}

// --- story:view-controls-and-changes, case 1 -------------------------------------------------

const FORMS: &str = "http://127.0.0.1:41000/forms/login.html";
const D: &str = "8e2b4d6f-1a3c-4e5f-9b7d-2c4e6f8a0b13";

/// The golden file `crates/view/tests/golden/<name>`, without the line feed it ends in.
fn golden(name: &str) -> String {
    let path = format!("{}/tests/golden/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    text.strip_suffix('\n')
        .expect("a line feed at its end")
        .to_string()
}

fn sign_in() -> PageLine {
    PageLine::Loaded {
        status: 200,
        media_type: "text/html".to_string(),
        address: FORMS.to_string(),
        title: Some("Sign in".to_string()),
    }
}

fn text_value(value: &str) -> ControlValue {
    ControlValue::Text {
        text: value.to_string(),
        cut: false,
    }
}

/// A control or button line of the login form, its line as the answer writes it.
fn control(handle: &str, kind: &str, label: &str, line: &str, state: ControlState) -> NowElement {
    NowElement {
        handle: handle.to_string(),
        kind: kind.to_string(),
        visibility: "?".to_string(),
        label: Some(label.to_string()),
        label_cut: Some(false),
        target: None,
        target_cut: None,
        unresolved: None,
        text: Some(line.to_string()),
        state: Some(state),
    }
}

fn flags(value: Option<ControlValue>, checked: bool, required: bool) -> ControlState {
    ControlState {
        value,
        disabled: false,
        checked,
        required,
        readonly: false,
        submits: None,
    }
}

/// The `now` of the login form (`crates/view-fixture/pages/forms/login.html`), the password
/// `(empty)` or `(set)`.
fn login_now(password: ControlValue) -> Now {
    let word = if password == ControlValue::Set {
        "(set)"
    } else {
        "(empty)"
    };
    let mut submit = flags(None, false, false);
    submit.submits = Some(Submits {
        form: "F1".to_string(),
        method: "post".to_string(),
        action: Some("/forms/answer.html".to_string()),
        action_cut: Some(false),
    });
    Now {
        shown: 5,
        total: 5,
        elements: vec![
            control(
                "f1",
                "text",
                "Name",
                r#"f1 text ? "Name" = """#,
                flags(Some(text_value("")), false, false),
            ),
            control(
                "f2",
                "password",
                "Password",
                &format!(r#"f2 password ? "Password" = {word} required"#),
                flags(Some(password), false, true),
            ),
            control(
                "f3",
                "checkbox",
                "Remember me",
                r#"f3 checkbox ? "Remember me" checked"#,
                flags(None, true, false),
            ),
            control(
                "f4",
                "select",
                "Role",
                r#"f4 select ? "Role" = "Editor""#,
                flags(Some(text_value("Editor")), false, false),
            ),
            control(
                "b1",
                "button",
                "Sign in",
                r#"b1 button ? "Sign in" submits F1 post "/forms/answer.html""#,
                submit,
            ),
        ],
        visibility_note: Some("hidden attribute only".to_string()),
    }
}

fn changed_text(text: &str) -> ChangedText {
    ChangedText {
        text: text.to_string(),
        cut: false,
    }
}

/// The answer entry of `go` to the login form.
fn forms_go_answer() -> Entry {
    Entry {
        number: 26,
        session: D.to_string(),
        index: 26,
        kept_at: 1_795_226_600_026,
        kind: EntryKind::Answer {
            page: Some(sign_in()),
            path: Some(Path {
                shown: 2,
                total: 2,
                actions: vec![
                    path_action(1, "open", None),
                    path_action(2, "go", Some(FORMS)),
                ],
            }),
            now: Some(login_now(ControlValue::Empty)),
            changed: Some(Changed::Page {
                from: None,
                to: Some(FORMS.to_string()),
                texts: vec![
                    changed_text("# Sign in"),
                    changed_text("Name [@f1 text: Name]"),
                    changed_text("Password [@f2 password required: Password]"),
                    changed_text("[@f3 checkbox checked: Remember me] Remember me"),
                    changed_text("Role [@f4 select: Role]"),
                    changed_text("[@b1 button: Sign in]"),
                ],
                more: 0,
            }),
        },
    }
}

/// The answer entry of `fill f2 -`, the password given on standard input.
fn forms_fill_answer() -> Entry {
    let fill = PathAction {
        sequence: 3,
        command: "fill".to_string(),
        handle: Some("f2".to_string()),
        argument: None,
        argument_cut: false,
    };
    Entry {
        number: 31,
        session: D.to_string(),
        index: 31,
        kept_at: 1_795_226_601_031,
        kind: EntryKind::Answer {
            page: Some(sign_in()),
            path: Some(Path {
                shown: 3,
                total: 3,
                actions: vec![
                    path_action(1, "open", None),
                    path_action(2, "go", Some(FORMS)),
                    fill,
                ],
            }),
            now: Some(login_now(ControlValue::Set)),
            changed: Some(Changed::Controls(vec![ControlChange {
                handle: "f2".to_string(),
                change: Change::Value {
                    before: ControlValue::Empty,
                    after: ControlValue::Set,
                },
            }])),
        },
    }
}

#[test]
fn case_01_controls_the_answer_of_the_form_page_equals_its_golden_file() {
    let written = text(&to_json(&forms_go_answer()));
    assert_eq!(written, golden("forms-go-answer.json"));
    assert_eq!(
        text(&json_block(r#"{"number":26,"session":"8e2b4d6f-"#)),
        written,
        "the document's example is the golden file"
    );
}

#[test]
fn case_01_controls_the_answer_after_a_fill_equals_its_golden_file() {
    let written = text(&to_json(&forms_fill_answer()));
    assert_eq!(written, golden("forms-fill-answer.json"));
    assert_eq!(
        text(&json_block(r#"{"number":31,"session":"8e2b4d6f-"#)),
        written,
        "the document's example is the golden file"
    );
    assert!(
        written.contains(r#""value":{"kind":"set","text":null,"cut":null}"#),
        "the password as the answer prints it: {written}"
    );
}

#[test]
fn case_01_controls_checked_and_selected_changes_and_a_cut_value() {
    let changed = Changed::Controls(vec![
        ControlChange {
            handle: "f3".to_string(),
            change: Change::Checked {
                before: true,
                after: false,
            },
        },
        ControlChange {
            handle: "f4".to_string(),
            change: Change::Selected {
                before: text_value("Editor"),
                after: ControlValue::Text {
                    text: "<Reader>".to_string(),
                    cut: true,
                },
            },
        },
    ]);
    let expected = concat!(
        r#"{"kind":"controls","lines":["#,
        r#"{"handle":"f3","what":"checked","before":true,"after":false},"#,
        r#"{"handle":"f4","what":"selected","#,
        r#""before":{"kind":"text","text":"Editor","cut":false},"#,
        r#""after":{"kind":"text","text":"\u003cReader\u003e","cut":true}}]}"#,
    );
    assert_eq!(text(&to_json(&changed)), expected);
    let unresolved = Submits {
        form: "F2".to_string(),
        method: "get".to_string(),
        action: None,
        action_cut: None,
    };
    assert_eq!(
        text(&to_json(&unresolved)),
        r#"{"form":"F2","method":"get","action":null,"action_cut":null}"#
    );
    let more = Changed::Page {
        from: Some(FORMS.to_string()),
        to: None,
        texts: Vec::new(),
        more: 3,
    };
    assert_eq!(
        text(&to_json(&more)),
        r#"{"kind":"page","from":"http://127.0.0.1:41000/forms/login.html","to":null,"texts":[],"more":3}"#
    );
}

/// A JSON reader for the tests: the values `surf-view` writes (no fraction, no exponent, no
/// sign), every escape of RFC 8259 read.
mod json {
    #[derive(Debug, PartialEq)]
    pub enum Value {
        Null,
        Bool(bool),
        Number(u64),
        Str(String),
        Array(Vec<Value>),
        Object(Vec<(String, Value)>),
    }

    impl Value {
        pub fn get(&self, name: &str) -> &Value {
            match self {
                Value::Object(members) => members
                    .iter()
                    .find(|(member, _)| member == name)
                    .map(|(_, value)| value)
                    .unwrap_or_else(|| panic!("no member {name}")),
                other => panic!("not an object: {other:?}"),
            }
        }

        pub fn str(&self) -> &str {
            match self {
                Value::Str(text) => text,
                other => panic!("not a string: {other:?}"),
            }
        }
    }

    pub fn parse(text: &str) -> Value {
        let chars: Vec<char> = text.chars().collect();
        let mut at = 0;
        let value = value(&chars, &mut at);
        assert_eq!(at, chars.len(), "text after the value");
        value
    }

    fn value(chars: &[char], at: &mut usize) -> Value {
        match chars[*at] {
            'n' => literal(chars, at, "null", Value::Null),
            't' => literal(chars, at, "true", Value::Bool(true)),
            'f' => literal(chars, at, "false", Value::Bool(false)),
            '"' => Value::Str(string(chars, at)),
            '[' => {
                *at += 1;
                let mut items = Vec::new();
                if chars[*at] == ']' {
                    *at += 1;
                    return Value::Array(items);
                }
                loop {
                    items.push(value(chars, at));
                    let next = chars[*at];
                    *at += 1;
                    match next {
                        ',' => {}
                        ']' => return Value::Array(items),
                        other => panic!("{other:?} in an array"),
                    }
                }
            }
            '{' => {
                *at += 1;
                let mut members = Vec::new();
                if chars[*at] == '}' {
                    *at += 1;
                    return Value::Object(members);
                }
                loop {
                    let name = string(chars, at);
                    assert_eq!(chars[*at], ':');
                    *at += 1;
                    members.push((name, value(chars, at)));
                    let next = chars[*at];
                    *at += 1;
                    match next {
                        ',' => {}
                        '}' => return Value::Object(members),
                        other => panic!("{other:?} in an object"),
                    }
                }
            }
            '0'..='9' => {
                let start = *at;
                while *at < chars.len() && chars[*at].is_ascii_digit() {
                    *at += 1;
                }
                let digits: String = chars[start..*at].iter().collect();
                Value::Number(digits.parse().expect("a number"))
            }
            other => panic!("{other:?} starts no value"),
        }
    }

    fn literal(chars: &[char], at: &mut usize, word: &str, value: Value) -> Value {
        let end = *at + word.chars().count();
        let found: String = chars[*at..end].iter().collect();
        assert_eq!(found, word);
        *at = end;
        value
    }

    fn hex4(chars: &[char], at: &mut usize) -> u32 {
        let digits: String = chars[*at..*at + 4].iter().collect();
        *at += 4;
        u32::from_str_radix(&digits, 16).expect("four hexadecimal digits")
    }

    fn string(chars: &[char], at: &mut usize) -> String {
        assert_eq!(chars[*at], '"');
        *at += 1;
        let mut out = String::new();
        loop {
            let c = chars[*at];
            *at += 1;
            match c {
                '"' => return out,
                '\\' => {
                    let escape = chars[*at];
                    *at += 1;
                    match escape {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let first = hex4(chars, at);
                            let code = if (0xd800..0xdc00).contains(&first) {
                                assert_eq!(chars[*at..*at + 2], ['\\', 'u']);
                                *at += 2;
                                let low = hex4(chars, at);
                                0x10000 + ((first - 0xd800) << 10) + (low - 0xdc00)
                            } else {
                                first
                            };
                            out.push(char::from_u32(code).expect("a scalar value"));
                        }
                        other => panic!("the escape \\{other}"),
                    }
                }
                c if u32::from(c) < 0x20 => panic!("a raw control character in a string"),
                c => out.push(c),
            }
        }
    }
}
