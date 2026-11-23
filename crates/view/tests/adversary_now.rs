//! The adversary's cases against `story:view-now-panel`: the JSON writer of the `now` member
//! with values the examples of `docs/view-stream.md` never hold (every flag there is `false`
//! but one `unresolved`), each expected value written out.

use surf_view::model::{Now, NowElement};
use surf_view::to_json;

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("UTF-8")
}

/// A cut label beside an uncut target, and the other way round: each flag is written under its
/// own name.
#[test]
fn a_cut_label_and_a_cut_target_are_each_written_under_their_own_name() {
    let now = Now {
        shown: 2,
        total: 5,
        elements: vec![
            NowElement {
                handle: "l1".to_string(),
                kind: "link".to_string(),
                visibility: "vis".to_string(),
                label: Some("L".repeat(60)),
                label_cut: Some(true),
                target: Some("/a".to_string()),
                target_cut: Some(false),
                unresolved: Some(false),
                text: None,
                state: None,
            },
            NowElement {
                handle: "l2".to_string(),
                kind: "link".to_string(),
                visibility: "?".to_string(),
                label: Some("<b>&</b>".to_string()),
                label_cut: Some(false),
                target: Some("javascript:x".to_string()),
                target_cut: Some(true),
                unresolved: Some(true),
                text: None,
                state: None,
            },
        ],
        visibility_note: None,
    };
    let expected = format!(
        concat!(
            r#"{{"shown":2,"total":5,"elements":["#,
            r#"{{"handle":"l1","kind":"link","visibility":"vis","label":"{label}","#,
            r#""label_cut":true,"target":"/a","target_cut":false,"unresolved":false,"text":null,"state":null}},"#,
            r#"{{"handle":"l2","kind":"link","visibility":"?","#,
            r#""label":"{markup}","label_cut":false,"#,
            r#""target":"javascript:x","target_cut":true,"unresolved":true,"text":null,"state":null}}],"#,
            r#""visibility_note":null}}"#,
        ),
        label = "L".repeat(60),
        // `<b>&</b>` as the writer escapes it, each `!` a backslash.
        markup = "!u003cb!u003e!u0026!u003c/b!u003e".replace('!', r"\"),
    );
    assert_eq!(text(&to_json(&now)), expected);
}
