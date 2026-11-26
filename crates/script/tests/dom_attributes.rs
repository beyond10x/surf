//! The tests of the slice `attributes` (`story:scripts-dom-attributes`): `Element`'s `id`,
//! `className` and `slot` and its 11 attribute operations over a parsed tree, in a runner with
//! the generated bindings and surf's own slices.
//!
//! Each row of `crates/webidl/selection.txt` with the slice `attributes` has entries in
//! [`CASES`], each a JavaScript expression and the value the DOM Standard gives it, compared
//! with `===` in a runner of its own over [`PAGE`]. The other tests hold what a single value
//! does not show: the exceptions leave the tree and the charges as they were, the limits of the
//! tree end the script, the writes the reading members see, the price of each call
//! (`budget_row`), and the memory charge of the bytes a script writes into nodes that exist
//! (`memory_first` and the cases beside it).

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use surf_html::limits::{ATTRIBUTE_BYTES, Limits as ParserLimits};
use surf_html::{Attribute, Namespace, NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{
    Document, Mode, Reason, Runner, RunnerLimits, SESSION_ENGINE_LIMITS, ScriptEnd, run_document,
};
use swell::Limits;

/// The limits of the table and of the tests that name no other: a memory budget of 64 MiB and
/// a `string_length` of 1 Mi units.
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

/// The engine's limits of a session (`SESSION_ENGINE_LIMITS`: 10,000,000 steps an entry, a
/// memory budget of 67,108,864 bytes, a `string_length` of 16,777,216 units), with the runner's
/// own room of [`limits`].
fn session() -> RunnerLimits {
    RunnerLimits {
        engine: SESSION_ENGINE_LIMITS,
        ..limits()
    }
}

/// The document of `page`, parsed under `parse`, with the same limits for its writes and
/// surf's own slices.
fn document(page: &str, parse: ParserLimits) -> Document {
    Document {
        tree: parse_document(
            page,
            &ParseOptions {
                limits: parse,
                ..ParseOptions::default()
            },
        ),
        limits: parse,
        slices: dom::slices(DocumentInfo::default()),
    }
}

/// Runs `body` with a runner under `limits` over `page`, parsed under `parse`.
fn with_page<R: Send>(
    limits: RunnerLimits,
    page: String,
    parse: ParserLimits,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(limits, Mode::Test, move || document(&page, parse), body)
        .expect("a runner with the bindings")
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
    let end = runner.evaluate(&format!("{CHECK}{source}"));
    assert_eq!(end, ScriptEnd::Ran, "{end}");
    assert_eq!(recorded(runner), None);
}

/// The steps `source` adds to the runner's total, after it ran.
fn steps(runner: &mut Runner, source: &str) -> u64 {
    let before = runner.steps();
    ran(runner, source);
    runner.steps() - before
}

fn heap(runner: &mut Runner) -> u64 {
    runner.agent_and_realm().0.heap_bytes().expect("heap_bytes")
}

/// The functions every script of a test starts with.
const CHECK: &str = r#"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
function threw(f) { try { f(); } catch (e) { return e.name; } return "no throw"; }
"#;

/// What the bindings and the tree hold, read between two scripts.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Figures {
    live: u64,
    charged: u64,
    handles: usize,
    node_charges: usize,
    totals: (u64, u64),
    stored: usize,
    writes: u64,
}

fn figures(runner: &mut Runner) -> Figures {
    let state = runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .expect("the bindings");
    let totals = state.totals();
    Figures {
        live: state.stored_live(),
        charged: state.stored_charged(),
        handles: state.stored_handles(),
        node_charges: state.node_charges(),
        totals: (totals.nodes, totals.bytes),
        stored: state.tree().stored_bytes(),
        writes: state.tree().writes(),
    }
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

/// The elements of `tree` whose local name is `name`, in tree order.
fn elements(tree: &Tree, name: &str) -> Vec<NodeId> {
    tree.descendants(tree.root())
        .into_iter()
        .filter(|&node| matches!(tree.data(node), NodeData::Element { local_name, .. } if local_name == name))
        .collect()
}

fn element(tree: &Tree, name: &str) -> NodeId {
    elements(tree, name)[0]
}

fn attributes(tree: &Tree, node: NodeId) -> &[Attribute] {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes,
        _ => panic!("an element"),
    }
}

/// The room of the attribute list of `node`, in records.
fn room_of(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes.capacity(),
        _ => panic!("an element"),
    }
}

/// The change of the stored live bytes a write makes beside the attribute's own bytes, from
/// the list's room before and after it: a list that grew takes the records beyond the new
/// one, a list that shrank gives back those beyond the removed one.
fn room_change(before: usize, after: usize) -> i64 {
    let records = if after > before {
        (after - before - 1) as i64
    } else if after < before {
        -((before - after - 1) as i64)
    } else {
        0
    };
    records * ATTRIBUTE_BYTES as i64
}

/// The bytes of room beyond the new records that `adds` adds take on a list of `len` records
/// with room for `capacity`, by the tree's rule: one record at a time below four, then as many
/// again as the list holds.
fn room_beyond(len: usize, capacity: usize, adds: usize) -> u64 {
    let (mut len, mut capacity, mut beyond) = (len, capacity, 0);
    for _ in 0..adds {
        if len == capacity {
            let grown = if len < 4 { len + 1 } else { 2 * len };
            beyond += grown - capacity - 1;
            capacity = grown;
        }
        len += 1;
    }
    (beyond * ATTRIBUTE_BYTES) as u64
}

/// The value of the attribute `name` in no namespace of `node`.
fn value<'t>(tree: &'t Tree, node: NodeId, name: &str) -> Option<&'t str> {
    attributes(tree, node)
        .iter()
        .find(|a| a.namespace.is_none() && a.local_name == name)
        .map(|a| a.value.as_str())
}

/// An entry of the table: a row of `selection.txt` as `<interface>.<member>`, a JavaScript
/// expression and the value the standard gives it, as JavaScript text.
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

/// The page of the table: two HTML elements, one with four attributes, an SVG element whose
/// `viewBox` the parser keeps in mixed case and an SVG `use` with an `xlink:href`.
const PAGE: &str = "<!DOCTYPE html><html><body>\
<p id=\"a\" class=\"k\" slot=\"s\" data-x=\"1\">t</p><p id=\"b\"></p>\
<svg id=\"g\" viewBox=\"0 0 1 1\"><use id=\"u\" xlink:href=\"#a\"></use></svg></body></html>";

/// The names of the table's expressions.
const PRELUDE: &str = r#"
var html = document.documentElement;
var a = document.getElementById("a");
var b = document.getElementById("b");
var g = document.getElementById("g");
var u = document.getElementById("u");
var XLINK = "http://www.w3.org/1999/xlink";
var SVG = "http://www.w3.org/2000/svg";
var XMLNS = "http://www.w3.org/2000/xmlns/";
var XML = "http://www.w3.org/XML/1998/namespace";
"#;

