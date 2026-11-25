//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`): a hostile page that
//! chooses the markup and every call. Each test names the claim it holds; the failing ones show
//! a defect of the unit at `ace7176`.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeId, ParseOptions, Tree, WriteError, parse_document};
use surf_script::bindings::{DomCx, DomEnd, DomException, DomResult, Mutation, Slices};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, run_document};
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

fn small_limits() -> RunnerLimits {
    let mut small = limits();
    small.engine.memory_bytes = 1024 * 1024;
    small
}

fn info() -> DocumentInfo {
    DocumentInfo {
        address: Url::parse(ADDRESS, None).expect("the address"),
        content_type: "text/html".to_string(),
        encoding: "UTF-8".to_string(),
    }
}

/// The test's `mutation` slice: `appendChild`, `removeChild` and `createDocumentFragment`
/// through the tree's public write surface.
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

fn heap(runner: &mut Runner) -> u64 {
    runner.agent_and_realm().0.heap_bytes().expect("heap_bytes")
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
/// (`refresh_collection` at `ace7176` deleted only the indices of the old data, which it changed
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
/// (a kept collection must not answer the old view).
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
/// hundred steps (the work outruns the steps past the wall clock).
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
/// with the comments before `html` (a member whose work grows with the
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
        "for (var i = 0; i < 10; i++) document.documentElement;",
        "for (var i = 0; i < 10; i++) document.doctype;",
        "for (var i = 0; i < 10; i++) document.lookupNamespaceURI(null);",
    ] {
        let (few, few_time) = read(0, source);
        let (many, many_time) = read(100_000, source);
        lines.push(format!(
            "{source}: 0 comments {few} steps {few_time:?}; 2 x 100,000 comments {many} steps {many_time:?}"
        ));
        if many < few + 10 * 100_000 {
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

// The cases below held at ace7176.

/// Every hostile shape of a kept `childNodes` and `children`, then each write of the
/// test slice: the read after it never faults, and when it answers, the object is the fresh
/// walk. The ends are printed; a shape whose read throws is listed with its error.
#[test]
fn hostile_shapes_of_a_kept_collection() {
    let started = Instant::now();
    let mut kids = String::new();
    for _ in 0..1_001 {
        kids.push_str("<i></i>");
    }
    let page =
        format!("<div id=\"p\"><b></b></div><div id=\"q\"><u></u></div><div id=\"r\">{kids}</div>");
    let shapes = [
        "Object.freeze(c)",
        "Object.preventExtensions(c)",
        "Object.seal(c)",
        "Object.defineProperty(c, '0', { get: function () { return 'accessor'; }, configurable: true })",
        "Object.defineProperty(c, '0', { value: 'fixed', writable: false, configurable: false })",
        "Object.defineProperty(c, 'length', { value: 99 })",
        "delete c[0]",
        "c[0] = 'set'",
        "c[5] = document.getElementById('q')",
        "Object.setPrototypeOf(c, Array.prototype)",
        "NodeList.prototype.item = function () { return 'replaced'; }; HTMLCollection.prototype.item = NodeList.prototype.item",
        "c[Symbol.iterator] = function* () { yield 'own'; }",
        "Array.prototype.values = function () { return [][Symbol.iterator](); }",
        "Array.prototype[Symbol.iterator] = function () { return [][Symbol.iterator](); }",
    ];
    let mut report = Vec::new();
    for member in ["childNodes", "children"] {
        for shape in shapes {
            let source = format!(
                r#"{CHECK}
var p = document.getElementById("p"), q = document.getElementById("q"), r = document.getElementById("r");
var list = function (n) {{ return "{member}" === "childNodes" ? walk(n) : elements(n); }};
var c = p.{member};
var item = NodeList.prototype.item;
{shape};
var ends = [];
var writes = [
  function () {{ p.appendChild(q.firstChild); }},
  function () {{ p.removeChild(p.firstChild); }},
  function () {{ q.appendChild(p.firstChild); }},
  function () {{ while (p.firstChild) q.appendChild(p.firstChild); }},
  function () {{ while (r.firstChild) p.appendChild(r.firstChild); }},
  function () {{ while (p.firstChild) r.appendChild(p.firstChild); p.appendChild(q.firstChild); }},
];
for (var w = 0; w < writes.length; w++) {{
  writes[w]();
  var got;
  try {{ got = p.{member}; }} catch (e) {{ ends.push(w + ":" + e.name); continue; }}
  check(got === c, "the kept object after write " + w);
  var fresh = list(p);
  var own = Object.getOwnPropertyDescriptor(c, "0");
  var at0 = own && "value" in own ? own.value : undefined;
  if (Object.isFrozen(c) || "{shape}".indexOf("'0'") >= 0 || "{shape}".indexOf("c[0]") >= 0) {{ ends.push(w + ":ok"); continue; }}
  var len = Object.getOwnPropertyDescriptor("{member}" === "childNodes" ? NodeList.prototype : HTMLCollection.prototype, "length");
  var n = (len && len.get ? len.get.call(c) : fresh.length);
  check(n === fresh.length, "length after write " + w + ": " + n + " against " + fresh.length);
  for (var i = 0; i < fresh.length; i++) check(c[i] === fresh[i], "index " + i + " after write " + w);
  for (var k = fresh.length; k < fresh.length + 8; k++) {{
    if (k === 5 && "{shape}".indexOf("c[5]") >= 0) continue;
    check(c[k] === undefined || typeof c[k] !== "object", "index " + k + " past the length " + fresh.length + " after write " + w);
  }}
  ends.push(w + ":ok");
}}
throw new Error("ends " + ends.join(" "));
"#
            );
            let end = with_page(limits(), page.clone(), |runner| runner.evaluate(&source));
            let line = match &end {
                ScriptEnd::Threw(error) => error.message.clone().unwrap_or_default(),
                other => format!("{other}"),
            };
            report.push(format!("{member} | {shape} | {line}"));
            assert!(
                !matches!(end, ScriptEnd::Faulted),
                "{member} {shape}: faulted"
            );
        }
    }
    eprintln!(
        "hostile shapes ({:?}):\n{}",
        started.elapsed(),
        report.join("\n")
    );
    let wrong: Vec<&String> = report
        .iter()
        .filter(|line| !line.contains("| ends "))
        .collect();
    assert!(wrong.is_empty(), "wrong views:\n{wrong:#?}");
}

/// Every getter, setter and method of the slice's interfaces taken from its prototype and
/// called with a `this` it does not implement answers a `TypeError`.
#[test]
fn a_wrong_this_is_a_type_error() {
    let page = "<!DOCTYPE html><div id=\"a\">x<p id=\"p\" name=\"n\">y</p><!--c--></div>";
    with_page(limits(), page.to_string(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var el = document.getElementById("a"), txt = el.firstChild, com = el.lastChild, dt = document.doctype;
var nl = el.childNodes, hc = el.children, f = document.createDocumentFragment();
var common = [["undefined", undefined], ["null", null], ["plain", {{}}], ["NodeList", nl],
  ["HTMLCollection", hc], ["window", window], ["bound function", function () {{}}.bind(el)],
  ["a copy of an element", Object.assign({{}}, el)], ["an array", [el]],
  ["inherits an element", Object.create(el)], ["inherits a text", Object.create(txt)],
  ["inherits a list", Object.create(nl)], ["number", 1], ["string", "s"], ["symbol", Symbol()]];
var other = {{ Element: [["a text", txt], ["the document", document]],
  CharacterData: [["an element", el], ["a doctype", dt]], Text: [["a comment", com], ["an element", el]],
  DocumentType: [["an element", el]], Document: [["an element", el], ["a fragment", f]],
  DocumentFragment: [["the document", document], ["an element", el]],
  NodeList: [["an HTMLCollection", hc], ["an element", el]], HTMLCollection: [["a NodeList", nl], ["the document", document]],
  ProcessingInstruction: [["a text", txt]], Node: [] }};
var skip = ["constructor", "forEach", "entries", "keys", "values"];
var wrong = [], calls = 0;
Object.keys(other).forEach(function (name) {{
  var proto = G[name].prototype;
  Object.getOwnPropertyNames(proto).forEach(function (member) {{
    if (skip.indexOf(member) >= 0) return;
    var d = Object.getOwnPropertyDescriptor(proto, member);
    var fns = [];
    if (d.get) fns.push(["get", d.get]);
    if (d.set) fns.push(["set", d.set]);
    if (typeof d.value === "function") fns.push(["call", d.value]);
    fns.forEach(function (pair) {{
      common.concat(other[name]).forEach(function (w) {{ if (w[0] === name) return;
        calls++;
        try {{ pair[1].call(w[1], el, "p", 0); wrong.push(name + "." + member + " " + pair[0] + " this " + w[0] + ": no throw"); }}
        catch (e) {{ if (!(e instanceof TypeError)) wrong.push(name + "." + member + " " + pair[0] + " this " + w[0] + ": " + e.name + " " + e.message); }}
      }});
    }});
  }});
}});
check(calls > 1000, "calls " + calls);
check(wrong.length === 0, wrong.length + " wrong: " + wrong.slice(0, 20).join("; "));
"#
            ),
        );
    });
}

