//! Adversarial cases of the fifth review of `story:html-script-hook`, against fix round 4: the
//! work on the stack of open elements is counted as steps (`settle_entries`, the removals and
//! the insertion of `OpenElements`, the `frameset` rule); the removal, insertion and
//! replacement of an entry inside the stack check with a `debug_assert` that no boundary of
//! "has an element in scope" is removed, replaced or passed. Each test names what it holds the
//! unit to: ruling 1 of fix round 4 ("the step limit bounds the work of a parse"), the crate
//! documentation's "Limits" and the documentation of `src/limits.rs`.

use surf_html::limits::Limits;
use surf_html::{
    NodeData, NodeId, NotRun, ParseOptions, ScriptHook, Tree, parse_document_with_hook,
};

/// A hook that moves the script's parent, the current node, to the end of the `body`: the
/// elements opened after it nest under it from there, so a page keeps any number of entries on
/// the stack of open elements and no element deeper than `PERIOD` and a few. Before there is
/// a `body`, or for a write the tree refuses, it does nothing.
struct Lift;

impl ScriptHook for Lift {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        let Some(parent) = tree.parent(script) else {
            return;
        };
        let Some(&html) = tree.children(tree.root()).first() else {
            return;
        };
        let body = tree.children(html).iter().copied().find(|&c| {
            matches!(tree.data(c), NodeData::Element { local_name, .. } if local_name == "body")
        });
        if let Some(body) = body {
            let _ = tree.append_child(body, parent);
        }
    }

    fn skip(&mut self, _: &Tree, _: NodeId, _: NotRun) {}
}

/// The elements between two hook runs.
const PERIOD: usize = 480;

/// `count` start tags `<name>`, a script after each `PERIOD`th; `count` is a multiple of
/// `PERIOD`, so the last tag's element is lifted.
fn open(name: &str, count: usize) -> String {
    assert_eq!(count % PERIOD, 0);
    let mut s = String::new();
    for i in 0..count {
        s.push('<');
        s.push_str(name);
        s.push('>');
        if (i + 1) % PERIOD == 0 {
            s.push_str("<script>x</script>");
        }
    }
    s
}

fn with_steps(max_steps: usize) -> ParseOptions {
    ParseOptions {
        scripting_disabled: false,
        limits: Limits {
            max_steps,
            ..Limits::default()
        },
    }
}

// The cases from here to the next such line fail on the tip they were written against
// (`dc175e9`).

/// Ruling 1 of fix round 4: "the step limit bounds the work of a parse"; the crate
/// documentation, "Limits": every limit "ends a parse with a defined result; none of them is a
/// hang". The step count is checked after each token, so one token's work must not grow
/// faster than the stack. The inner loop of the adoption agency algorithm removes each node
/// between the formatting element and the furthest block that has no entry in the list
/// (4.18.5), one at a time: each removal moves every entry above it and visits every scope
/// boundary (`OpenElements::remove`, counted since this round), and each iteration scans the
/// list of active formatting elements for the node (`active_position`). With a hook that keeps
/// the stack long (`Lift`), one `</b>` over N `span`, a `div` and M `span` costs N x M steps,
/// and one over K `object` below the `b` (a boundary and a marker each) costs 2 x N x K. In
/// release with the default limits
/// (2^27 steps), N = M = 105,600 (1.3 MB) ends at `Steps` with 11,246,297,508 steps, 84 times
/// the limit, after 26.9 s; the page without the `</b>` ends whole with 93,247,883 steps in
/// 0.46 s. Here N = M = K = 4,800 with a limit of 1,000,000, which the pages without the
/// `</b>` stay under: the parse ends inside twice the limit, whole or at `Steps`. On
/// `dc175e9` the two end with 23,323,368 and 46,339,328 steps, after about 206,500 without
/// the `</b>`.
#[test]
fn one_end_tag_of_the_adoption_agency_stays_near_the_step_limit() {
    const N: usize = 4_800;
    const MAX: usize = 1_000_000;
    let pages = [
        (
            "spans above the furthest block",
            format!("<body><b>{}<div>{}", open("span", N), open("span", N)),
        ),
        (
            "objects below the formatting element",
            format!("<body>{}<b>{}<div>", open("object", N), open("span", N)),
        ),
    ];
    let mut wrong = Vec::new();
    for (what, page) in pages {
        let before = parse_document_with_hook(&page, &with_steps(MAX), &mut Lift);
        assert_eq!(
            before.stopped_by(),
            None,
            "{what}: the page without the end tag"
        );
        assert!(
            before.steps() < MAX,
            "{what}: {} steps without the end tag",
            before.steps()
        );
        let tree = parse_document_with_hook(&format!("{page}</b>"), &with_steps(MAX), &mut Lift);
        if tree.steps() > 2 * MAX {
            wrong.push(format!(
                "{what}: {} steps before the end tag, {} after it, stopped by {:?}",
                before.steps(),
                tree.steps(),
                tree.stopped_by()
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
