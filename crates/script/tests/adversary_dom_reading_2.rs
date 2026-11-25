//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`), second review: the
//! refill of a kept collection after its fixes, the charges, `substringData` in code units,
//! `baseURI`, the sibling hints, the namespace lookups and the rows the first review left. A
//! hostile page chooses the markup and every call; the writes go through the test's slices.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeId, ParseOptions, WriteError, parse_document};
use surf_script::bindings::{Attributes, DomCx, DomEnd, DomException, DomResult, Mutation, Slices};
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

fn info(address: &str) -> DocumentInfo {
    DocumentInfo {
        address: Url::parse(address, None).expect("the address"),
        content_type: "text/html".to_string(),
        encoding: "UTF-8".to_string(),
    }
}

fn refused(error: WriteError) -> DomEnd {
    DomEnd::Throw(DomException::new(
        "HierarchyRequestError",
        format!("{error:?}"),
    ))
}

/// The test's `mutation` slice, through the tree's public write surface: `appendChild` (a
/// fragment gives its children), `insertBefore`, `removeChild`, `cloneNode`,
/// `createDocumentFragment` and the `nodeValue` setter of character data.
struct TestMutation;

impl Mutation for TestMutation {
    fn set_node_node_value(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: Option<String>,
    ) -> DomResult<()> {
        cx.tree_mut()
            .set_data(this, &value.unwrap_or_default())
            .map_err(refused)
    }

    fn node_append_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
    ) -> DomResult<NodeId> {
        cx.tree_mut().append_child(this, node).map_err(refused)?;
        Ok(node)
    }

    fn node_insert_before(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
        child: Option<NodeId>,
    ) -> DomResult<NodeId> {
        cx.tree_mut()
            .insert_before(this, node, child)
            .map_err(refused)?;
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

    fn node_clone_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        subtree: bool,
    ) -> DomResult<NodeId> {
        cx.tree_mut().clone_node(this, subtree).map_err(refused)
    }

    fn document_create_document_fragment(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<NodeId> {
        cx.tree_mut().create_document_fragment().map_err(refused)
    }
}

/// The test's `attributes` slice: `setAttribute` and `removeAttribute` through the tree.
struct TestAttributes;

impl Attributes for TestAttributes {
    fn element_set_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
        value: String,
    ) -> DomResult<()> {
        cx.tree_mut()
            .set_attribute(this, &qualified_name, &value)
            .map_err(refused)
    }

    fn element_remove_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<()> {
        cx.tree_mut()
            .remove_attribute(this, &qualified_name)
            .map_err(refused)
    }
}

fn slices(info: DocumentInfo) -> Slices {
    let mut slices = dom::slices(info);
    slices.mutation = Box::new(TestMutation);
    slices.attributes = Box::new(TestAttributes);
    slices
}