/// Arguments: missing, `undefined`, `null`, a Symbol, an object whose conversion throws or
/// answers an object, and conversions that read or write the same collection, the same node and
/// another member while the member converts them. The collection reads whole
/// after each.
#[test]
fn arguments_that_call_back_into_the_dom() {
    let page = "<!DOCTYPE html><div id=\"a\">x<p id=\"p\" name=\"n\">y</p><!--c--></div><div id=\"q\"><i></i></div>";
    with_page(limits(), page.to_string(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var el = document.getElementById("a"), q = document.getElementById("q");
var txt = el.firstChild, p = document.getElementById("p"), com = el.lastChild;
var c = el.childNodes, h = el.children, all = document.getElementsByTagName("*");
var seen = [];
var reader = {{ valueOf: function () {{
  seen.push(c.length, c.item(0) === txt, c[1] === p, h.length, h.namedItem("p") === p, all.item(0) === document.documentElement);
  for (var x of c) seen.push(x.nodeType);
  return 1; }} }};
check(c.item(reader) === p, "item with a reader");
check(h.item(reader) === null, "children item 1");
check(all.namedItem({{ toString: function () {{ seen.push(all.length, all.namedItem("n") === p); return "p"; }} }}) === p, "namedItem with a reader");
check(seen.join() === "3,true,true,1,true,true,3,1,8,3,true,true,1,true,true,3,1,8,7,true", "seen " + seen.join());
same(c, [txt, p, com], "after readers");
var thrower = {{ valueOf: function () {{ throw new RangeError("v"); }}, toString: function () {{ throw new RangeError("s"); }} }};
check(threw(function () {{ c.item(thrower); }}) === "RangeError", "item: the thrower's error");
check(threw(function () {{ h.namedItem(thrower); }}) === "RangeError", "namedItem: the thrower's error");
check(threw(function () {{ document.getElementById(thrower); }}) === "RangeError", "getElementById");
same(c, [txt, p, com], "after throwers");
check(threw(function () {{ c.item(Symbol()); }}) === "TypeError", "item(Symbol)");
check(threw(function () {{ h.namedItem(Symbol()); }}) === "TypeError", "namedItem(Symbol)");
check(threw(function () {{ c.item(); }}) === "TypeError", "item()");
check(threw(function () {{ h.namedItem(); }}) === "TypeError", "namedItem()");
check(threw(function () {{ txt.substringData(0); }}) === "TypeError", "substringData(0)");
var objects = {{ toString: function () {{ return {{}}; }}, valueOf: function () {{ return {{}}; }} }};
check(threw(function () {{ document.getElementById(objects); }}) === "TypeError", "an object from both conversions");
check(document.getElementById(undefined) === null && document.getElementById(null) === null, "undefined, null ids");
check(document.getElementsByTagName(null).length === 0, "a null tag name");
check(el.lookupNamespaceURI(undefined) === "http://www.w3.org/1999/xhtml" && el.lookupNamespaceURI(null) === "http://www.w3.org/1999/xhtml", "a null prefix");
check(el.contains(undefined) === false && el.contains(null) === false && el.isEqualNode(undefined) === false, "nullable nodes");
check(threw(function () {{ el.contains({{}}); }}) === "TypeError", "contains({{}})");
check(threw(function () {{ el.compareDocumentPosition(null); }}) === "TypeError", "compareDocumentPosition(null)");
check(threw(function () {{ el.getRootNode(1); }}) === "TypeError", "getRootNode(1)");
check(threw(function () {{ el.getRootNode({{ get composed() {{ throw new RangeError("c"); }} }}); }}) === "RangeError", "a throwing dictionary");
// A conversion that writes the tree: the member reads the tree as it is after the conversion.
var writer = {{ valueOf: function () {{ el.removeChild(txt); q.appendChild(txt); return 0; }} }};
check(txt.substringData(writer, 1) === "x", "substringData after a move");
check(c.item({{ valueOf: function () {{ el.removeChild(p); return 0; }} }}) === txt, "the snapshot's item while the tree moves");
check(el.childNodes === c, "the same object");
same(c, [com], "after writers");
el.appendChild(p);
same(el.childNodes, walk(el), "put back");
var tag = {{ toString: function () {{ q.appendChild(p); return "p"; }} }};
var got = el.getElementsByTagName(tag);
check(got.length === 0, "getElementsByTagName after the conversion moved the node: " + got.length);
"#
            ),
        );
    });
}