/// The table. Each entry's comment names the section of the DOM Standard whose value it
/// compares.
const CASES: &[Case] = &[
    // "Interface Element", id: reflects the content attribute `id` ("get an attribute value",
    // the empty string when absent; "set an attribute value").
    case("Element.id", "a.id", "\"a\""),
    case("Element.id", "html.id", "\"\""),
    case(
        "Element.id",
        "(b.id = \"z\", b.getAttribute(\"id\"))",
        "\"z\"",
    ),
    case(
        "Element.id",
        "(b.id = \"z\", document.getElementById(\"z\") === b && document.getElementById(\"b\") === null)",
        "true",
    ),
    case("Element.id", "(html.id = \"h\", html.id)", "\"h\""),
    // className reflects `class`.
    case("Element.className", "a.className", "\"k\""),
    case("Element.className", "html.className", "\"\""),
    case(
        "Element.className",
        "(b.className = \"c d\", b.getAttribute(\"class\"))",
        "\"c d\"",
    ),
    // slot reflects `slot`.
    case("Element.slot", "a.slot", "\"s\""),
    case("Element.slot", "b.slot", "\"\""),
    case(
        "Element.slot",
        "(b.slot = \"x\", b.getAttribute(\"slot\"))",
        "\"x\"",
    ),
    // hasAttributes: whether the attribute list is not empty.
    case("Element.hasAttributes", "a.hasAttributes()", "true"),
    case("Element.hasAttributes", "html.hasAttributes()", "false"),
    // getAttributeNames: the qualified names in order.
    case(
        "Element.getAttributeNames",
        "a.getAttributeNames().join()",
        "\"id,class,slot,data-x\"",
    ),
    case(
        "Element.getAttributeNames",
        "u.getAttributeNames().join()",
        "\"id,xlink:href\"",
    ),
    case(
        "Element.getAttributeNames",
        "(a.setAttribute(\"class\", \"m\"), a.getAttributeNames().join())",
        "\"id,class,slot,data-x\"",
    ),
    case(
        "Element.getAttributeNames",
        "html.getAttributeNames().length",
        "0",
    ),
    // getAttribute: "get an attribute by name", the name lowered on an HTML element.
    case("Element.getAttribute", "a.getAttribute(\"ID\")", "\"a\""),
    case("Element.getAttribute", "a.getAttribute(\"none\")", "null"),
    case(
        "Element.getAttribute",
        "g.getAttribute(\"viewBox\")",
        "\"0 0 1 1\"",
    ),
    case(
        "Element.getAttribute",
        "g.getAttribute(\"viewbox\")",
        "null",
    ),
    case(
        "Element.getAttribute",
        "u.getAttribute(\"xlink:href\")",
        "\"#a\"",
    ),
    case(
        "Element.getAttribute",
        "(a.setAttributeNS(null, \"FOO\", \"1\"), a.getAttribute(\"FOO\"))",
        "null",
    ),
    // getAttributeNS: "get an attribute by namespace and local name", the empty namespace null.
    case(
        "Element.getAttributeNS",
        "u.getAttributeNS(XLINK, \"href\")",
        "\"#a\"",
    ),
    case(
        "Element.getAttributeNS",
        "u.getAttributeNS(null, \"href\")",
        "null",
    ),
    case(
        "Element.getAttributeNS",
        "a.getAttributeNS(null, \"id\")",
        "\"a\"",
    ),
    case(
        "Element.getAttributeNS",
        "a.getAttributeNS(\"\", \"id\")",
        "\"a\"",
    ),
    case(
        "Element.getAttributeNS",
        "(a.setAttributeNS(null, \"FOO\", \"1\"), a.getAttributeNS(null, \"FOO\"))",
        "\"1\"",
    ),
    case(
        "Element.getAttributeNS",
        "a.getAttributeNS(\"urn:x\", \"id\")",
        "null",
    ),
    // setAttribute: validate the name, lower it on an HTML element, set or append.
    case(
        "Element.setAttribute",
        "(a.setAttribute(\"DATA-X\", \"2\"), a.getAttribute(\"data-x\"))",
        "\"2\"",
    ),
    case(
        "Element.setAttribute",
        "(b.setAttribute(\"DATA-Y\", \"1\"), b.getAttributeNames().join())",
        "\"id,data-y\"",
    ),
    case(
        "Element.setAttribute",
        "(a.setAttribute(\"id\", \"q\"), a.getAttributeNames().join() + \" \" + a.id)",
        "\"id,class,slot,data-x q\"",
    ),
    case(
        "Element.setAttribute",
        "(a.setAttribute(\"a:b\", \"v\"), a.getAttribute(\"a:b\"))",
        "\"v\"",
    ),
    case(
        "Element.setAttribute",
        "(g.setAttribute(\"Mixed\", \"v\"), g.getAttributeNames().join())",
        "\"id,viewBox,Mixed\"",
    ),
    case(
        "Element.setAttribute",
        "threw(function () { a.setAttribute(\"\", \"v\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.setAttribute",
        "threw(function () { a.setAttribute(\"a b\", \"v\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.setAttribute",
        "threw(function () { a.setAttribute(\"a=b\", \"v\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.setAttribute",
        "threw(function () { a.setAttribute(\"a/b\", \"v\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.setAttribute",
        "threw(function () { a.setAttribute(\"a>b\", \"v\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.setAttribute",
        "threw(function () { a.setAttribute(\"a\\u0000\", \"v\"); })",
        "\"InvalidCharacterError\"",
    ),
    // setAttributeNS: validate and extract, then set or append by namespace and local name.
    case(
        "Element.setAttributeNS",
        "(a.setAttributeNS(null, \"FOO\", \"1\"), a.hasAttribute(\"foo\"))",
        "false",
    ),
    case(
        "Element.setAttributeNS",
        "(g.setAttributeNS(XLINK, \"xlink:title\", \"t\"), g.getAttributeNS(XLINK, \"title\") + \" \" + g.getAttribute(\"xlink:title\"))",
        "\"t t\"",
    ),
    case(
        "Element.setAttributeNS",
        "(u.setAttributeNS(XLINK, \"other:href\", \"#b\"), u.getAttributeNames().join() + \" \" + u.getAttribute(\"xlink:href\"))",
        "\"id,xlink:href #b\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(SVG, \":a\", \"v\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(SVG, \"a b\", \"v\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(SVG, \"x:a b\", \"v\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(null, \"p:a\", \"v\"); })",
        "\"NamespaceError\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(\"\", \"p:a\", \"v\"); })",
        "\"NamespaceError\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(SVG, \"xml:a\", \"v\"); })",
        "\"NamespaceError\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(SVG, \"xmlns\", \"v\"); })",
        "\"NamespaceError\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(SVG, \"xmlns:a\", \"v\"); })",
        "\"NamespaceError\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(XMLNS, \"a\", \"v\"); })",
        "\"NamespaceError\"",
    ),
    case(
        "Element.setAttributeNS",
        "(a.setAttributeNS(XML, \"xml:lang\", \"en\"), a.getAttributeNS(XML, \"lang\"))",
        "\"en\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(\"urn:x\", \"a\", \"v\"); })",
        "\"NotSupportedError\"",
    ),
    case(
        "Element.setAttributeNS",
        "threw(function () { a.setAttributeNS(\"urn:x\", \"xml:a\", \"v\"); })",
        "\"NamespaceError\"",
    ),
    // removeAttribute: "remove an attribute by name".
    case(
        "Element.removeAttribute",
        "(a.removeAttribute(\"ID\"), a.hasAttribute(\"id\") + \" \" + a.getAttributeNames().join())",
        "\"false class,slot,data-x\"",
    ),
    case(
        "Element.removeAttribute",
        "(a.removeAttribute(\"none\"), a.getAttributeNames().length)",
        "4",
    ),
    case(
        "Element.removeAttribute",
        "a.removeAttribute(\"none\")",
        "undefined",
    ),
    // removeAttributeNS: "remove an attribute by namespace and local name".
    case(
        "Element.removeAttributeNS",
        "(u.removeAttributeNS(XLINK, \"href\"), u.hasAttribute(\"xlink:href\"))",
        "false",
    ),
    case(
        "Element.removeAttributeNS",
        "(a.removeAttributeNS(null, \"id\"), a.hasAttribute(\"id\"))",
        "false",
    ),
    case(
        "Element.removeAttributeNS",
        "(a.removeAttributeNS(\"urn:x\", \"id\"), a.getAttributeNames().length)",
        "4",
    ),
    case(
        "Element.removeAttributeNS",
        "a.removeAttributeNS(\"urn:x\", \"id\")",
        "undefined",
    ),
    // toggleAttribute: validate the name, then remove, add with the empty value, or neither.
    case(
        "Element.toggleAttribute",
        "a.toggleAttribute(\"ID\")",
        "false",
    ),
    case(
        "Element.toggleAttribute",
        "(a.toggleAttribute(\"ID\"), a.hasAttribute(\"id\"))",
        "false",
    ),
    case(
        "Element.toggleAttribute",
        "b.toggleAttribute(\"Hidden\")",
        "true",
    ),
    case(
        "Element.toggleAttribute",
        "(b.toggleAttribute(\"Hidden\"), b.getAttribute(\"hidden\"))",
        "\"\"",
    ),
    case(
        "Element.toggleAttribute",
        "a.toggleAttribute(\"id\", true)",
        "true",
    ),
    case(
        "Element.toggleAttribute",
        "(a.toggleAttribute(\"id\", true), a.getAttribute(\"id\"))",
        "\"a\"",
    ),
    case(
        "Element.toggleAttribute",
        "b.toggleAttribute(\"x\", true)",
        "true",
    ),
    case(
        "Element.toggleAttribute",
        "a.toggleAttribute(\"x\", false)",
        "false",
    ),
    case(
        "Element.toggleAttribute",
        "(a.toggleAttribute(\"id\", false), a.hasAttribute(\"id\"))",
        "false",
    ),
    case(
        "Element.toggleAttribute",
        "threw(function () { a.toggleAttribute(\"\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.toggleAttribute",
        "threw(function () { a.toggleAttribute(\" a\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.toggleAttribute",
        "threw(function () { a.toggleAttribute(\"a=\", false); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.toggleAttribute",
        "threw(function () { a.toggleAttribute(\"/\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.toggleAttribute",
        "threw(function () { a.toggleAttribute(\">\"); })",
        "\"InvalidCharacterError\"",
    ),
    case(
        "Element.toggleAttribute",
        "threw(function () { a.toggleAttribute(\"\\u0000\"); })",
        "\"InvalidCharacterError\"",
    ),
    // hasAttribute: "get an attribute by name" is not null.
    case("Element.hasAttribute", "a.hasAttribute(\"CLASS\")", "true"),
    case("Element.hasAttribute", "a.hasAttribute(\"none\")", "false"),
    case(
        "Element.hasAttribute",
        "g.hasAttribute(\"viewbox\")",
        "false",
    ),
    case(
        "Element.hasAttribute",
        "u.hasAttribute(\"xlink:href\")",
        "true",
    ),
    // hasAttributeNS: "get an attribute by namespace and local name" is not null.
    case(
        "Element.hasAttributeNS",
        "u.hasAttributeNS(XLINK, \"href\")",
        "true",
    ),
    case(
        "Element.hasAttributeNS",
        "u.hasAttributeNS(null, \"href\")",
        "false",
    ),
    case(
        "Element.hasAttributeNS",
        "a.hasAttributeNS(\"\", \"class\")",
        "true",
    ),
    case(
        "Element.hasAttributeNS",
        "a.hasAttributeNS(\"urn:x\", \"id\")",
        "false",
    ),
];

