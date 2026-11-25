//! The tests of the slice `reading` (`story:scripts-dom-reading`): surf's reading members and
//! its wrapper hook over a parsed tree, in a runner with the generated bindings.
//!
//! Each row of `crates/webidl/selection.txt` with the slice `reading` has entries in [`CASES`],
//! each a JavaScript expression and the value the standard gives it, compared with `===` in a
//! runner of its own over [`PAGE`]. The other tests hold what a single value does not show: one
//! wrapper per node, the collections and their deviations, the document's facts, the sibling
//! reads, the steps and the memory a member costs, and trees made to be hard to walk.
//!
//! The slices are `dom::slices(info)` with the slice `mutation` replaced by [`TestMutation`],
//! which writes the tree through its public write surface: `appendChild`, `removeChild` and
//! `createDocumentFragment`, so that a case reaches a fragment and a test writes between two
//! reads. Every other member of `mutation`, `attributes` and `selectors` answers the trait's
//! default.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{
    NodeData, NodeId, NotRun, ParseOptions, ScriptHook, Tree, WriteError, parse_document,
    parse_document_with_hook,
};
use surf_script::bindings::{DomCx, DomEnd, DomException, DomResult, Mutation, Slices};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{
    Document, Mode, Reason, Runner, RunnerLimits, SESSION_ENGINE_LIMITS, ScriptEnd, run_document,
};
use surf_url::Url;
use swell::{HOST_OBJECT_LIVE_BYTES, Limits};

/// The address of the documents of the tests that give one.
const ADDRESS: &str = "https://example.com/dir/page";

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
        scripts_per_runner: 100,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(50),
    }
}

/// The limits of [`limits`] with a memory budget of 1 MiB.
fn small_limits() -> RunnerLimits {
    let mut small = limits();
    small.engine.memory_bytes = 1024 * 1024;
    small
}

/// The document's facts of the tests that give an address.
fn info() -> DocumentInfo {
    DocumentInfo {
        address: Url::parse(ADDRESS, None).expect("the address"),
        content_type: "text/html".to_string(),
        encoding: "UTF-8".to_string(),
    }
}

/// The test's `mutation` slice: the writes the tests make between two reads, through the tree's
/// public write surface.
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

/// surf's slices for `info`, with the test's `mutation`.
fn slices(info: DocumentInfo) -> Slices {
    let mut slices = dom::slices(info);
    slices.mutation = Box::new(TestMutation);
    slices
}

/// The document of `tree` with `info`.
fn document(tree: Tree, info: DocumentInfo) -> Document {
    Document {
        tree,
        limits: ParserLimits::default(),
        slices: slices(info),
    }
}

