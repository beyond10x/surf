//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`): a hostile page that
//! chooses the markup and every call. Each test names the claim it holds; the failing ones show
//! a defect of the unit at `ace7176`.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeId, ParseOptions, Tree, WriteError, parse_document};
use surf_script::bindings::{DomCx, DomEnd, DomException, DomResult, Mutation, Slices};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{Document, Mode, Runner, RunnerLimits, ScriptEnd, run_document};
use surf_url::Url;
use swell::Limits;

const ADDRESS: &str = "https://example.com/dir/page";

fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 10_000_000,
            memory_bytes: 64 * 1024 * 1024,
            frame_depth: 1_000,
            reentry_depth: 64,
            parser_nesting: 1_000,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 1_000_000_000,
        scripts_per_runner: 100,
        source_bytes: 1 << 22,
        wall_clock: Duration::from_secs(50),
    }
}

fn info() -> DocumentInfo {
    DocumentInfo {
        address: Url::parse(ADDRESS, None).expect("the address"),
        content_type: "text/html".to_string(),
        encoding: "UTF-8".to_string(),
    }
}

/// The test's `mutation` slice: `appendChild`, `removeChild` and `createDocumentFragment`
/// through the tree's public write surface (decision 14).
struct TestMutation;

fn refused(error: WriteError) -> DomEnd {
    DomEnd::Throw(DomException::new(
        "HierarchyRequestError",
        format!("{error:?}"),
    ))
}

impl Mutation for TestMutation {
    fn node_append_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
    ) -> DomResult<NodeId> {
        cx.tree_mut().append_child(this, node).map_err(refused)?;
        Ok(node)
    }

    fn node_remove_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        child: NodeId,
    ) -> DomResult<NodeId> {
        if cx.tree().parent(child) != Some(this) {
            return Err(DomEnd::Throw(DomException::new(
                "NotFoundError",
                "not a child",
            )));
        }
        cx.tree_mut().remove_node(child).map_err(refused)?;
        Ok(child)
    }

    fn document_create_document_fragment(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<NodeId> {
        cx.tree_mut().create_document_fragment().map_err(refused)
    }
}

fn slices(info: DocumentInfo) -> Slices {
    let mut slices = dom::slices(info);
    slices.mutation = Box::new(TestMutation);
    slices
}

fn document(tree: Tree, info: DocumentInfo) -> Document {
    Document {
        tree,
        limits: ParserLimits::default(),
        slices: slices(info),
    }
}

fn with_page<R: Send>(
    limits: RunnerLimits,
    page: String,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(
        limits,
        Mode::Test,
        move || document(parse_document(&page, &ParseOptions::default()), info()),
        body,
    )
    .expect("a runner with the bindings")
}

fn recorded(runner: &mut Runner) -> Option<surf_html::limits::Limit> {
    runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .and_then(|dom| dom.limit())
}

/// Runs `source` and asserts it ran and recorded no limit.
fn ran(runner: &mut Runner, source: &str) {
    let end = runner.evaluate(source);
    assert_eq!(end, ScriptEnd::Ran, "{end}");
    assert_eq!(recorded(runner), None);
}

fn steps(runner: &mut Runner, source: &str) -> u64 {
    let before = runner.steps();
    ran(runner, source);
    runner.steps() - before
}

const CHECK: &str = r#"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
function threw(f) { try { f(); } catch (e) { return e.name; } return "no throw"; }
function walk(p) { var out = []; for (var x = p.firstChild; x; x = x.nextSibling) out.push(x); return out; }
function elements(p) { var out = []; for (var x = p.firstElementChild; x; x = x.nextElementSibling) out.push(x); return out; }
function same(c, list, what) {
  check(c.length === list.length, what + ": length " + c.length + " against " + list.length);
  for (var i = 0; i < list.length; i++) check(c[i] === list[i] && c.item(i) === list[i], what + ": index " + i);
  for (var k in c) { if (String(Number(k)) === k && Number(k) >= list.length) check(false, what + ": index " + k + " past the length"); }
  check(c[list.length] === undefined, what + ": an index past the length");
}
"#;

