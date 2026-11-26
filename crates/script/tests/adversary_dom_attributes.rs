//! An adversary's tests of the slice `attributes` (`story:scripts-dom-attributes`): what the
//! unit's own tests leave open. Each test names the property it holds; a test that fails at the
//! unit's tip names the defect in its message.

use std::time::Duration;

use surf_html::limits::{ATTRIBUTE_BYTES, Limits as ParserLimits};
use surf_html::{Attribute, NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{
    Document, Mode, Runner, RunnerLimits, SESSION_ENGINE_LIMITS, ScriptEnd, run_document,
};
use swell::Limits;

/// The limits of these tests: a memory budget of 64 MiB and a `string_length` of 1 Mi units.
fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 10_000_000,
            memory_bytes: 64 * 1024 * 1024,
            frame_depth: 1_000,
            reentry_depth: 64,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 1_000_000_000,
        scripts_per_runner: 1_000,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(50),
    }
}

/// The engine's limits of a session, with the runner's own room of [`limits`].
fn session() -> RunnerLimits {
    RunnerLimits {
        engine: SESSION_ENGINE_LIMITS,
        ..limits()
    }
}

/// Runs `body` with a runner under `limits` over `page`, parsed under the default limits.
fn with_page<R: Send>(
    limits: RunnerLimits,
    page: String,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(
        limits,
        Mode::Test,
        move || Document {
            tree: parse_document(&page, &ParseOptions::default()),
            limits: ParserLimits::default(),
            slices: dom::slices(DocumentInfo::default()),
        },
        body,
    )
    .expect("a runner with the bindings")
}

/// The functions every script starts with.
const CHECK: &str = r#"
function check(holds, what) { if (!holds) { throw new Error(what); } }
function threw(f) { try { f(); } catch (e) { return e.name; } return "no throw"; }
var SVG = "http://www.w3.org/2000/svg";
"#;

/// Runs `source` and asserts it ran and recorded no limit.
fn ran(runner: &mut Runner, source: &str) {
    let end = runner.evaluate(&format!("{CHECK}{source}"));
    assert_eq!(end, ScriptEnd::Ran, "{end}");
    let limit = runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .and_then(|dom| dom.limit());
    assert_eq!(limit, None);
}

/// The bytes the stored handle holds, read between two scripts.
fn charged(runner: &mut Runner) -> u64 {
    runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .expect("the bindings")
        .stored_charged()
}

fn tree(runner: &mut Runner) -> &Tree {
    runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .expect("the bindings")
        .tree()
}

/// The first element of `tree` whose local name is `name`.
fn element(tree: &Tree, name: &str) -> NodeId {
    tree.descendants(tree.root())
        .into_iter()
        .find(|&node| matches!(tree.data(node), NodeData::Element { local_name, .. } if local_name == name))
        .expect("the element")
}

/// The attribute list of `node`, with its room.
fn list(tree: &Tree, node: NodeId) -> &Vec<Attribute> {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes,
        _ => panic!("an element"),
    }
}

/// A push into an attribute list the parser built takes room for twice its records; the bytes
/// the tree holds because of that one write are those records, not the one the tree counts.
/// The story's decision 8 says the factor of two covers the growth of a list; it covers a list
/// a script grew from empty, not a list the parser built at its exact length.
#[test]
fn a_push_into_a_list_the_parser_built_is_charged_for_the_room_it_takes() {
    let names: String = (0..512).map(|i| format!(" a{i}=\"\"")).collect();
    let page = format!("<html{names}><body></body></html>");
    with_page(session(), page, |runner| {
        ran(runner, "var e = document.documentElement;");
        let html = element(tree(runner), "html");
        let before = list(tree(runner), html).capacity();
        assert_eq!(list(tree(runner), html).len(), 512);
        ran(runner, "e.setAttribute(\"z\", \"\");");
        let after = list(tree(runner), html).capacity();
        let held = (after - before) * ATTRIBUTE_BYTES + 1;
        let charged = charged(runner);
        eprintln!(
            "capacity {before} -> {after}, held {held} bytes for the write, charged {charged}"
        );
        assert!(
            u64::try_from(held).unwrap() <= charged,
            "one setAttribute grew the parsed list from {before} to {after} records: the tree \
             holds {held} more bytes for it, the stored handle charges {charged}"
        );
    });
}

/// "Validate and extract" (DOM, 1.4) tests the prefix against "valid namespace prefix", which
/// admits `=` (only ASCII whitespace, NUL, `/` and `>` are out), and only the local name after
/// the first colon against "valid attribute local name". `setAttributeNS(SVG, "a=b:c", "1")`
/// stores the attribute `c` with the prefix `a=b`.
#[test]
fn a_prefix_may_hold_an_equals_sign() {
    let page = "<html><body></body></html>".to_string();
    with_page(limits(), page, |runner| {
        ran(
            runner,
            r#"
var e = document.documentElement;
var got = threw(function () { e.setAttributeNS(SVG, "a=b:c", "1"); });
check(got === "no throw", "setAttributeNS(SVG, \"a=b:c\", \"1\") threw " + got);
check(e.getAttributeNS(SVG, "c") === "1" && e.getAttribute("a=b:c") === "1", "the attribute");"#,
        );
    });
}