/// Runs `body` with a runner over `page`, parsed with the default options, and `info`.
fn with_page<R: Send>(
    limits: RunnerLimits,
    page: String,
    info: DocumentInfo,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(
        limits,
        Mode::Test,
        move || document(parse_document(&page, &ParseOptions::default()), info),
        body,
    )
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
    let end = runner.evaluate(source);
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

/// The page of the table: a doctype, a title, an element with each kind of child, an SVG
/// element, two lists, two equal items, a text the builder splits in two, a character outside
/// the BMP, and a processing instruction the builder appends to `body` (the HTML parser makes
/// none).
const PAGE: &str = "<!DOCTYPE html><html><head><title> The  title\n here </title></head><body>\
<div id=\"a\" class=\"c D\"><p id=\"p1\" name=\"n1\">one</p>two<!--note--><span id=\"s1\"></span></div>\
<svg id=\"g\"><circle id=\"ci\" name=\"cn\" class=\"c\"></circle></svg>\
<ul id=\"u\"><li>1</li><li>2</li></ul>\
<ol id=\"o\"><li class=\"k\">x</li><li class=\"k\">x</li></ol>\
<b id=\"w\">x</b><i id=\"em\">a\u{1F600}</i></body></html>";

/// The names of the table's expressions.
const PRELUDE: &str = r#"
var html = document.documentElement;
var head = html.firstChild;
var body = html.lastChild;
var a = document.getElementById("a");
var p = document.getElementById("p1");
var one = p.firstChild;
var two = p.nextSibling;
var note = two.nextSibling;
var span = document.getElementById("s1");
var svg = document.getElementById("g");
var circle = document.getElementById("ci");
var ul = document.getElementById("u");
var ol = document.getElementById("o");
var w = document.getElementById("w");
var em = document.getElementById("em");
var pi = body.lastChild;
var doctype = document.firstChild;
var XHTML = "http://www.w3.org/1999/xhtml";
var SVG = "http://www.w3.org/2000/svg";
function fragment() { var f = document.createDocumentFragment(); f.appendChild(ul); return f; }
"#;

/// The tree of [`PAGE`]: the parse, then a processing instruction appended to `body` and a text
/// `y` to `#w`, which then holds two adjacent text nodes.
fn table_tree() -> Tree {
    let mut tree = parse_document(PAGE, &ParseOptions::default());
    let named = |tree: &Tree, name: &str| {
        tree.descendants(tree.root())
            .into_iter()
            .find(|&n| matches!(tree.data(n), NodeData::Element { local_name, .. } if local_name == name))
            .expect("the element")
    };
    let body = named(&tree, "body");
    let pi = tree
        .create_processing_instruction("tg", "dt")
        .expect("a processing instruction");
    tree.append_child(body, pi).expect("appended");
    let b = named(&tree, "b");
    let y = tree.create_text_node("y").expect("a text");
    tree.append_child(b, y).expect("appended");
    tree
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

/// The table. Each entry's comment names the section of the DOM Standard (the HTML Standard for
/// `Window` and `Document.title`) whose value it compares.
const CASES: &[Case] = &[
    // DOM, "Interface Node": the constants of nodeType.
    case("Node.ELEMENT_NODE", "Node.ELEMENT_NODE", "1"),
    case("Node.ATTRIBUTE_NODE", "Node.ATTRIBUTE_NODE", "2"),
    case("Node.TEXT_NODE", "a.TEXT_NODE", "3"),
    case("Node.CDATA_SECTION_NODE", "Node.CDATA_SECTION_NODE", "4"),
    case(
        "Node.ENTITY_REFERENCE_NODE",
        "Node.ENTITY_REFERENCE_NODE",
        "5",
    ),
    case("Node.ENTITY_NODE", "Node.ENTITY_NODE", "6"),
    case(
        "Node.PROCESSING_INSTRUCTION_NODE",
        "Node.PROCESSING_INSTRUCTION_NODE",
        "7",
    ),
    case("Node.COMMENT_NODE", "Node.COMMENT_NODE", "8"),
    case("Node.DOCUMENT_NODE", "document.DOCUMENT_NODE", "9"),
    case("Node.DOCUMENT_TYPE_NODE", "Node.DOCUMENT_TYPE_NODE", "10"),
    case(
        "Node.DOCUMENT_FRAGMENT_NODE",
        "Node.DOCUMENT_FRAGMENT_NODE",
        "11",
    ),
    case("Node.NOTATION_NODE", "Node.NOTATION_NODE", "12"),
    // DOM, "Interface Node": the constants of compareDocumentPosition.
    case(
        "Node.DOCUMENT_POSITION_DISCONNECTED",
        "Node.DOCUMENT_POSITION_DISCONNECTED",
        "1",
    ),
    case(
        "Node.DOCUMENT_POSITION_PRECEDING",
        "Node.DOCUMENT_POSITION_PRECEDING",
        "2",
    ),
    case(
        "Node.DOCUMENT_POSITION_FOLLOWING",
        "Node.DOCUMENT_POSITION_FOLLOWING",
        "4",
    ),
    case(
        "Node.DOCUMENT_POSITION_CONTAINS",
        "Node.DOCUMENT_POSITION_CONTAINS",
        "8",
    ),
    case(
        "Node.DOCUMENT_POSITION_CONTAINED_BY",
        "Node.DOCUMENT_POSITION_CONTAINED_BY",
        "16",
    ),
    case(
        "Node.DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC",
        "a.DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC",
        "32",
    ),
    // DOM, "Interface Node", the nodeType getter.
    case("Node.nodeType", "a.nodeType", "1"),
    case("Node.nodeType", "one.nodeType", "3"),
    case("Node.nodeType", "pi.nodeType", "7"),
    case("Node.nodeType", "note.nodeType", "8"),
    case("Node.nodeType", "document.nodeType", "9"),
    case("Node.nodeType", "doctype.nodeType", "10"),
    case("Node.nodeType", "fragment().nodeType", "11"),
    // DOM, "Interface Node", the nodeName getter: an HTML element's qualified name in upper case.
    case("Node.nodeName", "a.nodeName", "\"DIV\""),
    case("Node.nodeName", "circle.nodeName", "\"circle\""),
    case("Node.nodeName", "one.nodeName", "\"#text\""),
    case("Node.nodeName", "pi.nodeName", "\"tg\""),
    case("Node.nodeName", "note.nodeName", "\"#comment\""),
    case("Node.nodeName", "document.nodeName", "\"#document\""),
    case("Node.nodeName", "doctype.nodeName", "\"html\""),
    case(
        "Node.nodeName",
        "fragment().nodeName",
        "\"#document-fragment\"",
    ),
    // DOM, "Interface Node", the baseURI getter: the document base URL, here its address.
    case(
        "Node.baseURI",
        "a.baseURI",
        "\"https://example.com/dir/page\"",
    ),
    case(
        "Node.baseURI",
        "document.baseURI",
        "\"https://example.com/dir/page\"",
    ),
    // DOM, "Interface Node", isConnected: the shadow-including root is a document.
    case("Node.isConnected", "a.isConnected", "true"),
    case(
        "Node.isConnected",
        "fragment().firstChild.isConnected",
        "false",
    ),
    // DOM, "Interface Node", ownerDocument: null for a document.
    case("Node.ownerDocument", "a.ownerDocument", "document"),
    case("Node.ownerDocument", "document.ownerDocument", "null"),
    // DOM, "Interface Node", getRootNode: the root.
    case("Node.getRootNode", "one.getRootNode()", "document"),
    case(
        "Node.getRootNode",
        "a.getRootNode({ composed: true })",
        "document",
    ),
    case(
        "Node.getRootNode",
        "(function () { var f = fragment(); return f.firstChild.firstChild.getRootNode() === f; })()",
        "true",
    ),
    // DOM, "Interface Node", parentNode and parentElement.
    case("Node.parentNode", "p.parentNode", "a"),
    case("Node.parentNode", "html.parentNode", "document"),
    case("Node.parentNode", "document.parentNode", "null"),
    case("Node.parentElement", "p.parentElement", "a"),
    case("Node.parentElement", "html.parentElement", "null"),
    // DOM, "Interface Node", hasChildNodes.
    case("Node.hasChildNodes", "a.hasChildNodes()", "true"),
    case("Node.hasChildNodes", "span.hasChildNodes()", "false"),
    // DOM, "Interface Node", childNodes: the children, in order.
    case("Node.childNodes", "a.childNodes.length", "4"),
    case("Node.childNodes", "a.childNodes[1]", "two"),
    case("Node.childNodes", "a.childNodes === a.childNodes", "true"),
    // DOM, "Interface Node", firstChild and lastChild.
    case("Node.firstChild", "a.firstChild", "p"),
    case("Node.firstChild", "span.firstChild", "null"),
    case("Node.lastChild", "a.lastChild", "span"),
    case("Node.lastChild", "span.lastChild", "null"),
    // DOM, "Interface Node", previousSibling and nextSibling.
    case("Node.previousSibling", "two.previousSibling", "p"),
    case("Node.previousSibling", "p.previousSibling", "null"),
    case("Node.nextSibling", "p.nextSibling", "two"),
    case("Node.nextSibling", "span.nextSibling", "null"),
    // DOM, "Interface Node", the nodeValue getter: character data's data, else null.
    case("Node.nodeValue", "one.nodeValue", "\"one\""),
    case("Node.nodeValue", "note.nodeValue", "\"note\""),
    case("Node.nodeValue", "pi.nodeValue", "\"dt\""),
    case("Node.nodeValue", "a.nodeValue", "null"),
    // DOM, "Interface Node", the textContent getter: the descendant text content of an element.
    case("Node.textContent", "a.textContent", "\"onetwo\""),
    case("Node.textContent", "ul.textContent", "\"12\""),
    case("Node.textContent", "note.textContent", "\"note\""),
    case("Node.textContent", "document.textContent", "null"),
    case("Node.textContent", "doctype.textContent", "null"),
    // DOM, "Interface Node", isEqualNode: the same kind, data, attributes and children.
    case(
        "Node.isEqualNode",
        "ol.firstChild.isEqualNode(ol.lastChild)",
        "true",
    ),
    case(
        "Node.isEqualNode",
        "ul.firstChild.isEqualNode(ul.lastChild)",
        "false",
    ),
    case("Node.isEqualNode", "a.isEqualNode(null)", "false"),
    // DOM, "Interface Node", isSameNode.
    case("Node.isSameNode", "a.isSameNode(a)", "true"),
    case("Node.isSameNode", "a.isSameNode(p)", "false"),
    case("Node.isSameNode", "a.isSameNode(null)", "false"),
    // DOM, "Interface Node", compareDocumentPosition.
    case(
        "Node.compareDocumentPosition",
        "p.compareDocumentPosition(a)",
        "10",
    ),
    case(
        "Node.compareDocumentPosition",
        "a.compareDocumentPosition(p)",
        "20",
    ),
    case(
        "Node.compareDocumentPosition",
        "p.compareDocumentPosition(span)",
        "4",
    ),
    case(
        "Node.compareDocumentPosition",
        "span.compareDocumentPosition(p)",
        "2",
    ),
    case(
        "Node.compareDocumentPosition",
        "a.compareDocumentPosition(a)",
        "0",
    ),
    case(
        "Node.compareDocumentPosition",
        "a.compareDocumentPosition(fragment()) & 33",
        "33",
    ),
    // DOM, "Interface Node", contains: an inclusive descendant.
    case("Node.contains", "a.contains(one)", "true"),
    case("Node.contains", "a.contains(a)", "true"),
    case("Node.contains", "p.contains(a)", "false"),
    case("Node.contains", "a.contains(null)", "false"),
    // DOM, "Interface Node", lookupPrefix: "locate a namespace prefix".
    case("Node.lookupPrefix", "a.lookupPrefix(XHTML)", "null"),
    case("Node.lookupPrefix", "a.lookupPrefix(null)", "null"),
    // DOM, "Interface Node", lookupNamespaceURI: "locate a namespace".
    case(
        "Node.lookupNamespaceURI",
        "a.lookupNamespaceURI(null)",
        "XHTML",
    ),
    case(
        "Node.lookupNamespaceURI",
        "circle.lookupNamespaceURI(null)",
        "SVG",
    ),
    case(
        "Node.lookupNamespaceURI",
        "note.lookupNamespaceURI(null)",
        "XHTML",
    ),
    case(
        "Node.lookupNamespaceURI",
        "document.lookupNamespaceURI(null)",
        "XHTML",
    ),
    case(
        "Node.lookupNamespaceURI",
        "a.lookupNamespaceURI(\"xml\")",
        "\"http://www.w3.org/XML/1998/namespace\"",
    ),
    case(
        "Node.lookupNamespaceURI",
        "doctype.lookupNamespaceURI(null)",
        "null",
    ),
    // DOM, "Interface Node", isDefaultNamespace.
    case(
        "Node.isDefaultNamespace",
        "a.isDefaultNamespace(XHTML)",
        "true",
    ),
    case(
        "Node.isDefaultNamespace",
        "circle.isDefaultNamespace(XHTML)",
        "false",
    ),
    case(
        "Node.isDefaultNamespace",
        "a.isDefaultNamespace(\"\")",
        "false",
    ),
    // DOM, "Interface Element": namespaceURI, prefix, localName, tagName.
    case("Element.namespaceURI", "a.namespaceURI", "XHTML"),
    case("Element.namespaceURI", "circle.namespaceURI", "SVG"),
    case("Element.prefix", "a.prefix", "null"),
    case("Element.localName", "a.localName", "\"div\""),
    case("Element.localName", "circle.localName", "\"circle\""),
    case("Element.tagName", "a.tagName", "\"DIV\""),
    case("Element.tagName", "circle.tagName", "\"circle\""),
    // DOM, "Interface Element", shadowRoot: no shadow root is attached.
    case("Element.shadowRoot", "a.shadowRoot", "null"),
    // DOM, "Interface Element", getElementsByTagName: "list of elements with qualified name".
    case(
        "Element.getElementsByTagName",
        "a.getElementsByTagName(\"*\").length",
        "2",
    ),
    case(
        "Element.getElementsByTagName",
        "a.getElementsByTagName(\"P\")[0]",
        "p",
    ),
    case(
        "Element.getElementsByTagName",
        "svg.getElementsByTagName(\"CIRCLE\").length",
        "0",
    ),
    // DOM, "Interface Element", getElementsByTagNameNS.
    case(
        "Element.getElementsByTagNameNS",
        "svg.getElementsByTagNameNS(SVG, \"circle\")[0]",
        "circle",
    ),
    case(
        "Element.getElementsByTagNameNS",
        "a.getElementsByTagNameNS(\"*\", \"*\").length",
        "2",
    ),
    case(
        "Element.getElementsByTagNameNS",
        "a.getElementsByTagNameNS(null, \"p\").length",
        "0",
    ),
    // DOM, "Interface Element", getElementsByClassName: every class given.
    case(
        "Element.getElementsByClassName",
        "body.getElementsByClassName(\"c D\").length",
        "1",
    ),
    case(
        "Element.getElementsByClassName",
        "body.getElementsByClassName(\"d\").length",
        "0",
    ),
    case(
        "Element.getElementsByClassName",
        "body.getElementsByClassName(\" \").length",
        "0",
    ),
    // DOM, "Mixin ParentNode": children, firstElementChild, lastElementChild,
    // childElementCount.
    case("Element.children", "a.children.length", "2"),
    case("Element.children", "a.children[1]", "span"),
    case("Element.children", "a.children === a.children", "true"),
    case("Element.firstElementChild", "a.firstElementChild", "p"),
    case(
        "Element.firstElementChild",
        "span.firstElementChild",
        "null",
    ),
    case("Element.lastElementChild", "a.lastElementChild", "span"),
    case("Element.childElementCount", "a.childElementCount", "2"),
    // DOM, "Mixin NonDocumentTypeChildNode".
    case(
        "Element.previousElementSibling",
        "span.previousElementSibling",
        "p",
    ),
    case(
        "Element.previousElementSibling",
        "p.previousElementSibling",
        "null",
    ),
    case("Element.nextElementSibling", "p.nextElementSibling", "span"),
    case(
        "Element.nextElementSibling",
        "span.nextElementSibling",
        "null",
    ),
    // DOM, "Mixin Slottable": no slot assigns it.
    case("Element.assignedSlot", "a.assignedSlot", "null"),
    // DOM, "Interface Document": the document's URL, mode and the facts of its response.
    case(
        "Document.URL",
        "document.URL",
        "\"https://example.com/dir/page\"",
    ),
    case(
        "Document.documentURI",
        "document.documentURI",
        "\"https://example.com/dir/page\"",
    ),
    case(
        "Document.compatMode",
        "document.compatMode",
        "\"CSS1Compat\"",
    ),
    case(
        "Document.characterSet",
        "document.characterSet",
        "\"UTF-8\"",
    ),
    case("Document.charset", "document.charset", "\"UTF-8\""),
    case(
        "Document.inputEncoding",
        "document.inputEncoding",
        "\"UTF-8\"",
    ),
    case(
        "Document.contentType",
        "document.contentType",
        "\"text/html\"",
    ),
    // DOM, "Interface Document", doctype and documentElement.
    case("Document.doctype", "document.doctype", "doctype"),
    case("Document.doctype", "document.doctype.nodeType", "10"),
    case(
        "Document.documentElement",
        "document.documentElement",
        "document.lastChild",
    ),
    case("Document.documentElement", "html.localName", "\"html\""),
    // DOM, "Interface Document", the getElementsBy... members over the whole document.
    case(
        "Document.getElementsByTagName",
        "document.getElementsByTagName(\"*\").length",
        "17",
    ),
    case(
        "Document.getElementsByTagName",
        "document.getElementsByTagName(\"LI\").length",
        "4",
    ),
    case(
        "Document.getElementsByTagNameNS",
        "document.getElementsByTagNameNS(SVG, \"*\").length",
        "2",
    ),
    case(
        "Document.getElementsByTagNameNS",
        "document.getElementsByTagNameNS(\"\", \"li\").length",
        "0",
    ),
    case(
        "Document.getElementsByClassName",
        "document.getElementsByClassName(\"c\").length",
        "2",
    ),
    case(
        "Document.getElementsByClassName",
        "document.getElementsByClassName(\"k\")[1]",
        "ol.lastChild",
    ),
    // DOM, "Mixin NonElementParentNode", getElementById.
    case(
        "Document.getElementById",
        "document.getElementById(\"s1\")",
        "span",
    ),
    case(
        "Document.getElementById",
        "document.getElementById(\"x\")",
        "null",
    ),
    case(
        "Document.getElementById",
        "document.getElementById(\"\")",
        "null",
    ),
    // DOM, "Mixin ParentNode" of the document.
    case("Document.children", "document.children.length", "1"),
    case("Document.children", "document.children[0]", "html"),
    case(
        "Document.children",
        "document.children === document.children",
        "true",
    ),
    case(
        "Document.firstElementChild",
        "document.firstElementChild",
        "html",
    ),
    case(
        "Document.lastElementChild",
        "document.lastElementChild",
        "html",
    ),
    case(
        "Document.childElementCount",
        "document.childElementCount",
        "1",
    ),
    // HTML, "The title element" and "document.title": stripped and collapsed.
    case("Document.title", "document.title", "\"The title here\""),
    // DOM, "Interface CharacterData".
    case("CharacterData.data", "one.data", "\"one\""),
    case("CharacterData.data", "pi.data", "\"dt\""),
    case("CharacterData.length", "one.length", "3"),
    case("CharacterData.length", "em.firstChild.length", "3"),
    case(
        "CharacterData.substringData",
        "one.substringData(1, 1)",
        "\"n\"",
    ),
    case(
        "CharacterData.substringData",
        "em.firstChild.substringData(1, 2)",
        "\"\\uD83D\\uDE00\"",
    ),
    case(
        "CharacterData.substringData",
        "one.substringData(1, 10)",
        "\"ne\"",
    ),
    case(
        "CharacterData.substringData",
        "threw(function () { one.substringData(4, 1); })",
        "\"IndexSizeError\"",
    ),
    case(
        "CharacterData.previousElementSibling",
        "two.previousElementSibling",
        "p",
    ),
    case(
        "CharacterData.nextElementSibling",
        "two.nextElementSibling",
        "span",
    ),
    // DOM, "Interface Text", wholeText: the contiguous Text nodes.
    case("Text.wholeText", "w.firstChild.wholeText", "\"xy\""),
    case("Text.wholeText", "w.lastChild.wholeText", "\"xy\""),
    case("Text.wholeText", "two.wholeText", "\"two\""),
    case("Text.assignedSlot", "two.assignedSlot", "null"),
    // DOM, "Interface ProcessingInstruction".
    case("ProcessingInstruction.target", "pi.target", "\"tg\""),
    // DOM, "Interface DocumentType".
    case("DocumentType.name", "doctype.name", "\"html\""),
    case("DocumentType.publicId", "doctype.publicId", "\"\""),
    case("DocumentType.systemId", "doctype.systemId", "\"\""),
    // DOM, "Mixin NonElementParentNode" and "Mixin ParentNode" of a fragment.
    case(
        "DocumentFragment.getElementById",
        "fragment().getElementById(\"u\")",
        "ul",
    ),
    case(
        "DocumentFragment.getElementById",
        "fragment().getElementById(\"a\")",
        "null",
    ),
    case("DocumentFragment.children", "fragment().children[0]", "ul"),
    case(
        "DocumentFragment.children",
        "(function () { var f = fragment(); return f.children === f.children; })()",
        "true",
    ),
    case(
        "DocumentFragment.firstElementChild",
        "fragment().firstElementChild",
        "ul",
    ),
    case(
        "DocumentFragment.lastElementChild",
        "fragment().lastElementChild",
        "ul",
    ),
    case(
        "DocumentFragment.childElementCount",
        "fragment().childElementCount",
        "1",
    ),
    // DOM, "Interface NodeList".
    case("NodeList.item", "a.childNodes.item(0)", "p"),
    case("NodeList.item", "a.childNodes.item(4)", "null"),
    case("NodeList.length", "a.childNodes.length", "4"),
    // Web IDL, "Iterable declarations", over an indexed getter: the Array iteration methods.
    case(
        "NodeList.iterable",
        "(function () { var n = 0; for (var x of a.childNodes) n++; return n; })()",
        "4",
    ),
    case("NodeList.iterable", "[...a.childNodes][3]", "span"),
    // DOM, "Interface HTMLCollection".
    case("HTMLCollection.length", "a.children.length", "2"),
    case("HTMLCollection.item", "a.children.item(1)", "span"),
    case("HTMLCollection.item", "a.children.item(2)", "null"),
    case(
        "HTMLCollection.namedItem",
        "body.getElementsByTagName(\"*\").namedItem(\"p1\")",
        "p",
    ),
    case(
        "HTMLCollection.namedItem",
        "body.getElementsByTagName(\"*\").namedItem(\"n1\")",
        "p",
    ),
    case(
        "HTMLCollection.namedItem",
        "body.getElementsByTagName(\"*\").namedItem(\"cn\")",
        "null",
    ),
    case(
        "HTMLCollection.namedItem",
        "body.getElementsByTagName(\"*\").namedItem(\"\")",
        "null",
    ),
    // HTML, "The Window object": window, self, document; DOM, "Interface Window" (event).
    case("Window.window", "window.window", "G"),
    case("Window.self", "self", "G"),
    case("Window.document", "window.document", "document"),
    case("Window.document", "G.document.nodeType", "9"),
    case("Window.event", "window.event", "undefined"),
];

/// Runs one entry: a runner of its own over the table's tree, `expression === expected`
/// thrown on a mismatch, the end `Ran`, and no limit recorded.
fn run_case(case: &Case) {
    let source = format!(
        "{CHECK}{PRELUDE}\nvar got = ({});\nif (got !== ({})) {{ throw new Error({:?} + \": \" + String(got)); }}",
        case.expression, case.expected, case.row
    );
    run_document(
        limits(),
        Mode::Test,
        || document(table_tree(), info()),
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

/// Runs every entry whose row is of one of `interfaces`; answers how many ran.
fn run_rows(interfaces: &[&str]) -> usize {
    let mut count = 0;
    for case in CASES {
        let interface = case.row.split('.').next().unwrap_or_default();
        if interfaces.contains(&interface) {
            run_case(case);
            count += 1;
        }
    }
    count
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
        .filter(|(_, _, slice, _)| slice == "reading")
        .map(|(interface, member, _, _)| format!("{interface}.{member}"))
        .collect();
    let mut per_interface: BTreeMap<&str, usize> = BTreeMap::new();
    for row in &rows {
        *per_interface
            .entry(row.split('.').next().unwrap_or_default())
            .or_default() += 1;
    }
    let expected: BTreeMap<&str, usize> = [
        ("Node", 41),
        ("Element", 15),
        ("Document", 18),
        ("CharacterData", 5),
        ("Text", 2),
        ("ProcessingInstruction", 1),
        ("DocumentType", 3),
        ("DocumentFragment", 5),
        ("NodeList", 3),
        ("HTMLCollection", 3),
        ("Window", 4),
    ]
    .into_iter()
    .collect();
    assert_eq!(per_interface, expected);
    assert_eq!(rows.len(), 100, "99 members and NodeList.iterable");
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

#[test]
fn node_rows() {
    assert!(run_rows(&["Node"]) > 41);
}

#[test]
fn element_rows() {
    assert!(run_rows(&["Element"]) > 15);
}

#[test]
fn document_rows() {
    assert!(run_rows(&["Document"]) > 18);
}

#[test]
fn returned_interface_rows() {
    let interfaces = [
        "CharacterData",
        "Text",
        "ProcessingInstruction",
        "DocumentType",
        "DocumentFragment",
        "NodeList",
        "HTMLCollection",
    ];
    assert!(run_rows(&interfaces) > 21);
}

#[test]
fn window_rows() {
    assert!(run_rows(&["Window"]) >= 4);
}

/// One wrapper per node for the document's life: every member that returns a node answers the
/// same object for it, and a property a script set on a wrapper is read back in a later script,
/// after a collection, though no script holds the wrapper.
#[test]
fn one_wrapper_per_node() {
    run_document(
        limits(),
        Mode::Test,
        || document(table_tree(), info()),
        |runner| {
            ran(
                runner,
                &format!(
                    r#"{CHECK}{PRELUDE}
check(a.parentNode === a.parentNode, "parentNode");
check(document.documentElement === document.getElementById("a").ownerDocument.documentElement,
  "documentElement");
var getters = ["parentNode", "parentElement", "firstChild", "lastChild", "previousSibling",
  "nextSibling", "ownerDocument", "firstElementChild", "lastElementChild",
  "previousElementSibling", "nextElementSibling"];
var nodes = [a, p, one, two, note, span, html, body, document, doctype, pi];
for (var i = 0; i < nodes.length; i++) {{
  for (var k = 0; k < getters.length; k++) {{
    var v = nodes[i][getters[k]];
    check(v === nodes[i][getters[k]], getters[k] + " of node " + i);
  }}
  check(nodes[i].getRootNode() === nodes[i].getRootNode(), "getRootNode " + i);
}}
check(document.doctype === doctype && G.document === document, "doctype, document");
check(a.childNodes[0] === p && a.children.item(0) === p, "a collection's nodes");
document.getElementById("s1").marker = 42;
"#
                ),
            );
            let collected = runner.agent_and_realm().0.collect();
            assert!(
                matches!(collected, Ok(swell::Outcome::Normal(_))),
                "{collected:?}"
            );
            ran(
                runner,
                r#"if (document.getElementById("s1").marker !== 42) throw new Error("lost");
                   if (document.getElementById("a").lastChild.marker !== 42) throw new Error("lost");"#,
            );
        },
    )
    .expect("a runner");
}

/// The collections: iteration, spread, `length`, `item`, the index properties, `forEach`, the
/// same node object from a collection and from a direct member, and `namedItem`.
#[test]
fn collections() {
    run_document(
        limits(),
        Mode::Test,
        || document(table_tree(), info()),
        |runner| {
            ran(
                runner,
                &format!(
                    r#"{CHECK}{PRELUDE}
var seen = [];
for (var x of document.childNodes) seen.push(x);
check(seen.length === 2 && seen[0] === doctype && seen[1] === html, "for of");
var nodes = body.childNodes;
var spread = [...nodes];
check(spread.length === nodes.length && nodes.length === 7, "spread " + spread.length);
for (var i = 0; i < nodes.length; i++) {{
  check(nodes.item(i) === nodes[i] && spread[i] === nodes[i], "item " + i);
}}
var n = 0;
nodes.forEach(function (node, index, list) {{
  check(node === nodes[index] && list === nodes, "forEach " + index); n++;
}});
check(n === 7, "forEach count");
check(body.childNodes[0] === body.firstChild && body.childNodes[0] === a, "same object");
check(document.documentElement.childNodes[0] === document.documentElement.firstChild,
  "the document element's first child");
var all = body.getElementsByTagName("*");
check(all.namedItem("p1") === p && all.namedItem("n1") === p, "by id and by name");
check(all.namedItem("") === null && all.namedItem("missing") === null, "none");
check(all.namedItem("cn") === null && all.namedItem("ci") === circle, "SVG: by id only");
check(all["ci"] === undefined, "no named properties");
"#
                ),
            );
        },
    )
    .expect("a runner");
}

/// The deviations the selection names: a snapshot does not follow the tree; `[SameObject]` as
/// the same object refreshed when read, with the old index properties gone; a new object per
/// call of a `getElementsBy...`.
#[test]
fn snapshots_and_the_same_object() {
    run_document(
        limits(),
        Mode::Test,
        || document(table_tree(), info()),
        |runner| {
            ran(
                runner,
                &format!(
                    r#"{CHECK}{PRELUDE}
var lis = document.getElementsByTagName("li");
var kept = a.childNodes;
var children = a.children;
check(kept.length === 4 && children.length === 2 && lis.length === 4, "before");
a.removeChild(span);
check(kept.length === 4 && kept[3] === span, "a snapshot does not change when the tree would");
check(lis.length === 4, "nor does a getElementsBy... snapshot");
check(a.childNodes === kept && kept.length === 3, "refreshed when read");
check(kept[3] === undefined && !("3" in kept), "the old index property is gone");
check(a.children === children && children.length === 1 && !("1" in children), "children");
check(document.children === document.children, "the document's children");
var f = fragment();
check(f.children === f.children && f.children.length === 1, "a fragment's children");
check(document.getElementsByTagName("p") !== document.getElementsByTagName("p"),
  "getElementsByTagName: a new object per call");
check(document.getElementsByClassName("c") !== document.getElementsByClassName("c"),
  "getElementsByClassName: a new object per call");
"#
                ),
            );
        },
    )
    .expect("a runner");
}

/// The write counter: a second read after no write answers the kept object for its one step and
/// visits no node, whatever the children; after a write it answers the same object with the
/// new children.
#[test]
fn a_read_after_no_write_costs_one_step() {
    let page = format!("<div id=\"a\">{}</div>", "<b></b>".repeat(1_000));
    with_page(limits(), page, info(), |runner| {
        ran(
            runner,
            r#"var a = document.getElementById("a"); var kept = a.childNodes;"#,
        );
        let one = steps(runner, "a.childNodes;");
        let two = steps(runner, "a.childNodes; a.childNodes;");
        let plain_one = steps(runner, "a.nodeType;");
        let plain_two = steps(runner, "a.nodeType; a.nodeType;");
        eprintln!(
            "childNodes over 1,000 children, kept: one read {one} steps, two {two}; nodeType \
             {plain_one}, {plain_two}"
        );
        assert_eq!(two - one, plain_two - plain_one, "a second read: one step");
        assert!(one < 100, "a read of a kept object visits no node: {one}");
        ran(
            runner,
            &format!(
                r#"{CHECK}
a.removeChild(a.firstChild);
check(a.childNodes === kept && kept.length === 999, "refreshed: " + kept.length);
check(kept[999] === undefined, "the last index property is gone");"#
            ),
        );
    });
}

/// `Object.prototype.toString` names the interface of the wrapper's prototype; a custom
/// element's wrapper is an `Element`. At the top level of a script `this`, `globalThis`,
/// `window` and `self` are the global object.
#[test]
fn the_tags_and_the_global() {
    let page = "<div id=\"d\">t</div><custom-x id=\"x\"></custom-x>".to_string();
    with_page(limits(), page, info(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var d = document.getElementById("d");
var tag = function (v) {{ return Object.prototype.toString.call(v); }};
check(tag(d) === "[object Element]", tag(d));
check(tag(document.getElementById("x")) === "[object Element]", "custom-x");
check(tag(d.firstChild) === "[object Text]", "text");
check(tag(document) === "[object Document]", "document");
check(tag(d.childNodes) === "[object NodeList]", "NodeList");
check(tag(d.children) === "[object HTMLCollection]", "HTMLCollection");
check(this === window && globalThis === window, "this and globalThis");
check(window.window === window && self === window, "window and self");
check(window.document === document && window.event === undefined, "document and event");
"#
            ),
        );
    });
}

/// `document.title`: the first `title` element's text, stripped and collapsed; the empty
/// string when there is none.
#[test]
fn the_title() {
    for (page, title) in [
        ("<title>\t a \n b  </title><title>c</title>", "a b"),
        ("<p>no title</p>", ""),
        ("<title></title>", ""),
    ] {
        with_page(limits(), page.to_string(), info(), |runner| {
            ran(
                runner,
                &format!(
                    "if (document.title !== {title:?}) throw new Error(JSON.stringify(document.title));"
                ),
            );
        });
    }
}

/// `compatMode` follows the mode the parser wrote into the tree, and so does the match of
/// `getElementsByClassName`, ASCII case-insensitive in quirks mode.
#[test]
fn the_mode() {
    let cases = [
        ("", "BackCompat", 1),
        ("<!DOCTYPE html>", "CSS1Compat", 0),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \
             \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">",
            "CSS1Compat",
            0,
        ),
    ];
    for (doctype, mode, found) in cases {
        let page = format!("{doctype}<div class=\"a\"></div>");
        with_page(limits(), page, info(), |runner| {
            ran(
                runner,
                &format!(
                    r#"{CHECK}
check(document.compatMode === {mode:?}, document.compatMode);
check(document.getElementsByClassName("A").length === {found}, "class A");
check(document.getElementsByClassName("a").length === 1, "class a");"#
                ),
            );
        });
    }
}

/// The facts of `DocumentInfo`, and its default: `about:blank`, `text/html`, `UTF-8`.
#[test]
fn the_document_info() {
    let given = DocumentInfo {
        address: Url::parse("https://example.com/x?q#f", None).expect("an address"),
        content_type: "application/xhtml+xml".to_string(),
        encoding: "windows-1252".to_string(),
    };
    with_page(limits(), "<p></p>".to_string(), given, |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
check(document.URL === "https://example.com/x?q#f", document.URL);
check(document.documentURI === document.URL, "documentURI");
check(document.contentType === "application/xhtml+xml", "contentType");
check(document.characterSet === "windows-1252" && document.charset === "windows-1252"
  && document.inputEncoding === "windows-1252", "the encoding");"#
            ),
        );
    });
    let default = DocumentInfo::default();
    assert_eq!(default.address.href(), "about:blank");
    assert_eq!(default.content_type, "text/html");
    assert_eq!(default.encoding, "UTF-8");
    with_page(limits(), "<p></p>".to_string(), default, |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
check(document.URL === "about:blank" && document.documentURI === "about:blank", "URL");
check(document.baseURI === "about:blank", "baseURI " + document.baseURI);
check(document.documentElement.baseURI === "about:blank", "an element's baseURI");
check(document.contentType === "text/html" && document.characterSet === "UTF-8", "defaults");"#
            ),
        );
    });
}

