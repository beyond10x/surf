//! Adversarial cases of the sixth review of `story:html-script-hook`, against fix round 5
//! (ruling D423): the step limit is read before each entry of every loop over the stack of open
//! elements, its scope boundaries and the list of active formatting elements, before every
//! write of the parser to the tree and before a token is processed again; a loop the limit
//! ends answers "not found"; an adoption agency iteration the limit ends puts the clones it
//! made back at the place of step 4.14 and stops before step 4.24. A parse ends fewer than
//! 4 x `max_nodes` steps past `max_steps` (`src/limits.rs`, `max_steps`). Each test names what
//! it holds the unit to.
//! The case below fails on the tip it was written against (`5d9b011`).

use surf_html::limits::{Limit, Limits};
use surf_html::{ParseOptions, parse_document};

/// Ruling D423 of fix round 5: "A loop that ends at the limit leaves the tree a tree ... and
/// the parse ends with stopped_by Steps, as when the limit is read after a token"; claim 3: "A
/// loop ended at the limit answers 'not found'". The `html` and `body` start tags of "in body"
/// add their attributes only when no `template` is on the stack; `template_on_stack` scans the
/// stack from its first entry, so a `template` near the top is the last entry it reaches. When
/// the limit ends that scan it answers "no template", and `merge_attributes` checks the
/// attribute limit before it reads the step limit (in `admit`): an `html` that already holds
/// 1,024 attributes (the default limit) then stops the parse at `Attributes`, a limit the page
/// never reaches, since with the `template` open the tag is ignored. The whole parse ends with
/// no limit; every parse that stops should name `Steps`. On `5d9b011` the limits inside that
/// scan name `Attributes`, for `<html>` and for `<body>`.
#[test]
fn a_scan_ended_at_the_step_limit_names_no_other_limit() {
    let full: String = (0..1024).map(|i| format!(" a{i}=v")).collect();
    let options = |max_steps| ParseOptions {
        scripting_disabled: true,
        limits: Limits {
            max_steps,
            ..Limits::default()
        },
    };
    let mut wrong = Vec::new();
    for (page, what) in [
        (
            format!("<html{full}>{}<template><html b=2>", "<div>".repeat(40)),
            "html",
        ),
        (
            format!("<body{full}>{}<template><body b=2>", "<div>".repeat(40)),
            "body",
        ),
    ] {
        let whole = parse_document(&page, &options(usize::MAX));
        assert_eq!(whole.stopped_by(), None, "{what}");
        let mut named = Vec::new();
        for max in 0..whole.steps() {
            let tree = parse_document(&page, &options(max));
            if tree.stopped_by() != Some(Limit::Steps) {
                named.push((max, tree.stopped_by()));
            }
        }
        if !named.is_empty() {
            wrong.push(format!(
                "{what}: {} limits of {} name another limit, first {:?}",
                named.len(),
                whole.steps(),
                named[0]
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
