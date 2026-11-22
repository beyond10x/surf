//! What `story:agent-press-submit` adds to the format: the seven refusal reasons of `press`,
//! and the `submits` part of a button line. Each reads back to its typed value and renders the
//! same bytes; what `render` refuses and the lines `parse` refuses.

use surf_answer::{
    Answer, Cut, Did, Element, Head, Now, RefusalReason, RenderError, SessionState, State, Submits,
    TARGET_CHARS_MAX, Target, Visibility, parse, render,
};

#[test]
fn the_seven_reasons_of_press_read_back() {
    for (reason, word) in [
        (RefusalReason::NoDefaultAction, "no-default-action"),
        (RefusalReason::PasswordInQuery, "password-in-query"),
        (RefusalReason::NoFormOwner, "no-form-owner"),
        (RefusalReason::InvalidControls, "invalid-controls"),
        (RefusalReason::InvalidAction, "invalid-action"),
        (RefusalReason::UnsupportedEnctype, "unsupported-enctype"),
        (RefusalReason::DialogMethod, "dialog-method"),
    ] {
        assert_eq!(reason.word(), word);
        let text = format!("surf 1 session 7f3a open\ndid refused {word}\n");
        let parsed = parse(&text).expect("the reason parses");
        assert_eq!(parsed.did, Some(Did::Refused(reason)));
        assert_eq!(render(&parsed).unwrap(), text);
    }
}

fn button(handle: &str, label: &str, disabled: bool, submits: Option<Submits>) -> Element {
    Element {
        handle: handle.to_string(),
        kind: "button".to_string(),
        visibility: Visibility::Unknown,
        label: Cut::of(label, 60),
        target: Target::State(State {
            disabled,
            submits,
            ..State::default()
        }),
    }
}

fn submits(form: &str, method: &str, action: &str, unresolved: bool) -> Option<Submits> {
    Some(Submits {
        form: form.to_string(),
        method: method.to_string(),
        action: (!unresolved).then(|| Cut::of(action, TARGET_CHARS_MAX)),
    })
}

fn buttons(elements: Vec<Element>) -> Answer {
    Answer {
        head: Head {
            session: "7f3a".to_string(),
            state: SessionState::Open,
        },
        token: None,
        did: Some(Did::Action {
            command: "look".to_string(),
            argument: None,
        }),
        page: None,
        path: None,
        now: Some(Now {
            total: elements.len() as u64,
            elements,
        }),
        notes: Vec::new(),
        text: None,
    }
}

fn four() -> Answer {
    buttons(vec![
        button(
            "b1",
            "Sign in",
            false,
            submits("F1", "post", "/session", false),
        ),
        button("b2", "Later", true, submits("F12", "get", "/a\"b\n", false)),
        button(
            "b3",
            "Bad",
            false,
            submits("F2", "dialog", "http://[x", true),
        ),
        button(
            "b4",
            "Long",
            false,
            submits("F3", "get", &"/x".repeat(TARGET_CHARS_MAX), false),
        ),
        button("b5", "Plain", false, None),
    ])
}

/// The part after the flags: the form's handle, the method, and the action quoted and cut at
/// 1,024 characters, or `unresolved` alone for an action that does not parse.
#[test]
fn a_submits_part_renders_and_reads_back() {
    let answer = four();
    let text = render(&answer).expect("renders");
    for line in [
        "  b1 button ? \"Sign in\" submits F1 post \"/session\"\n",
        "  b2 button ? \"Later\" disabled submits F12 get \"/a\\x22b\\x0a\"\n",
        "  b3 button ? \"Bad\" submits F2 dialog unresolved\n",
        "  b5 button ? \"Plain\"\n",
    ] {
        assert!(text.contains(line), "{line:?} in\n{text}");
    }
    let long = format!(
        "  b4 button ? \"Long\" submits F3 get \"{}\"...\n",
        "/x".repeat(TARGET_CHARS_MAX / 2)
    );
    assert!(text.contains(&long), "{text}");
    let parsed = parse(&text).expect("parses");
    assert_eq!(parsed, answer);
    assert_eq!(render(&parsed).unwrap(), text);
}

#[test]
fn render_refuses_a_submits_part_it_could_not_read_back() {
    for (form, method) in [
        ("F0", "post"),
        ("F01", "post"),
        ("f1", "post"),
        ("F", "post"),
        ("F1x", "post"),
        ("F1", "POST"),
        ("F1", ""),
        ("F1", "po st"),
    ] {
        let answer = buttons(vec![button(
            "b1",
            "Go",
            false,
            submits(form, method, "/x", false),
        )]);
        assert!(
            matches!(render(&answer), Err(RenderError::NotAWord { .. })),
            "{form} {method}: {:?}",
            render(&answer)
        );
    }
    let mut answer = four();
    if let Target::State(State {
        submits: Some(part),
        ..
    }) = &mut answer.now.as_mut().unwrap().elements[0].target
    {
        part.action = Some(Cut {
            kept: "/x".to_string(),
            cut: true,
        });
    }
    assert!(matches!(
        render(&answer),
        Err(RenderError::CutLength { .. })
    ));
}

#[test]
fn submits_lines_the_renderer_does_not_write_are_refused() {
    let base = render(&four()).unwrap();
    for (from, to) in [
        ("submits F1 post", "submits F0 post"),
        ("submits F1 post", "submits F01 post"),
        ("submits F1 post", "submits f1 post"),
        ("submits F1 post", "submits F1 POST"),
        ("submits F1 post", "submits F1  post"),
        ("submits F1 post", "submits  F1 post"),
        ("submits F1 post", "submits F1"),
        ("\"/session\"\n", "/session\n"),
        ("\"/session\"\n", "\"/session\" unresolved\n"),
        ("dialog unresolved\n", "dialog unresolved unresolved\n"),
        ("dialog unresolved\n", "dialog \"http://[x\" unresolved\n"),
        ("dialog unresolved\n", "dialog unresolved \"/y\"\n"),
        ("\"/session\"\n", "\"/session\" resolved\n"),
        ("\"/session\"\n", "\"/session\" submits F2 get \"/y\"\n"),
        ("\"/session\"\n", "\"/session\" disabled\n"),
        (
            "\"Later\" disabled submits",
            "\"Later\" submits F1 get \"/z\" disabled submits",
        ),
    ] {
        let forged = base.replacen(from, to, 1);
        assert_ne!(forged, base, "{from}");
        assert!(parse(&forged).is_err(), "{to} was read");
    }
}