/// `baseURI` over the address `https://example.com/dir/page`: the document base URL, one case
/// per line.
#[test]
fn the_base_url() {
    let long = format!(
        "https://other.test/{}",
        "a".repeat(surf_url::MAX_INPUT_LENGTH)
    );
    let cases: Vec<(String, &str)> =
        vec![
        ("<p id=\"t\"></p>".into(), ADDRESS),
        (
            "<p id=\"t\"></p><base href=\"https://other.test/x/\">".into(),
            "https://other.test/x/",
        ),
        (
            "<base href=\"https://one.test/\"><base href=\"https://two.test/\"><p id=\"t\"></p>"
                .into(),
            "https://one.test/",
        ),
        (
            "<base target=\"x\"><base href=\"https://two.test/\"><p id=\"t\"></p>".into(),
            "https://two.test/",
        ),
        ("<base target=\"x\"><p id=\"t\"></p>".into(), ADDRESS),
        ("<base href=\"http://f:b/c\"><p id=\"t\"></p>".into(), ADDRESS),
        (
            "<base href=\"../up/\"><p id=\"t\"></p>".into(),
            "https://example.com/up/",
        ),
        (format!("<base href=\"{long}\"><p id=\"t\"></p>"), ADDRESS),
        (
            "<svg><base href=\"https://svg.test/\"></base></svg><p id=\"t\"></p>".into(),
            ADDRESS,
        ),
    ];
    for (page, expected) in cases {
        with_page(limits(), page, info(), |runner| {
            ran(
                runner,
                &format!(
                    r#"var t = document.getElementById("t");
if (t.baseURI !== {expected:?} || document.baseURI !== {expected:?}) throw new Error(t.baseURI);"#
                ),
            );
        });
    }
}

