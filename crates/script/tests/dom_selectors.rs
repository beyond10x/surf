//! The tests of the slice `selectors` (`story:scripts-dom-selectors`): `querySelector`,
//! `querySelectorAll`, `closest`, `matches` and `webkitMatchesSelector` over a parsed tree, in a
//! runner with the generated bindings, for the subset of Selectors Level 4 the slice holds.
//!
//! Each row of `crates/webidl/selection.txt` with the slice `selectors` has entries in [`CASES`],
//! each a JavaScript expression and the value the standards give it, compared with `===` in a
//! runner of its own over [`PAGE`]. The other tests hold what a single value does not show: the
//! scope of each member, the case rules, the one exception and its two messages, the bounds,
//! the lexical rules, the table of attribute values compared without case, the reads and their
//! charges, and the cost of each member on its worst shape (the module [`cost`]).
//!
//! The slices are `dom::slices(info)` with the slice `mutation` replaced by [`TestMutation`],
//! which writes the tree through its public write surface: `appendChild`, `removeChild` and
//! `createDocumentFragment`, so that a case reaches a fragment and a test writes between two
//! reads.

use std::collections::BTreeSet;
use std::time::Duration;

use surf_html::limits::Limits as ParserLimits;
use surf_html::{Namespace, NodeData, NodeId, ParseOptions, Tree, WriteError, parse_document};
use surf_script::bindings::{
    DomCx, DomEnd, DomException, DomResult, EXCEPTION_TEXT_BYTES, Mutation, Slices,
};
use surf_script::dom::{self, DocumentInfo, Frames, SurfSelectors};
use surf_script::{Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, run_document};
use swell::Limits;

fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 10_000_000,
            memory_bytes: 64 * 1024 * 1024,
            frame_depth: 1_000,
            reentry_depth: 64,
            parser_nesting: 100,
            string_length: 1 << 22,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 1_000_000_000,
        scripts_per_runner: 100,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(50),
    }
}

fn refused(error: WriteError) -> DomEnd {
    DomEnd::Throw(DomException::new(
        "HierarchyRequestError",
        format!("{error:?}"),
    ))
}

/// The test's `mutation` slice: the writes the tests make between two reads, through the tree's
/// public write surface. With `filled`, `createDocumentFragment` answers a fragment that already
/// holds that many `b` elements of that many attributes each, built in Rust.
struct TestMutation {
    filled: Option<(usize, usize)>,
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
        let tree = cx.tree_mut();
        let fragment = tree.create_document_fragment().map_err(refused)?;
        if let Some((elements, attributes)) = self.filled {
            for _ in 0..elements {
                let b = tree
                    .create_element_ns(Namespace::Html, "b")
                    .map_err(refused)?;
                for at in 0..attributes {
                    tree.set_attribute_ns(b, None, &format!("a{at}"), "")
                        .map_err(refused)?;
                }
                tree.append_child(fragment, b).map_err(refused)?;
            }
        }
        Ok(fragment)
    }
}

/// surf's slices, with the test's `mutation`.
fn slices(filled: Option<(usize, usize)>) -> Slices {
    let mut slices = dom::slices(DocumentInfo::default());
    slices.mutation = Box::new(TestMutation { filled });
    slices
}

/// The parser's bounds, with room for many attributes and long values.
fn raised() -> ParserLimits {
    ParserLimits {
        max_attributes: 2_048,
        max_stored_bytes: 256 * 1024 * 1024,
        ..ParserLimits::default()
    }
}

/// The document of `tree`.
fn document(tree: Tree) -> Document {
    Document {
        tree,
        limits: raised(),
        slices: slices(None),
    }
}

/// `page` parsed under [`raised`].
fn parsed(page: &str) -> Tree {
    parse_document(
        page,
        &ParseOptions {
            limits: raised(),
            ..ParseOptions::default()
        },
    )
}

/// Runs `body` with a runner over the tree `tree` makes.
fn with_tree<R: Send>(
    limits: RunnerLimits,
    tree: impl FnOnce() -> Tree + Send,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(limits, Mode::Test, move || document(tree()), body)
        .expect("a runner with the bindings")
}

/// Runs `body` with a runner over `page`.
fn with_page<R: Send>(page: &str, body: impl FnOnce(&mut Runner) -> R + Send) -> R {
    let page = page.to_string();
    with_tree(limits(), move || parsed(&page), body)
}

/// The limit the bindings recorded, read between two scripts.
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

/// The functions of [`CHECK`], then the elements of the tree registered by their ids, then
/// `source`.
fn script(runner: &mut Runner, source: &str) -> String {
    format!("{CHECK}known([{}]);\n{source}", page_ids(runner))
}

/// Runs [`script`] of `source`, and asserts it ran and recorded no limit.
fn checked(runner: &mut Runner, source: &str) {
    let source = script(runner, source);
    ran(runner, &source);
}

/// The ids of the elements of the runner's tree, read in Rust before a script runs, as a list
/// of JavaScript strings: the slice `attributes`, which `Element.id` is of, has no members yet.
fn page_ids(runner: &mut Runner) -> String {
    let (agent, _) = runner.agent_and_realm();
    let Some(dom) = agent.host().document() else {
        return String::new();
    };
    let tree = dom.tree();
    let mut found = Vec::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if let NodeData::Element { attributes, .. } = tree.data(node)
            && let Some(id) = attributes
                .iter()
                .find(|attribute| attribute.namespace.is_none() && attribute.local_name == "id")
        {
            found.push(format!("{:?}", id.value));
        }
        pending.extend(tree.children(node).iter().rev());
    }
    found.join(", ")
}

/// The functions every script of a test starts with: `check`; `known`, which registers the
/// element of each id; `id` of an element and `ids` of a list, by those ids, and `qa` and `q`
/// of `document`; `err`, the name, code and message of what a call throws; `rep`, a String of
/// `n` copies of `s` made in few steps.
const CHECK: &str = r##"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
var E = [];
function known(names) {
  for (var i = 0; i < names.length; i++) { E.push([names[i], document.getElementById(names[i])]); }
}
function id(node) {
  if (node === null) { return "null"; }
  for (var i = 0; i < E.length; i++) { if (E[i][1] === node) { return E[i][0]; } }
  return "?";
}
function ids(list) {
  var out = [];
  for (var i = 0; i < list.length; i++) { out.push(id(list[i])); }
  return out.join(",");
}
function qa(s) { return ids(document.querySelectorAll(s)); }
function q(s) { return id(document.querySelector(s)); }
function err(f) {
  try { f(); } catch (e) { return e.name + " " + e.code + " " + e.message; }
  return "no throw";
}
function rep(s, n) {
  var out = "";
  while (n > 0) { if (n % 2 === 1) { out += s; } s += s; n = (n - n % 2) / 2; }
  return out;
}
var INVALID = "SyntaxError 12 selector is not valid";
var UNSUPPORTED = "SyntaxError 12 selector is not supported by surf: ";
"##;

/// The page of the table: nested elements with ids, classes and attributes, an SVG and a
/// MathML subtree, and a template whose contents no member reaches.
const PAGE: &str = "<!DOCTYPE html><html id=\"h\"><head id=\"hd\"><title id=\"tt\">t</title></head>\
<body id=\"bd\">\
<div id=\"a\" class=\"c D\" title=\"x\">\
<p id=\"p1\" class=\"k\">one<span id=\"s1\" class=\"k m\"></span></p>\
<p id=\"p2\" lang=\"EN\"><i id=\"i1\"></i></p>\
</div>\
<section id=\"b\"><p id=\"p3\"><span id=\"s2\"></span></p><b id=\"b1\"></b></section>\
<svg id=\"g\" viewBox=\"0 0 1 1\"><foreignObject id=\"fo\"></foreignObject>\
<a xlink:href=\"#x\" id=\"xl\"></a></svg>\
<math id=\"m\"><mi id=\"mi\"></mi></math>\
<template id=\"tp\"><p id=\"t\"></p></template>\
</body></html>";

/// The names of the table's expressions.
const PRELUDE: &str = r##"
var a = document.getElementById("a");
var s1 = document.getElementById("s1");
var p1 = document.getElementById("p1");
function fragment() {
  var f = document.createDocumentFragment();
  f.appendChild(document.getElementById("b"));
  return f;
}
"##;

/// An entry of the table: a row of `selection.txt` as `<interface>.<member>`, a JavaScript
/// expression and the value the standards give it, as JavaScript text.
struct Case {
    row: &'static str,
    expression: &'static str,
    expected: &'static str,
}

const fn case(row: &'static str, expression: &'static str, expected: &'static str) -> Case {
    Case {
        row,
        expression,
        expected,
    }
}

/// The elements of [`PAGE`] in tree order, by id; the template's contents are not among them.
const ALL: &str = "\"h,hd,tt,bd,a,p1,s1,p2,i1,b,p3,s2,b1,g,fo,xl,m,mi,tp\"";