/// A refill that a script's own non-configurable index stops half way, then a write that
/// shortens the list and a second refill: no index past the new length may answer a node
/// (brief 9; `refresh_collection` deletes only the indices of the old data, which it changes
/// last).
#[test]
fn a_failed_refill_leaves_no_node_past_the_length() {
    let page = "<div id=\"p\"><b></b></div><div id=\"q\"><i></i><i></i><i></i><i></i></div>";
    with_page(limits(), page.to_string(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var p = document.getElementById("p"), q = document.getElementById("q");
var c = p.childNodes;
check(c.length === 1, "one child");
Object.defineProperty(c, "3", {{ value: "x", writable: false, configurable: false }});
for (var i = 0; i < 4; i++) p.appendChild(q.firstChild);
var first = threw(function () {{ p.childNodes; }});
while (p.lastChild !== p.firstChild) q.appendChild(p.lastChild);
var second = threw(function () {{ return p.childNodes; }});
check(p.childNodes === c, "the kept object");
check(c.length === 1, "one child again: " + c.length);
check(c[1] === undefined && c[2] === undefined,
  "an index past the length answers a node: c[1] " + c[1] + ", c[2] " + c[2] +
  " (refills: " + first + ", " + second + ")");
"#
            ),
        );
    });
}

/// A kept collection made non-extensible: a refill that cannot define its new indices must not
/// leave an object whose `length` and `item` answer the old list while its indices are gone
/// (brief 9: "must not answer the old view").
#[test]
fn a_kept_collection_never_answers_two_views() {
    let page = "<div id=\"p\"><b></b></div><div id=\"q\"><i></i></div>";
    with_page(limits(), page.to_string(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var p = document.getElementById("p"), q = document.getElementById("q");
var c = p.childNodes;
var b = c[0];
Object.preventExtensions(c);
p.appendChild(q.firstChild);
var end = threw(function () {{ p.childNodes; }});
var view = [];
for (var i = 0; i < c.length; i++) view.push(c[i] === c.item(i));
check(view.every(function (v) {{ return v; }}),
  "after a refill that ended " + end + ": length " + c.length + ", c[0] " + c[0] +
  ", item(0) is the old first child: " + (c.item(0) === b));
"#
            ),
        );
    });
}

/// A single call of `getElementsByClassName` compares every wanted token with every class of an
/// element for one step per element: a script runs past a wall clock of 1 s within a few
/// hundred steps (brief 12: a must when the work outruns the steps past the wall clock).
#[test]
fn get_elements_by_class_name_costs_what_it_charges() {
    let tokens = 6_000;
    let class = format!("{}a", "b ".repeat(tokens));
    let wanted = "a ".repeat(tokens);
    let page = format!("<div id=\"d\" class=\"{class}\"></div>");
    let mut quick = limits();
    quick.wall_clock = Duration::from_secs(1);
    let (spent, took, end) = with_page(quick, page, |runner| {
        let source = format!("var found = document.getElementsByClassName({wanted:?});");
        let before = runner.steps();
        let started = Instant::now();
        let end = runner.evaluate(&source);
        (runner.steps() - before, started.elapsed(), end)
    });
    eprintln!(
        "getElementsByClassName of {tokens} wanted against {tokens} classes: {spent} steps, \
         {took:?}, {end}"
    );
    assert!(
        took < Duration::from_secs(1),
        "one call of {spent} steps ran {took:?} past a wall clock of 1 s ({end})"
    );
}

/// `documentElement` and `doctype` scan the document's children: the steps of one read grow
/// with the comments before `html` (brief 12, decision 7: a member whose work grows with the
/// tree charges one step per node it visits).
#[test]
fn document_element_and_doctype_charge_their_scan() {
    let read = |comments: usize, source: &'static str| {
        let page = format!(
            "{}<html><body></body></html>{}",
            "<!---->".repeat(comments),
            "<!---->".repeat(comments)
        );
        with_page(limits(), page, move |runner| {
            let started = Instant::now();
            let spent = steps(runner, source);
            (spent, started.elapsed())
        })
    };
    let mut lines = Vec::new();
    let mut flat = Vec::new();
    for source in [
        "for (var i = 0; i < 100; i++) document.documentElement;",
        "for (var i = 0; i < 100; i++) document.doctype;",
        "for (var i = 0; i < 100; i++) document.lookupNamespaceURI(null);",
    ] {
        let (few, few_time) = read(0, source);
        let (many, many_time) = read(100_000, source);
        lines.push(format!(
            "{source}: 0 comments {few} steps {few_time:?}; 2 x 100,000 comments {many} steps {many_time:?}"
        ));
        if many < few + 100 * 100_000 {
            flat.push(source);
        }
    }
    eprintln!("{}", lines.join("\n"));
    assert!(
        flat.is_empty(),
        "a scan of 100,000 children charged no step: {flat:?}"
    );
}