/// The three deviations of `baseURI` are named in the note of its row.
#[test]
fn the_deviations_of_base_uri_are_in_its_note() {
    let rows = selection();
    let (_, _, slice, note) = rows
        .iter()
        .find(|(interface, member, _, _)| interface == "Node" && member == "baseURI")
        .expect("the row Node baseURI");
    assert_eq!(slice, "reading");
    let deviations: Vec<&str> = note
        .split("; ")
        .filter_map(|clause| clause.strip_prefix("deviation: "))
        .collect();
    assert_eq!(deviations.len(), 3, "{note}");
    assert!(deviations.iter().any(|d| d.contains("base-uri")), "{note}");
    assert!(
        deviations.iter().any(|d| d.contains("about:srcdoc")),
        "{note}"
    );
    assert!(deviations.iter().any(|d| d.contains("UTF-8")), "{note}");
}

/// A hook of the test: in `run` it reads `document.compatMode` in a runner over a clone of the
/// tree the parse built so far, and keeps it.
#[derive(Default)]
struct ReadMode {
    read: Vec<String>,
}

impl ScriptHook for ReadMode {
    fn run(&mut self, tree: &mut Tree, _script: NodeId) {
        let clone = tree.clone();
        let end = run_document(
            limits(),
            Mode::Test,
            move || document(clone, DocumentInfo::default()),
            |runner| runner.evaluate("throw new Error(document.compatMode);"),
        )
        .expect("a runner");
        match end {
            ScriptEnd::Threw(error) => self.read.push(error.message.unwrap_or_default()),
            other => self.read.push(format!("{other}")),
        }
    }

