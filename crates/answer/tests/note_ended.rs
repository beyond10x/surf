//! What `story:session-process-hardening` adds to the format: the forms of the `note ended`
//! line of a session that ended by a signal, each against its golden file and read back, the
//! one bound word and where the parser takes it, and the refusal `scripts-unavailable`.

use surf_answer::{
    Answer, Bound, Did, Head, Note, RefusalReason, RenderError, SessionState, parse, render,
};

fn golden(name: &str) -> String {
    let path = format!("{}/tests/golden/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn died(signal: &str, bound: Option<Bound>) -> Answer {
    let mut answer = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Died,
    });
    answer.did = Some(Did::Refused(RefusalReason::SessionDied));
    answer.notes = vec![Note::Ended {
        signal: signal.to_string(),
        bound,
    }];
    answer
}

#[test]
fn golden_note_ended_with_a_bound_and_without() {
    for (name, signal, bound) in [
        ("note-ended-bound", "SIGXCPU", Some(Bound::Cpu)),
        ("note-ended-no-bound", "SIGXCPU", None),
        ("note-ended", "SIGABRT", None),
    ] {
        let text = golden(name);
        assert_eq!(
            render(&died(signal, bound)).as_deref(),
            Ok(text.as_str()),
            "{name}"
        );
        let parsed = parse(&text).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(parsed, died(signal, bound), "{name}");
        assert_eq!(render(&parsed).as_deref(), Ok(text.as_str()), "{name}");
    }
}

#[test]
fn the_bound_word_is_cpu_after_sigxcpu_only() {
    assert_eq!(Bound::Cpu.word(), "cpu");
    assert_eq!(Bound::Cpu.signal(), "SIGXCPU");
    for signal in ["SIGXFSZ", "SIGKILL", "SIGABRT"] {
        let text = format!(
            "surf 1 session 7f3a died\ndid refused session-died\nnote ended: signal {signal}\n"
        );
        assert_eq!(render(&died(signal, None)).as_deref(), Ok(text.as_str()));
        assert_eq!(parse(&text), Ok(died(signal, None)));
        assert!(
            matches!(
                render(&died(signal, Some(Bound::Cpu))),
                Err(RenderError::NotAWord {
                    field: "note ended bound",
                    ..
                })
            ),
            "rendered a bound after {signal}"
        );
    }
}

#[test]
fn note_ended_refuses_another_bound_word_and_a_bound_out_of_place() {
    for line in [
        "note ended: signal SIGXCPU bound memory",
        "note ended: signal SIGXCPU bound as",
        "note ended: signal SIGXCPU bound CPU",
        "note ended: signal SIGXCPU bound",
        "note ended: signal SIGXCPU bound cpu ",
        "note ended: signal SIGXCPU  bound cpu",
        "note ended: signal SIGXCPU bound fsize",
        "note ended: signal SIGXFSZ bound fsize",
        "note ended: signal SIGXFSZ bound cpu",
        "note ended: signal SIGABRT bound cpu",
        "note ended: signal SIGABRT bound fsize",
        "note ended: signal sigabrt",
        "note ended: signal SIG",
        "note ended: signal ABRT",
        "note ended: signal SIG-ABRT",
        "note ended: signal  SIGABRT",
        "note ended: SIGABRT",
    ] {
        let text = format!("surf 1 session 7f3a died\ndid refused session-died\n{line}\n");
        assert!(parse(&text).is_err(), "parsed: {line:?}");
    }
}

#[test]
fn note_ended_renders_no_signal_name_outside_its_set() {
    for name in [
        "SIGabrt",
        "SIG",
        "ABRT",
        "SIGABRT\nnote text: no-css",
        "SIG ABRT",
    ] {
        assert!(
            matches!(
                render(&died(name, None)),
                Err(RenderError::NotAWord {
                    field: "note ended signal",
                    ..
                })
            ),
            "rendered: {name:?}"
        );
    }
}

#[test]
fn the_reason_scripts_unavailable_reads_back() {
    assert_eq!(
        RefusalReason::ScriptsUnavailable.word(),
        "scripts-unavailable"
    );
    let text = "surf 1 session 7f3a unknown\ndid refused scripts-unavailable\n";
    let parsed = parse(text).expect("the reason parses");
    assert_eq!(
        parsed.did,
        Some(Did::Refused(RefusalReason::ScriptsUnavailable))
    );
    assert_eq!(render(&parsed).as_deref(), Ok(text));
}
