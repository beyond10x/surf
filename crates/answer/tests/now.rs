//! The blocks and lines of `story:agent-follow-links`: the `now` block and its link lines, `note
//! visibility: hidden attribute only`, the `path` line of an action on an element (`3 follow l1
//! "our offers"`), and the refusal reasons `unknown-handle`, `hidden` and `unresolved`. Each with
//! its golden file and its round trip, and what `render` refuses.

use surf_answer::{
    Answer, Cut, Did, Element, Head, LABEL_CHARS_MAX, Note, Now, Page, Path, PathEntry,
    RefusalReason, RenderError, SessionState, TARGET_CHARS_MAX, Target, Visibility, parse, render,
};

fn golden(name: &str) -> String {
    let path = format!("{}/tests/golden/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn assert_golden_and_round_trip(name: &str, answer: &Answer) {
    let rendered = render(answer).expect("the golden value renders");
    let text = golden(name);
    assert_eq!(rendered, text, "{name}");
    let parsed = parse(&text).expect("the golden file parses");
    assert_eq!(&parsed, answer, "{name}");
    assert_eq!(render(&parsed).unwrap(), text, "{name}");
}

fn shop() -> Answer {
    let mut answer = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Open,
    });
    answer.page = Some(Page::Loaded {
        status: 200,
        media_type: "text/html".to_string(),
        address: "http://shop.example/".to_string(),
        title: Some("Fruit shop".to_string()),
    });
    answer.path = Some(Path {
        total: 2,
        entries: vec![
            PathEntry {
                sequence: 1,
                command: "open".to_string(),
                handle: None,
                argument: None,
                argument_cut: false,
            },
            PathEntry {
                sequence: 2,
                command: "go".to_string(),
                handle: None,
                argument: Some("http://shop.example/".to_string()),
                argument_cut: false,
            },
        ],
    });
    answer
}

fn link(handle: &str, visibility: Visibility, label: &str, target: Target) -> Element {
    Element {
        handle: handle.to_string(),
        kind: "link".to_string(),
        visibility,
        label: Cut::of(label, LABEL_CHARS_MAX),
        target,
    }
}

fn resolved(target: &str) -> Target {
    Target::Resolved(Cut::of(target, TARGET_CHARS_MAX))
}

fn now_shop() -> Answer {
    let mut answer = shop();
    answer.now = Some(Now {
        total: 5,
        elements: vec![
            link("l1", Visibility::Unknown, "our offers", resolved("/offers")),
            link("l2", Visibility::Hidden, "Members", resolved("/members")),
            link(
                "l3",
                Visibility::Unknown,
                "Elsewhere",
                resolved("http://other.example/a"),
            ),
            link(
                "l4",
                Visibility::Unknown,
                "Broken",
                Target::Unresolved(Cut::of("http://[", TARGET_CHARS_MAX)),
            ),
            link(
                "l5",
                Visibility::Unknown,
                &"Sign in to read the archive of everything this site has ever said".repeat(2),
                resolved("/archive"),
            ),
        ],
    });
    answer.notes = vec![Note::Visibility];
    answer
}

fn follow_answer() -> Answer {
    let mut answer = shop();
    answer.did = Some(Did::Action {
        command: "follow".to_string(),
        argument: Some("l1".to_string()),
    });
    if let Some(path) = &mut answer.path {
        path.total = 4;
        path.entries.push(PathEntry {
            sequence: 3,
            command: "follow".to_string(),
            handle: Some("l1".to_string()),
            argument: Some("our offers".to_string()),
            argument_cut: false,
        });
        let long = Cut::of(&"x".repeat(200), LABEL_CHARS_MAX);
        path.entries.push(PathEntry {
            sequence: 4,
            command: "follow".to_string(),
            handle: Some("l12".to_string()),
            argument: Some(long.kept),
            argument_cut: long.cut,
        });
        path.entries.push(PathEntry {
            sequence: 5,
            command: "press".to_string(),
            handle: Some("b1".to_string()),
            argument: None,
            argument_cut: false,
        });
        path.total = 5;
    }
    answer
}

#[test]
fn golden_now() {
    let answer = now_shop();
    assert_golden_and_round_trip("now", &answer);
    let text = golden("now");
    assert!(text.contains("\nnow 5 of 5\n  l1 link ? \"our offers\" -> \"/offers\"\n"));
    assert!(text.contains("\n  l4 link ? \"Broken\" -> \"http://[\" unresolved\n"));
    assert!(text.ends_with("\nnote visibility: hidden attribute only\n"));
}

#[test]
fn golden_path_follow() {
    assert_golden_and_round_trip("path-follow", &follow_answer());
    let text = golden("path-follow");
    assert!(text.contains("\n  3 follow l1 \"our offers\"\n"));
    assert!(text.contains(&format!("\n  4 follow l12 \"{}\"...\n", "x".repeat(60))));
    assert!(text.contains("\n  5 press b1\n"));
}

#[test]
fn the_three_reasons_of_follow_read_back() {
    for (reason, word) in [
        (RefusalReason::UnknownHandle, "unknown-handle"),
        (RefusalReason::Hidden, "hidden"),
        (RefusalReason::Unresolved, "unresolved"),
    ] {
        assert_eq!(reason.word(), word);
        let text = format!("surf 1 session 7f3a open\ndid refused {word}\n");
        let parsed = parse(&text).expect("the reason parses");
        assert_eq!(parsed.did, Some(Did::Refused(reason)));
        assert_eq!(render(&parsed).unwrap(), text);
    }
}