/// One wrapper per node: every pair of members that answer the same node answers the same
/// object; a property a script set survives `collect()` and a refill; a node the test slice
/// removed answers what the DOM says for a detached node, and is the same object when put back.
#[test]
fn one_object_per_node_through_every_member() {
    let page = "<!DOCTYPE html><html><head><title>t</title></head><body><div id=\"a\">x<p id=\"p\" name=\"n\" class=\"k\">y</p><!--c--></div></body></html>";
    with_page(limits(), page.to_string(), |runner| {
        ran(
            runner,
            &format!(
                r#"{CHECK}
var el = document.getElementById("a"), p = document.getElementById("p"), txt = el.firstChild, com = el.lastChild;
p.mark = 7;
var ways = [
  el.childNodes[1], el.childNodes.item(1), el.children[0], el.children.item(0), el.children.namedItem("p"),
  el.children.namedItem("n"), txt.nextSibling, com.previousSibling, txt.nextElementSibling, com.previousElementSibling,
  el.firstElementChild, el.lastElementChild, document.getElementsByTagName("p")[0],
  document.getElementsByTagNameNS("http://www.w3.org/1999/xhtml", "p")[0], document.getElementsByClassName("k")[0],
  el.getElementsByTagName("p").item(0), [...el.childNodes][1], Array.from(el.children)[0], p.firstChild.parentNode,
  p.firstChild.parentElement];
for (var i = 0; i < ways.length; i++) check(ways[i] === p, "way " + i);
check(p.ownerDocument === document && p.getRootNode() === document && document.documentElement.ownerDocument === document, "document");
check(document.documentElement === document.documentElement.lastChild.parentNode && document.doctype === document.firstChild && document.doctype === document.childNodes[0], "documentElement, doctype");
for (var x of el.childNodes) if (x.nodeType === 1) check(x === p, "iteration");
el.childNodes.forEach(function (n, i, l) {{ if (i === 1) check(n === p && l === el.childNodes, "forEach"); }});
document.documentElement.lastChild.getElementsByTagName("*")[1].tag = "kept";
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
            &format!(
                r#"{CHECK}
var el = document.getElementById("a"), p = document.getElementById("p");
check(p.mark === 7 && p.tag === "kept", "properties after collect");
var c = el.childNodes;
el.removeChild(p);
check(p.parentNode === null && p.parentElement === null && p.isConnected === false, "detached: parent");
check(p.getRootNode() === p && p.ownerDocument === document, "detached: root " + p.getRootNode());
check(document.contains(p) === false && p.contains(p.firstChild), "detached: contains");
var pos = p.compareDocumentPosition(document), back = document.compareDocumentPosition(p);
check((pos & 1) === 1 && (pos & 32) === 32 && (back & 1) === 1 && ((pos & 6) ^ (back & 6)) === 6, "detached: position " + pos + " " + back);
check(p.nextSibling === null && p.previousSibling === null && p.nextElementSibling === null, "detached: siblings");
check(p.baseURI === document.baseURI && p.firstChild.wholeText === "y", "detached: baseURI, wholeText");
check(document.getElementById("p") === null && document.getElementsByTagName("p").length === 0, "detached: not found");
check(el.childNodes === c && c.length === 2, "refilled");
var f = document.createDocumentFragment();
f.appendChild(p);
check(p.getRootNode() === f && f.getElementById("p") === p && f.children[0] === p && f.firstElementChild === p && f.contains(p), "in a fragment");
check(p.ownerDocument === document && f.ownerDocument === document && p.isConnected === false, "fragment's owner");
el.appendChild(p);
check(el.lastChild === p && c === el.childNodes && c[2] === p && p.mark === 7 && f.childNodes.length === 0, "put back");
"#
            ),
        );
    });
}

/// The wrapper table under the memory limit (`wrapper_limit`, `collection_limit`):
/// reached by wrappers alone, by collections alone and by both; then what a later script can
/// still do. Asserts the ends the story names and that nothing faults; prints what it saw.
#[test]
fn what_a_script_can_do_after_the_memory_limit() {
    let page = format!("<p id=\"p\">{}</p>", "<b></b>".repeat(20_000));
    let ways = [
        (
            "wrappers",
            "var p = document.getElementById('p'); for (var x = p.firstChild; x; x = x.nextSibling) {}",
        ),
        (
            "collections",
            "var keep = []; for (;;) keep.push(document.getElementsByTagName('html'));",
        ),
        (
            "both",
            "var p = document.getElementById('p'), keep = []; for (var x = p.firstChild; x; x = x.nextSibling) keep.push(x.childNodes);",
        ),
    ];
    let later = [
        (
            "an old wrapper",
            "document.getElementById('p').firstChild.nodeName",
        ),
        ("a kept collection", "document.childNodes.length"),
        (
            "a new wrapper",
            "document.getElementById('p').lastChild.nodeName",
        ),
        (
            "a new collection",
            "document.getElementsByTagName('b').length",
        ),
        ("plain", "1 + 1"),
    ];
    let mut lines = Vec::new();
    for (name, source) in ways {
        let page = page.clone();
        let seen = with_page(small_limits(), page, |runner| {
            ran(
                runner,
                "document.childNodes; document.getElementById('p').firstChild;",
            );
            let end = runner.evaluate(source);
            let mut seen = vec![format!("{name}: {end}, heap {}", heap(runner))];
            for (what, later) in later {
                let end = runner.evaluate(later);
                seen.push(format!("  then {what}: {end}, heap {}", heap(runner)));
                assert!(
                    !matches!(end, ScriptEnd::Faulted),
                    "{name} then {what}: faulted"
                );
            }
            (end, seen)
        });
        assert!(
            matches!(
                seen.0,
                ScriptEnd::Limit(Reason::Memory | Reason::StepsPerScript)
            ),
            "{name}: {}",
            seen.0
        );
        lines.extend(seen.1);
    }
    eprintln!("{}", lines.join("\n"));
}

/// A member of [`steps_and_time_of_the_walking_members`]: its name, the page for `n`, the read.
type Member<'a> = (&'a str, &'a dyn Fn(usize) -> String, &'a str);