    fn skip(&mut self, _tree: &Tree, _script: NodeId, _reason: NotRun) {}
}

/// A script in `head` reads the mode during the parse: the parser wrote it into the tree where
/// the initial insertion mode decided it. The hooked parse builds the plain parse's tree.
#[test]
fn a_script_in_the_parse_reads_the_mode() {
    let options = ParseOptions {
        scripting_disabled: false,
        ..ParseOptions::default()
    };
    for (page, mode) in [
        (
            "<html><head><script>x</script></head><body></body></html>",
            "BackCompat",
        ),
        (
            "<!DOCTYPE html><html><head><script>x</script></head><body></body></html>",
            "CSS1Compat",
        ),
    ] {
        let mut hook = ReadMode::default();
        let hooked = parse_document_with_hook(page, &options, &mut hook);
        assert_eq!(hook.read, [mode], "{page}");
        assert_eq!(hooked, parse_document(page, &options), "{page}");
    }
}

/// A parent with `count` children made of `child`.
fn parent_of(count: usize, child: &str) -> String {
    format!("<p id=\"p\">{}</p>", child.repeat(count))
}

/// The steps of `walk` over `count` children made of `child`.
fn walk_steps(count: usize, child: &str, walk: &str) -> u64 {
    with_page(limits(), parent_of(count, child), info(), |runner| {
        ran(runner, r#"var p = document.getElementById("p");"#);
        steps(runner, walk)
    })
}

/// Sibling reads by position hints: `walk` over `small` times 100 children made of `child`
/// costs fewer than 150 times its steps over `small` (a linear walk gives 100 times, a scan from
/// the start at each read 10,000 times). Each walk counts what it visits and checks the count
/// with `childElementCount`.
fn assert_linear(name: &str, small: usize, child: &str, walk: &str) {
    let started = Instant::now();
    let few = walk_steps(small, child, walk);
    let many = walk_steps(small * 100, child, walk);
    eprintln!(
        "{name}: {small} children {few} steps, {} children {many} steps, {:?}",
        small * 100,
        started.elapsed()
    );
    assert!(many < 150 * few, "{many} >= 150 x {few}");
}

#[test]
fn a_sibling_walk_is_linear() {
    assert_linear(
        "nextSibling",
        1_000,
        "<b></b>",
        "var n = 0; for (var x = p.firstChild; x; x = x.nextSibling) n++; if (n !== p.childElementCount) throw new Error(String(n));",
    );
}

#[test]
fn a_backward_sibling_walk_is_linear() {
    assert_linear(
        "previousSibling",
        1_000,
        "<b></b>",
        "var n = 0; for (var x = p.lastChild; x; x = x.previousSibling) n++; if (n !== p.childElementCount) throw new Error(String(n));",
    );
}

/// The element variants over a parent with a text node between each element: `Element`'s, and
/// `CharacterData`'s from each text node.
#[test]
fn an_element_sibling_walk_is_linear() {
    assert_linear(
        "nextElementSibling",
        500,
        "x<b></b>",
        "var n = 0; for (var x = p.firstElementChild; x; x = x.nextElementSibling) n++; if (n !== p.childElementCount) throw new Error(String(n));",
    );
}

#[test]
fn a_backward_element_sibling_walk_is_linear() {
    assert_linear(
        "previousElementSibling",
        500,
        "x<b></b>",
        "var n = 0; for (var x = p.lastElementChild; x; x = x.previousElementSibling) n++; if (n !== p.childElementCount) throw new Error(String(n));",
    );
}

#[test]
fn a_character_data_element_sibling_walk_is_linear() {
    assert_linear(
        "CharacterData nextElementSibling",
        500,
        "x<b></b>",
        "var n = 0; for (var t = p.firstChild; t; t = t.nextElementSibling ? t.nextElementSibling.nextSibling : null) n++; if (n !== p.childElementCount) throw new Error(String(n));",
    );
}

/// Two walks in step over two parents; a walk that goes on after a write between two reads; a
/// hint that names another node costs a scan and answers the right node.
#[test]
fn sibling_hints_are_never_trusted() {
    let items = |tag: &str, prefix: &str| -> String {
        (0..100)
            .map(|i| format!("<{tag}>{prefix}{i}</{tag}>"))
            .collect()
    };
    let page = format!(
        "<div id=\"p1\">{}</div><div id=\"p2\">{}</div>",
        items("b", ""),
        items("i", "q")
    );
    with_page(limits(), page, info(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var p1 = document.getElementById("p1"), p2 = document.getElementById("p2");
var x = p1.firstChild, y = p2.firstChild, i = 0;
while (x && y) {{
  check(x.textContent === String(i) && y.textContent === "q" + i, "in step at " + i);
  x = x.nextSibling; y = y.lastChild ? y.nextSibling : null; i++;
}}
check(i === 100 && x === null && y === null, "both ends: " + i);
var n = p1.firstChild;
for (var k = 0; k < 10; k++) n = n.nextSibling;
p1.removeChild(p1.childNodes[5]);
check(n.textContent === "10" && n.nextSibling.textContent === "11", "after a removal before");
p1.removeChild(n.nextSibling);
check(n.nextSibling.textContent === "12", "the node after the removed one");
check(n.previousSibling.textContent === "9", "backwards after a removal");
p1.removeChild(n);
check(n.nextSibling === null && n.previousSibling === null, "a node no longer a child");
"#
            ),
        );
        // The hint of `p1` names index 50; a read about its 10th child misses and scans.
        ran(
            runner,
            "var m = p1.firstChild; for (var k = 0; k < 50; k++) m = m.nextSibling; var t = p1.childNodes[10];",
        );
        let scan = steps(runner, "var u = t.nextSibling;");
        let hit = steps(runner, "var v = t.nextSibling;");
        ran(
            runner,
            &format!(
                r#"{CHECK}check(u === v && u === p1.childNodes[11] && u.textContent === "{}", u.textContent);"#,
                // children 5, 11 and 10 were removed: index 11 holds the item 14.
                14
            ),
        );
        eprintln!("a missed hint: {scan} steps; a hit: {hit} steps");
        assert!(scan >= hit + 10, "a miss scans: {scan} against {hit}");
    });
}