/// The fix round of the adversary's cases on `agent-follow-links`: the three refusals of
/// `follow` before any navigation.
#[test]
fn the_three_reasons_follow_refuses_before_navigating_read_back() {
    for (reason, word) in [
        (RefusalReason::AddressTooLong, "address-too-long"),
        (RefusalReason::UnsupportedScheme, "unsupported-scheme"),
        (RefusalReason::ForbiddenDestination, "forbidden-destination"),
    ] {
        assert_eq!(reason.word(), word);
        let text = format!("surf 1 session 7f3a open\ndid refused {word}\n");
        let parsed = parse(&text).expect("the reason parses");
        assert_eq!(parsed.did, Some(Did::Refused(reason)));
        assert_eq!(render(&parsed).unwrap(), text);
    }
}

/// A `path` entry with a handle, no argument and `argument_cut` set cannot be written so that it
/// reads back: `render` refuses it.
#[test]
fn render_refuses_a_cut_mark_without_an_argument() {
    let mut answer = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Open,
    });
    answer.path = Some(Path {
        total: 1,
        entries: vec![PathEntry {
            sequence: 1,
            command: "follow".to_string(),
            handle: Some("l1".to_string()),
            argument: None,
            argument_cut: true,
        }],
    });
    assert_eq!(
        render(&answer),
        Err(RenderError::CutLength {
            field: "path label"
        })
    );
}

#[test]
fn a_now_block_that_shows_fewer_lines_than_its_total_reads_back() {
    let mut answer = now_shop();
    if let Some(now) = &mut answer.now {
        now.total = 400;
    }
    let text = render(&answer).unwrap();
    assert!(text.contains("\nnow 5 of 400\n"));
    assert_eq!(parse(&text).unwrap(), answer);
}

#[test]
fn render_refuses_a_now_block_it_could_not_read_back() {
    let refused = |change: &dyn Fn(&mut Now)| {
        let mut answer = now_shop();
        change(answer.now.as_mut().unwrap());
        render(&answer).expect_err("refused")
    };
    assert_eq!(
        refused(&|now| now.total = 1),
        RenderError::NowLongerThanTotal
    );
    assert!(matches!(
        refused(&|now| now.elements[0].handle = "l 1".to_string()),
        RenderError::NotAWord { .. }
    ));
    assert!(matches!(
        refused(&|now| now.elements[0].kind = "Link".to_string()),
        RenderError::NotAWord { .. }
    ));
    // A label marked cut that is not exactly its limit, and one over the limit not marked cut.
    assert!(matches!(
        refused(&|now| now.elements[0].label.cut = true),
        RenderError::CutLength { .. }
    ));
    assert!(matches!(
        refused(&|now| now.elements[0].label.kept = "y".repeat(61)),
        RenderError::CutLength { .. }
    ));
    assert!(matches!(
        refused(&|now| {
            now.elements[0].target = Target::Resolved(Cut {
                kept: "t".repeat(TARGET_CHARS_MAX + 1),
                cut: false,
            })
        }),
        RenderError::CutLength { .. }
    ));
}

#[test]
fn render_refuses_a_path_label_it_could_not_read_back() {
    let refused = |change: &dyn Fn(&mut PathEntry)| {
        let mut answer = follow_answer();
        change(&mut answer.path.as_mut().unwrap().entries[2]);
        render(&answer).expect_err("refused")
    };
    assert!(matches!(
        refused(&|entry| entry.handle = Some("L1".to_string())),
        RenderError::NotAWord { .. }
    ));
    assert!(matches!(
        refused(&|entry| entry.argument = Some("z".repeat(61))),
        RenderError::CutLength { .. }
    ));
    // A cut argument without a handle: an address is never cut.
    assert!(matches!(
        refused(&|entry| {
            entry.handle = None;
            entry.argument = Some("z".repeat(60));
            entry.argument_cut = true;
        }),
        RenderError::CutLength { .. }
    ));
}

#[test]
fn parse_refuses_lines_the_renderer_does_not_write() {
    let base = golden("now");
    for (from, to) in [
        ("now 5 of 5", "now 6 of 5"),
        ("now 5 of 5", "now 6 of 6"),
        ("  l1 link ? ", "  l1 link vis? "),
        ("  l1 link ? ", "  l1  link ? "),
        ("-> \"/offers\"", "->\"/offers\""),
        ("\"http://[\" unresolved", "\"http://[\" resolved"),
        (
            "note visibility: hidden attribute only",
            "note visibility: computed",
        ),
        ("\"our offers\"", "\"our offers\"..."),
    ] {
        let forged = base.replacen(from, to, 1);
        assert_ne!(forged, base, "{from}");
        assert!(parse(&forged).is_err(), "{to} was read");
    }
    // The block stands after `path` and before the notes, not elsewhere.
    let moved = base.replacen("note visibility: hidden attribute only\n", "", 1);
    let moved = moved.replacen(
        "now 5 of 5\n",
        "note visibility: hidden attribute only\nnow 5 of 5\n",
        1,
    );
    assert!(parse(&moved).is_err());
}