/// The table. Each entry's comment names the section of Selectors Level 4 (the Editor's Draft
/// of 10 November 2026, as the story read it), of the DOM Standard or of the HTML Standard whose
/// result it compares.
const CASES: &[Case] = &[
    // Selectors 5.1, type selector; the HTML Standard, "Case-sensitivity of selectors": the
    // name is lowered for an element of the HTML namespace only.
    case("Document.querySelectorAll", "qa(\"p\")", "\"p1,p2,p3\""),
    case("Document.querySelectorAll", "qa(\"DIV\")", "\"a\""),
    case("Document.querySelectorAll", "qa(\"svg\")", "\"g\""),
    case("Document.querySelectorAll", "qa(\"SVG\")", "\"\""),
    case(
        "Document.querySelectorAll",
        "qa(\"foreignObject\")",
        "\"fo\"",
    ),
    case("Document.querySelectorAll", "qa(\"foreignobject\")", "\"\""),
    // Selectors 5.2, universal selector; DOM 4.2.6: the template's contents are not among the
    // descendants of the document.
    case("Document.querySelectorAll", "qa(\"*\")", ALL),
    // Selectors 6.7, ID selectors.
    case("Document.querySelectorAll", "qa(\"#p2\")", "\"p2\""),
    case("Document.querySelector", "q(\"#t\")", "\"null\""),
    // Selectors 6.6, class selectors: a token of `class` split at ASCII whitespace.
    case("Document.querySelectorAll", "qa(\".k\")", "\"p1,s1\""),
    case("Document.querySelectorAll", "qa(\".m\")", "\"s1\""),
    // Selectors 6.1, attribute presence; the HTML Standard: the name lowered for an HTML
    // element; an attribute in a namespace is not matched by a name in none.
    case("Document.querySelectorAll", "qa(\"[lang]\")", "\"p2\""),
    case(
        "Document.querySelectorAll",
        "qa(\"[CLASS]\")",
        "\"a,p1,s1\"",
    ),
    case("Document.querySelectorAll", "qa(\"[href]\")", "\"\""),
    case("Document.querySelectorAll", "qa(\"[viewBox]\")", "\"g\""),
    case("Document.querySelectorAll", "qa(\"[viewbox]\")", "\"\""),
    // Selectors 6.1, attribute value: an identifier or a string, compared exactly.
    case("Document.querySelectorAll", "qa(\"[title=x]\")", "\"a\""),
    case(
        "Document.querySelectorAll",
        "qa('[class=\"k m\"]')",
        "\"s1\"",
    ),
    case("Document.querySelectorAll", "qa(\"[title=X]\")", "\"\""),
    // The HTML Standard: `lang` is in the list of values compared without case.
    case("Document.querySelectorAll", "qa(\"[lang=en]\")", "\"p2\""),
    // Selectors 3.1, a compound selector: every simple selector matches the element.
    case("Document.querySelectorAll", "qa(\"p.k\")", "\"p1\""),
    case(
        "Document.querySelectorAll",
        "qa(\"span.k.m#s1[class]\")",
        "\"s1\"",
    ),
    case("Document.querySelectorAll", "qa(\"p.m\")", "\"\""),
    // Selectors 14.1, descendant combinator.
    case("Document.querySelectorAll", "qa(\"div span\")", "\"s1\""),
    case("Document.querySelectorAll", "qa(\"body i\")", "\"i1\""),
    case(
        "Document.querySelectorAll",
        "qa(\"html section span\")",
        "\"s2\"",
    ),
    // Selectors 14.2, child combinator.
    case("Document.querySelectorAll", "qa(\"div > p\")", "\"p1,p2\""),
    case("Document.querySelectorAll", "qa(\"div > span\")", "\"\""),
    case("Document.querySelectorAll", "qa(\"div>p>span\")", "\"s1\""),
    case(
        "Document.querySelectorAll",
        "qa(\"body > * > p > *\")",
        "\"s1,i1,s2\"",
    ),
    // Selectors 4.1, a selector list: an element that matches any, once, in tree order.
    case(
        "Document.querySelectorAll",
        "qa(\"i, #b, p\")",
        "\"p1,p2,i1,b,p3\"",
    ),
    case("Document.querySelectorAll", "qa(\"p,p\")", "\"p1,p2,p3\""),
    // DOM 4.2.6, querySelector: the first result in tree order, or null.
    case("Document.querySelector", "q(\"p\")", "\"p1\""),
    case("Document.querySelector", "q(\"zzz\")", "\"null\""),
    case("Document.querySelector", "q(\"section p span\")", "\"s2\""),
    case("Document.querySelector", "q(\"i, span\")", "\"s1\""),
    // DOM 1.3 and 4.2.6: the descendants of `this`, never `this`; an ancestor outside `this`
    // still counts for a combinator.
    case(
        "Element.querySelectorAll",
        "ids(a.querySelectorAll(\"p\"))",
        "\"p1,p2\"",
    ),
    case(
        "Element.querySelectorAll",
        "ids(a.querySelectorAll(\"div\"))",
        "\"\"",
    ),
    case(
        "Element.querySelectorAll",
        "ids(a.querySelectorAll(\"body p\"))",
        "\"p1,p2\"",
    ),
    case(
        "Element.querySelectorAll",
        "ids(a.querySelectorAll(\"div > p > span\"))",
        "\"s1\"",
    ),
    case(
        "Element.querySelector",
        "id(a.querySelector(\"div\"))",
        "\"null\"",
    ),
    case(
        "Element.querySelector",
        "id(a.querySelector(\"#bd .k\"))",
        "\"p1\"",
    ),
    case(
        "Element.querySelector",
        "id(a.querySelector(\"i\"))",
        "\"i1\"",
    ),
    // DOM 4.9, closest: `this`, then its element ancestors upward; the first match or null.
    case("Element.closest", "id(s1.closest(\"div\"))", "\"a\""),
    case("Element.closest", "id(s1.closest(\"span\"))", "\"s1\""),
    case("Element.closest", "id(s1.closest(\"section\"))", "\"null\""),
    case(
        "Element.closest",
        "id(s1.closest(\"p.k > span\"))",
        "\"s1\"",
    ),
    case("Element.closest", "id(s1.closest(\"body > *\"))", "\"a\""),
    case("Element.closest", "id(s1.closest(\"html\"))", "\"h\""),
    // DOM 4.9, matches: whether `this` matches; webkitMatchesSelector is the same steps.
    case("Element.matches", "s1.matches(\"div span\")", "true"),
    case("Element.matches", "s1.matches(\"p\")", "false"),
    case("Element.matches", "s1.matches(\"p > .m\")", "true"),
    case(
        "Element.webkitMatchesSelector",
        "s1.webkitMatchesSelector(\"div span\")",
        "true",
    ),
    case(
        "Element.webkitMatchesSelector",
        "s1.webkitMatchesSelector(\"p\")",
        "false",
    ),
    // DOM 1.3: a fragment is searched as its own tree, so an ancestor it does not hold counts
    // for nothing.
    case(
        "DocumentFragment.querySelectorAll",
        "ids(fragment().querySelectorAll(\"p, span\"))",
        "\"p3,s2\"",
    ),
    case(
        "DocumentFragment.querySelectorAll",
        "ids(fragment().querySelectorAll(\"body span\"))",
        "\"\"",
    ),
    case(
        "DocumentFragment.querySelector",
        "id(fragment().querySelector(\"section span\"))",
        "\"s2\"",
    ),
    case(
        "DocumentFragment.querySelector",
        "id(fragment().querySelector(\"#bd span\"))",
        "\"null\"",
    ),
    // DOM 1.3, scope-match a selectors string: a failure is a SyntaxError. Every member parses
    // its argument first: one not-valid and one not-supported text each.
    case(
        "Element.closest",
        "err(function () { s1.closest(\"a[\"); })",
        "INVALID",
    ),
    case(
        "Element.closest",
        "err(function () { s1.closest(\"a:hover\"); })",
        "UNSUPPORTED + \"pseudo-class\"",
    ),
    case(
        "Element.matches",
        "err(function () { s1.matches(\"\"); })",
        "INVALID",
    ),
    case(
        "Element.matches",
        "err(function () { s1.matches(\"a + b\"); })",
        "UNSUPPORTED + \"sibling combinator\"",
    ),
    case(
        "Element.webkitMatchesSelector",
        "err(function () { s1.webkitMatchesSelector(\"a,\"); })",
        "INVALID",
    ),
    case(
        "Element.webkitMatchesSelector",
        "err(function () { s1.webkitMatchesSelector(\"a::before\"); })",
        "UNSUPPORTED + \"pseudo-element\"",
    ),
    case(
        "Element.querySelector",
        "err(function () { a.querySelector(\"> a\"); })",
        "INVALID",
    ),
    case(
        "Element.querySelector",
        "err(function () { a.querySelector(\"[a^=b]\"); })",
        "UNSUPPORTED + \"attribute matcher\"",
    ),
    case(
        "Element.querySelectorAll",
        "err(function () { a.querySelectorAll(\"a >\"); })",
        "INVALID",
    ),
    case(
        "Element.querySelectorAll",
        "err(function () { a.querySelectorAll(\"[a=b i]\"); })",
        "UNSUPPORTED + \"attribute flag\"",
    ),
    case(
        "Document.querySelector",
        "err(function () { document.querySelector(\"ns|a\"); })",
        "INVALID",
    ),
    case(
        "Document.querySelector",
        "err(function () { document.querySelector(\"*|a\"); })",
        "UNSUPPORTED + \"namespace prefix\"",
    ),
    case(
        "Document.querySelectorAll",
        "err(function () { document.querySelectorAll(\"a b)\"); })",
        "INVALID",
    ),
    case(
        "Document.querySelectorAll",
        "err(function () { document.querySelectorAll(\"& a\"); })",
        "UNSUPPORTED + \"nesting selector\"",
    ),
    case(
        "DocumentFragment.querySelector",
        "err(function () { fragment().querySelector(\"#123\"); })",
        "INVALID",
    ),
    case(
        "DocumentFragment.querySelector",
        "err(function () { fragment().querySelector(\"a ~ b\"); })",
        "UNSUPPORTED + \"sibling combinator\"",
    ),
    case(
        "DocumentFragment.querySelectorAll",
        "err(function () { fragment().querySelectorAll(\"[a=\"); })",
        "INVALID",
    ),
    case(
        "DocumentFragment.querySelectorAll",
        "err(function () { fragment().querySelectorAll(\"|a\"); })",
        "UNSUPPORTED + \"namespace prefix\"",
    ),
];