/// The memory of a wrapper, measured by a probe before the code: 423 bytes per wrapper with its
/// `NodeData` (1,000 wrappers made against 1,000 calls that answer `null`), 349 bytes per empty
/// collection object, and about 121 bytes per node of a collection beside its wrapper (the
/// index property). This walk measured 335 bytes per node on its first run and 1,520 bytes in
/// all on its second. The assertions are the floor of 256 bytes, `HOST_OBJECT_LIVE_BYTES`, per
/// wrapper, and a second run under a tenth of the first.
#[test]
fn wrapper_charge_and_reuse() {
    let page = parent_of(1_000, "<b></b>");
    with_page(limits(), page, info(), |runner| {
        ran(runner, r#"var p = document.getElementById("p");"#);
        let walk = "for (var x = p.firstChild; x; x = x.nextSibling) {}";
        let before = heap(runner);
        ran(runner, walk);
        let first = heap(runner) - before;
        let before = heap(runner);
        ran(runner, walk);
        let second = heap(runner) - before;
        eprintln!(
            "1,000 wrappers: the first walk grows the heap by {first} bytes ({} a node), the \
             second by {second}",
            first / 1_000
        );
        assert_eq!(HOST_OBJECT_LIVE_BYTES, 256);
        assert!(first >= 1_000 * HOST_OBJECT_LIVE_BYTES, "{first}");
        assert!(second * 10 < first, "a second wrapper per node: {second}");
    });
}

/// Under a memory budget of 1 MiB a script that wraps every node of 100,000 ends at the budget;
/// the wrappers stay rooted, so the heap stays near the budget, and never above it by more
/// than one wrapper.
#[test]
fn wrapper_limit() {
    let page = parent_of(100_000, "<b></b>");
    let (end, later, after) = with_page(small_limits(), page, info(), |runner| {
        let end = runner.evaluate(
            r#"var p = document.getElementById("p");
               for (var x = p.firstChild; x; x = x.nextSibling) {}"#,
        );
        let later = runner.evaluate("1 + 1");
        (end, later, heap(runner))
    });
    eprintln!("wrapper_limit: {end}; then {later}; heap after {after}");
    assert_eq!(end, ScriptEnd::Limit(Reason::Memory), "{end}");
    assert!(
        matches!(later, ScriptEnd::Ran | ScriptEnd::Limit(Reason::Memory)),
        "{later}"
    );
    let wrapper = HOST_OBJECT_LIVE_BYTES + std::mem::size_of::<NodeId>() as u64;
    assert!(after < 1024 * 1024 + wrapper, "{after}");
}

/// A script that makes collections without end ends at the budget or at the steps, and the
/// next script runs.
#[test]
fn collection_limit() {
    let (end, later) = with_page(small_limits(), PAGE.to_string(), info(), |runner| {
        let end = runner.evaluate(r#"for (;;) document.getElementsByTagName("*");"#);
        (end, runner.evaluate("1 + 1"))
    });
    eprintln!("collection_limit: {end}; then {later}");
    assert!(
        matches!(
            end,
            ScriptEnd::Limit(Reason::Memory | Reason::StepsPerScript)
        ),
        "{end}"
    );
    assert_eq!(later, ScriptEnd::Ran);
}

/// The steps of `source` over a page of 1,000 of `child` under `<p id="p">`.
fn steps_over_1000(child: &str, source: &str) -> u64 {
    with_page(limits(), parent_of(1_000, child), info(), |runner| {
        ran(runner, r#"var p = document.getElementById("p");"#);
        steps(runner, source)
    })
}

/// A member whose work grows with the tree charges a step per node it visits.
#[test]
fn steps_in_proportion() {
    let tags = steps_over_1000("<b></b>", r#"document.getElementsByTagName("*");"#);
    let text = steps_over_1000("<b>t</b>", "p.textContent;");
    let contains = steps_over_1000("<b></b>", "document.contains(p.lastChild);");
    let position = steps_over_1000(
        "<b></b>",
        "p.firstChild.compareDocumentPosition(p.lastChild);",
    );
    eprintln!(
        "steps over 1,000 children: getElementsByTagName {tags}, textContent {text}, contains \
         {contains}, compareDocumentPosition {position}"
    );
    assert!(tags >= 1_000, "{tags}");
    assert!(text >= 1_000, "{text}");
    assert!(contains >= 1_000, "{contains}");
    assert!(position >= 999, "{position}");
}

/// A String a member builds costs a step per 64 bytes.
#[test]
fn string_steps() {
    let read = |bytes: usize| {
        let page = format!("<p id=\"p\">{}</p>", "a".repeat(bytes));
        with_page(limits(), page, info(), |runner| {
            ran(
                runner,
                r#"var t = document.getElementById("p").firstChild;"#,
            );
            steps(runner, "t.textContent;")
        })
    };
    let (long, short) = (read(64_000), read(64));
    eprintln!("textContent of 64,000 bytes: {long} steps; of 64 bytes: {short}");
    assert!(long >= short + 999, "{long} against {short}");
}

/// A text of 16,777,217 bytes, one more than the session's `string_length`, parsed under raised
/// limits. The story expected `textContent` to throw a `RangeError`; at swell 0.8.0 the engine
/// applies `string_length` to a String made at run time from other values only, not to one the
/// host makes with `new_string`, and the read ends with the whole String. The case asserts the
/// end it reached: the String's length.
#[test]
fn string_length() {
    let started = Instant::now();
    let parse_limits = ParserLimits {
        max_input_bytes: 32 * 1024 * 1024,
        max_stored_bytes: 128 * 1024 * 1024,
        ..ParserLimits::default()
    };
    let mut session = limits();
    session.engine = SESSION_ENGINE_LIMITS;
    let end = run_document(
        session,
        Mode::Test,
        move || {
            let text = "a".repeat(16_777_217);
            let options = ParseOptions {
                limits: parse_limits,
                ..ParseOptions::default()
            };
            Document {
                tree: parse_document(&text, &options),
                limits: parse_limits,
                slices: slices(DocumentInfo::default()),
            }
        },
        |runner| {
            runner.evaluate(
                r#"var t = document.documentElement.lastChild.firstChild;
                   if (t.length !== 16777217) throw new Error("length " + t.length);
                   var read = t.textContent;
                   if (read.length !== 16777217) throw new Error("read " + read.length);"#,
            )
        },
    )
    .expect("a runner");
    eprintln!("string_length: {end} in {:?}", started.elapsed());
    assert_eq!(end, ScriptEnd::Ran, "{end}");
}

/// The loops over a collection read `length` and the index properties: after no write a read of
/// `childNodes` is its kept object, so 5,000 children cost fewer than 15 times the steps of 500
/// (a refresh at each read gives 100 times); `length` copies nothing.
#[test]
fn collection_loops_are_linear() {
    let child_nodes = "for (var i = 0; i < p.childNodes.length; i++) p.childNodes[i];";
    let tags =
        r#"var c = document.getElementsByTagName("*"); for (var i = 0; i < c.length; i++) c[i];"#;
    for (name, walk) in [("childNodes", child_nodes), ("getElementsByTagName", tags)] {
        let small = walk_steps(500, "<b></b>", walk);
        let large = walk_steps(5_000, "<b></b>", walk);
        eprintln!("collection loop {name}: 500 {small} steps, 5,000 {large} steps");
        assert!(large < 15 * small, "{name}: {large} >= 15 x {small}");
    }
}

/// Trees made to be hard to walk: every member ends on the runner's thread, `Ran`.
#[test]
fn hostile_trees() {
    let deep = format!("{}x{}", "<div>".repeat(500), "</div>".repeat(500));
    with_page(limits(), deep, info(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var divs = document.getElementsByTagName("div");
var deepest = divs[divs.length - 1];
check(divs.length === 500, "500 divs: " + divs.length);
check(deepest.getRootNode() === document, "getRootNode");
check(document.contains(deepest), "contains");
check(deepest.compareDocumentPosition(document) === 10, "compareDocumentPosition");
check(document.documentElement.lastChild.textContent === "x", "textContent");
check(divs[0].isEqualNode(divs[0]), "isEqualNode of itself");"#
            ),
        );
    });
    let chain = |id: &str, text: &str| {
        format!(
            "<div id=\"{id}\">{}{text}{}</div>",
            "<div>".repeat(249),
            "</div>".repeat(249)
        )
    };
    let page = format!("{}{}{}", chain("a", "t"), chain("b", "t"), chain("c", "u"));
    with_page(limits(), page, info(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var a = document.getElementById("a"), b = document.getElementById("b");
var c = document.getElementById("c");
check(a.firstChild.isEqualNode(b.firstChild), "equal chains");
check(!a.firstChild.isEqualNode(c.firstChild), "chains whose texts differ");
check(!a.isEqualNode(b), "chains whose ids differ");"#
            ),
        );
    });
    with_page(limits(), parent_of(100_000, "<i>a</i>"), info(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var p = document.getElementById("p");
var first = p.firstChild, last = p.lastChild;
check(p.textContent.length === 100000, "textContent");
check(last.getRootNode() === document, "getRootNode");
check(last.compareDocumentPosition(first) === 2, "preceding");
check(first.compareDocumentPosition(last) === 4, "following");"#
            ),
        );
    });
}

/// The members of the other slices still throw `NotSupportedError` naming the member.
#[test]
fn other_slices_still_throw() {
    run_document(
        limits(),
        Mode::Test,
        || Document {
            tree: parse_document("<p id=\"p\"></p>", &ParseOptions::default()),
            limits: ParserLimits::default(),
            slices: dom::slices(DocumentInfo::default()),
        },
        |runner| {
            ran(
                runner,
                &format!(
                    r#"{CHECK}
function notSupported(f, message) {{
  try {{ f(); }} catch (e) {{
    check(e instanceof DOMException && e.name === "NotSupportedError" && e.message === message,
      message + ": " + e.name + ": " + e.message);
    return;
  }}
  throw new Error(message + ": no throw");
}}
var el = document.getElementById("p");
notSupported(function () {{ document.appendChild(el); }}, "Node.appendChild is not supported");
notSupported(function () {{ el.getAttribute("id"); }}, "Element.getAttribute is not supported");
notSupported(function () {{ el.querySelector("p"); }}, "Element.querySelector is not supported");"#
                ),
            );
        },
    )
    .expect("a runner");
}