/// Runs one entry: a runner of its own over [`PAGE`], `expression === expected` thrown on a
/// mismatch, the end `Ran`, and no limit recorded.
fn run_case(case: &Case) {
    let source = format!(
        "{CHECK}{PRELUDE}\nvar got = ({});\nif (got !== ({})) {{ throw new Error({:?} + \": \" + String(got)); }}",
        case.expression, case.expected, case.row
    );
    run_document(
        limits(),
        Mode::Test,
        || document(PAGE, ParserLimits::default()),
        |runner| {
            let end = runner.evaluate(&source);
            assert_eq!(
                end,
                ScriptEnd::Ran,
                "{} `{}`: {end}",
                case.row,
                case.expression
            );
            assert_eq!(recorded(runner), None, "{}", case.row);
        },
    )
    .expect("a runner");
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

#[test]
fn cases_are_the_rows() {
    let rows: BTreeSet<String> = selection()
        .into_iter()
        .filter(|(_, _, slice, _)| slice == "attributes")
        .map(|(interface, member, _, _)| format!("{interface}.{member}"))
        .collect();
    let expected: BTreeSet<String> = [
        "Element.id",
        "Element.className",
        "Element.slot",
        "Element.hasAttributes",
        "Element.getAttributeNames",
        "Element.getAttribute",
        "Element.getAttributeNS",
        "Element.setAttribute",
        "Element.setAttributeNS",
        "Element.removeAttribute",
        "Element.removeAttributeNS",
        "Element.toggleAttribute",
        "Element.hasAttribute",
        "Element.hasAttributeNS",
    ]
    .map(str::to_string)
    .into_iter()
    .collect();
    assert_eq!(rows, expected);
    assert_eq!(rows.len(), 14);
    let named: BTreeSet<String> = CASES.iter().map(|case| case.row.to_string()).collect();
    assert_eq!(named, rows);
    for case in CASES {
        assert!(
            !case.expression.trim().is_empty() && !case.expected.trim().is_empty(),
            "{}",
            case.row
        );
    }
}

/// Runs every entry whose row is `row`; answers how many ran.
fn run_row(row: &str) -> usize {
    let mut count = 0;
    for case in CASES.iter().filter(|case| case.row == row) {
        run_case(case);
        count += 1;
    }
    count
}

#[test]
fn reflected_rows() {
    assert!(run_row("Element.id") >= 5);
    assert!(run_row("Element.className") >= 3);
    assert!(run_row("Element.slot") >= 3);
}

#[test]
fn read_rows() {
    for row in [
        "Element.hasAttributes",
        "Element.getAttributeNames",
        "Element.getAttribute",
        "Element.getAttributeNS",
        "Element.hasAttribute",
        "Element.hasAttributeNS",
    ] {
        assert!(run_row(row) >= 2, "{row}");
    }
}

#[test]
fn write_rows() {
    for row in [
        "Element.setAttribute",
        "Element.setAttributeNS",
        "Element.removeAttribute",
        "Element.removeAttributeNS",
        "Element.toggleAttribute",
    ] {
        assert!(run_row(row) >= 3, "{row}");
    }
}

/// The calls that throw (acceptance 1d and 1e): each throws its exception and leaves the tree,
/// the node totals, the stored live bytes and the stored charge as they were.
#[test]
fn exceptions_leave_the_tree_and_the_charges() {
    let throws = [
        ("a.setAttribute(\"\", \"v\")", "InvalidCharacterError"),
        ("a.setAttribute(\"a b\", \"v\")", "InvalidCharacterError"),
        ("a.setAttribute(\"a=\", \"v\")", "InvalidCharacterError"),
        ("a.setAttribute(\"/\", \"v\")", "InvalidCharacterError"),
        ("a.setAttribute(\">\", \"v\")", "InvalidCharacterError"),
        (
            "a.setAttribute(\"\\u0000\", \"v\")",
            "InvalidCharacterError",
        ),
        ("a.toggleAttribute(\"\")", "InvalidCharacterError"),
        ("a.toggleAttribute(\"a b\")", "InvalidCharacterError"),
        ("a.toggleAttribute(\"a=\")", "InvalidCharacterError"),
        ("a.toggleAttribute(\"/\")", "InvalidCharacterError"),
        ("a.toggleAttribute(\">\")", "InvalidCharacterError"),
        ("a.toggleAttribute(\"\\u0000\")", "InvalidCharacterError"),
        (
            "a.setAttributeNS(SVG, \":a\", \"v\")",
            "InvalidCharacterError",
        ),
        (
            "a.setAttributeNS(SVG, \"a b\", \"v\")",
            "InvalidCharacterError",
        ),
        (
            "a.setAttributeNS(SVG, \"x:a b\", \"v\")",
            "InvalidCharacterError",
        ),
        ("a.setAttributeNS(null, \"p:a\", \"v\")", "NamespaceError"),
        ("a.setAttributeNS(\"\", \"p:a\", \"v\")", "NamespaceError"),
        ("a.setAttributeNS(SVG, \"xml:a\", \"v\")", "NamespaceError"),
        ("a.setAttributeNS(SVG, \"xmlns\", \"v\")", "NamespaceError"),
        (
            "a.setAttributeNS(SVG, \"xmlns:a\", \"v\")",
            "NamespaceError",
        ),
        ("a.setAttributeNS(XMLNS, \"a\", \"v\")", "NamespaceError"),
        (
            "a.setAttributeNS(\"urn:x\", \"a\", \"v\")",
            "NotSupportedError",
        ),
        (
            "a.setAttributeNS(\"urn:x\", \"xml:a\", \"v\")",
            "NamespaceError",
        ),
    ];
    with_page(
        limits(),
        PAGE.to_string(),
        ParserLimits::default(),
        |runner| {
            ran(runner, PRELUDE);
            // A write first, so that the stored charge is above 0 and a throw that moved it shows.
            ran(runner, "a.setAttribute(\"w\", \"written\");");
            for (call, name) in throws {
                let before = figures(runner);
                let tree_before = tree(runner).clone();
                ran(
                    runner,
                    &format!(
                        "check(threw(function () {{ {call}; }}) === {name:?}, {call:?} + \": \" + threw(function () {{ {call}; }}));"
                    ),
                );
                assert_eq!(figures(runner), before, "{call}");
                assert_eq!(tree(runner), &tree_before, "{call}");
            }
            // The message of the namespace outside the six names the member.
            ran(
                runner,
                r#"var m = null;
try { a.setAttributeNS("urn:x", "a", "v"); } catch (e) { m = e.message; }
check(m === "Element.setAttributeNS is not supported", m);"#,
            );
        },
    );
}

/// A namespace outside the six (acceptance 1e): `getAttributeNS`, `hasAttributeNS` and
/// `removeAttributeNS` answer `null`, `false` and `undefined` with no scan, so the same calls
/// on an element of 4 attributes and on one of 1,024 cost the same steps.
#[test]
fn a_namespace_outside_the_six_scans_nothing() {
    let names = |count: usize| -> String {
        (0..count)
            .map(|n| format!(" a{n}=\"\""))
            .collect::<String>()
    };
    let page = format!("<i{}></i><i{}></i>", names(4), names(1_024));
    with_page(limits(), page, ParserLimits::default(), |runner| {
        ran(
            runner,
            "var list = document.getElementsByTagName(\"i\"); var e = list[0];",
        );
        let calls = [
            "check(e.getAttributeNS(\"urn:x\", \"a0\") === null, \"null\");",
            "check(e.hasAttributeNS(\"urn:x\", \"a0\") === false, \"false\");",
            "check(e.removeAttributeNS(\"urn:x\", \"a0\") === undefined, \"undefined\");",
        ];
        let small: Vec<u64> = calls.iter().map(|call| steps(runner, call)).collect();
        ran(runner, "e = list[1];");
        let large: Vec<u64> = calls.iter().map(|call| steps(runner, call)).collect();
        eprintln!("a namespace outside the six: 4 attributes {small:?}, 1,024 {large:?}");
        assert_eq!(small, large);
        let tree = tree(runner);
        assert_eq!(attributes(tree, elements(tree, "i")[1]).len(), 1_024);
    });
}

/// The brand check (acceptance 1g): a member called on an object of another brand throws a
/// `TypeError` and changes nothing.
#[test]
fn a_member_of_another_brand_throws_a_type_error() {
    let page = "<p id=\"t\">x</p>".to_string();
    with_page(limits(), page, ParserLimits::default(), |runner| {
        ran(
            runner,
            "var t = document.getElementById(\"t\"); var text = t.firstChild;",
        );
        let before = figures(runner);
        let tree_before = tree(runner).clone();
        ran(
            runner,
            r#"
var e1 = null, e2 = null, e3 = null;
try { Element.prototype.setAttribute.call(document, "a", "b"); } catch (e) { e1 = e; }
try {
  Object.getOwnPropertyDescriptor(Element.prototype, "id").set.call(document, "x");
} catch (e) { e2 = e; }
try { Element.prototype.getAttribute.call(text, "a"); } catch (e) { e3 = e; }
check(e1 instanceof TypeError, "setAttribute on the document: " + e1);
check(e2 instanceof TypeError, "the id setter on the document: " + e2);
check(e3 instanceof TypeError, "getAttribute on a text: " + e3);"#,
        );
        assert_eq!(figures(runner), before);
        assert_eq!(tree(runner), &tree_before);
    });
}

/// The messages (acceptance 1h): an argument's text is in no message, so the message of a
/// call with a name of 1 MiB is that of the same call with a short name.
#[test]
fn a_message_holds_no_argument() {
    with_page(
        session(),
        PAGE.to_string(),
        ParserLimits::default(),
        |runner| {
            ran(
                runner,
                &format!(
                    r#"{PRELUDE}
function message(f) {{ try {{ f(); }} catch (e) {{ return e.name + ": " + e.message; }} return "no throw"; }}
var long = "a".repeat(1048575) + " ";
var short = message(function () {{ a.setAttribute("a b", "v"); }});
check(long.length === 1048576, "the long name");
check(message(function () {{ a.setAttribute(long, "v"); }}) === short, "setAttribute");
check(short === "InvalidCharacterError: Element.setAttribute: the name is not a valid attribute name", short);
var longToggle = message(function () {{ a.toggleAttribute(long); }});
check(longToggle === "InvalidCharacterError: Element.toggleAttribute: the name is not a valid attribute name", longToggle);
var prefixed = "p:" + "a".repeat(1048574);
var ns = message(function () {{ a.setAttributeNS(null, "p:a", "v"); }});
check(message(function () {{ a.setAttributeNS(null, prefixed, "v"); }}) === ns, "setAttributeNS");
check(ns === "NamespaceError: Element.setAttributeNS: the namespace does not fit the name", ns);
var nsChar = message(function () {{ a.setAttributeNS(SVG, "p:" + long, "v"); }});
check(nsChar === "InvalidCharacterError: Element.setAttributeNS: the name is not a valid attribute name", nsChar);"#
                ),
            );
        },
    );
}

/// The limit of a test's own parse, given to the parse and to the document.
fn attributes_at(max: usize) -> ParserLimits {
    ParserLimits {
        max_attributes: max,
        ..ParserLimits::default()
    }
}

/// The end of `source`, after which a later script of the same runner reads that neither the
/// `catch` nor the `finally` of `source` ran, and the record of the limit was cleared.
fn ends_at_a_limit(runner: &mut Runner, source: &str, reason: Reason) {
    ran(runner, "G.flag = false;");
    let end = runner.evaluate(&format!(
        "{CHECK}try {{ {source} }} catch (e) {{ G.flag = true; }} finally {{ G.flag = true; }}"
    ));
    assert_eq!(end, ScriptEnd::Limit(reason), "{source}");
    ran(
        runner,
        "check(G.flag === false, \"a catch or a finally ran\");",
    );
}

/// The attribute limit (acceptance 2a and 2c): each writer that adds a fourth attribute where
/// three are allowed ends the script `limit: attributes`, also inside `try`, and the element
/// keeps its three attributes; a replacement at the limit runs, and so do the removals.
#[test]
fn the_attribute_limit_ends_the_script() {
    let page = "<p a=\"1\" b=\"2\" c=\"3\"></p>".to_string();
    with_page(limits(), page, attributes_at(3), |runner| {
        ran(runner, "var p = document.getElementsByTagName(\"p\")[0];");
        let p = element(tree(runner), "p");
        let three = attributes(tree(runner), p).to_vec();
        for write in [
            "p.setAttribute(\"d\", \"4\");",
            "p.setAttributeNS(null, \"d\", \"4\");",
            "p.toggleAttribute(\"d\");",
            "p.id = \"d\";",
            "p.className = \"d\";",
            "p.slot = \"d\";",
        ] {
            let charged = figures(runner).charged;
            ends_at_a_limit(runner, write, Reason::Attributes);
            assert_eq!(attributes(tree(runner), p), three.as_slice(), "{write}");
            assert_eq!(figures(runner).charged, charged, "{write}: put back");
        }
        ran(
            runner,
            "p.setAttribute(\"a\", \"one\"); p.setAttributeNS(null, \"b\", \"two\");",
        );
        assert_eq!(value(tree(runner), p, "a"), Some("one"));
        assert_eq!(value(tree(runner), p, "b"), Some("two"));
        ran(
            runner,
            "p.removeAttribute(\"none\"); p.removeAttributeNS(null, \"none\");",
        );
        ran(
            runner,
            "p.removeAttribute(\"a\"); p.removeAttributeNS(null, \"b\");",
        );
        assert_eq!(attributes(tree(runner), p).len(), 1);
    });
}

/// The stored-byte limit (acceptance 2b and 2c): just above what the page stores, a new
/// attribute and a longer value that would pass it end the script `limit: stored-bytes` with
/// the value unchanged; once a large attribute is removed the same write runs.
#[test]
fn the_stored_byte_limit_ends_the_script() {
    let page = format!("<p id=\"p\" big=\"{}\"></p>", "b".repeat(1_000));
    let stored = parse_document(&page, &ParseOptions::default()).stored_bytes();
    let parse = ParserLimits {
        max_stored_bytes: stored + 100,
        ..ParserLimits::default()
    };
    with_page(limits(), page, parse, |runner| {
        ran(
            runner,
            "var p = document.getElementById(\"p\"); var v = \"x\".repeat(200);",
        );
        let p = element(tree(runner), "p");
        let before = attributes(tree(runner), p).to_vec();
        for write in [
            "p.setAttribute(\"n\", v);",
            "p.setAttribute(\"id\", v);",
            "p.id = v;",
            "p.setAttributeNS(null, \"n\", v);",
            "p.toggleAttribute(v);",
        ] {
            let charged = figures(runner).charged;
            ends_at_a_limit(runner, write, Reason::StoredBytes);
            assert_eq!(attributes(tree(runner), p), before.as_slice(), "{write}");
            assert_eq!(figures(runner).charged, charged, "{write}: put back");
        }
        ran(
            runner,
            "p.removeAttribute(\"none\"); p.removeAttributeNS(\"\", \"none\");",
        );
        ran(
            runner,
            "p.removeAttribute(\"big\"); p.setAttribute(\"n\", v);",
        );
        assert_eq!(value(tree(runner), p, "n").map(str::len), Some(200));
        assert_eq!(value(tree(runner), p, "big"), None);
    });
}

/// The write of the epic's example 2 (acceptance 3): `href` set on a parsed link, read back
/// through the tree; an argument of an object is its `toString` (the `TrustedType` deviation
/// of both rows); a lone surrogate is stored as U+FFFD.
#[test]
fn a_link_set_by_script() {
    let page = "<a href=\"/old\">x</a>".to_string();
    with_page(limits(), page, ParserLimits::default(), |runner| {
        ran(runner, "var a = document.getElementsByTagName(\"a\")[0];");
        let link = element(tree(runner), "a");
        ran(runner, "a.setAttribute(\"href\", \"/next\");");
        assert_eq!(value(tree(runner), link, "href"), Some("/next"));
        ran(
            runner,
            "a.setAttribute(\"x\", { toString: function () { return \"y\"; } });",
        );
        assert_eq!(value(tree(runner), link, "x"), Some("y"));
        ran(
            runner,
            "a.setAttributeNS(null, \"z\", { toString: function () { return \"y\"; } });",
        );
        assert_eq!(value(tree(runner), link, "z"), Some("y"));
        ran(runner, "a.setAttribute(\"s\", \"a\\uD800b\");");
        assert_eq!(value(tree(runner), link, "s"), Some("a\u{FFFD}b"));
    });
}

/// What a write made, read through the reading members (acceptance 4 and 4a): the id and the
/// class a script set are found, the wrapper is the same object, and the kept collections of
/// the parent stay the same objects with the same nodes.
#[test]
fn the_reading_members_see_the_writes() {
    let page = "<div id=\"p\"><b id=\"old\"></b><i></i></div>".to_string();
    with_page(limits(), page, ParserLimits::default(), |runner| {
        ran(
            runner,
            r#"
var p = document.getElementById("p");
var el = document.getElementById("old");
var children = p.children;
var nodes = p.childNodes;
el.id = "z";
check(document.getElementById("z") === el, "the new id");
check(document.getElementById("old") === null, "the old id");
el.className = "k";
var found = document.getElementsByClassName("k");
check(found.length === 1 && found[0] === el, "the class");
el.setAttribute("x", "1");
check(p.children === children && p.childNodes === nodes, "the same objects");
el.removeAttribute("x");
check(p.children === children && p.childNodes === nodes, "the same objects after a removal");
el.id = "y";
check(p.children === children && children.length === 2 && children[0] === el, "the nodes");
check(nodes.length === 2 && nodes[0] === el, "the child nodes");
check(document.getElementById("y") === el, "the same wrapper");"#,
        );
    });
}

/// The tree's write counter (acceptance 4b): no writer and no reader of the slice moves it,
/// and each writer's write took effect.
#[test]
fn no_member_moves_the_write_counter() {
    let page = "<p id=\"e\" class=\"c\" slot=\"s\" r=\"1\" t=\"\"></p>".to_string();
    with_page(limits(), page, ParserLimits::default(), |runner| {
        ran(runner, "var e = document.getElementById(\"e\");");
        let writes = figures(runner).writes;
        let calls = [
            ("e.id = \"f\";", "e.getAttribute(\"id\") === \"f\""),
            (
                "e.className = \"d\";",
                "e.getAttribute(\"class\") === \"d\"",
            ),
            ("e.slot = \"t\";", "e.getAttribute(\"slot\") === \"t\""),
            (
                "e.setAttribute(\"n\", \"1\");",
                "e.getAttribute(\"n\") === \"1\"",
            ),
            (
                "e.setAttributeNS(null, \"m\", \"2\");",
                "e.getAttribute(\"m\") === \"2\"",
            ),
            ("e.removeAttribute(\"r\");", "!e.hasAttribute(\"r\")"),
            (
                "e.removeAttributeNS(null, \"t\");",
                "!e.hasAttribute(\"t\")",
            ),
            (
                "e.toggleAttribute(\"h\");",
                "e.getAttribute(\"h\") === \"\"",
            ),
            ("e.id;", "true"),
            ("e.className;", "true"),
            ("e.slot;", "true"),
            ("e.hasAttributes();", "true"),
            ("e.getAttributeNames();", "true"),
            ("e.getAttribute(\"n\");", "true"),
            ("e.getAttributeNS(null, \"n\");", "true"),
            ("e.hasAttribute(\"n\");", "true"),
            ("e.hasAttributeNS(null, \"n\");", "true"),
        ];
        for (call, effect) in calls {
            ran(runner, &format!("{call} check({effect}, {call:?});"));
            assert_eq!(figures(runner).writes, writes, "{call}");
        }
    });
}

/// `n` attributes `a0` .. `a<n-1>` with empty values, as markup.
fn numbered(prefix: &str, n: usize) -> String {
    (0..n).map(|i| format!(" {prefix}{i}=\"\"")).collect()
}

/// The bytes of the names `<prefix>0` .. `<prefix><n-1>`.
fn numbered_bytes(prefix: &str, n: usize) -> u64 {
    (0..n)
        .map(|i| (prefix.len() + i.to_string().len()) as u64)
        .sum()
}

/// Prices of a scan (acceptance 5a): a read of the last of 1,024 attributes costs at least
/// 1,024 / 4 - 4 / 4 = 255 steps more than the same read of the last of 4; and
/// `getAttributeNames` of 1,024 costs at least 1,024 + 8 x 1,024 = 9,216.
#[test]
fn a_scan_costs_a_step_per_four_attributes() {
    let page = format!(
        "<i{}></i><i{}></i><b{} id=\"x\"></b><b{} id=\"x\"></b>",
        numbered("a", 1_024),
        numbered("a", 4),
        numbered("a", 1_023),
        numbered("a", 3)
    );
    with_page(session(), page, ParserLimits::default(), |runner| {
        ran(
            runner,
            "var i = document.getElementsByTagName(\"i\"); var b = document.getElementsByTagName(\"b\");",
        );
        let reads = [
            ("getAttribute", "e.getAttribute(\"aLAST\");"),
            ("hasAttribute", "e.hasAttribute(\"aLAST\");"),
            ("getAttributeNS", "e.getAttributeNS(null, \"aLAST\");"),
            ("hasAttributeNS", "e.hasAttributeNS(null, \"aLAST\");"),
        ];
        let mut lines = Vec::new();
        for (name, read) in reads {
            ran(runner, "var e = i[0];");
            let large = steps(runner, &read.replace("LAST", "1023"));
            ran(runner, "e = i[1];");
            let small = steps(runner, &read.replace("LAST", "3"));
            lines.push(format!("{name}: 1,024 attributes {large}, 4 {small}"));
            assert!(large >= small + 255, "{name}: {large} against {small}");
        }
        ran(runner, "var e = b[0];");
        let large = steps(runner, "e.id;");
        ran(runner, "e = b[1];");
        let small = steps(runner, "e.id;");
        lines.push(format!("id: 1,024 attributes {large}, 4 {small}"));
        assert!(large >= small + 255, "id: {large} against {small}");
        ran(runner, "e = i[0];");
        let names = steps(runner, "e.getAttributeNames();");
        lines.push(format!("getAttributeNames of 1,024: {names}"));
        assert!(names >= 9_216, "getAttributeNames: {names}");
        eprintln!("{}", lines.join("\n"));
    });
}

/// Prices of a value (acceptance 5b): a read of a value of 64,000 bytes costs at least
/// 2 x (64,000 - 64) / 16 = 7,992 steps more than one of 64 bytes, and a write of it at least
/// 7,992 + 3,996 = 11,988 more.
#[test]
fn a_value_costs_a_step_per_16_bytes_each_time_it_is_copied() {
    let page = format!(
        "<i a=\"{}\"></i><i a=\"{}\"></i><b></b><b></b>",
        "x".repeat(64_000),
        "x".repeat(64)
    );
    with_page(session(), page, ParserLimits::default(), |runner| {
        ran(
            runner,
            r#"var i = document.getElementsByTagName("i"); var b = document.getElementsByTagName("b");
var big = "x".repeat(64000); var small = "x".repeat(64); var e = i[0]; var f = b[0];"#,
        );
        let large = steps(runner, "e.getAttribute(\"a\");");
        ran(runner, "e = i[1];");
        let short = steps(runner, "e.getAttribute(\"a\");");
        let set_large = steps(runner, "f.setAttribute(\"a\", big);");
        ran(runner, "f = b[1];");
        let set_short = steps(runner, "f.setAttribute(\"a\", small);");
        eprintln!(
            "a value of 64,000 bytes: read {large}, of 64: {short}; set {set_large}, {set_short}"
        );
        assert!(large >= short + 7_992, "read: {large} against {short}");
        assert!(
            set_large >= set_short + 11_988,
            "set: {set_large} against {set_short}"
        );
    });
}

/// What a row of [`budget_row`] reads in its tree before and after the call.
type Holds = fn(&Tree) -> bool;

/// A row of the prices before the work (acceptance 5c).
struct Row {
    page: String,
    parse: ParserLimits,
    setup: &'static str,
    call: String,
    /// The charges the call makes, in their order, from the prices of decision 7.
    parts: Vec<u64>,
    /// The most steps the call takes beside its parts.
    rest: u64,
    /// For a writer: the tree as before the call, and as the write leaves it.
    before: Option<Holds>,
    after: Option<Holds>,
    /// For a writer: whether the write adds stored bytes, so that the handle holds some after
    /// it; a removal of the parser's bytes or a call that finds nothing leaves it at 0.
    adds: bool,
}

/// One run of `source` over the row's page with `steps_per_entry` steps an entry: its end, the
/// runner's steps, the stored charge and whether the tree holds `before` and `after`.
fn budget_run(row: &Row, source: &str, steps_per_entry: u64) -> (ScriptEnd, u64, u64, bool, bool) {
    let mut limits = session();
    limits.engine.steps_per_entry = steps_per_entry;
    let (before, after) = (row.before, row.after);
    with_page(limits, row.page.clone(), row.parse, |runner| {
        let end = runner.evaluate(source);
        let steps = runner.steps();
        let charged = figures(runner).charged;
        let tree = tree(runner);
        (
            end,
            steps,
            charged,
            before.is_none_or(|holds| holds(tree)),
            after.is_none_or(|holds| holds(tree)),
        )
    })
}

/// Four runs of the row, each on its own runner: the setup alone (S0), with the call (S1), and
/// with the call under a budget B that every charge but the last fits and the last does not,
/// and under B and the last. Each charge of the call comes before the work it pays for: under
/// B a writer leaves the tree and the stored charge as before the call, and under B and the
/// last it writes. A read row ends under B in the glue's charge of its answer.
fn budget_row(name: &str, row: &Row) {
    let session = SESSION_ENGINE_LIMITS.steps_per_entry;
    let at = Instant::now();
    let setup = format!("{}\n", row.setup);
    let with_call = format!("{}\n{};", row.setup, row.call);
    let (end, s0, _, _, _) = budget_run(row, &setup, session);
    assert_eq!(end, ScriptEnd::Ran, "{name}: the setup: {end}");
    let (end, s1, charged, _, after) = budget_run(row, &with_call, session);
    assert_eq!(end, ScriptEnd::Ran, "{name}: the call: {end}");
    assert!(after, "{name}: the write took effect");
    let c = s1 - s0;
    let sum: u64 = row.parts.iter().sum();
    assert!(
        sum <= c && c <= sum + row.rest,
        "{name}: the call cost {c}, its parts {:?} sum {sum}",
        row.parts
    );
    let m = c - sum;
    let last = *row.parts.last().expect("a part");
    let budget = s0 + m + (sum - last) + last / 2;
    let (end, _, charged_b, before, _) = budget_run(row, &with_call, budget);
    assert_eq!(
        end,
        ScriptEnd::Limit(Reason::StepsPerScript),
        "{name}: under B = {budget}"
    );
    assert!(before, "{name}: under B the tree is as before the call");
    if row.before.is_some() {
        assert_eq!(
            charged_b, 0,
            "{name}: under B nothing was charged for memory"
        );
        if row.adds {
            assert!(charged > 0, "{name}: the write was charged for memory");
        } else {
            assert_eq!(
                charged, 0,
                "{name}: nothing is charged for memory after the call"
            );
        }
    }
    let (end, _, _, _, after) = budget_run(row, &with_call, budget + last);
    assert_eq!(end, ScriptEnd::Ran, "{name}: under B and the last part");
    assert!(
        after,
        "{name}: under B and the last part the write took effect"
    );
    eprintln!(
        "{name}: S0 {s0}, S1 {s1}, C {c}, M {m}, parts {:?}, B {budget}, {:?}",
        row.parts,
        at.elapsed()
    );
}

/// The setup of the writer rows.
const WRITE_SETUP: &str = "var e = document.documentElement; var v = \"x\".repeat(1048576);";

/// The value of 1 MiB the writer rows set.
fn holds_v(tree: &Tree, name: &str) -> bool {
    value(tree, element(tree, "html"), name).map(str::len) == Some(1 << 20)
}

fn html_has_no_attribute(tree: &Tree) -> bool {
    attributes(tree, element(tree, "html")).is_empty()
}

fn writer_row(call: &str, parts: Vec<u64>, after: Holds) -> Row {
    Row {
        page: "<p></p>".to_string(),
        parse: ParserLimits::default(),
        setup: WRITE_SETUP,
        call: call.to_string(),
        parts,
        rest: 1_000,
        before: Some(html_has_no_attribute),
        after: Some(after),
        adds: true,
    }
}

#[test]
fn budget_set_attribute() {
    // The glue's copy of v, 1,048,576 x 2 / 16 (the name of 1 unit costs 0); the bytes the tree
    // copies, (1 + 1,048,576) / 16.
    budget_row(
        "setAttribute(\"a\", v)",
        &writer_row("e.setAttribute(\"a\", v)", vec![131_072, 65_536], |t| {
            holds_v(t, "a")
        }),
    );
}

#[test]
fn budget_id_setter() {
    // The glue's copy; (2 + 1,048,576) / 16.
    budget_row(
        "e.id = v",
        &writer_row("e.id = v", vec![131_072, 65_536], |t| holds_v(t, "id")),
    );
}

#[test]
fn budget_class_name_setter() {
    // The glue's copy; (5 + 1,048,576) / 16.
    budget_row(
        "e.className = v",
        &writer_row("e.className = v", vec![131_072, 65_536], |t| {
            holds_v(t, "class")
        }),
    );
}

#[test]
fn budget_slot_setter() {
    // The glue's copy; (4 + 1,048,576) / 16.
    budget_row(
        "e.slot = v",
        &writer_row("e.slot = v", vec![131_072, 65_536], |t| holds_v(t, "slot")),
    );
}

#[test]
fn budget_set_attribute_ns() {
    // The glue: 1,048,576 x 2 / 16 for the name and 26 x 2 / 16 = 3 for the URL; the one pass
    // of validate and extract, 1,048,576 / 16; the bytes the tree copies, (1 + 1,048,574) / 16
    // (the colon is not stored).
    let mut row = writer_row(
        "e.setAttributeNS(\"http://www.w3.org/2000/svg\", n, \"\")",
        vec![131_075, 65_536, 65_535],
        |t| {
            let html = element(t, "html");
            t.attribute_position_ns(html, Some(Namespace::Svg), &"y".repeat(1_048_574))
                .is_some()
        },
    );
    row.setup = "var e = document.documentElement; var n = \"x:\" + \"y\".repeat(1048574);";
    budget_row("setAttributeNS(SVG, n, \"\")", &row);
}

#[test]
fn budget_toggle_attribute_adding() {
    // The glue; the validation, 1,048,576 / 16; the bytes the tree copies, (1,048,576 + 0) / 16.
    let mut row = writer_row("e.toggleAttribute(n)", vec![131_072, 65_536, 65_536], |t| {
        let html = element(t, "html");
        t.attribute_position(html, &"y".repeat(1 << 20)).is_some()
    });
    row.setup = "var e = document.documentElement; var n = \"y\".repeat(1048576);";
    budget_row("toggleAttribute(n), adding", &row);
}

#[test]
fn budget_set_attribute_replacing() {
    // The glue; the bytes the tree copies, (1 + 1,048,576) / 16 (`set_attribute` copies the
    // name also when it only replaces the value). The scans of 1 attribute charge 1 / 4 = 0
    // and 1 x 1 / 16 = 0.
    let row = Row {
        page: "<html a=\"\">".to_string(),
        parse: ParserLimits::default(),
        setup: WRITE_SETUP,
        call: "e.setAttribute(\"a\", v)".to_string(),
        parts: vec![131_072, 65_536],
        rest: 1_000,
        before: Some(|t| value(t, element(t, "html"), "a") == Some("")),
        after: Some(|t| holds_v(t, "a")),
        adds: true,
    };
    budget_row("setAttribute(\"a\", v), replacing", &row);
}

/// The page of the removal rows: 16,384 attributes, the last named by 1,048,576 `z` bytes.
fn removal_page() -> String {
    format!(
        "<html{} {}=\"\">",
        numbered("n", 16_383),
        "z".repeat(1 << 20)
    )
}

fn long_present(tree: &Tree) -> bool {
    tree.attribute_position(element(tree, "html"), &"z".repeat(1 << 20))
        .is_some()
}

fn long_absent(tree: &Tree) -> bool {
    !long_present(tree) && attributes(tree, element(tree, "html")).len() == 16_383
}

fn removal_row(call: &str, parts: Vec<u64>) -> Row {
    Row {
        page: removal_page(),
        parse: attributes_at(16_384),
        setup: "var e = document.documentElement; var n = \"z\".repeat(1048576);",
        call: call.to_string(),
        parts,
        rest: 1_000,
        before: Some(long_present),
        after: Some(long_absent),
        adds: false,
    }
}

#[test]
fn budget_remove_attribute() {
    // The glue's copy of n; `find`'s scan, 16,384 / 4; `find`'s bytes, 1 x 1,048,576 / 16 (`k`
    // is 1); the tree's own scan, one charge call, 4,096 + 65,536.
    budget_row(
        "removeAttribute(n)",
        &removal_row("e.removeAttribute(n)", vec![131_072, 4_096, 65_536, 69_632]),
    );
}

#[test]
fn budget_remove_attribute_ns() {
    budget_row(
        "removeAttributeNS(null, n)",
        &removal_row(
            "e.removeAttributeNS(null, n)",
            vec![131_072, 4_096, 65_536, 69_632],
        ),
    );
}

#[test]
fn budget_toggle_attribute_removing() {
    // The glue; the validation, 1,048,576 / 16; then the removal's charges.
    budget_row(
        "toggleAttribute(n), removing",
        &removal_row(
            "e.toggleAttribute(n)",
            vec![131_072, 65_536, 4_096, 65_536, 69_632],
        ),
    );
}

/// A removal that finds nothing: `k` and `c` are 0, so the scans charge nothing and the row's
/// weight is its upper bound.
fn finds_nothing_row(call: &str, parts: Vec<u64>) -> Row {
    Row {
        page: "<p></p>".to_string(),
        parse: ParserLimits::default(),
        setup: "var e = document.documentElement; var n = \"z\".repeat(1048576);",
        call: call.to_string(),
        parts,
        rest: 1_000,
        before: Some(html_has_no_attribute),
        after: Some(html_has_no_attribute),
        adds: false,
    }
}

#[test]
fn budget_remove_attribute_finding_nothing() {
    budget_row(
        "removeAttribute(n), nothing found",
        &finds_nothing_row("e.removeAttribute(n)", vec![131_072]),
    );
}

#[test]
fn budget_remove_attribute_ns_finding_nothing() {
    budget_row(
        "removeAttributeNS(null, n), nothing found",
        &finds_nothing_row("e.removeAttributeNS(null, n)", vec![131_072]),
    );
}

#[test]
fn budget_toggle_attribute_false_finding_nothing() {
    budget_row(
        "toggleAttribute(n, false), nothing found",
        &finds_nothing_row("e.toggleAttribute(n, false)", vec![131_072, 65_536]),
    );
}

/// A read row: the value of 1 MiB from the page.
fn read_row(name: &str, call: &str) -> Row {
    Row {
        page: format!("<html {name}=\"{}\">", "x".repeat(1 << 20)),
        parse: ParserLimits::default(),
        setup: "var e = document.documentElement;",
        call: call.to_string(),
        // The slice's charge, V / 16; the glue's charge for the String it answers, V / 16.
        parts: vec![65_536, 65_536],
        rest: 1_000,
        before: None,
        after: None,
        adds: false,
    }
}

#[test]
fn budget_get_attribute() {
    budget_row(
        "getAttribute(\"a\")",
        &read_row("a", "e.getAttribute(\"a\")"),
    );
}

#[test]
fn budget_get_attribute_ns() {
    budget_row(
        "getAttributeNS(null, \"a\")",
        &read_row("a", "e.getAttributeNS(null, \"a\")"),
    );
}

#[test]
fn budget_id_getter() {
    // `k` is 1 (`id` has the length 2 of the wanted name): 1 x 2 / 16 = 0; 1 / 4 = 0.
    budget_row("e.id", &read_row("id", "e.id"));
}

#[test]
fn budget_class_name_getter() {
    budget_row("e.className", &read_row("class", "e.className"));
}

#[test]
fn budget_slot_getter() {
    budget_row("e.slot", &read_row("slot", "e.slot"));
}

#[test]
fn budget_get_attribute_names() {
    // The count, 4,096, and the names' 19,370 bytes over 16, 1,210: 5,306; the glue's one
    // charge of 8 x 4,096 before its loop (each name is under 16 bytes: no bytes charge). M
    // holds the engine's steps of an Array of 4,096 Strings, capped at a quarter of the last
    // part.
    assert_eq!(numbered_bytes("n", 4_096), 19_370);
    let row = Row {
        page: format!("<html{}>", numbered("n", 4_096)),
        parse: attributes_at(16_384),
        setup: "var e = document.documentElement;",
        call: "e.getAttributeNames()".to_string(),
        parts: vec![4_096 + 1_210, 32_768],
        rest: 8_192,
        before: None,
        after: None,
        adds: false,
    };
    budget_row("getAttributeNames() of 4,096", &row);
}

/// A loop that writes the class of each of 5,000 children costs a constant number of steps a
/// turn (acceptance 5d): under 15 times the same loop over 500, and the kept `children` is the
/// same object after it.
#[test]
fn a_loop_of_writes_over_children_is_linear() {
    let run = |count: usize| -> u64 {
        let page = format!("<p id=\"p\">{}</p>", "<i></i>".repeat(count));
        with_page(session(), page, ParserLimits::default(), |runner| {
            ran(runner, "var p = document.getElementById(\"p\"); var i;");
            let spent = steps(
                runner,
                "var a = p.children; for (i = 0; i < p.children.length; i++) p.children[i].className = \"x\";",
            );
            ran(
                runner,
                "check(a === p.children && a.length > 0, \"the same object\");",
            );
            let tree = tree(runner);
            let p = element(tree, "p");
            for &child in tree.children(p) {
                assert_eq!(value(tree, child, "class"), Some("x"));
            }
            spent
        })
    };
    let large = run(5_000);
    let small = run(500);
    eprintln!("a class written on each child: 5,000 children {large} steps, 500 {small}");
    assert!(large < 15 * small, "{large} against {small}");
}

/// `getAttributeNames` of 1,000 attributes (acceptance 5e): the names in order, and at least
/// 9 x 1,000 steps and the names' bytes over 16.
#[test]
fn get_attribute_names_of_1_000() {
    let page = format!("<i{}></i>", numbered("a", 1_000));
    with_page(limits(), page, ParserLimits::default(), |runner| {
        ran(
            runner,
            "var e = document.getElementsByTagName(\"i\")[0]; var names;",
        );
        let spent = steps(runner, "names = e.getAttributeNames();");
        ran(
            runner,
            r#"check(names.length === 1000, "the length");
for (var j = 0; j < 1000; j++) check(names[j] === "a" + j, "name " + j);"#,
        );
        let floor = 9 * 1_000 + numbered_bytes("a", 1_000) / 16;
        eprintln!("getAttributeNames of 1,000: {spent} steps, floor {floor}");
        assert!(spent >= floor, "{spent} against {floor}");
    });
}

/// The memory charge, step by step (acceptance 6a): each write moves the stored live bytes by
/// the tree's own change and the charge to twice them; an equal value, a removal of what the
/// parser stored while nothing is live, and every throw move nothing.
#[test]
fn the_stored_charge_follows_each_write() {
    let page = "<html lang=\"en\"><body></body></html>".to_string();
    with_page(session(), page, ParserLimits::default(), |runner| {
        ran(runner, "var e = document.documentElement;");
        let start = figures(runner);
        assert_eq!(
            (start.live, start.charged, start.handles, start.node_charges),
            (0, 0, 0, 0)
        );
        let html = element(tree(runner), "html");
        let writes = [
            ("e.setAttribute(\"n\", \"abc\");", true),
            ("e.setAttribute(\"n\", \"abcdef\");", false),
            ("e.setAttribute(\"n\", \"a\");", false),
            (
                "e.setAttributeNS(\"http://www.w3.org/2000/svg\", \"s:t\", \"v\");",
                true,
            ),
            ("e.toggleAttribute(\"tg\");", true),
            ("e.id = \"x\";", true),
            ("e.setAttribute(\"n\", \"a\");", false),
            ("e.removeAttribute(\"n\");", false),
            (
                "e.removeAttributeNS(\"http://www.w3.org/2000/svg\", \"t\");",
                false,
            ),
            ("e.toggleAttribute(\"tg\");", false),
            ("e.removeAttribute(\"id\");", false),
        ];
        for (write, adds) in writes {
            let before = figures(runner);
            let count = attributes(tree(runner), html).len();
            let before_room = room_of(tree(runner), html);
            ran(runner, write);
            let after = figures(runner);
            let rise = after.stored as i64 - before.stored as i64;
            let room = room_change(before_room, room_of(tree(runner), html));
            // The live bytes stop at 0: a list that kept its room after a removal gave back the
            // record, so a later shrink may give back more than is live.
            let next = (before.live as i64 + rise + room).max(0);
            assert_eq!(after.live as i64, next, "{write}");
            assert_eq!(after.charged, 2 * after.live, "{write}");
            assert_eq!(after.handles, 1, "{write}");
            assert_eq!(after.node_charges, 0, "{write}");
            assert_eq!(after.totals, (0, 0), "{write}");
            if adds {
                let list = attributes(tree(runner), html);
                assert_eq!(list.len(), count + 1);
                let added = list.last().unwrap();
                assert_eq!(added.stored_bytes() as i64, rise, "{write}");
            }
        }
        let now = figures(runner);
        assert_eq!((now.live, now.charged), (0, 0));
        ran(runner, "e.removeAttribute(\"lang\");");
        let after = figures(runner);
        assert!(after.stored < now.stored, "the parser's attribute is gone");
        assert_eq!((after.live, after.charged, after.handles), (0, 0, 1));
    });
}

/// A row of [`memory_first`]: its page, setup, write and bound.
struct MemoryRow {
    page: String,
    setup: &'static str,
    write: &'static str,
    /// The bytes one write adds (`None` for r0, where they grow with the name).
    b: Option<u64>,
    bound: usize,
    /// Whether an element holds the write.
    holds: fn(&Tree, NodeId) -> bool,
}

/// The page of the rows r1 to r7: 1,000 elements, each written once.
fn thousand(element: &str) -> String {
    format!("<body>{}</body>", element.repeat(1_000))
}

const COLLECT: &str = r#"var els = []; var c = document.getElementsByTagName("i");
for (var j = 0; j < c.length; j++) els.push(c[j]);
var v = "x".repeat(40000); var n = "y".repeat(40000); var i;"#;

/// Memory first (acceptance 6b): a loop of writes ends `limit: memory`, the refused write is
/// not in the tree, the tree rose by the stored live bytes and the charge is twice them.
fn memory_first(name: &str, row: MemoryRow) {
    let at = Instant::now();
    let MemoryRow {
        page,
        setup,
        write,
        b,
        bound,
        holds,
    } = row;
    with_page(session(), page, ParserLimits::default(), |runner| {
        ran(runner, setup);
        let before = figures(runner);
        let start = runner.steps();
        let end = runner.evaluate(&format!("for (i = 0; ; i++) {write}"));
        let spent = runner.steps() - start;
        assert_eq!(end, ScriptEnd::Limit(Reason::Memory), "{name}");
        let after = figures(runner);
        assert_eq!(after.handles, 1, "{name}");
        assert!(
            after.live as usize >= after.stored - before.stored,
            "{name}"
        );
        assert_eq!(after.charged, 2 * after.live, "{name}");
        let tree = tree(runner);
        let n = match b {
            Some(b) => {
                let els = elements(tree, "i");
                let n = els.iter().take_while(|&&el| holds(tree, el)).count();
                assert!(n > 0 && n < els.len(), "{name}: {n}");
                assert!(holds(tree, els[n - 1]), "{name}");
                assert!(
                    !holds(tree, els[n]),
                    "{name}: the refused write is in the tree"
                );
                assert_eq!(
                    els.iter().filter(|&&el| holds(tree, el)).count(),
                    n,
                    "{name}"
                );
                assert_eq!(after.live, n as u64 * b, "{name}");
                n
            }
            None => {
                let html = element(tree, "html");
                let n = attributes(tree, html).len();
                let live: u64 = (0..n)
                    .map(|i| (ATTRIBUTE_BYTES + format!("a{i}").len() + 40_000) as u64)
                    .sum();
                assert_eq!(after.live, live + room_beyond(0, 0, n), "{name}");
                assert!(value(tree, html, &format!("a{n}")).is_none(), "{name}");
                assert!(
                    value(tree, html, &format!("a{}", n - 1)).is_some(),
                    "{name}"
                );
                n
            }
        };
        assert!(n <= bound, "{name}: {n} writes, bound {bound}");
        let heap = heap(runner);
        eprintln!(
            "{name}: {n} writes (bound {bound}), {spent} steps, heap_bytes {heap}, {:?}",
            at.elapsed()
        );
    });
}

fn has_value(tree: &Tree, el: NodeId, name: &str) -> bool {
    value(tree, el, name).map(str::len) == Some(40_000)
}

#[test]
fn memory_first_r0_one_element_new_names() {
    memory_first(
        "r0",
        MemoryRow {
            page: "<p></p>".to_string(),
            setup: "var e = document.documentElement; var v = \"x\".repeat(40000); var i;",
            write: "e.setAttribute(\"a\" + i, v);",
            b: None,
            bound: 837,
            holds: |_, _| true,
        },
    );
}

#[test]
fn memory_first_r1_set_attribute() {
    memory_first(
        "r1",
        MemoryRow {
            page: thousand("<i></i>"),
            setup: COLLECT,
            write: "els[i].setAttribute(\"a\", v);",
            b: Some(ATTRIBUTE_BYTES as u64 + 1 + 40_000),
            bound: 837,
            holds: |t, el| has_value(t, el, "a"),
        },
    );
}

#[test]
fn memory_first_r2_a_longer_value() {
    memory_first(
        "r2",
        MemoryRow {
            page: thousand("<i a=\"x\"></i>"),
            setup: COLLECT,
            write: "els[i].setAttribute(\"a\", v);",
            b: Some(40_000 - 1),
            bound: 838,
            holds: |t, el| has_value(t, el, "a"),
        },
    );
}

#[test]
fn memory_first_r3_set_attribute_ns() {
    memory_first(
        "r3",
        MemoryRow {
            page: thousand("<i></i>"),
            setup: COLLECT,
            write: "els[i].setAttributeNS(\"http://www.w3.org/2000/svg\", \"x:a\", v);",
            b: Some(ATTRIBUTE_BYTES as u64 + 1 + 1 + 40_000),
            bound: 837,
            holds: |t, el| {
                t.attribute_position_ns(el, Some(Namespace::Svg), "a")
                    .is_some()
            },
        },
    );
}

#[test]
fn memory_first_r4_toggle_attribute() {
    memory_first(
        "r4",
        MemoryRow {
            page: thousand("<i></i>"),
            setup: COLLECT,
            write: "els[i].toggleAttribute(n);",
            b: Some(ATTRIBUTE_BYTES as u64 + 40_000),
            bound: 837,
            holds: |t, el| t.attribute_position(el, &"y".repeat(40_000)).is_some(),
        },
    );
}

#[test]
fn memory_first_r5_id() {
    memory_first(
        "r5",
        MemoryRow {
            page: thousand("<i></i>"),
            setup: COLLECT,
            write: "els[i].id = v;",
            b: Some(ATTRIBUTE_BYTES as u64 + 2 + 40_000),
            bound: 837,
            holds: |t, el| has_value(t, el, "id"),
        },
    );
}

#[test]
fn memory_first_r6_class_name() {
    memory_first(
        "r6",
        MemoryRow {
            page: thousand("<i></i>"),
            setup: COLLECT,
            write: "els[i].className = v;",
            b: Some(ATTRIBUTE_BYTES as u64 + 5 + 40_000),
            bound: 837,
            holds: |t, el| has_value(t, el, "class"),
        },
    );
}

#[test]
fn memory_first_r7_slot() {
    memory_first(
        "r7",
        MemoryRow {
            page: thousand("<i></i>"),
            setup: COLLECT,
            write: "els[i].slot = v;",
            b: Some(ATTRIBUTE_BYTES as u64 + 4 + 40_000),
            bound: 837,
            holds: |t, el| has_value(t, el, "slot"),
        },
    );
}

/// Attributes first (acceptance 6c): with values of 1,000 bytes the loop of r0 ends
/// `limit: attributes` at the 1,025th write, and the refused write's grow is put back.
#[test]
fn attributes_first() {
    with_page(
        session(),
        "<p></p>".to_string(),
        ParserLimits::default(),
        |runner| {
            ran(
                runner,
                "var e = document.documentElement; var v = \"x\".repeat(1000); var i;",
            );
            let start = runner.steps();
            let end = runner.evaluate("for (i = 0; ; i++) e.setAttribute(\"a\" + i, v);");
            let spent = runner.steps() - start;
            assert_eq!(end, ScriptEnd::Limit(Reason::Attributes));
            let after = figures(runner);
            let html = element(tree(runner), "html");
            assert_eq!(attributes(tree(runner), html).len(), 1_024);
            // 1,024 x (80 + 1,000) + 4,010, the bytes of the names a0 to a1023.
            // and the room beyond the new records of the eight doublings from 4 to 1,024.
            let live = 1_024 * (ATTRIBUTE_BYTES as u64 + 1_000)
                + numbered_bytes("a", 1_024)
                + room_beyond(0, 0, 1_024);
            assert_eq!(room_beyond(0, 0, 1_024), 1_012 * ATTRIBUTE_BYTES as u64);
            assert_eq!(numbered_bytes("a", 1_024), 4_010);
            assert_eq!(after.live, live);
            assert_eq!(after.charged, 2 * live);
            assert_eq!(after.handles, 1);
            eprintln!("attributes first: {spent} steps, live {live}");
        },
    );
}

/// A repeat is a grow and a refund (acceptance 6d): 40 rounds of a set of 1 MiB and its
/// removal run, where a charge that only grew would end at the 32nd.
#[test]
fn a_repeat_is_a_grow_and_a_refund() {
    with_page(
        session(),
        "<p></p>".to_string(),
        ParserLimits::default(),
        |runner| {
            ran(
                runner,
                "var e = document.documentElement; var v = \"x\".repeat(1048576); var r;",
            );
            let spent = steps(
                runner,
                "for (r = 0; r < 40; r++) { e.setAttribute(\"a\", v); e.removeAttribute(\"a\"); }",
            );
            let after = figures(runner);
            assert_eq!((after.live, after.charged, after.handles), (0, 0, 1));
            ran(runner, "e.setAttribute(\"a\", v);");
            let set = ATTRIBUTE_BYTES as u64 + 1 + (1 << 20);
            assert_eq!(figures(runner).charged, 2 * set);
            eprintln!("40 rounds of a set of 1 MiB and its removal: {spent} steps");
        },
    );
}

/// Growth by replacement (acceptance 6e): doubling a value from 1 KiB to 1 MiB charges twice
/// the last length and the fixed terms, not the sum; a short value refunds; doubling again
/// charges each grow by its difference.
#[test]
fn growth_by_replacement() {
    with_page(
        session(),
        "<p></p>".to_string(),
        ParserLimits::default(),
        |runner| {
            let start = runner.steps();
            ran(
                runner,
                r#"var e = document.documentElement; var v = "x".repeat(1048576);
for (var size = 1024; size <= 1048576; size *= 2) e.setAttribute("a", v.slice(0, size));"#,
            );
            let full = ATTRIBUTE_BYTES as u64 + 1 + (1 << 20);
            let after = figures(runner);
            assert_eq!(
                (after.live, after.charged, after.handles),
                (full, 2 * full, 1)
            );
            ran(runner, "e.setAttribute(\"a\", v.slice(0, 1024));");
            let short = ATTRIBUTE_BYTES as u64 + 1 + 1_024;
            let after = figures(runner);
            assert_eq!(
                (after.live, after.charged, after.handles),
                (short, 2 * short, 1)
            );
            ran(
                runner,
                "for (var size = 2048; size <= 1048576; size *= 2) e.setAttribute(\"a\", v.slice(0, size));",
            );
            let after = figures(runner);
            assert_eq!(
                (after.live, after.charged, after.handles),
                (full, 2 * full, 1)
            );
            eprintln!("growth by replacement: {} steps", runner.steps() - start);
        },
    );
}

/// A removal of what the parser stored while bytes are live (acceptance 6g) refunds, never
/// more than was charged.
#[test]
fn a_removal_of_parser_bytes_refunds_what_was_charged() {
    let page = format!("<html p=\"{}\">", "P".repeat(1_000));
    with_page(session(), page, ParserLimits::default(), |runner| {
        ran(
            runner,
            "var e = document.documentElement; e.setAttribute(\"s\", \"x\".repeat(2000));",
        );
        let f = figures(runner);
        assert_eq!((f.live, f.charged, f.handles), (2_081, 4_162, 1));
        ran(runner, "e.removeAttribute(\"p\");");
        let f = figures(runner);
        assert_eq!((f.live, f.charged, f.handles), (1_000, 2_000, 1));
        ran(runner, "e.removeAttribute(\"s\");");
        let f = figures(runner);
        assert_eq!((f.live, f.charged, f.handles), (0, 0, 1));
    });
}

/// Other slices and `[SameObject]` (acceptance 8): the rows of `throws` beside this slice
/// stay `throws`.
#[test]
fn the_attribute_node_members_still_throw() {
    with_page(
        limits(),
        PAGE.to_string(),
        ParserLimits::default(),
        |runner| {
            ran(
                runner,
                r#"var el = document.getElementById("a");
function notSupported(f, message) {
  try { f(); } catch (e) {
    check(e instanceof DOMException && e.name === "NotSupportedError" && e.message === message,
      message + ": " + e.name + ": " + e.message);
    return;
  }
  throw new Error(message + ": no throw");
}
notSupported(function () { el.attributes; }, "Element.attributes is not supported: attribute nodes");
notSupported(function () { el.classList; }, "Element.classList is not supported: token lists");
notSupported(function () { el.getAttributeNode("a"); }, "Element.getAttributeNode is not supported: attribute nodes");
notSupported(function () { document.createAttribute("a"); }, "Document.createAttribute is not supported: attribute nodes");
notSupported(function () { return el.attributes === el.attributes; }, "Element.attributes is not supported: attribute nodes");
notSupported(function () { return el.classList === el.classList; }, "Element.classList is not supported: token lists");"#,
            );
        },
    );
}
