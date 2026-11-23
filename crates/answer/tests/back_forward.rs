//! What `story:agent-back-forward` adds to the format: the three refusal reasons of `back` and
//! `forward`, `did back` and `did forward`, and their `path` lines, each read back to its typed
//! value and rendered to the same bytes.

use surf_answer::{Did, RefusalReason, parse, render};

#[test]
fn the_three_reasons_of_back_and_forward_read_back() {
    for (reason, word) in [
        (RefusalReason::AtStart, "at-start"),
        (RefusalReason::AtEnd, "at-end"),
        (RefusalReason::PostEntry, "post-entry"),
    ] {
        assert_eq!(reason.word(), word);
        let text = format!("surf 1 session 7f3a open\ndid refused {word}\n");
        let parsed = parse(&text).expect("the reason parses");
        assert_eq!(parsed.did, Some(Did::Refused(reason)));
        assert_eq!(render(&parsed).unwrap(), text);
    }
}

#[test]
fn did_and_path_lines_of_back_and_forward_read_back() {
    for command in ["back", "forward"] {
        let text = format!(
            "surf 1 session 7f3a open\ndid {command}\npage none\npath 2 of 2\n  1 open\n  2 \
             {command}\n"
        );
        let parsed = parse(&text).unwrap_or_else(|e| panic!("{e:?}: {text}"));
        assert_eq!(
            parsed.did,
            Some(Did::Action {
                command: command.to_string(),
                argument: None,
            })
        );
        let path = parsed.path.as_ref().expect("a path block");
        assert_eq!(path.entries[1].command, command);
        assert_eq!(path.entries[1].argument, None);
        assert_eq!(render(&parsed).unwrap(), text);
    }
}
