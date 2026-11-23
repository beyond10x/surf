//! Adversarial cases on `story:agent-follow-links` in the format: every answer `render` writes
//! reads back through `parse` to the same value (decision D20), or `render` refuses it.

use surf_answer::{Answer, Head, Path, PathEntry, SessionState, parse, render};

fn with_path_entry(entry: PathEntry) -> Answer {
    let mut answer = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Open,
    });
    answer.path = Some(Path {
        total: 1,
        entries: vec![entry],
    });
    answer
}

/// A `path` entry with a handle, no label and `argument_cut` set: `render` writes `  1 follow
/// l1`, which reads back with `argument_cut` false. `RenderError::CutLength` names "a `path`
/// argument marked cut on a line without a handle"; the same mark without an argument passes.
#[test]
fn adversary_follow_a_cut_mark_without_a_label_is_refused_or_reads_back() {
    let answer = with_path_entry(PathEntry {
        sequence: 1,
        command: "follow".to_string(),
        handle: Some("l1".to_string()),
        argument: None,
        argument_cut: true,
    });
    if let Ok(text) = render(&answer) {
        let parsed = parse(&text).unwrap_or_else(|error| panic!("{error}: {text}"));
        assert_eq!(
            parsed, answer,
            "render wrote {text:?}, which reads back otherwise"
        );
    }
}