/// Runs one entry: a runner of its own over [`PAGE`], `expression === expected` thrown on a
/// mismatch, the end `Ran`, and no limit recorded.
fn run_case(case: &Case) {
    let body = format!(
        "{PRELUDE}\nvar got = ({});\nif (got !== ({})) {{ throw new Error({:?} + \": \" + String(got)); }}",
        case.expression, case.expected, case.row
    );
    with_page(PAGE, |runner| {
        let source = script(runner, &body);
        let end = runner.evaluate(&source);
        assert_eq!(
            end,
            ScriptEnd::Ran,
            "{} `{}`: {end}",
            case.row,
            case.expression
        );
        assert_eq!(recorded(runner), None, "{}", case.row);
    });
}

/// A row of `selection.txt` as `selection()` of `tests/bindings.rs` reads it: interface,
/// member, slice and the note.
fn selection() -> Vec<(String, String, String, String)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../webidl/selection.txt");
    let text = std::fs::read_to_string(path).expect("read selection.txt");
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|line| !line.starts_with("definitions:") && !line.starts_with("global:"))
        .map(|line| {
            let mut words = line.splitn(4, ' ');
            let mut next = || words.next().unwrap_or_default().to_string();
            (next(), next(), next(), next())
        })
        .collect()
}

/// The rows of the slice `selectors`, as `<interface>.<member>`, with their notes.
fn selector_rows() -> Vec<(String, String)> {
    selection()
        .into_iter()
        .filter(|(_, _, slice, _)| slice == "selectors")
        .map(|(interface, member, _, note)| (format!("{interface}.{member}"), note))
        .collect()
}

#[test]
fn cases_are_the_rows() {
    let rows: BTreeSet<String> = selector_rows().into_iter().map(|(row, _)| row).collect();
    let nine: BTreeSet<String> = [
        "Element.closest",
        "Element.matches",
        "Element.webkitMatchesSelector",
        "Element.querySelector",
        "Element.querySelectorAll",
        "Document.querySelector",
        "Document.querySelectorAll",
        "DocumentFragment.querySelector",
        "DocumentFragment.querySelectorAll",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    assert_eq!(rows, nine);
    let named: BTreeSet<String> = CASES.iter().map(|case| case.row.to_string()).collect();
    assert_eq!(named, rows);
    for case in CASES {
        assert!(
            !case.expression.trim().is_empty() && !case.expected.trim().is_empty(),
            "{}",
            case.row
        );
        run_case(case);
    }
}

/// Each of the nine rows carries both clauses of the slice's deviation note.
#[test]
fn the_rows_carry_the_note() {
    let rows = selector_rows();
    assert_eq!(rows.len(), 9);
    for (row, note) in rows {
        for words in [
            "deviation: a valid selector outside the subset",
            "recognised by its shape",
        ] {
            assert!(note.contains(words), "{row}: {note}");
        }
    }
}

/// DOM 1.3, 4.2.6 and 4.9: what each member answers, over a removed subtree, a fragment and a
/// template, and the brand check of the glue.
#[test]
fn the_meaning_of_the_members() {
    with_page(PAGE, |runner| {
        checked(
            runner,
            r##"
var a = document.getElementById("a");
var s1 = document.getElementById("s1");
check(document.querySelector("#a") === a, "querySelector answers the element");
check(a.querySelector("#a") === null && a.querySelectorAll("*").length === 4,
  "never this, its descendants");
check(s1.closest("span") === s1 && s1.closest("zzz") === null, "closest");
check(s1.matches("p span") === s1.webkitMatchesSelector("p span"), "the two agree");
check(s1.matches("i") === s1.webkitMatchesSelector("i"), "the two agree on no");
check(document.querySelector("#t") === null, "the template's contents are not searched");
check(qa("template *") === "", "nor reached through the template");
check(err(function () { Element.prototype.closest.call(document, "a"); }).indexOf("TypeError") === 0,
  "a member called on an object of another brand");
var removed = document.getElementById("a");
document.getElementById("bd").removeChild(removed);
check(document.querySelector("#a") === null, "a removed subtree is out of the document");
check(ids(removed.querySelectorAll("span")) === "s1", "and searched as its own tree");
check(removed.querySelector("body span") === null, "with no ancestor it no longer has");
check(removed.querySelector("div span") === s1, "its own root counts");
check(s1.closest("body") === null && s1.closest("div") === removed, "closest stops at its root");
var f = document.createDocumentFragment();
f.appendChild(document.getElementById("b"));
check(ids(f.querySelectorAll("*")) === "b,p3,s2,b1", "a fragment's elements");
check(f.querySelector("body *") === null, "a fragment has no body above it");
var s2 = f.querySelector("#s2");
check(id(s2) === "s2" && s2.closest("*") === s2, "closest in a fragment");
check(s2.closest("body") === null && id(s2.closest("section")) === "b", "the fragment ends the walk");
"##,
        );
    });
}

/// The HTML Standard's case rules in quirks mode: `#id` and `.class` compare ASCII
/// case-insensitively in a page without a doctype, and exactly in no-quirks and limited-quirks
/// mode.
#[test]
fn ids_and_classes_in_the_three_modes() {
    let cases = [
        ("", "BackCompat", true),
        ("<!DOCTYPE html>", "CSS1Compat", false),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \
             \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">",
            "CSS1Compat",
            false,
        ),
    ];
    for (doctype, mode, folded) in cases {
        let page = format!("{doctype}<div id=\"a\" class=\"a\"></div>");
        let found = if folded { "1" } else { "0" };
        with_page(&page, |runner| {
            checked(
                runner,
                &format!(
                    r##"
check(document.compatMode === {mode:?}, document.compatMode);
check(document.querySelectorAll("#A").length === {found}, "#A");
check(document.querySelectorAll(".A").length === {found}, ".A");
check(document.querySelectorAll("#a").length === 1 && document.querySelectorAll(".a").length === 1,
  "exact");
check(document.querySelector("div").matches("div#A.A") === {folded}, "matches");"##
                ),
            );
        });
    }
}

/// A tree built in Rust: `body` holds an HTML element and an SVG element whose local name is
/// `DIV`, a `p` with the attribute `FOO` made with `set_attribute_ns(None, "FOO")`, and a
/// parser-made `p` with `foo`.
fn written_tree() -> Tree {
    let mut tree = parsed("<p id=\"low\" foo=\"1\"></p>");
    let body = elements_named(&tree, "body")[0];
    let html_div = tree
        .create_element_ns(Namespace::Html, "DIV")
        .expect("an element");
    tree.set_attribute_ns(html_div, None, "id", "html")
        .expect("an id");
    let svg_div = tree
        .create_element_ns(Namespace::Svg, "DIV")
        .expect("an element");
    tree.set_attribute_ns(svg_div, None, "id", "svg")
        .expect("an id");
    let upper = tree
        .create_element_ns(Namespace::Html, "p")
        .expect("an element");
    tree.set_attribute_ns(upper, None, "id", "up")
        .expect("an id");
    tree.set_attribute_ns(upper, None, "FOO", "1")
        .expect("an attribute");
    for node in [html_div, svg_div, upper] {
        tree.append_child(body, node).expect("appended");
    }
    tree
}

/// The elements named `name`, in tree order, read in Rust.
fn elements_named(tree: &Tree, name: &str) -> Vec<NodeId> {
    let mut found = Vec::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if let NodeData::Element { local_name, .. } = tree.data(node)
            && local_name == name
        {
            found.push(node);
        }
        pending.extend(tree.children(node).iter().rev());
    }
    found
}

/// The HTML Standard: the selector's type and attribute names are lowered for an HTML element
/// and compared exactly, and compared as given for any other.
#[test]
fn names_built_in_rust() {
    with_tree(limits(), written_tree, |runner| {
        checked(
            runner,
            r##"
check(qa("DIV") === "svg", "DIV: lowered for the HTML element, as given for the SVG one");
check(qa("div") === "", "div finds neither");
check(qa("[FOO]") === "low", "[FOO] is [foo] on an HTML element");
check(qa("[foo]") === "low", "[foo] does not find FOO");
check(qa("p[foo]") === "low" && qa("p") === "low,up", "the p elements");
"##,
        );
    });
}