/// The steps and the time of the walking members at n = 500 and n = 5,000; printed for the
/// report. Each member is read 20 times. A member whose time grows faster than its steps is
/// named in the report.
#[test]
fn steps_and_time_of_the_walking_members() {
    let members: [Member<'_>; 10] = [
        (
            "textContent wide",
            &|n| format!("<p id=\"p\">{}</p>", "<i>a</i>".repeat(n)),
            "p.textContent",
        ),
        (
            "textContent deep",
            &|n| {
                format!(
                    "<p id=\"p\">{}x{}</p>",
                    "<i>".repeat(n.min(500)),
                    "</i>".repeat(n.min(500))
                )
            },
            "p.textContent",
        ),
        (
            "contains",
            &|n| format!("<p id=\"p\">{}</p>", "<i>a</i>".repeat(n)),
            "p.contains(document.documentElement)",
        ),
        (
            "compareDocumentPosition",
            &|n| format!("<p id=\"p\">{}</p>", "<i>a</i>".repeat(n)),
            "p.firstChild.compareDocumentPosition(p.lastChild)",
        ),
        (
            "isEqualNode",
            &|n| format!("<p id=\"p\"><b>{0}</b><b>{0}</b></p>", "<i>a</i>".repeat(n)),
            "p.firstChild.isEqualNode(p.lastChild)",
        ),
        (
            "getRootNode",
            &|n| {
                format!(
                    "<p id=\"p\">{}x{}</p>",
                    "<i>".repeat(n.min(500)),
                    "</i>".repeat(n.min(500))
                )
            },
            "document.getElementsByTagName('i')[0].getRootNode()",
        ),
        (
            "lookupNamespaceURI",
            &|n| {
                format!(
                    "<p id=\"p\">{}x{}</p>",
                    "<i>".repeat(n.min(500)),
                    "</i>".repeat(n.min(500))
                )
            },
            "p.lastChild.lookupNamespaceURI('q')",
        ),
        (
            "getElementsByTagName",
            &|n| format!("<p id=\"p\">{}</p>", "<i>a</i>".repeat(n)),
            "document.getElementsByTagName('i')",
        ),
        (
            "namedItem",
            &|n| format!("<p id=\"p\">{}</p>", "<i>a</i>".repeat(n)),
            "p.children.namedItem('none')",
        ),
        (
            "children of texts",
            &|n| format!("<p id=\"p\">{}<b></b></p>", "a<!---->".repeat(n)),
            "(p.appendChild(p.firstChild), p.children)",
        ),
    ];
    let mut lines = Vec::new();
    for (name, make, read) in members {
        let mut cells = Vec::new();
        for n in [500, 5_000] {
            let page = make(n);
            let source = format!("for (var k = 0; k < 20; k++) {read};");
            let (spent, took) = with_page(limits(), page, |runner| {
                ran(runner, "var p = document.getElementById('p');");
                let started = Instant::now();
                let spent = steps(runner, &source);
                (spent, started.elapsed())
            });
            cells.push(format!("n {n}: {spent} steps {took:?}"));
        }
        lines.push(format!("{name}: {}", cells.join("; ")));
    }
    eprintln!("{}", lines.join("\n"));
}