fn with_page<R: Send>(
    limits: RunnerLimits,
    address: &str,
    page: String,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    let info = info(address);
    run_document(
        limits,
        Mode::Test,
        move || Document {
            tree: parse_document(&page, &ParseOptions::default()),
            limits: ParserLimits::default(),
            slices: slices(info),
        },
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

/// The message of the error a script threw, or the end's text.
fn outcome(runner: &mut Runner, source: &str) -> String {
    match runner.evaluate(source) {
        ScriptEnd::Threw(error) => error.message.unwrap_or_default(),
        other => format!("{other}"),
    }
}

const CHECK: &str = r#"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
function threw(f) { try { f(); } catch (e) { return e.name; } return "no throw"; }
var fails = [];
function expect(got, want, what) { if (got !== want) fails.push(what + ": " + String(got) + " against " + String(want)); }
function done(what) { if (fails.length) throw new Error(fails.length + " wrong: " + fails.slice(0, 12).join(" / ")); throw new Error("held " + what); }
"#;

/// A refill that the memory limit ends in its middle: the getter of a kept `childNodes` makes a
/// wrapper for each new node while it defines the indices, and the run ends there. The script
/// ends, the runner collects, and a later script of the same document reads the same kept
/// object without its getter. The rule: the object is the old snapshot whole, or the new one
/// whole; no index at or past `length` answers a node the glue put there. `refresh_collection`
/// propagates a `Terminated` from `wrap` and `index_key` in its step 2 with `?`, after it
/// defined the first indices and before it wrote the data.
#[test]
fn a_refill_ended_by_the_memory_limit_leaves_one_list() {
    let started = Instant::now();
    let mut small = limits();
    small.engine.memory_bytes = 1024 * 1024;
    let page = format!(
        "<div id=\"p\"><b></b></div><div id=\"q\">{}</div>",
        "<i></i>".repeat(50)
    );
    let (refill, seen, wrong) = with_page(small, ADDRESS, page, |runner| {
        ran(
            runner,
            "var p = document.getElementById('p'), q = document.getElementById('q');\
             var c = p.childNodes, b = c[0];\
             var f = document.createDocumentFragment();\
             while (q.firstChild) f.appendChild(q.firstChild);\
             for (var k = 0; k < 120; k++) p.insertBefore(f.cloneNode(true), b);",
        );
        let refill = runner.evaluate("p.childNodes;");
        let checks = [
            ("length is the old 1", "if (c.length !== 1) throw 1;"),
            ("item(0) is the old child", "if (c.item(0) !== b) throw 1;"),
            ("c[0] is the old child", "if (c[0] !== b) throw 1;"),
            ("c[1] is undefined", "if (c[1] !== undefined) throw 1;"),
            ("c[100] is undefined", "if (c[100] !== undefined) throw 1;"),
        ];
        let mut seen = vec![format!("refill: {refill}")];
        let mut wrong = Vec::new();
        for (what, source) in checks {
            let end = runner.evaluate(source);
            seen.push(format!("{what}: {end}"));
            if end != ScriptEnd::Ran {
                wrong.push(what);
            }
        }
        (refill, seen, wrong)
    });
    eprintln!(
        "memory end in a refill ({:?}):\n{}",
        started.elapsed(),
        seen.join("\n")
    );
    assert_eq!(refill, ScriptEnd::Limit(Reason::Memory), "{seen:#?}");
    assert!(
        wrong.is_empty(),
        "a kept collection shows two lists after a refill the memory limit ended: {seen:#?}"
    );
}

/// A refill that the step limit ends: the getter charges the old and the new nodes before any
/// define, so the kept object is untouched and the next script reads the old snapshot.
#[test]
fn a_refill_past_the_step_limit_changes_nothing() {
    let mut few = limits();
    few.engine.steps_per_entry = 5_000;
    let page = format!(
        "<div id=\"p\"><b></b></div><div id=\"q\">{}</div>",
        "<i></i>".repeat(50)
    );
    let (refill, seen, wrong) = with_page(few, ADDRESS, page, |runner| {
        ran(
            runner,
            "var p = document.getElementById('p'), q = document.getElementById('q');\
             var c = p.childNodes, b = c[0];\
             var f = document.createDocumentFragment();\
             while (q.firstChild) f.appendChild(q.firstChild);",
        );
        for _ in 0..4 {
            ran(
                runner,
                "for (var k = 0; k < 30; k++) p.insertBefore(f.cloneNode(true), b);",
            );
        }
        let refill = runner.evaluate("p.childNodes;");
        let mut seen = vec![format!("refill: {refill}")];
        let mut wrong = Vec::new();
        for (what, source) in [
            ("length", "if (c.length !== 1) throw 1;"),
            ("item(0)", "if (c.item(0) !== b) throw 1;"),
            ("c[0]", "if (c[0] !== b) throw 1;"),
            ("c[1]", "if (c[1] !== undefined) throw 1;"),
        ] {
            let end = runner.evaluate(source);
            seen.push(format!("{what}: {end}"));
            if end != ScriptEnd::Ran {
                wrong.push(what);
            }
        }
        (refill, seen, wrong)
    });
    eprintln!("step end in a refill:\n{}", seen.join("\n"));
    assert_eq!(
        refill,
        ScriptEnd::Limit(Reason::StepsPerScript),
        "{seen:#?}"
    );
    assert!(wrong.is_empty(), "{seen:#?}");
}

/// Every shape a script can give a kept `childNodes` or `children` object, at the first, a
/// middle and the last index and one past the old end, for a list that grows, shrinks, keeps
/// its length with other nodes, becomes empty and was empty: after the getter, the object is
/// the new list (it answered) or the old one (it threw a `TypeError`, and throws again), index
/// by index, `length` and `item` read through the prototype's own functions; no own index at or
/// past the length holds a node the script did not put there; no accessor of the script runs
/// in a refill. Then a kept object made non-extensible after a shrink and grown again.
#[test]
fn refill_shapes_index_by_index() {
    let started = Instant::now();
    let page = format!(
        "<div id=\"pool\">{}</div><p id=\"stranger\"></p>{}",
        "<i></i>".repeat(30),
        "<section></section>".repeat(520)
    );
    let source = format!(
        r#"{CHECK}
var pool = document.getElementById("pool"), stranger = document.getElementById("stranger");
var hosts = document.getElementsByTagName("section");
var NL = NodeList.prototype, HC = HTMLCollection.prototype;
var lenOf = {{ childNodes: Object.getOwnPropertyDescriptor(NL, "length").get, children: Object.getOwnPropertyDescriptor(HC, "length").get }};
var itemOf = {{ childNodes: NL.item, children: HC.item }};
var protoOf = {{ childNodes: NL, children: HC }};
var defineProperty = Object.defineProperty, gopd = Object.getOwnPropertyDescriptor, names = Object.getOwnPropertyNames;
var NodeI = Node, TypeErrorI = TypeError;
var calls = 0, used = 0, refilled = 0, refusedCount = 0;
function getter() {{ calls++; return "g"; }}
function setter(v) {{ calls++; }}
function fresh(n) {{ var out = []; for (var i = 0; i < n; i++) {{ var x = pool.firstChild; pool.removeChild(x); out.push(x); }} return out; }}
function giveBack(h) {{ while (h.firstChild) pool.appendChild(h.firstChild); }}
function isIndex(k) {{ var n = Number(k); return String(n) === k && n >= 0 && n % 1 === 0 && n < 4294967295; }}
function view(member, c, list, touched, refusedRefill, what) {{
  var len = lenOf[member].call(c);
  if (len !== list.length) {{ fails.push(what + ": length " + len + " against " + list.length); return; }}
  for (var i = 0; i < list.length; i++) {{
    if (itemOf[member].call(c, i) !== list[i]) fails.push(what + ": item(" + i + ")");
    if (refusedRefill && touched.indexOf(i) >= 0) continue;
    var d = gopd(c, String(i));
    if (!d || d.value !== list[i]) fails.push(what + ": index " + i + " is " + (d ? ("value" in d ? String(d.value) : "an accessor") : "absent"));
  }}
  var keys = names(c);
  for (var j = 0; j < keys.length; j++) {{
    if (!isIndex(keys[j]) || Number(keys[j]) < list.length) continue;
    var e = gopd(c, keys[j]);
    if ("value" in e && e.value instanceof NodeI && e.value !== stranger) fails.push(what + ": a node at " + keys[j] + " past the length " + list.length);
  }}
}}
var transitions = [
  ["grow", 5, 9, function (old) {{ return old.concat(fresh(4)); }}],
  ["shrink", 9, 3, function (old) {{ return old.slice(0, 3); }}],
  ["same length", 5, 5, function (old) {{ return fresh(5); }}],
  ["to empty", 5, 0, function (old) {{ return []; }}],
  ["from empty", 0, 4, function (old) {{ return fresh(4); }}],
];
var indexShapes = [
  ["non-configurable", function (c, k) {{ defineProperty(c, k, {{ value: "x", writable: false, enumerable: true, configurable: false }}); }}],
  ["non-writable configurable", function (c, k) {{ defineProperty(c, k, {{ value: "x", writable: false, configurable: true }}); }}],
  ["writable configurable", function (c, k) {{ defineProperty(c, k, {{ value: "x", writable: true, configurable: true }}); }}],
  ["accessor without setter", function (c, k) {{ defineProperty(c, k, {{ get: getter, configurable: true }}); }}],
  ["accessor with setter", function (c, k) {{ defineProperty(c, k, {{ get: getter, set: setter, configurable: true }}); }}],
  ["non-configurable accessor", function (c, k) {{ defineProperty(c, k, {{ get: getter, set: setter, configurable: false }}); }}],
  ["deleted", function (c, k) {{ delete c[k]; }}],
  ["a node, configurable", function (c, k) {{ defineProperty(c, k, {{ value: stranger, writable: true, configurable: true }}); }}],
  ["a node, non-configurable", function (c, k) {{ defineProperty(c, k, {{ value: stranger, writable: false, configurable: false }}); }}],
];
function protoWithIndices(member) {{
  var pr = Object.create(protoOf[member]);
  for (var i = 0; i < 12; i++) defineProperty(pr, String(i), {{ get: getter, set: setter, configurable: true }});
  return pr;
}}
var objectShapes = [
  ["freeze", function (c) {{ Object.freeze(c); }}, []],
  ["seal", function (c) {{ Object.seal(c); }}, []],
  ["prevent extensions", function (c) {{ Object.preventExtensions(c); }}, []],
  ["own length", function (c) {{ defineProperty(c, "length", {{ value: 99 }}); }}, []],
  ["odd keys", function (c) {{ ["4294967295", "4294967296", "01", "1.0", "-0", "1e3", "+1"].forEach(function (k) {{ defineProperty(c, k, {{ value: stranger, configurable: false }}); }}); }}, []],
  ["prototype with index accessors", function (c, member) {{ Object.setPrototypeOf(c, protoWithIndices(member)); }}, []],
  ["prototype with index accessors, own deleted, non-extensible", function (c, member) {{
     Object.setPrototypeOf(c, protoWithIndices(member));
     for (var i = 0; i < 12; i++) delete c[i];
     Object.preventExtensions(c); }}, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]],
];
var outcomes = {{}};
function runCase(member, t, shapeName, shape, k, touched) {{
  var h = hosts[used++];
  var what = member + " | " + t[0] + " | " + shapeName + (k === null ? "" : " at " + k);
  var old = fresh(t[1]);
  for (var i = 0; i < old.length; i++) h.appendChild(old[i]);
  var c = h[member];
  shape(c, k === null ? member : String(k));
  var nodes = t[3](old);
  giveBack(h);
  for (var n = 0; n < nodes.length; n++) h.appendChild(nodes[n]);
  var before = calls, error = null, got;
  try {{ got = h[member]; }} catch (e) {{ error = e; }}
  if (calls !== before) fails.push(what + ": script code ran in the refill");
  if (error) {{
    if (!(error instanceof TypeErrorI)) fails.push(what + ": threw " + error);
    view(member, c, old, touched, true, what + " (refused)");
    var again = null;
    try {{ h[member]; }} catch (e2) {{ again = e2; }}
    if (!again) fails.push(what + ": a second read did not throw");
    refusedCount++;
  }} else {{
    if (got !== c) fails.push(what + ": another object");
    view(member, c, nodes, touched, false, what);
    refilled++;
  }}
  outcomes[shapeName + " | " + t[0]] = (outcomes[shapeName + " | " + t[0]] || "") + (error ? "T" : "r");
  giveBack(h);
}}
["childNodes", "children"].forEach(function (member) {{
  transitions.forEach(function (t) {{
    var most = Math.max(t[1], t[2]);
    var at = [0, (most >> 1), most - 1, t[1]].filter(function (k, i, all) {{ return k >= 0 && all.indexOf(k) === i; }});
    indexShapes.forEach(function (s) {{
      at.forEach(function (k) {{ runCase(member, t, s[0], s[1], k, [k]); }});
    }});
    objectShapes.forEach(function (s) {{ runCase(member, t, s[0], s[1], null, s[2]); }});
  }});
}});
// Non-extensible after a refill that shrank the list, then grown again, then back.
(function () {{
  var h = hosts[used++];
  var old = fresh(6);
  old.forEach(function (x) {{ h.appendChild(x); }});
  var c = h.childNodes;
  pool.appendChild(old[5]); pool.appendChild(old[4]);
  check(h.childNodes === c, "the kept object");
  view("childNodes", c, old.slice(0, 4), [], false, "shrunk");
  Object.preventExtensions(c);
  pool.appendChild(old[3]);
  var shrunk = threw(function () {{ h.childNodes; }});
  view("childNodes", c, old.slice(0, 3), [], false, "shrunk while non-extensible (" + shrunk + ")");
  var extra = fresh(1)[0];
  h.appendChild(extra);
  var grown = threw(function () {{ h.childNodes; }});
  expect(grown, "TypeError", "grown while non-extensible");
  view("childNodes", c, old.slice(0, 3), [], true, "grown while non-extensible");
  pool.appendChild(extra);
  var back = threw(function () {{ h.childNodes; }});
  expect(back, "no throw", "back to the old three");
  view("childNodes", c, old.slice(0, 3), [], false, "back to the old three");
  giveBack(h);
}})();
var summary = [];
for (var key in outcomes) summary.push(key + " " + outcomes[key]);
G.summary = summary.join("\n");
done(refilled + " refilled, " + refusedCount + " refused, " + used + " objects");
"#
    );
    let (end, summary) = with_page(limits(), ADDRESS, page, |runner| {
        let end = outcome(runner, &source);
        let summary = outcome(runner, "throw new Error(G.summary);");
        (end, summary)
    });
    eprintln!(
        "refill shapes ({:?}): {end}\n(T refused, r refilled, one letter per index tried)\n{summary}",
        started.elapsed()
    );
    assert!(end.starts_with("held "), "{end}");
}

/// The charges of the walks that read attributes, against acceptance 17: per element one step
/// and one per 64 bytes of its `class` attribute for `getElementsByClassName`. A page of 10,000
/// elements with 10 attributes of 100 bytes each and `class="c"`.
#[test]
fn attribute_walks_charge_what_they_read() {
    let started = Instant::now();
    let value = "v".repeat(100);
    let mut attrs = String::new();
    for index in 0..10 {
        attrs.push_str(&format!(" data-a{index}=\"{value}\""));
    }
    let element = format!("<p class=\"c\"{attrs}></p>");
    let mut lines = Vec::new();
    let mut class_steps = 0;
    for count in [1_000_usize, 10_000] {
        let page = format!(
            "<div>{}</div><div>{}</div>",
            element.repeat(count / 2),
            element.repeat(count / 2)
        );
        let (seen, class) = with_page(limits(), ADDRESS, page, |runner| {
            ran(
                runner,
                "var G = this; G.all = document.getElementsByTagName('p');\
                 G.t1 = document.getElementsByTagName('div')[0]; G.t2 = t1.nextElementSibling;\
                 document.getElementsByTagName('*');",
            );
            let mut seen = Vec::new();
            let mut class = 0;
            for (name, source) in [
                (
                    "getElementsByClassName('c')",
                    "document.getElementsByClassName('c');",
                ),
                (
                    "getElementsByTagName('*')",
                    "document.getElementsByTagName('*');",
                ),
                ("getElementById('none')", "document.getElementById('none');"),
                ("namedItem('none')", "all.namedItem('none');"),
                (
                    "isEqualNode of two equal trees",
                    "if (!t1.isEqualNode(t2)) throw 1;",
                ),
                ("textContent", "document.documentElement.textContent;"),
            ] {
                let at = Instant::now();
                let spent = steps(runner, source);
                seen.push(format!(
                    "{count} elements, {name}: {spent} steps, {:?}",
                    at.elapsed()
                ));
                if name.starts_with("getElementsByClassName") {
                    class = spent;
                }
            }
            (seen, class)
        });
        lines.extend(seen);
        class_steps = class;
    }
    lines.push(format!("all: {:?}", started.elapsed()));
    eprintln!("{}", lines.join("\n"));
    // Acceptance 17: one step per element and one per 64 bytes of its one-byte `class`, the
    // walk's step per other node, the collection's step per node: about 30,000. The bound
    // leaves twice that.
    assert!(
        class_steps < 60_000,
        "getElementsByClassName charges {class_steps} steps for 10,000 elements whose class is \
         one byte: it charges every attribute's bytes, not its class attribute's\n{}",
        lines.join("\n")
    );
}

/// `substringData` in UTF-16 code units: offsets and counts at 0, at the length, one past it,
/// a count past what is left, ToUint32 of the arguments, a text of pairs only, U+0000, an
/// offset inside the last pair; each answer's `length` and code units.
#[test]
fn substring_data_in_code_units() {
    let page = "<p id=\"t\">a\u{1F600}b\u{1F600}</p><p id=\"u\">\u{1F600}\u{1F600}\u{1F600}</p><p id=\"z\">xyz</p>".to_string();
    let source = format!(
        r#"{CHECK}
function units(s) {{ var out = []; for (var i = 0; i < s.length; i++) out.push(s.charCodeAt(i).toString(16)); return out.join(","); }}
var t = document.getElementById("t").firstChild, u = document.getElementById("u").firstChild, z = document.getElementById("z").firstChild;
expect(t.length, 6, "length");
expect(units(t.substringData(0, 0)), "", "0,0");
expect(units(t.substringData(6, 0)), "", "at the length");
expect(units(t.substringData(6, 5)), "", "at the length, count 5");
expect(threw(function () {{ t.substringData(7, 0); }}), "IndexSizeError", "one past the length");
expect(units(t.substringData(1, 1)), "d83d", "a high half");
expect(units(t.substringData(2, 1)), "de00", "a low half");
expect(units(t.substringData(5, 100)), "de00", "inside the last pair, count past the end");
expect(units(t.substringData(4, 4294967295)), "d83d,de00", "the largest count");
expect(threw(function () {{ t.substringData(-1, 1); }}), "IndexSizeError", "offset -1 is 4294967295");
expect(units(t.substringData(0, -1)), "61,d83d,de00,62,d83d,de00", "count -1 is 4294967295");
expect(units(t.substringData(1.9, 1)), "d83d", "offset 1.9");
expect(units(t.substringData(4294967297, 1)), "d83d", "offset 2^32 + 1");
expect(units(t.substringData(NaN, 1)), "61", "offset NaN");
expect(threw(function () {{ t.substringData(Infinity, 1); }}), "no throw", "offset Infinity is 0");
expect(units(u.substringData(1, 4)), "de00,d83d,de00,d83d", "pairs only, shifted by one");
expect(u.substringData(1, 4).length, 4, "pairs only, length");
z.nodeValue = "x\u0000y";
expect(units(z.substringData(1, 1)), "0", "U+0000");
expect(units(z.substringData(0, 3)), "78,0,79", "U+0000 inside");
expect(threw(function () {{ t.substringData(0); }}), "TypeError", "one argument");
done("substringData");
"#
    );
    let end = with_page(limits(), ADDRESS, page, |runner| outcome(runner, &source));
    assert!(end.starts_with("held "), "{end}");
}

/// The steps of `substringData` near the end of a text of 1,000,000 units, and the String
/// length limit on its answer with an agent whose `string_length` is 1,000.
#[test]
fn substring_data_steps_and_string_length() {
    let page = format!("<p id=\"t\">{}</p>", "a".repeat(1_000_000));
    let near_end = with_page(limits(), ADDRESS, page, |runner| {
        ran(runner, "var t = document.getElementById('t').firstChild;");
        let start = steps(runner, "t.substringData(0, 5);");
        let end = steps(runner, "t.substringData(999990, 5);");
        let long = steps(runner, "t.substringData(0, 900000);");
        (start, end, long)
    });
    eprintln!(
        "substringData of 5 units at offset 0: {} steps; at offset 999,990: {} steps; \
         900,000 units from offset 0: {} steps",
        near_end.0, near_end.1, near_end.2
    );
    assert!(near_end.1 >= 999_990 / 64, "{near_end:?}");
    assert!(
        near_end.2 >= 900_000 / 64,
        "the answer's units are charged: {near_end:?}"
    );
    let mut short = limits();
    short.engine.string_length = 1_000;
    let page = format!("<p id=\"t\">{}</p>", "b".repeat(2_000));
    let ends = with_page(short, ADDRESS, page, |runner| {
        ran(runner, "var t = document.getElementById('t').firstChild;");
        [
            outcome(
                runner,
                "throw new Error(String(t.substringData(0, 1000).length));",
            ),
            outcome(
                runner,
                "throw new Error(String(t.substringData(1000, 5000).length));",
            ),
            outcome(
                runner,
                "try { t.substringData(0, 1001); } catch (e) { throw new Error(e.name); } throw new Error('no throw');",
            ),
        ]
    });
    eprintln!("string_length 1,000: {ends:?}");
    assert_eq!(ends, ["1000", "1000", "RangeError"], "{ends:?}");
}

/// `baseURI` against HTML's "set the frozen base URL": schemes in other cases, spaces, tabs and
/// newlines the URL parser strips, `blob:`, `about:`, `vbscript:`, the first `base` with an
/// empty `href`, a `base` in the HTML namespace inside SVG's `foreignObject`, a relative `href`
/// against a `data:` address.
#[test]
fn base_uri_forms_of_the_scheme() {
    let cases: [(&str, &str, &str); 16] = [
        (ADDRESS, "<base href=\"DATA:text/html,x\">", ADDRESS),
        (ADDRESS, "<base href=\"JavaScript:alert(1)\">", ADDRESS),
        (ADDRESS, "<base href=\"JAVASCRIPT:x\">", ADDRESS),
        (ADDRESS, "<base href=\"  \t\n data:,x\">", ADDRESS),
        (ADDRESS, "<base href=\"da\nta:,x\">", ADDRESS),
        (ADDRESS, "<base href=\"java\tscript:x\">", ADDRESS),
        (ADDRESS, "<base href=\"\u{1}javascript:x\">", ADDRESS),
        (
            ADDRESS,
            "<base href=\"blob:https://example.com/0b2\">",
            "blob:https://example.com/0b2",
        ),
        (ADDRESS, "<base href=\"about:blank\">", "about:blank"),
        (ADDRESS, "<base href=\"vbscript:x\">", "vbscript:x"),
        (
            ADDRESS,
            "<base href=\"\"><base href=\"https://two.example/\">",
            ADDRESS,
        ),
        (
            ADDRESS,
            "<base href><base href=\"https://two.example/\">",
            ADDRESS,
        ),
        (
            ADDRESS,
            "<body><svg><foreignObject><base href=\"https://fo.example/x/\"></foreignObject></svg><base href=\"https://later.example/\">",
            "https://fo.example/x/",
        ),
        (
            ADDRESS,
            "<body><svg><base href=\"https://svg.example/\"></svg><base href=\"https://html.example/\">",
            "https://html.example/",
        ),
        (
            "data:text/html,abc",
            "<base href=\"foo\">",
            "data:text/html,abc",
        ),
        (
            "data:text/html,abc",
            "<base href=\"https://x.example/a\">",
            "https://x.example/a",
        ),
    ];
    let mut wrong = Vec::new();
    for (address, page, want) in cases {
        let got = with_page(limits(), address, page.to_string(), |runner| {
            outcome(runner, "throw new Error(document.baseURI);")
        });
        if got != want {
            wrong.push(format!("{address} {page:?}: {got} against {want}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A `base` whose `href` the test slice removes after the first read: the next read takes the
/// first `base` that still has an `href`; one whose `href` is set again decides again.
#[test]
fn base_uri_after_its_href_is_removed() {
    let page =
        "<base id=\"one\" href=\"https://one.example/a/\"><base href=\"https://two.example/b/\">"
            .to_string();
    let source = format!(
        r#"{CHECK}
var one = document.getElementById("one");
expect(document.baseURI, "https://one.example/a/", "first read");
one.removeAttribute("href");
expect(document.baseURI, "https://two.example/b/", "after the removal");
one.setAttribute("href", "javascript:x");
expect(document.baseURI, "{ADDRESS}", "a javascript href set again");
one.setAttribute("href", "c/");
expect(document.baseURI, "https://example.com/dir/c/", "a relative href set again");
done("baseURI");
"#
    );
    let end = with_page(limits(), ADDRESS, page, |runner| outcome(runner, &source));
    assert!(end.starts_with("held "), "{end}");
}

/// The sibling members after writes through the test slice: a child removed between the two
/// positions of a hint, two positions swapped by a move, the parent emptied and refilled with
/// other nodes, a node moved to another parent that has an entry; twelve parents read in
/// turn from both ends. After each write every sibling member of every child answers what the
/// parent's child list answers.
#[test]
fn siblings_after_writes_answer_the_list() {
    let mut page = String::from("<div id=\"spare\">");
    for index in 0..40 {
        page.push_str(&format!("<i>{index}</i>t{index}"));
    }
    page.push_str("</div>");
    for _ in 0..12 {
        page.push_str("<ul>");
        for index in 0..20 {
            page.push_str(&format!("<li>{index}</li>x"));
        }
        page.push_str("</ul>");
    }
    let source = format!(
        r#"{CHECK}
var uls = document.getElementsByTagName("ul"), spare = document.getElementById("spare");
function list(p) {{ return Array.prototype.slice.call(p.childNodes); }}
function audit(p, what) {{
  var l = list(p), els = l.filter(function (x) {{ return x.nodeType === 1; }});
  for (var i = 0; i < l.length; i++) {{
    expect(l[i].nextSibling, l[i + 1] || null, what + ": next of " + i);
    expect(l[i].previousSibling, l[i - 1] || null, what + ": previous of " + i);
    var e = els.indexOf(l[i]);
    var nextEl = null, prevEl = null;
    for (var a = i + 1; a < l.length; a++) if (l[a].nodeType === 1) {{ nextEl = l[a]; break; }}
    for (var b = i - 1; b >= 0; b--) if (l[b].nodeType === 1) {{ prevEl = l[b]; break; }}
    expect(l[i].nextElementSibling, nextEl, what + ": next element of " + i);
    expect(l[i].previousElementSibling, prevEl, what + ": previous element of " + i);
  }}
}}
function warm(p) {{ var a = p.firstChild, z = p.lastChild; for (var k = 0; k < 6 && a && z; k++) {{ a = a.nextSibling; z = z.previousSibling; }} return [a, z]; }}
for (var round = 0; round < 3; round++) {{
  for (var n = 0; n < uls.length; n++) {{
    var p = uls[n];
    var ends = warm(p);
    var kind = (n + round) % 4;
    if (kind === 0) {{ var l = list(p); p.removeChild(l[l.length >> 1]); }}
    else if (kind === 1) {{ p.insertBefore(ends[1], ends[0]); }}
    else if (kind === 2) {{ while (p.firstChild) spare.appendChild(p.firstChild); for (var m = 0; m < 9; m++) p.appendChild(spare.firstChild); }}
    else {{ var other = uls[(n + 1) % uls.length]; warm(other); other.insertBefore(ends[0], other.lastChild); audit(other, "the other parent " + n); }}
    audit(p, "round " + round + " parent " + n + " write " + kind);
  }}
}}
done("siblings");
"#
    );
    let end = with_page(limits(), ADDRESS, page, |runner| outcome(runner, &source));
    assert!(end.starts_with("held "), "{end}");
}

/// Three readers on one parent (from the first child, from the last, from the middle), in
/// turn: the steps at 500 and at 5,000 children. A hint holds two positions, so the third
/// reader misses; printed for the report, the end asserted.
#[test]
fn three_readers_on_one_parent() {
    let mut seen = Vec::new();
    for count in [500_usize, 5_000] {
        let page = format!("<p id=\"p\">{}</p>", "<b></b>".repeat(count));
        let (end, spent, time) = with_page(limits(), ADDRESS, page, |runner| {
            ran(runner, "var p = document.getElementById('p');");
            let before = runner.steps();
            let at = Instant::now();
            let end = runner.evaluate(
                "var a = p.firstChild, z = p.lastChild, m = p.childNodes[p.childNodes.length >> 1];\
                 while (a && z && m) { a = a.nextSibling; z = z.previousSibling; m = m.nextSibling; }",
            );
            (end, runner.steps() - before, at.elapsed())
        });
        seen.push(format!("{count} children: {end}, {spent} steps, {time:?}"));
    }
    eprintln!("three readers:\n{}", seen.join("\n"));
    assert!(
        seen.iter().all(|line| !line.contains("faulted")),
        "{seen:#?}"
    );
}

/// The namespace lookups against the DOM's "locate a namespace" and "locate a namespace
/// prefix" for every node kind, with the prefixes null, "", "xml", "xmlns", a declared and an
/// undeclared one.
#[test]
fn namespace_lookups_of_every_node_kind() {
    let page = "<!--before--><!DOCTYPE html><html><body>\
                <div id=\"d\" xmlns=\"urn:x\" xmlns:p=\"urn:p\"><span id=\"s\">t</span></div>\
                <svg id=\"g\" xmlns:xlink=\"http://www.w3.org/1999/xlink\"><foreignObject id=\"fo\"><div id=\"fd\">u</div></foreignObject><g id=\"gg\"></g></svg>\
                <svg id=\"e\" xmlns=\"\"><g id=\"eg\"></g></svg>\
                <math id=\"m\" xmlns:xlink=\"http://www.w3.org/1999/xlink\"><mi id=\"mi\">v</mi></math>\
                <b id=\"plain\"></b></body></html><!--after-->"
        .to_string();
    const XHTML: &str = "http://www.w3.org/1999/xhtml";
    const SVG: &str = "http://www.w3.org/2000/svg";
    const MATHML: &str = "http://www.w3.org/1998/Math/MathML";
    const XML: &str = "http://www.w3.org/XML/1998/namespace";
    const XMLNS: &str = "http://www.w3.org/2000/xmlns/";
    const XLINK: &str = "http://www.w3.org/1999/xlink";
    let q = |text: &str| format!("\"{text}\"");
    let null = "null".to_string();
    let first: [(&str, String); 9] = [
        ("d.lookupNamespaceURI(null)", q(XHTML)),
        ("d.lookupNamespaceURI('')", q(XHTML)),
        ("d.lookupNamespaceURI(undefined)", q(XHTML)),
        ("d.lookupNamespaceURI('xml')", q(XML)),
        ("d.lookupNamespaceURI('xmlns')", q(XMLNS)),
        ("d.lookupNamespaceURI('p')", null.clone()),
        ("d.lookupNamespaceURI('xmlns:p')", null.clone()),
        ("d.lookupNamespaceURI('q')", null.clone()),
        ("d.lookupPrefix('urn:p')", null.clone()),
    ];
    let mut cases: Vec<(String, String)> = first
        .into_iter()
        .map(|(expression, want)| (expression.to_string(), want))
        .collect();
    let more: [(&str, String); 48] = [
        (
            "d.lookupPrefix('http://www.w3.org/1999/xhtml')",
            null.clone(),
        ),
        ("d.lookupPrefix(null)", null.clone()),
        ("d.lookupPrefix('')", null.clone()),
        (
            "d.isDefaultNamespace('http://www.w3.org/1999/xhtml')",
            "true".into(),
        ),
        ("d.isDefaultNamespace(null)", "false".into()),
        ("d.isDefaultNamespace('')", "false".into()),
        ("d.isDefaultNamespace('urn:x')", "false".into()),
        ("s.firstChild.lookupNamespaceURI(null)", q(XHTML)),
        ("s.firstChild.lookupNamespaceURI('xml')", q(XML)),
        ("s.firstChild.lookupNamespaceURI('xmlns')", q(XMLNS)),
        ("g.lookupNamespaceURI(null)", q(SVG)),
        ("g.lookupNamespaceURI('xlink')", q(XLINK)),
        ("g.lookupPrefix('http://www.w3.org/1999/xlink')", q("xlink")),
        ("g.lookupPrefix('http://www.w3.org/2000/svg')", null.clone()),
        ("gg.lookupNamespaceURI('xlink')", q(XLINK)),
        ("fd.lookupNamespaceURI(null)", q(XHTML)),
        ("fd.lookupNamespaceURI('xlink')", q(XLINK)),
        (
            "fd.lookupPrefix('http://www.w3.org/1999/xlink')",
            q("xlink"),
        ),
        (
            "fd.firstChild.lookupPrefix('http://www.w3.org/1999/xlink')",
            q("xlink"),
        ),
        ("e.lookupNamespaceURI(null)", q(SVG)),
        ("eg.lookupNamespaceURI(null)", q(SVG)),
        ("e.isDefaultNamespace('')", "false".into()),
        ("m.lookupNamespaceURI(null)", q(MATHML)),
        ("m.lookupNamespaceURI('xlink')", q(XLINK)),
        (
            "mi.lookupPrefix('http://www.w3.org/1999/xlink')",
            q("xlink"),
        ),
        ("after.lookupNamespaceURI('xml')", null.clone()),
        ("after.lookupNamespaceURI(null)", null.clone()),
        (
            "after.lookupPrefix('http://www.w3.org/1999/xhtml')",
            null.clone(),
        ),
        ("after.isDefaultNamespace(null)", "true".into()),
        ("after.isDefaultNamespace('')", "true".into()),
        ("before.lookupNamespaceURI('xmlns')", null.clone()),
        ("document.doctype.lookupNamespaceURI('xml')", null.clone()),
        ("document.doctype.lookupNamespaceURI(null)", null.clone()),
        (
            "document.doctype.lookupPrefix('http://www.w3.org/1999/xhtml')",
            null.clone(),
        ),
        ("document.doctype.isDefaultNamespace(null)", "true".into()),
        ("document.lookupNamespaceURI(null)", q(XHTML)),
        ("document.lookupNamespaceURI('xml')", q(XML)),
        (
            "document.lookupPrefix('http://www.w3.org/1999/xhtml')",
            null.clone(),
        ),
        (
            "document.isDefaultNamespace('http://www.w3.org/1999/xhtml')",
            "true".into(),
        ),
        ("detached.lookupNamespaceURI(null)", q(XHTML)),
        ("detached.lookupNamespaceURI('xml')", q(XML)),
        ("detached.lookupNamespaceURI('p')", null.clone()),
        ("fragment.lookupNamespaceURI('xml')", null.clone()),
        ("fragment.lookupNamespaceURI(null)", null.clone()),
        ("fragment.isDefaultNamespace(null)", "true".into()),
        ("inFragment.lookupNamespaceURI(null)", q(XHTML)),
        ("inFragment.lookupNamespaceURI('xmlns')", q(XMLNS)),
        (
            "inFragment.lookupPrefix('http://www.w3.org/1999/xhtml')",
            null.clone(),
        ),
    ];
    cases.extend(more.into_iter().map(|(e, w)| (e.to_string(), w)));
    let mut body = String::new();
    for (expression, want) in &cases {
        body.push_str(&format!(
            "try {{ expect({expression}, {want}, {expression:?}); }} catch (e) {{ fails.push({expression:?} + ' threw ' + e); }}\n"
        ));
    }
    let source = format!(
        r#"{CHECK}
var d = document.getElementById("d"), s = document.getElementById("s"), g = document.getElementById("g"),
    gg = document.getElementById("gg"), fd = document.getElementById("fd"), e = document.getElementById("e"),
    eg = document.getElementById("eg"), m = document.getElementById("m"), mi = document.getElementById("mi");
var after = document.lastChild, before = document.firstChild;
check(after.nodeType === 8 && before.nodeType === 8, "comments under the document: " + after.nodeType + " " + before.nodeType);
var detached = d.cloneNode(false);
var fragment = document.createDocumentFragment();
var inFragment = document.getElementById("plain").cloneNode(false);
fragment.appendChild(inFragment);
{body}
var html = document.documentElement;
document.removeChild(html);
expect(document.lookupNamespaceURI(null), null, "no document element: null");
expect(document.lookupNamespaceURI("xml"), null, "no document element: xml");
expect(document.isDefaultNamespace(""), true, "no document element: isDefaultNamespace('')");
expect(document.lookupPrefix("http://www.w3.org/1999/xhtml"), null, "no document element: lookupPrefix");
done({count} + " lookups");
"#,
        count = cases.len() + 4
    );
    let end = with_page(limits(), ADDRESS, page, |runner| outcome(runner, &source));
    assert!(end.starts_with("held "), "{end}");
}

/// What the first review left: the `Window` rows with a wrong `this` and after the script
/// redefined or deleted globals; the `NodeList` iterable and `Symbol.iterator` of a kept
/// collection used after a refill; the constants of `Node` as Web IDL defines them;
/// `toString` and `Symbol.toStringTag`.
#[test]
fn window_rows_iterable_constants_and_tags() {
    let page = "<div id=\"p\"><i>0</i><i>1</i><i>2</i><i>3</i><i>4</i></div><div id=\"q\"><u></u><u></u></div>"
        .to_string();
    let source = format!(
        r#"{CHECK}
var gopd = Object.getOwnPropertyDescriptor;
var dd = gopd(G, "document"), wd = gopd(G, "window"), sd = gopd(G, "self"), ed = gopd(G, "event");
expect(dd.get.call(undefined), document, "document with undefined this");
expect(dd.get.call(null), document, "document with null this");
expect(threw(function () {{ dd.get.call({{}}); }}), "TypeError", "document with an object this");
expect(threw(function () {{ dd.get.call(document); }}), "TypeError", "document with the document as this");
expect(threw(function () {{ dd.get.call(1); }}), "TypeError", "document with a number this");
expect(dd.configurable, false, "document is unforgeable");
expect(wd.get.call(undefined), G, "window with undefined this");
expect(wd.configurable, false, "window is unforgeable");
expect(threw(function () {{ wd.get.call(Object.create(G)); }}), "TypeError", "window with an object whose prototype is the global");
expect(threw(function () {{ sd.set.call({{}}, 1); }}), "TypeError", "self's setter with an object this");
expect(threw(function () {{ ed.get.call("x"); }}), "TypeError", "event with a string this");
expect(event, undefined, "event");
expect(delete G.document, false, "delete document");
expect(document.nodeType, 9, "document after delete");
var Sym = Symbol, Arr = Array; G.Node = 1; G.NodeList = 2; delete G.HTMLCollection; G.Array = function () {{ throw new Error("Array ran"); }};
Object.defineProperty(G, "Symbol", {{ value: 3, configurable: true }});
var p = document.getElementById("p"), q = document.getElementById("q");
var c = p.childNodes;
expect(c.length, 5, "childNodes after globals were replaced");
expect(Object.prototype.toString.call(c), "[object NodeList]", "NodeList tag after globals were replaced");
expect(Object.prototype.toString.call(p.children), "[object HTMLCollection]", "HTMLCollection tag");
self = 5;
expect(self, 5, "self replaced");
expect(window, G, "window after self was replaced");
Object.defineProperty(G, "Symbol", {{ value: Sym, writable: true, configurable: true }}); G.Array = Arr;
var NL = Object.getPrototypeOf(c);
expect(NL.forEach, [].forEach, "forEach is Array's");
expect(NL.entries, [].entries, "entries is Array's");
expect(NL.keys, [].keys, "keys is Array's");
expect(NL.values, [].values, "values is Array's");
expect(NL[Symbol.iterator], [].values, "@@iterator is values");
expect(gopd(NL, "forEach").enumerable, true, "forEach enumerable");
expect(gopd(NL, Symbol.iterator).enumerable, false, "@@iterator not enumerable");
expect(threw(function () {{ c.forEach(function () {{ throw new RangeError("cb"); }}); }}), "RangeError", "forEach callback throws");
var seen = [];
c.forEach(function (x, i, all) {{
  if (i === 0) {{ q.appendChild(p.lastChild); q.appendChild(p.lastChild); check(p.childNodes === all, "the kept object"); }}
  seen.push(x);
}});
expect(seen.length, 3, "forEach after the callback shrank the kept collection");
expect(seen[2], c[2], "forEach saw the refilled list");
var it = c[Symbol.iterator]();
expect(it.next().value, c[0], "first of the iterator");
p.appendChild(q.firstChild); p.appendChild(q.firstChild); p.appendChild(q.firstChild);
check(p.childNodes === c && c.length === 6, "refilled to 6: " + c.length);
var rest = []; for (var r = it.next(); !r.done; r = it.next()) rest.push(r.value);
expect(rest.length, 5, "the iterator after a refill reads the new length");
expect(rest[4], c[5], "the iterator's last");
var pairs = []; for (var en = c.entries(), x = en.next(); !x.done; x = en.next()) pairs.push(x.value[0] + ":" + (x.value[1] === c[x.value[0]]));
expect(pairs.join(" "), "0:true 1:true 2:true 3:true 4:true 5:true", "entries");
var keys = []; for (var ke = c.keys(), y = ke.next(); !y.done; y = ke.next()) keys.push(y.value);
expect(keys.join(","), "0,1,2,3,4,5", "keys");
var count = 0; c.forEach(function (x) {{ count++; check(p.childNodes === c, "reading the same collection"); }});
expect(count, 6, "forEach reading the same collection");
var NodeC = Object.getPrototypeOf(Object.getPrototypeOf(p)).constructor;
var names = ["ELEMENT_NODE", "ATTRIBUTE_NODE", "TEXT_NODE", "CDATA_SECTION_NODE", "ENTITY_REFERENCE_NODE", "ENTITY_NODE", "PROCESSING_INSTRUCTION_NODE", "COMMENT_NODE", "DOCUMENT_NODE", "DOCUMENT_TYPE_NODE", "DOCUMENT_FRAGMENT_NODE", "NOTATION_NODE",
  "DOCUMENT_POSITION_DISCONNECTED", "DOCUMENT_POSITION_PRECEDING", "DOCUMENT_POSITION_FOLLOWING", "DOCUMENT_POSITION_CONTAINS", "DOCUMENT_POSITION_CONTAINED_BY", "DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC"];
var values = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 1, 2, 4, 8, 16, 32];
names.forEach(function (name, i) {{
  [NodeC, NodeC.prototype].forEach(function (on, j) {{
    var d = gopd(on, name);
    if (!d) {{ fails.push(name + " missing on " + j); return; }}
    expect(d.value + "/" + d.writable + "/" + d.enumerable + "/" + d.configurable, values[i] + "/false/true/false", name + " on " + (j ? "the prototype" : "the constructor"));
  }});
  expect(p[name], values[i], name + " on a node");
}});
expect(Object.prototype.toString.call(p), "[object Element]", "an element's tag");
expect(Object.prototype.toString.call(p.firstChild.firstChild), "[object Text]", "a text's tag");
expect(Object.prototype.toString.call(document), "[object Document]", "the document's tag");
expect(Object.prototype.toString.call(G), "[object Window]", "the global's tag");
expect(Object.prototype.toString.call(NL), "[object NodeList]", "the prototype's tag");
expect(String(c), "[object NodeList]", "String of a NodeList");
expect(gopd(NL, Symbol.toStringTag).writable, false, "@@toStringTag not writable");
expect(gopd(NL, Symbol.toStringTag).configurable, true, "@@toStringTag configurable");
G.kept = [p, c, q.childNodes];
G.shared = {{}}; G.shared.key = c;
p.marker = "m";
done("window, iterable, constants, tags");
"#
    );
    let (end, later) = with_page(limits(), ADDRESS, page, |runner| {
        let end = outcome(runner, &source);
        runner.agent_and_realm().0.collect().expect("a collection");
        let later = outcome(
            runner,
            "var p = document.getElementById('p'), q = document.getElementById('q');\
             throw new Error([kept[0] === p, kept[1] === p.childNodes, kept[2] === q.childNodes, shared.key === p.childNodes, p.marker].join(' '));",
        );
        (end, later)
    });
    assert!(end.starts_with("held "), "{end}");
    assert_eq!(later, "true true true true m", "{later}");
}

/// `getElementsByClassName` counts a class once per element (a class repeated on an element
/// does not stand for another wanted one) and charges the set of wanted classes it builds from
/// its argument, beside the argument's copy: one step per 64 bytes each.
#[test]
fn class_names_once_per_element_and_the_argument_charged() {
    let page = "<p id=\"aa\" class=\"a a\"></p><p id=\"ab\" class=\"a b\"></p><p id=\"bba\" class=\"b b a\"></p>"
        .to_string();
    let (found, spent) = with_page(limits(), ADDRESS, page, |runner| {
        let found = outcome(
            runner,
            "var c = document.getElementsByClassName('a b'), ids = [];\
             for (var i = 0; i < c.length; i++) {\
               ['aa', 'ab', 'bba'].forEach(function (id) { if (c[i] === document.getElementById(id)) ids.push(id); });\
             }\
             throw new Error(ids.join(' '));",
        );
        ran(
            runner,
            "var G = this; G.big = 'x '; while (big.length < 65536) big = big + big;",
        );
        let spent = steps(runner, "document.getElementsByClassName(big);");
        (found, spent)
    });
    eprintln!("'a b' finds: {found}; an argument of 65,536 bytes: {spent} steps");
    assert_eq!(found, "ab bba");
    assert!(
        spent >= 2 * 1_024,
        "the copy and the set, 1,024 steps each: {spent}"
    );
}