/// Selectors Level 4, 3.9, and DOM 1.3: every refusal is a SyntaxError, of one of two messages,
/// none of which holds text of the page.
#[test]
fn the_one_exception_and_its_two_messages() {
    with_page(PAGE, |runner| {
        checked(
            runner,
            r##"
var unsupported = [
  ["a:hover", "pseudo-class"], ["a::before", "pseudo-element"], [":scope > a", "pseudo-class"],
  ["a + b", "sibling combinator"], ["a ~ b", "sibling combinator"],
  ["[a^=b]", "attribute matcher"], ["[a~=b]", "attribute matcher"], ["[a|=b]", "attribute matcher"],
  ["[a$=b]", "attribute matcher"], ["[a*=b]", "attribute matcher"],
  ["[a=b i]", "attribute flag"], ["[a=b s]", "attribute flag"], ["[a=b I]", "attribute flag"],
  ["[a='b'i]", "attribute flag"],
  ["*|a", "namespace prefix"], ["|a", "namespace prefix"], ["[*|a]", "namespace prefix"],
  ["[|a]", "namespace prefix"],
  ["a:not(b)", "pseudo-class"], ["a:not(b", "pseudo-class"], ["& a", "nesting selector"],
  ["a:has(\")\")", "pseudo-class"],
  // A browser calls these two texts not valid: surf recognises a pseudo-class by its shape
  // and validates neither its name nor its argument, a deviation the rows name.
  ["a:hovr", "pseudo-class"], ["a:nth-child(x y z)", "pseudo-class"],
  // Selectors 16: the four pseudo-elements of Level 2 written with one colon are
  // pseudo-elements, never pseudo-classes, in any case of their names.
  ["a:before", "pseudo-element"], ["a:after", "pseudo-element"],
  ["a:first-line", "pseudo-element"], ["a:First-Letter", "pseudo-element"],
  ["a:before:hover", "pseudo-element"], ["a:hover:before", "pseudo-class"]
];
for (var i = 0; i < unsupported.length; i++) {
  var text = unsupported[i][0];
  var got = err(function () { document.querySelector(text); });
  check(got === UNSUPPORTED + unsupported[i][1], text + ": " + got);
}
var invalid = ["", "a[", "a:", "a,", "a >", "> a", "a:hover[", "a b)", "[a=", "[a=\"x\ny\"]",
  "ns|a", "[ns|a]", "a || b", "#123", ".1", "a/**/b", "[a=1]", "a ( b", "[a~b]", "a::", ",a",
  "a,,b", "[]", "[=b]", "[a=b c]", "a{", "a}", "[a=b]]", "*a", ".a div.", "a::before.b",
  "a:before.b", "a:after[x]", "a:first-letter#i"];
for (var i = 0; i < invalid.length; i++) {
  var got = err(function () { document.querySelector(invalid[i]); });
  check(got === INVALID, JSON.stringify(invalid[i]) + ": " + got);
}
var long = rep("x", 2000);
var notValid = err(function () { document.querySelector("[" + long + "=]"); });
var notSupported = err(function () { document.querySelector("a:" + long); });
check(notValid === INVALID, notValid);
check(notSupported === UNSUPPORTED + "pseudo-class", notSupported);
check(notValid.indexOf("xx") < 0 && notSupported.indexOf("xx") < 0, "no text of the page");
G.lengths = [notValid.length, notSupported.length];
try { document.querySelector("a:hover"); } catch (e) {
  check(e instanceof DOMException && e.name === "SyntaxError" && e.code === 12, "a DOMException");
}
"##,
        );
    });
}

/// The two messages are told apart: each not-supported message starts with its words, the
/// not-valid one does not, and both are under the bound of an exception's text.
#[test]
fn the_two_messages_are_apart() {
    let valid = "selector is not valid";
    let words = "selector is not supported by surf: ";
    assert!(!valid.starts_with(words));
    for word in [
        "pseudo-class",
        "pseudo-element",
        "sibling combinator",
        "attribute matcher",
        "attribute flag",
        "namespace prefix",
        "nesting selector",
        "text over 4096 bytes",
        "more than 512 simple selectors",
    ] {
        let message = format!("{words}{word}");
        assert_ne!(message, valid);
        assert!(message.len() < EXCEPTION_TEXT_BYTES);
    }
    assert!(valid.len() < EXCEPTION_TEXT_BYTES);
}