/// A member of the slice never changes the tree's write counter.
#[test]
fn no_reading_member_writes() {
    let page = "<!DOCTYPE html><title>t</title><div id=\"a\" class=\"c\">x<p id=\"p\" name=\"n\">y</p><!--c--></div>";
    let (before, after) = run_document(
        limits(),
        Mode::Test,
        move || document(parse_document(page, &ParseOptions::default()), info()),
        |runner| {
            let before = runner
                .agent_and_realm()
                .0
                .host()
                .document()
                .map(|dom| dom.tree().writes());
            ran(
                runner,
                r#"var el = document.getElementById("a"), p = document.getElementById("p"), t = el.firstChild;
var reads = [document.title, document.compatMode, document.URL, document.baseURI, document.doctype, document.documentElement,
  el.childNodes, el.children, document.children, el.textContent, t.wholeText, t.substringData(0, 1), t.length,
  p.isEqualNode(p), p.compareDocumentPosition(t), document.contains(t), p.getRootNode(), p.lookupPrefix("x"),
  p.lookupNamespaceURI("x"), p.isDefaultNamespace(null), el.getElementsByClassName("c"), el.getElementsByTagNameNS("*", "*"),
  t.nextSibling, p.previousElementSibling, el.childElementCount, document.getElementById("p"), el.children.namedItem("n")];"#,
            );
            let after = runner
                .agent_and_realm()
                .0
                .host()
                .document()
                .map(|dom| dom.tree().writes());
            (before, after)
        },
    )
    .expect("a runner");
    assert!(before.is_some());
    assert_eq!(before, after);
}