/// DOM "locate a namespace": a DocumentType and a DocumentFragment answer null for every
/// prefix, `xml` and `xmlns` included; so does a document without a document element.
#[test]
fn locate_a_namespace_from_a_node_without_an_element() {
    with_page(limits(), "<!DOCTYPE html><p></p>".to_string(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var f = document.createDocumentFragment();
var wrong = [];
[["doctype", document.doctype], ["fragment", f]].forEach(function (pair) {{
  ["xml", "xmlns"].forEach(function (prefix) {{
    var got = pair[1].lookupNamespaceURI(prefix);
    if (got !== null) wrong.push(pair[0] + ".lookupNamespaceURI(" + prefix + ") = " + got);
  }});
}});
check(wrong.length === 0, wrong.join("; "));
"#
            ),
        );
    });
}

/// HTML "set the frozen base URL", step 3: a `base` whose `href` parses to a URL with the
/// scheme `data` or `javascript` gives the fallback base URL.
#[test]
fn a_data_or_javascript_base_gives_the_fallback() {
    for href in ["data:text/html,x", "javascript:void(0)"] {
        let page = format!("<base href=\"{href}\"><p id=\"t\"></p>");
        with_page(limits(), page, |runner| {
            ran(
                runner,
                &format!(
                    r#"if (document.baseURI !== {ADDRESS:?}) throw new Error({href:?} + " gives " + document.baseURI);"#
                ),
            );
        });
    }
}

/// One parent read from both ends, in turn: each read is a sibling of the node last read about
/// the same parent's other end, so the eight hints miss each time. Linear reads keep 5,000
/// children under 15 times the steps of 500.
#[test]
fn a_parent_read_from_both_ends_stays_linear() {
    let walk = r#"var p = document.getElementById("p");
var a = p.firstChild, z = p.lastChild, n = 0;
while (a && z && a !== z && a.previousSibling !== z) { a = a.nextSibling; z = z.previousSibling; n++; }"#;
    let run = |count: usize| {
        let page = format!("<p id=\"p\">{}</p>", "<b></b>".repeat(count));
        with_page(limits(), page, |runner| {
            let before = runner.steps();
            let end = runner.evaluate(walk);
            (runner.steps() - before, end)
        })
    };
    let started = Instant::now();
    let (small, small_end) = run(500);
    let (large, large_end) = run(5_000);
    eprintln!(
        "both ends: 500 children {small} steps ({small_end}), 5,000 children {large} steps \
         ({large_end}), {:?}",
        started.elapsed()
    );
    assert_eq!(small_end, ScriptEnd::Ran);
    assert_eq!(
        large_end,
        ScriptEnd::Ran,
        "a walk over 5,000 children from both ends"
    );
    assert!(large < 15 * small, "{large} >= 15 x {small}");
}

/// DOM "substring data" counts UTF-16 code units: an offset inside a surrogate pair answers the
/// lone half; surf answers U+FFFD, which selection.txt row `CharacterData substringData` does
/// not name as a deviation.
#[test]
fn substring_data_splitting_a_pair() {
    with_page(
        limits(),
        "<p id=\"p\">a\u{1F600}</p>".to_string(),
        |runner| {
            ran(
                runner,
                &format!(
                    r#"{CHECK}
var t = document.getElementById("p").firstChild;
check(t.length === 3, "length " + t.length);
var high = t.substringData(1, 1), low = t.substringData(2, 1);
check(high === "\ud83d" && low === "\ude00",
  "halves " + high.charCodeAt(0).toString(16) + " " + low.charCodeAt(0).toString(16));
"#
                ),
            );
        },
    );
}