/// The bounds of the text: 4,096 bytes and 512 simple selectors (decision 6).
#[test]
fn the_bounds_of_a_text() {
    with_page(PAGE, |runner| {
        checked(
            runner,
            r##"
var text4096 = "p, " + rep(" ", 4092) + "i";
check(text4096.length === 4096, "4,096");
check(qa(text4096) === "p1,p2,i1,p3", "4,096 bytes run");
G.over = text4096 + " ";
G.simple512 = rep("p,", 511) + "p";
G.simple513 = rep("p,", 512) + "p";
check(qa(G.simple512) === "p1,p2,p3", "512 simple selectors run");
check(err(function () { document.querySelector(G.simple513); }) ===
  UNSUPPORTED + "more than 512 simple selectors", "513");
check(err(function () { document.querySelector(G.simple513 + "["); }) ===
  UNSUPPORTED + "more than 512 simple selectors", "513 then a fault");
check(err(function () { document.querySelector(rep("p", 1) + "["); }) === INVALID, "p[");
check(err(function () { document.querySelector(rep("p,", 300) + "a:hover," + rep("p,", 300)); }) ===
  UNSUPPORTED + "pseudo-class", "a construct before the 513th is named");
"##,
        );
        // The script that makes the call, its text made by an earlier one. The glue charges
        // the call's step and, before the copy, a step for each 16 bytes of the text as the
        // engine holds it, two bytes a code unit: 4,097 code units are 512 steps, so at least
        // 513. The script's own few steps sit under the ceiling of 640, and the slice charges
        // nothing before it refuses: its charge of the text, a step and one for each 16 bytes,
        // would add 257 and pass the ceiling.
        let over = refusal_steps(runner, "G.over");
        checked(runner, r##"G.huge = rep("a", 1048576);"##);
        // The glue's copy of 1,048,576 code units at 8 a step is 131,072 steps, charged before
        // the slice refuses the text; the call and the script add a few, under 135,000.
        let huge = refusal_steps(runner, "G.huge");
        eprintln!("the call of 4,097 bytes: {over} steps; of 1,048,576 bytes: {huge} steps");
        assert!((513..=640).contains(&over), "{over}");
        assert!((131_072..135_000).contains(&huge), "{huge}");
    });
}

/// The steps of a script that passes the text `text` names to `document.querySelector` and
/// catches the throw, which must be the refusal of a text over 4,096 bytes.
fn refusal_steps(runner: &mut Runner, text: &str) -> u64 {
    let before = runner.steps();
    ran(
        runner,
        &format!("try {{ document.querySelector({text}); }} catch (e) {{ G.e = e.message; }}"),
    );
    let steps = runner.steps() - before;
    checked(
        runner,
        r##"check(G.e === "selector is not supported by surf: text over 4096 bytes", G.e);"##,
    );
    steps
}

/// The page of the lexical cases.
const LEXICAL: &str = "<!DOCTYPE html><body>\
<div id=\"123\"></div><div id=\"a:b\" class=\"md:flex\"></div><div id=\"ab\"></div>\
<div id=\"a\"><b id=\"ba\"></b></div><div id=\"\u{FFFD}\"></div>\
<p id=\"t1\" title='x\"y'></p><p id=\"t2\" title=\"xy\"></p><p id=\"t3\" title=\"x\"></p>\
<p id=\"t4\" title=\"x:y\"></p><p id=\"t5\" title=\"x]\"></p>\
</body>";

/// CSS Syntax Level 3 as the selector's tokens (decision 15): escapes, strings, comments, the
/// end of the text and the newlines.
#[test]
fn the_lexical_rules() {
    with_page(LEXICAL, |runner| {
        checked(
            runner,
            r##"
var F = "�";
function no(text) { return err(function () { document.querySelector(text); }); }
check(qa("#\\31 23") === "123", "an escaped digit starts an identifier");
check(no("#123") === INVALID, "#123");
check(qa("#a\\:b") === "a:b" && qa(".md\\:flex") === "a:b", "an escaped colon");
check(qa("#\\000061 b") === "ab", "six digits and a space");
check(qa("#\\61  b") === "ba", "the escape, then the descendant combinator");
check(qa("#\\0") === F && qa("#\\d800") === F && qa("#\\110000") === F, "U+FFFD");
check(qa('[title="x\\"y"]') === "t1", "an escaped quote");
check(qa('[title="x\\\ny"]') === "t2", "a continuation");
check(qa("[title='x']") === "t3" && qa("[title=x]") === "t3", "string and identifier");
check(qa("[title=x\\:y]") === "t4", "an escape in a value");
check(qa('[title="x') === "t3" && qa("[title=x") === "t3", "the end closes a string and a block");
check(qa('[title="x]') === "t5", "the string takes the ]");
check(no('[title="x\ny"]') === INVALID && no("a\\\nb") === INVALID, "a newline");
check(qa("#\\") === F, "a backslash at the end of an identifier");
check(no("a\\") === "no throw" && document.querySelector("a\\") === null, "a then U+FFFD");
check(qa("div /* c */ b") === qa("div b") && qa("div b") === "ba", "a comment");
check(qa("div /* c */ > /* d */ b") === qa("div > b"), "comments around a combinator");
check(qa("div /* b") === qa("div"), "an unterminated comment");
check(qa("/* a */ div > b /* b */") === "ba", "comments at both ends");
var newlines = ["\r", "\f", "\r\n"];
for (var i = 0; i < newlines.length; i++) {
  var nl = newlines[i];
  var name = JSON.stringify(nl);
  check(no('[title="x' + nl + 'y"]') === INVALID, name + " (1)");
  check(qa('[title="x\\' + nl + 'y"]') === "t2", name + " (2)");
  check(qa("#\\61" + nl + "b") === "ab", name + " (3)");
  check(qa("#\\61" + nl + nl + "b") === "ba", name + " (3) twice");
  check(no("a\\" + nl + "b") === INVALID, name + " (4)");
}
var comment = "/*" + rep("x", 4096 - 5) + "*/";
check(qa(comment + "b") === "ba" && (comment + "b").length === 4096, "4,096 bytes of comment");
check(no(comment + "bb") === UNSUPPORTED + "text over 4096 bytes", "4,097");
check(no("b/*" + rep("x", 4094)) === UNSUPPORTED + "text over 4096 bytes", "unterminated, 4,097");
check(no(rep("\\61 ,", 512) + "\\61") === UNSUPPORTED + "more than 512 simple selectors", "513");
check(no(rep("\\61 ,", 512) + "\\61[") === UNSUPPORTED + "more than 512 simple selectors", "513[");
check(no("a[") === INVALID, "a[ alone");
"##,
        );
    });
}

/// The case of attribute values (decision 16): the names of the HTML Standard's list compare
/// ASCII case-insensitively on an HTML element, every other name and every other element
/// exactly.
#[test]
fn attribute_values_and_their_case() {
    let page = "<!DOCTYPE html><body><p id=\"lo\" accept=\"v\" data-surf-x=\"v\"></p>\
<p id=\"up\" accept=\"V\"></p><svg><g id=\"svg\" accept=\"v\"></g></svg>\
<math><mi id=\"ml\" accept=\"v\"></mi></math></body>";
    with_page(page, |runner| {
        checked(
            runner,
            r##"
check(qa('[accept="V"]') === "lo,up", "accept on HTML elements, either case");
check(qa('[accept="v"]') === "lo,up,svg,ml", "swapped; the SVG and MathML ones exact");
check(qa('[ACCEPT="V"]') === "lo,up", "the name lowered first");
check(qa('[data-surf-x="V"]') === "", "a name not in the list: exact");
check(qa('[data-surf-x="v"]') === "lo", "exact");
check(qa('g[accept="V"]') === "" && qa('g[accept="v"]') === "svg", "SVG: exact");
check(qa('mi[accept="V"]') === "" && qa('mi[accept="v"]') === "ml", "MathML: exact");
check(err(function () { document.querySelector("[a=b i]"); }) === UNSUPPORTED + "attribute flag", "i");
check(err(function () { document.querySelector("[a=b I]"); }) === UNSUPPORTED + "attribute flag", "I");
check(err(function () { document.querySelector("[a=b s]"); }) === UNSUPPORTED + "attribute flag", "s");
"##,
        );
    });
}

/// The list of the HTML Standard's section "Case-sensitivity of selectors", typed here a second
/// time: the table of `selectors.rs` must be this list, sorted.
const CASE_INSENSITIVE: [&str; 46] = [
    "accept",
    "accept-charset",
    "align",
    "alink",
    "axis",
    "bgcolor",
    "charset",
    "checked",
    "clear",
    "codetype",
    "color",
    "compact",
    "declare",
    "defer",
    "dir",
    "direction",
    "disabled",
    "enctype",
    "face",
    "frame",
    "hreflang",
    "http-equiv",
    "lang",
    "language",
    "link",
    "media",
    "method",
    "multiple",
    "nohref",
    "noresize",
    "noshade",
    "nowrap",
    "readonly",
    "rel",
    "rev",
    "rules",
    "scope",
    "scrolling",
    "selected",
    "shape",
    "target",
    "text",
    "type",
    "valign",
    "valuetype",
    "vlink",
];

/// Every name of the list compares without case on an HTML element.
#[test]
fn every_name_of_the_list_compares_without_case() {
    let attributes: String = CASE_INSENSITIVE
        .iter()
        .map(|name| format!(" {name}=\"v\""))
        .collect();
    let page = format!("<!DOCTYPE html><p id=\"all\"{attributes}></p><p id=\"none\"></p>");
    let names: Vec<String> = CASE_INSENSITIVE
        .iter()
        .map(|name| format!("{name:?}"))
        .collect();
    with_page(&page, |runner| {
        checked(
            runner,
            &format!(
                r##"
var names = [{}];
for (var i = 0; i < names.length; i++) {{
  check(qa("[" + names[i] + "=V]") === "all", names[i]);
}}
check(names.length === 46, "46");"##,
                names.join(", ")
            ),
        );
    });
}

/// Collections (decision 9): a static NodeList, a new object per call, the old nodes after a
/// write, and one wrapper per node.
#[test]
fn the_lists_are_static() {
    with_page(PAGE, |runner| {
        checked(
            runner,
            r##"
var a = document.getElementById("a");
var list = a.querySelectorAll("p");
check(list instanceof NodeList && list.length === 2, "a NodeList");
check(list.item(0) === list[0] && list.item(1) === list[1] && list.item(2) === null, "item");
var seen = [];
for (var x of list) { seen.push(id(x)); }
check(seen.join(",") === "p1,p2", "for of");
var each = [];
list.forEach(function (node, index, all) { check(all === list, "forEach"); each.push(index); });
check(each.join(",") === "0,1", "forEach");
check(a.querySelectorAll("p") !== a.querySelectorAll("p"), "a new object per call");
check(document.querySelectorAll("p") !== document.querySelectorAll("p"), "with no write");
var p2 = document.getElementById("p2");
a.removeChild(p2);
check(list.length === 2 && list[1] === p2, "an old list holds the old nodes");
check(ids(a.querySelectorAll("p")) === "p1", "a new call finds the new ones");
document.getElementById("b").appendChild(p2);
check(ids(document.querySelectorAll("section > p")) === "p3,p2", "after an append");
check(document.querySelector("p") === document.getElementsByTagName("p")[0], "one wrapper");
"##,
        );
    });
}

/// The two cases of `scripted_foster01.dat`: the tree the parse builds holds a table that
/// `querySelector("table")` finds; their scripts are not run here.
#[test]
fn the_foster_parenting_cases_find_their_table() {
    let data = include_str!(
        "../../html-conformance/data/wpt/html/syntax/parsing/resources/scripted_foster01.dat"
    );
    let inputs: Vec<&str> = data
        .split("#data\n")
        .skip(1)
        .map(|case| case.lines().next().unwrap_or_default())
        .collect();
    assert_eq!(inputs.len(), 2);
    for input in inputs {
        with_page(input, |runner| {
            checked(
                runner,
                r##"var t = document.querySelector("table");
check(t !== null && t.tagName === "TABLE", "the table");"##,
            );
        });
    }
}

/// The table of `selectors.rs` is the HTML Standard's list: 46 names, sorted, equal to the copy
/// typed in this file.
#[test]
fn the_table_of_values_compared_without_case() {
    let table = SurfSelectors::CASE_INSENSITIVE_VALUES;
    assert_eq!(table.len(), 46);
    assert!(table.windows(2).all(|pair| pair[0] < pair[1]), "sorted");
    assert_eq!(table, CASE_INSENSITIVE);
}

/// The members read the tree, and charge, only through `selector_reads.rs`, which writes nothing,
/// charges through the reads' `charge`, and reads a child list only inside `Frames::next`.
#[test]
fn the_selectors_read_the_tree_only_through_the_reads() {
    let selectors = include_str!("../src/dom/selectors.rs");
    for call in ["tree()", "tree_mut()", "charge(", "charge_steps("] {
        assert!(!selectors.contains(call), "selectors.rs holds `{call}`");
    }
    let reads = include_str!("../src/dom/selector_reads.rs");
    for call in ["tree_mut()", "charge_steps("] {
        assert!(!reads.contains(call), "selector_reads.rs holds `{call}`");
    }
    let start = reads.find("fn next(").expect("Frames::next");
    let end = start
        + reads[start..]
            .find("\n    }\n")
            .expect("the end of Frames::next");
    assert!(reads[start..end].contains("children("), "Frames::next");
    let rest = [&reads[..start], &reads[end..]].concat();
    assert!(
        !rest.contains("children("),
        "selector_reads.rs reads a child list outside Frames::next"
    );
}

/// `selector_reads.rs` lends the members no reference into the tree and no context: no
/// `pub(super) fn` of it has `&`, `Tree` or `Cx` in its return type, the rule the reading
/// slice holds for `reads.rs`.
#[test]
fn the_selector_reads_answer_no_reference() {
    let reads = include_str!("../src/dom/selector_reads.rs");
    let mut found = Vec::new();
    for (at, _) in reads.match_indices("pub(super) fn ") {
        let rest = &reads[at..];
        let signature = &rest[..rest.find('{').unwrap_or(rest.len())];
        let returns = signature
            .rfind("->")
            .map_or("", |arrow| &signature[arrow + 2..]);
        for word in ["&", "Tree", "Cx"] {
            if returns.contains(word) {
                found.push(format!(
                    "selector_reads.rs: `{word}` in the return type of {:?}",
                    signature.split_whitespace().collect::<Vec<_>>().join(" ")
                ));
            }
        }
    }
    assert!(reads.matches("pub(super) fn ").count() >= 12);
    assert!(found.is_empty(), "{found:#?}");
}

/// The nodes under `root` in tree order, `root` left out, read in Rust.
fn under(tree: &Tree, root: NodeId) -> Vec<NodeId> {
    let mut nodes = tree.descendants(root);
    nodes.remove(0);
    nodes
}

/// The walk's stack: every node under the document in tree order, with a frame a level that
/// has children, so a parent of 100,000 children holds one frame and not its children.
#[test]
fn the_walk_keeps_a_frame_a_level() {
    let wide = parsed(&"<b></b>".repeat(100_000));
    let deep = parsed(&"<div>".repeat(510));
    for (tree, count) in [(wide, 100_003), (deep, 513)] {
        let mut frames = Frames::new(tree.root());
        let mut walked = Vec::new();
        while let Some(node) = frames.next(&tree) {
            walked.push(node);
        }
        assert_eq!(walked.len(), count);
        assert_eq!(walked, under(&tree, tree.root()));
        assert!(frames.most() <= 513, "{}", frames.most());
        assert!(frames.is_empty());
        assert_eq!(frames.len(), 0);
        eprintln!("{count} nodes: most frames {}", frames.most());
    }
}

/// The cost of each member on its worst shape (decisions 7, 8 and 19): each case measures the
/// steps of its calls, prints them, and asserts floors (and ceilings where the story gives one)
/// from the story's arithmetic, never a time. The steps measured are in each case's comment.
mod cost {
    use std::time::Instant;

    use super::*;

    /// One measured call: what it is, its steps, its time, and the floor of its steps.
    struct Call {
        what: String,
        steps: u64,
        took: Duration,
        floor: u64,
    }

    /// Runs `source` once and measures it; the script must run to its end and record no limit.
    fn call(runner: &mut Runner, what: &str, source: &str, floor: u64) -> Call {
        let before = runner.steps();
        let at = Instant::now();
        let end = runner.evaluate(source);
        let took = at.elapsed();
        assert_eq!(end, ScriptEnd::Ran, "{what}: {end}");
        assert_eq!(recorded(runner), None, "{what}");
        Call {
            what: what.to_string(),
            steps: runner.steps() - before,
            took,
            floor,
        }
    }

    /// Prints a line a call and asserts each call's floor of steps.
    fn report(name: &str, calls: &[Call]) {
        let lines: Vec<String> = calls
            .iter()
            .map(|call| {
                format!(
                    "{}: {} steps (floor {}), {:?}",
                    call.what, call.steps, call.floor, call.took
                )
            })
            .collect();
        eprintln!("{name}:\n{}", lines.join("\n"));
        for call in calls {
            assert!(call.steps >= call.floor, "{}: {lines:#?}", call.what);
        }
    }

    /// Asserts the steps of `call` are under `ceiling`.
    fn under(call: &Call, ceiling: u64) {
        assert!(call.steps < ceiling, "{}: {} steps", call.what, call.steps);
    }

    /// The end of `source`, which must be a limit's, and no record of a limit of the parser.
    fn ends(runner: &mut Runner, source: &str, reason: Reason) {
        let end = runner.evaluate(source);
        assert_eq!(end, ScriptEnd::Limit(reason), "{end}");
        assert_eq!(recorded(runner), None);
    }

    /// The limits of [`limits`] with an entry's step limit of `steps`.
    fn limits_of(steps: u64) -> RunnerLimits {
        let mut limits = limits();
        limits.engine.steps_per_entry = steps;
        limits
    }

    /// A tree of `count` `b` elements in `body`, each with the attributes `x0`, `x1`, ... up to
    /// `attributes` of them, built in Rust, after `<!DOCTYPE html>`.
    fn flat(count: usize, attributes: usize) -> Tree {
        let mut tree = parsed("<!DOCTYPE html>");
        let body = elements_named(&tree, "body")[0];
        for _ in 0..count {
            let b = tree
                .create_element_ns(Namespace::Html, "b")
                .expect("an element");
            for at in 0..attributes {
                tree.set_attribute_ns(b, None, &format!("x{at}"), "")
                    .expect("an attribute");
            }
            tree.append_child(body, b).expect("appended");
        }
        tree
    }

    /// A tree of `count` `b` elements in `body` with the attributes `pairs`, built in Rust; with
    /// a doctype or without one (quirks mode).
    fn flat_with(count: usize, pairs: &[(&str, &str)], doctype: bool) -> Tree {
        let mut tree = parsed(if doctype { "<!DOCTYPE html>" } else { "" });
        let body = elements_named(&tree, "body")[0];
        for _ in 0..count {
            let b = tree
                .create_element_ns(Namespace::Html, "b")
                .expect("an element");
            for (name, value) in pairs {
                tree.set_attribute_ns(b, None, name, value)
                    .expect("an attribute");
            }
            tree.append_child(body, b).expect("appended");
        }
        tree
    }

    /// The page of 6b, 6d and 17a: 500 nested `div` and a `span` in the innermost, no doctype:
    /// 504 elements, the `span`'s chain 503.
    fn nested() -> Tree {
        parsed(&format!("{}<span></span>", "<div>".repeat(500)))
    }

    /// The selector of 6b: 10 compounds, no `x` exists.
    const TEN: &str = "x div div div div div div div div span";

    /// The runner of a document whose `createDocumentFragment` answers a fragment of
    /// `elements` `b` elements of `attributes` attributes each.
    fn with_fragment<R: Send>(
        elements: usize,
        attributes: usize,
        body: impl FnOnce(&mut Runner) -> R + Send,
    ) -> R {
        run_document(
            limits(),
            Mode::Test,
            move || Document {
                tree: parsed("<!DOCTYPE html>"),
                limits: raised(),
                slices: slices(Some((elements, attributes))),
            },
            body,
        )
        .expect("a runner with the bindings")
    }

    /// The setup of a measured call: `G`, the global, `G.body` and `G.b`, the first `b`, read
    /// before the call.
    const SETUP: &str = r##"var G = this;
G.body = document.getElementsByTagName("body")[0];
G.b = document.getElementsByTagName("b")[0];"##;

    /// 6a: a visit and a gate for each element: 1,003 elements, measured 2,023 steps; without
    /// the walk's step or the gate's, about 1,020.
    #[test]
    fn a_walk_charges_a_visit_and_a_gate() {
        let page = "<b></b>".repeat(1_000);
        with_tree(
            limits(),
            move || parsed(&page),
            |runner| {
                let calls = [call(
                    runner,
                    "6a Document.querySelector zzz",
                    "document.querySelector(\"zzz\");",
                    2_000,
                )];
                report("6a", &calls);
            },
        );
    }

    /// 6b: the matcher of a long selector over the deep page. Ten compounds, measured 7,215
    /// steps (504 visits, 504 gates, 503 for the chain, 628 for the memo, 5,030 cells, and the
    /// glue and the parse); nine compounds, measured 6,646 (504 + 504 + 503 + 565 + 4,527).
    #[test]
    fn a_long_selector_over_a_deep_chain() {
        with_tree(limits(), nested, |runner| {
            ran(runner, SETUP);
            let calls = [
                call(
                    runner,
                    "6b ten compounds",
                    &format!("G.r = document.querySelector({TEN:?});"),
                    7_000,
                ),
                call(
                    runner,
                    "6b nine compounds",
                    "G.r9 = document.querySelector(\"div div div div div div div div span\");",
                    6_500,
                ),
            ];
            checked(
                runner,
                "check(G.r === null, \"null\");\ncheck(G.r9 === document.getElementsByTagName(\"span\")[0], \"the span\");",
            );
            report("6b", &calls);
            under(&calls[0], 8_000);
            under(&calls[1], 8_000);
        });
    }

    /// 6c: a loop of calls over 100,000 elements parsed from text ends at the step limit of
    /// 1,000,000 within five calls (a call costs at least 200,000 steps). The parse alone took
    /// 140.9 ms on the build node.
    #[test]
    fn a_loop_of_calls_ends_at_the_step_limit() {
        let at = Instant::now();
        let tree = parsed(&"<b></b>".repeat(100_000));
        eprintln!("6c: the parse of 100,000 elements: {:?}", at.elapsed());
        with_tree(
            limits_of(1_000_000),
            move || tree,
            |runner| {
                ends(
                    runner,
                    "var n = 0; for (;;) { n++; document.querySelector(\"zzz\"); }",
                    Reason::StepsPerScript,
                );
                checked(runner, "check(n >= 1 && n <= 5, String(n));");
            },
        );
    }

    /// 6d: 512 simple selectors over the deep page, under a step limit of 1,000,000: measured
    /// 292,667 steps (504 + 504 + 503, the memo of 515,072 bytes, 32,192 steps, 257,536 cells,
    /// and the glue and the parse); without the memo's charge about 260,500, without the cells'
    /// about 35,100.
    #[test]
    fn five_hundred_and_twelve_compounds_over_a_deep_chain() {
        with_tree(limits_of(1_000_000), nested, |runner| {
            ran(runner, SETUP);
            let selector = format!("{}span", "div ".repeat(511));
            let calls = [call(
                runner,
                "6d 512 compounds",
                &format!("G.r = document.querySelector({selector:?});"),
                280_000,
            )];
            checked(runner, "check(G.r === null, \"null\");");
            report("6d", &calls);
            under(&calls[0], 300_000);
        });
    }

    /// 6e: lists of 1,003 nodes that fill a heap of 1 MiB end at the memory limit, and the
    /// runner runs the next script.
    #[test]
    fn lists_end_at_the_memory_limit() {
        let mut small = limits();
        small.engine.memory_bytes = 1024 * 1024;
        let page = "<b></b>".repeat(1_000);
        with_tree(
            small,
            move || parsed(&page),
            |runner| {
                ends(
                    runner,
                    "for (;;) document.querySelectorAll(\"*\");",
                    Reason::Memory,
                );
                ran(runner, "1 + 1");
            },
        );
    }

    /// 6f: deep and wide trees end `Ran` on the runner's thread.
    #[test]
    fn deep_and_wide_trees_run() {
        with_tree(
            limits(),
            || parsed(&"<div>".repeat(500)),
            |runner| {
                checked(
                    runner,
                    r##"
var all = document.getElementsByTagName("div");
var d = all[all.length - 1];
check(d.closest("span") === null && d.closest("body > div") === all[0], "closest");
check(d.matches("body div div") && !d.matches("span div"), "matches");
"##,
                );
            },
        );
        let page = format!("<p>{}</p>", "<i></i>".repeat(20_000));
        with_tree(
            limits(),
            move || parsed(&page),
            |runner| {
                checked(
                    runner,
                    r##"
var p = document.getElementsByTagName("p")[0];
check(p.querySelectorAll("p > i").length === 20000, "20,000");
"##,
                );
            },
        );
    }

    /// 15b: the parse's charges for the text's bytes and for its tokens. Measured: 4,096 bytes
    /// 795 steps, 16 bytes 30 (a difference of 765: the glue's copy 510, `selector_text` 255);
    /// 1,000 tokens 1,773, 2 tokens 775 (a difference of 998). The floor of the bytes is 700:
    /// the glue's copy of 4,080 more code units at 8 a step is 510, and the parse's 4,080 more
    /// bytes at 16 a step are 255 more, so a parse that leaves the bytes out ends under it.
    #[test]
    fn the_parse_charges_bytes_and_tokens() {
        with_tree(
            limits(),
            || parsed("<b></b>"),
            |runner| {
                checked(
                    runner,
                    r##"
G.long = "/*" + rep("x", 4096 - 5) + "*/a";
G.short = "/*" + rep("x", 11) + "*/a";
G.many = rep("/**/", 999) + "a";
G.one = "/*" + rep("x", 3992) + "*/a";
check(G.long.length === 4096 && G.short.length === 16, "the first pair");
check(G.many.length === 3997 && G.one.length === 3997, "the second pair");
"##,
                );
                let calls = [
                    call(
                        runner,
                        "15b 4,096 bytes",
                        "document.querySelector(G.long);",
                        0,
                    ),
                    call(
                        runner,
                        "15b 16 bytes",
                        "document.querySelector(G.short);",
                        0,
                    ),
                    call(
                        runner,
                        "15b 1,000 tokens",
                        "document.querySelector(G.many);",
                        0,
                    ),
                    call(runner, "15b 2 tokens", "document.querySelector(G.one);", 0),
                ];
                report("15b", &calls);
                let bytes = calls[0].steps - calls[1].steps;
                let tokens = calls[2].steps - calls[3].steps;
                eprintln!("15b: {bytes} steps more for the bytes, {tokens} for the tokens");
                assert!(bytes >= 700, "{bytes}");
                assert!(tokens >= 990, "{tokens}");
            },
        );
    }

    /// 15c: an attribute test charges the scan of the list and the name's bytes for each
    /// attribute. Measured: `[a]` over 100 bare elements 226 steps, over 100 of 1,024
    /// attributes 25,826 (25,600 more); a name of 4,000 bytes over 10 elements of 100 attributes
    /// 259,304, and with `="v"` 259,310.
    #[test]
    fn attribute_tests_charge_the_list_and_the_name() {
        let mut calls = Vec::new();
        with_tree(
            limits(),
            || flat(100, 0),
            |runner| {
                calls.push(call(
                    runner,
                    "15c [a], bare",
                    "document.querySelector(\"[a]\");",
                    0,
                ));
            },
        );
        with_tree(
            limits(),
            || flat(100, 1_024),
            |runner| {
                calls.push(call(
                    runner,
                    "15c [a], 1,024 attributes",
                    "document.querySelector(\"[a]\");",
                    0,
                ));
            },
        );
        with_tree(
            limits(),
            || flat(10, 100),
            |runner| {
                checked(runner, "G.name = rep(\"n\", 4000);");
                calls.push(call(
                    runner,
                    "15c [name of 4,000 bytes]",
                    "document.querySelector(\"[\" + G.name + \"]\");",
                    250_000,
                ));
                calls.push(call(
                    runner,
                    "15c [name of 4,000 bytes=\"v\"]",
                    "document.querySelector(\"[\" + G.name + \"=\\\"v\\\"]\");",
                    250_000,
                ));
            },
        );
        report("15c", &calls);
        let scans = calls[1].steps - calls[0].steps;
        assert!(scans >= 100 * 256, "{scans}");
    }

    /// 15d: a class test charges the value's bytes and its tokens, in quirks mode too: measured
    /// 3,686,625 and 3,686,624 steps.
    #[test]
    fn class_tests_charge_bytes_and_tokens() {
        let mut calls = Vec::new();
        for (doctype, what) in [(true, "15d .a"), (false, "15d .a in quirks mode")] {
            let value = "b ".repeat(32_768);
            with_tree(
                limits(),
                move || flat_with(100, &[("class", value.as_str())], doctype),
                |runner| {
                    calls.push(call(
                        runner,
                        what,
                        "document.querySelector(\".a\");",
                        100 * (4_096 + 32_768),
                    ));
                },
            );
        }
        report("15d", &calls);
    }

    /// 15e: an equality test charges the value it compares and no other: measured 409,828 steps.
    #[test]
    fn an_equality_test_charges_its_value_alone() {
        let value = "x".repeat(65_536);
        with_tree(
            limits(),
            move || flat_with(100, &[("a", value.as_str()), ("b", value.as_str())], true),
            |runner| {
                let calls = [call(
                    runner,
                    "15e [a=\"v\"]",
                    "document.querySelector('[a=\"v\"]');",
                    409_600,
                )];
                report("15e", &calls);
                under(&calls[0], 410_600);
            },
        );
    }

    /// 15f: the fold of a name is charged once, at the parse, and the name's bytes at each
    /// element the type test reads: measured 1,774 steps for the name, 775 for the comment form
    /// (999 more), 1,774 again for the same name.
    #[test]
    fn a_type_name_is_charged_at_the_parse_and_at_each_test() {
        with_tree(
            limits(),
            || parsed(""),
            |runner| {
                checked(
                    runner,
                    "G.name = rep(\"a\", 4000); G.comment = \"/*\" + rep(\"x\", 3995) + \"*/a\";\ncheck(G.comment.length === 4000, \"4,000\");",
                );
                let calls = [
                    call(runner, "15f a name", "document.querySelector(G.name);", 0),
                    call(
                        runner,
                        "15f a comment",
                        "document.querySelector(G.comment);",
                        0,
                    ),
                    call(
                        runner,
                        "15f the name again",
                        "document.querySelector(G.name);",
                        0,
                    ),
                ];
                report("15f", &calls);
                let more = calls[0].steps - calls[1].steps;
                assert!(more >= 990, "{more}");
                assert_eq!(calls[0].steps, calls[2].steps);
            },
        );
    }

    /// 15g: the memo is made late: nothing past the gate for an element that fails it. Measured
    /// over 1,003 elements: `zzz` 2,023 steps, `x y` 2,025, `x div` 11,025 (9,002 more).
    #[test]
    fn the_memo_is_made_late() {
        let page = "<div></div>".repeat(1_000);
        with_tree(
            limits(),
            move || parsed(&page),
            |runner| {
                let calls = [
                    call(runner, "15g zzz", "document.querySelector(\"zzz\");", 2_006),
                    call(runner, "15g x y", "document.querySelector(\"x y\");", 2_006),
                    call(runner, "15g x div", "document.querySelector(\"x div\");", 0),
                ];
                report("15g", &calls);
                under(&calls[0], 2_100);
                under(&calls[1], 2_100);
                let more = calls[2].steps - calls[0].steps;
                assert!(more >= 7_500, "{more}");
            },
        );
    }

    /// 17a: `Element.querySelector` from `body`: `[zzz]` over 5,000 elements of 100 attributes,
    /// measured 135,018 steps; the ten compounds of 6b over the deep page, measured 7,208.
    #[test]
    fn element_query_selector() {
        let mut calls = Vec::new();
        with_tree(
            limits(),
            || flat(5_000, 100),
            |runner| {
                ran(runner, SETUP);
                calls.push(call(
                    runner,
                    "17a Element.querySelector [zzz]",
                    "G.body.querySelector(\"[zzz]\");",
                    135_000,
                ));
            },
        );
        with_tree(limits(), nested, |runner| {
            ran(runner, SETUP);
            calls.push(call(
                runner,
                "17a Element.querySelector, ten compounds",
                &format!("G.r = G.body.querySelector({TEN:?});"),
                7_000,
            ));
            checked(runner, "check(G.r === null, \"null\");");
        });
        report("17a", &calls);
    }

    /// 17b: `Element.querySelectorAll("*")` from `body` over 50,000 elements: measured 500,021
    /// steps.
    #[test]
    fn element_query_selector_all() {
        with_tree(
            limits(),
            || flat(50_000, 0),
            |runner| {
                ran(runner, SETUP);
                let calls = [call(
                    runner,
                    "17b Element.querySelectorAll *",
                    "G.r = G.body.querySelectorAll(\"*\");",
                    500_000,
                )];
                checked(runner, "check(G.r.length === 50000, \"50,000\");");
                report("17b", &calls);
            },
        );
    }

    /// 17c: `Element.closest` of the ten compounds from the `span` of the deep page, measured
    /// 7,211 steps; and of 512 `div` from the innermost of 500 nested `div`, which ends at the
    /// step limit of 1,000,000 in its fourth candidate.
    #[test]
    fn element_closest() {
        let mut calls = Vec::new();
        with_tree(limits(), nested, |runner| {
            ran(runner, SETUP);
            checked(
                runner,
                "G.span = document.getElementsByTagName(\"span\")[0];",
            );
            calls.push(call(
                runner,
                "17c Element.closest, ten compounds",
                &format!("G.r = G.span.closest({TEN:?});"),
                7_000,
            ));
            checked(runner, "check(G.r === null, \"null\");");
        });
        report("17c", &calls);
        with_tree(
            limits_of(1_000_000),
            || parsed(&"<div>".repeat(500)),
            |runner| {
                checked(
                    runner,
                    "var all = document.getElementsByTagName(\"div\"); G.d = all[all.length - 1];",
                );
                let selector = ["div"; 512].join(" ");
                ends(
                    runner,
                    &format!("G.d.closest({selector:?});"),
                    Reason::StepsPerScript,
                );
                ran(runner, "1 + 1");
            },
        );
    }

    /// The tree of 17d and 17g: one `b` with the attributes `a`, `b`, `c` and 1,021 more.
    fn three_of_many() -> Tree {
        let mut tree = parsed("<!DOCTYPE html>");
        let body = elements_named(&tree, "body")[0];
        let b = tree
            .create_element_ns(Namespace::Html, "b")
            .expect("an element");
        for name in ["a", "b", "c"] {
            tree.set_attribute_ns(b, None, name, "")
                .expect("an attribute");
        }
        for at in 0..1_021 {
            tree.set_attribute_ns(b, None, &format!("x{at}"), "")
                .expect("an attribute");
        }
        tree.append_child(body, b).expect("appended");
        tree
    }

    /// 17d: `Element.matches("[a][b][c]")` of an element of 1,024 attributes: measured 800 steps.
    #[test]
    fn element_matches() {
        with_tree(limits(), three_of_many, |runner| {
            ran(runner, SETUP);
            let calls = [call(
                runner,
                "17d Element.matches [a][b][c]",
                "G.r = G.b.matches(\"[a][b][c]\");",
                3 * 256,
            )];
            checked(runner, "check(G.r === true, \"true\");");
            report("17d", &calls);
        });
    }

    /// 17e: `Document.querySelector` of a list of 512 selectors `a` over 1,000 `b`: measured
    /// 515,769 steps; over 10,000 under a step limit of 1,000,000 it ends at the limit.
    #[test]
    fn a_list_of_512() {
        let list = ["a"; 512].join(",");
        let source = format!("document.querySelector({list:?});");
        let looped = source.clone();
        with_tree(
            limits(),
            || flat(1_000, 0),
            |runner| {
                let calls = [call(
                    runner,
                    "17e Document.querySelector a,a,...,a",
                    &source,
                    1_000 * 512,
                )];
                report("17e", &calls);
            },
        );
        with_tree(
            limits_of(1_000_000),
            || flat(10_000, 0),
            |runner| {
                ends(runner, &looped, Reason::StepsPerScript);
            },
        );
    }

    /// 17f: a text at the byte bound made of the most tokens: every byte one token (single
    /// spaces, one-letter identifiers, one-character delimiters), 4,096 tokens and 342 simple
    /// selectors: measured 4,907 steps.
    #[test]
    fn a_text_of_the_most_tokens() {
        let mut text = ["[ a = b ]"; 341].join(" > ");
        text.push_str(" > [ a ");
        assert_eq!(text.len(), 4096);
        let tokens = u64::try_from(text.len()).expect("a count");
        let source = format!("document.querySelector({text:?});");
        with_tree(
            limits(),
            || flat(10, 0),
            |runner| {
                let calls = [call(
                    runner,
                    &format!("17f Document.querySelector, {tokens} tokens"),
                    &source,
                    514 + tokens,
                )];
                report("17f", &calls);
            },
        );
    }

    /// 17g: the five other members. Measured: `Document.querySelector("[zzz]")` 135,026 steps,
    /// `Document.querySelectorAll("*")` 500,048, `DocumentFragment.querySelector("[zzz]")`
    /// 27,018, `DocumentFragment.querySelectorAll("*")` 100,021,
    /// `Element.webkitMatchesSelector("[a][b][c]")` 795.
    #[test]
    fn the_other_members() {
        let mut calls = Vec::new();
        with_tree(
            limits(),
            || flat(5_000, 100),
            |runner| {
                calls.push(call(
                    runner,
                    "17g Document.querySelector [zzz]",
                    "document.querySelector(\"[zzz]\");",
                    135_000,
                ));
            },
        );
        with_tree(
            limits(),
            || flat(50_000, 0),
            |runner| {
                calls.push(call(
                    runner,
                    "17g Document.querySelectorAll *",
                    "document.querySelectorAll(\"*\");",
                    500_000,
                ));
            },
        );
        with_fragment(1_000, 100, |runner| {
            ran(runner, SETUP);
            ran(runner, "G.f = document.createDocumentFragment();");
            calls.push(call(
                runner,
                "17g DocumentFragment.querySelector [zzz]",
                "G.f.querySelector(\"[zzz]\");",
                27_000,
            ));
        });
        with_fragment(10_000, 0, |runner| {
            ran(runner, SETUP);
            ran(runner, "G.f = document.createDocumentFragment();");
            calls.push(call(
                runner,
                "17g DocumentFragment.querySelectorAll *",
                "G.r = G.f.querySelectorAll(\"*\");",
                100_000,
            ));
            checked(runner, "check(G.r.length === 10000, \"10,000\");");
        });
        with_tree(limits(), three_of_many, |runner| {
            ran(runner, SETUP);
            calls.push(call(
                runner,
                "17g Element.webkitMatchesSelector [a][b][c]",
                "G.b.webkitMatchesSelector(\"[a][b][c]\");",
                768,
            ));
        });
        report("17g", &calls);
    }

    /// 17h: the memo of 100 compounds at chains of 3 to 102 elements: measured 596,540 steps;
    /// without the memo's charge about 530,900.
    #[test]
    fn the_memo_of_a_hundred_compounds() {
        let selector = format!("{}div", "x > ".repeat(99));
        let source = format!("G.r = G.body.querySelector({selector:?});");
        with_tree(
            limits(),
            || parsed(&"<div>".repeat(100)),
            |runner| {
                ran(runner, SETUP);
                let calls = [call(
                    runner,
                    "17h Element.querySelector x > ... > div",
                    &source,
                    560_000,
                )];
                checked(runner, "check(G.r === null, \"null\");");
                report("17h", &calls);
                under(&calls[0], 650_000);
            },
        );
    }

    /// 17i: the further simple selectors of one compound of 512 over 1,000 `<b a>`: measured
    /// 514,844 steps; without the 511 further steps about 3,800.
    #[test]
    fn the_further_simple_selectors_of_a_compound() {
        let selector = format!("{}[zzz]", "[a]".repeat(511));
        let source = format!("G.r = G.body.querySelector({selector:?});");
        with_tree(
            limits(),
            || flat_with(1_000, &[("a", "")], true),
            |runner| {
                ran(runner, SETUP);
                let calls = [call(
                    runner,
                    "17i Element.querySelector [a]...[zzz]",
                    &source,
                    500_000,
                )];
                checked(runner, "check(G.r === null, \"null\");");
                report("17i", &calls);
                under(&calls[0], 520_000);
            },
        );
    }
}