/// The walking members that no test of the unit holds to their steps: one read of each
/// charges at least the nodes it walks (`getRootNode` and `isConnected` over 500 ancestors,
/// `isEqualNode` over 1,000 pairs of nodes, `children` refilled over 2,000 texts, an element
/// sibling passing 2,000 texts) and `baseURI` one step per 64 bytes of a long `href` for its
/// parse and as many for its answer.
#[test]
fn the_walking_members_charge_what_they_walk() {
    let deep = format!("{}x{}", "<i>".repeat(500), "</i>".repeat(500));
    let wide = format!(
        "<p id=\"p\"><b>{0}</b><b>{0}</b></p>",
        "<i></i>".repeat(1_000)
    );
    let texts = format!(
        "<p id=\"p\"><b></b>{}<b></b></p><div id=\"q\"><u></u></div>",
        "a<!---->".repeat(1_000)
    );
    let href = format!("<base href=\"https://example.com/{}\">", "a".repeat(64_000));
    let cases: [(&str, String, &str, &str, u64); 6] = [
        (
            "getRootNode",
            deep.clone(),
            "var x = document.getElementsByTagName('i')[499];",
            "x.getRootNode();",
            500,
        ),
        (
            "isConnected",
            deep,
            "var x = document.getElementsByTagName('i')[499];",
            "x.isConnected;",
            500,
        ),
        (
            "isEqualNode, per pair",
            wide,
            "var p = document.getElementById('p');",
            "p.firstChild.isEqualNode(p.lastChild);",
            1_000,
        ),
        (
            "children refilled",
            texts.clone(),
            "var p = document.getElementById('p'), q = document.getElementById('q'); p.children; p.appendChild(q.firstChild);",
            "p.children;",
            2_000,
        ),
        (
            "nextElementSibling past texts",
            texts,
            "var p = document.getElementById('p');",
            "p.firstChild.nextElementSibling;",
            2_000,
        ),
        ("baseURI", href, "", "document.baseURI;", 1_500),
    ];
    let mut short = Vec::new();
    for (name, page, setup, read, floor) in cases {
        let spent = with_page(limits(), page, |runner| {
            ran(runner, setup);
            let empty = steps(runner, "0;");
            steps(runner, read).saturating_sub(empty)
        });
        eprintln!("{name}: {spent} steps (floor {floor})");
        if spent < floor {
            short.push(format!("{name}: {spent} < {floor}"));
        }
    }
    assert!(short.is_empty(), "{short:?}");
}

/// `baseURI` and the document's facts for the cases of the standard that the unit's tests do not
/// have, against the HTML Standard's document base URL and the URL Standard's parser.
#[test]
fn base_url_cases_of_the_standard() {
    let with = |address: &str| DocumentInfo {
        address: Url::parse(address, None).expect("an address"),
        content_type: "text/html".to_string(),
        encoding: "UTF-8".to_string(),
    };
    let cases: [(&str, &str, &str, &str); 10] = [
        (
            "https://u:pw@example.com/x?q#f",
            "<p></p>",
            "",
            "https://u:pw@example.com/x?q#f",
        ),
        (
            "file:///srv/a/b.html",
            "<p></p>",
            "",
            "file:///srv/a/b.html",
        ),
        (
            "https://example.com/x?q#f",
            "<base href=\"\">",
            "",
            "https://example.com/x?q",
        ),
        (
            "https://example.com/x",
            "<base href=\"  https://s.test/  \">",
            "",
            "https://s.test/",
        ),
        (
            "https://example.com/x",
            "<template><base href=\"https://t.test/\"></template>",
            "",
            "https://example.com/x",
        ),
        (
            "https://example.com/x",
            "<body><p></p></body></html><base href=\"https://late.test/\">",
            "",
            "https://late.test/",
        ),
        (
            "https://example.com/dir/page",
            "<base href=\"#frag\">",
            "",
            "https://example.com/dir/page#frag",
        ),
        (
            "https://example.com/x",
            "<base href=\"//other.test\">",
            "",
            "https://other.test/",
        ),
        (
            "https://example.com/x",
            "<base id=\"b\" href=\"https://gone.test/\">",
            "var b = document.getElementById('b'); b.parentNode.removeChild(b);",
            "https://example.com/x",
        ),
        (
            "https://example.com/x",
            "<math><base href=\"https://m.test/\"></base></math>",
            "",
            "https://example.com/x",
        ),
    ];
    let mut wrong = Vec::new();
    for (address, page, setup, expected) in cases {
        let info = with(address);
        let got = run_document(
            limits(),
            Mode::Test,
            move || document(parse_document(page, &ParseOptions::default()), info),
            |runner| {
                ran(runner, setup);
                match runner.evaluate(
                    "throw new Error(document.baseURI + ' ' + document.URL + ' ' + document.documentURI);",
                ) {
                    ScriptEnd::Threw(error) => error.message.unwrap_or_default(),
                    other => format!("{other}"),
                }
            },
        )
        .expect("a runner");
        let want = format!("{expected} {address} {address}");
        let url = Url::parse(address, None).expect("an address").href();
        let want = want.replace(&format!(" {address} {address}"), &format!(" {url} {url}"));
        if got != want {
            wrong.push(format!("{page} at {address}: {got}, expected {want}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
