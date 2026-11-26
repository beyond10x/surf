//! The generated DOM bindings (`story:webidl-bindings`) in a realm of the runner: each member
//! row of `crates/webidl/selection.txt` on its prototype, interface object or global with Web
//! IDL's property attributes, the brand checks, the members of `throws`, `DOMException`, the
//! realm's heap, the rule of decision 10, the conversions before the copies (decision 12) and
//! the node charge (decision 13).
//!
//! A test that needs nodes runs with the test slices below: a `Wrap` that makes a wrapper of
//! the node's interface (the element `pi` stands for a processing instruction, which the HTML
//! parser does not make), `Window.document` and `Document.getElementById` (by local name), and
//! the `attributes` and `mutation` members each test names. Every other member answers the
//! trait's default. Each test makes its own runner; acceptance 8's test measures the time.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use surf_html::limits::{Limit, Limits as ParserLimits, NODE_BYTES};
use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_script::bindings::{
    Attributes, DomCx, DomEnd, DomException, DomResult, EXCEPTION_TEXT_BYTES, Interface, Mutation,
    Reading, Selectors, Slices, StringOrElementCreationOptions, Wrap, new_wrapper, not_supported,
    uncharged_copies, wrapper_node,
};
use surf_script::dom::DocumentInfo;
use surf_script::{
    Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, ScriptErrorKind, ScriptHost, dom,
    run_document,
};
use swell::{BINDINGS_REALM_HEAP_BYTES, Completion, Cx, HostCall, Limits, PropertyKey, Value};

/// The page of the tests: a document, its element `html` with `head` and `body`, a `p`, the
/// element `pi` and a comment.
const PAGE: &str =
    "<!DOCTYPE html><html><head></head><body><p>a</p><pi></pi><!--c--></body></html>";

/// The names every script of a test with nodes starts with.
const PRELUDE: &str = r#"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
function desc(object, key) { return Object.getOwnPropertyDescriptor(object, key); }
var document = G.document;
var el = document.getElementById("p");
var pi = document.getElementById("pi");
check(el instanceof Element && pi instanceof ProcessingInstruction, "the test's wrappers");
"#;

fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 10_000_000,
            memory_bytes: 16 * 1024 * 1024,
            frame_depth: 1_000,
            reentry_depth: 64,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 100_000_000,
        scripts_per_runner: 100,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(20),
    }
}

/// What the test's `attributes` slice was handed.
#[derive(Clone, Default)]
struct Record(Arc<Mutex<Vec<(String, String)>>>);

impl Record {
    fn taken(&self) -> Vec<(String, String)> {
        self.0.lock().expect("the record").clone()
    }
}

/// How the test's `mutation` slice answers `createElement` and `createTextNode`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Writes {
    /// The trait's default.
    None,
    /// `createElement` answers `DomEnd::Limit(Nodes)`.
    LimitNodes,
    /// `createTextNode` makes the node charge of decision 13 and nothing else.
    Charge,
    /// `createTextNode` answers the document without a charge: what the call costs besides it.
    Uncharged,
}

/// The node of the element whose local name is `name`.
fn element(tree: &Tree, name: &str) -> Option<NodeId> {
    tree.descendants(tree.root())
        .into_iter()
        .find(|&node| matches!(tree.data(node), NodeData::Element { local_name, .. } if local_name == name))
}

struct TestReading;

impl Reading for TestReading {
    fn window_document(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<NodeId> {
        Ok(cx.tree().root())
    }

    fn document_get_element_by_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        element_id: String,
    ) -> DomResult<Option<NodeId>> {
        if element_id == "throw-long" {
            return Err(DomEnd::Throw(DomException::new(
                &"N".repeat(2_000),
                format!("a{}", "\u{e9}".repeat(1_000)),
            )));
        }
        Ok(element(cx.tree(), &element_id))
    }
}

struct TestAttributes(Record);

impl Attributes for TestAttributes {
    fn element_set_attribute(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        qualified_name: String,
        value: String,
    ) -> DomResult<()> {
        self.0
            .0
            .lock()
            .expect("the record")
            .push((qualified_name, value));
        Ok(())
    }

    fn set_element_id(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        value: String,
    ) -> DomResult<()> {
        self.0
            .0
            .lock()
            .expect("the record")
            .push(("id".to_string(), value));
        Ok(())
    }
}

struct TestMutation(Writes);

impl Mutation for TestMutation {
    fn document_create_element(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        _local_name: String,
        _options: StringOrElementCreationOptions,
    ) -> DomResult<NodeId> {
        match self.0 {
            Writes::LimitNodes => Err(DomEnd::Limit(Limit::Nodes)),
            Writes::None | Writes::Charge | Writes::Uncharged => {
                Err(not_supported("Document.createElement"))
            }
        }
    }

    fn document_create_text_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        data: String,
    ) -> DomResult<NodeId> {
        match self.0 {
            Writes::Charge => {
                cx.charge_node(data.len())?;
                Ok(this)
            }
            Writes::Uncharged => Ok(this),
            Writes::None | Writes::LimitNodes => Err(not_supported("Document.createTextNode")),
        }
    }
}

struct TestSelectors;

impl Selectors for TestSelectors {}

struct TestWrap;

impl Wrap for TestWrap {
    fn wrap(&mut self, cx: &mut DomCx<'_, '_>, node: NodeId) -> Completion<Value> {
        let interface = match cx.tree().data(node) {
            NodeData::Document => Interface::Document,
            NodeData::DocumentFragment => Interface::DocumentFragment,
            NodeData::Doctype { .. } => Interface::DocumentType,
            NodeData::Element { local_name, .. } if local_name == "pi" => {
                Interface::ProcessingInstruction
            }
            NodeData::Element { .. } => Interface::Element,
            NodeData::Text(_) => Interface::Text,
            NodeData::Comment(_) => Interface::Comment,
            NodeData::ProcessingInstruction { .. } => Interface::ProcessingInstruction,
        };
        new_wrapper(cx, interface, node).map(Value::Object)
    }

    fn unwrap(&self, cx: &DomCx<'_, '_>, value: Value) -> Option<NodeId> {
        wrapper_node(cx, value)
    }
}

/// The document of the tests with the test slices.
fn test_document(writes: Writes, record: Record) -> impl FnOnce() -> Document + Send {
    move || Document {
        tree: parse_document(PAGE, &ParseOptions::default()),
        limits: ParserLimits::default(),
        slices: Slices {
            reading: Box::new(TestReading),
            attributes: Box::new(TestAttributes(record)),
            mutation: Box::new(TestMutation(writes)),
            selectors: Box::new(TestSelectors),
            wrap: Box::new(TestWrap),
        },
    }
}

/// The document of the tests with surf's own slices.
fn surf_document() -> Document {
    Document {
        tree: parse_document(PAGE, &ParseOptions::default()),
        limits: ParserLimits::default(),
        slices: dom::slices(DocumentInfo::default()),
    }
}

/// `[SameObject]` is held by the `Wrap` that keeps the objects: with a `TestWrap`, whose
/// `same_object` is the trait's default, a getter of the four makes a new object at each read.
#[test]
fn a_test_wrap_makes_a_new_collection_per_read() {
    let document = || {
        let mut slices = dom::slices(DocumentInfo::default());
        slices.wrap = Box::new(TestWrap);
        Document {
            tree: parse_document(PAGE, &ParseOptions::default()),
            limits: ParserLimits::default(),
            slices,
        }
    };
    with_document(limits(), document, |runner| {
        let end = runner.evaluate(
            r#"var html = this.document.documentElement;
               if (html.childNodes === html.childNodes) throw new Error("childNodes kept");
               if (html.children === html.children) throw new Error("children kept");
               if (html.childNodes.length !== 2) throw new Error("the children");"#,
        );
        assert_eq!(end, ScriptEnd::Ran, "{end}");
    });
}

fn with_document<R: Send>(
    limits: RunnerLimits,
    document: impl FnOnce() -> Document + Send,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(limits, Mode::Test, document, body).expect("a runner with the bindings")
}

/// Runs `source` after [`PRELUDE`] and asserts it ran.
fn ran(runner: &mut Runner, source: &str) {
    let end = runner.evaluate(&format!("{PRELUDE}{source}"));
    assert_eq!(end, ScriptEnd::Ran, "{end}");
}

/// A row of `selection.txt`: interface, member, slice and the note.
struct Row {
    interface: String,
    member: String,
    slice: String,
    note: String,
}

fn selection() -> Vec<Row> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../webidl/selection.txt");
    let text = std::fs::read_to_string(path).expect("read selection.txt");
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|line| !line.starts_with("definitions:") && !line.starts_with("global:"))
        .map(|line| {
            let mut words = line.splitn(4, ' ');
            let mut next = || words.next().unwrap_or_default().to_string();
            let (interface, member, slice, note) = (next(), next(), next(), next());
            Row {
                interface,
                member,
                slice,
                note,
            }
        })
        .collect()
}

/// The writable attributes of the selection: those not `readonly`, those with
/// `[Replaceable]`, and `classList` with `[PutForwards=value]`.
const WRITABLE: &[&str] = &[
    "Node.nodeValue",
    "Node.textContent",
    "Element.id",
    "Element.className",
    "Element.classList",
    "Element.slot",
    "Document.title",
    "CharacterData.data",
    "Window.self",
    "Window.event",
];

/// The `[LegacyUnforgeable]` members: own properties of the global, not configurable.
const UNFORGEABLE: &[&str] = &["Window.window", "Window.document"];

fn js_set(names: &[&str]) -> String {
    let entries: Vec<String> = names.iter().map(|name| format!("{name:?}: true")).collect();
    format!("{{ {} }}", entries.join(", "))
}

#[test]
fn every_member_row_has_its_property() {
    let rows = selection();
    let members: Vec<&Row> = rows
        .iter()
        .filter(|row| row.member != "constructor" && row.member != "iterable")
        .collect();
    assert_eq!(members.len(), 226);
    let mut cases = String::new();
    for row in &rows {
        let kind = if row.member == "constructor" {
            "constructor"
        } else if row.member == "iterable" {
            "iterable"
        } else if row
            .member
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b == b'_')
            && row.member.contains('_')
        {
            "constant"
        } else {
            "member"
        };
        cases.push_str(&format!(
            "row({:?}, {:?}, {kind:?});\n",
            row.interface, row.member
        ));
    }
    let source = format!(
        r#"
var writable = {writable};
var unforgeable = {unforgeable};
var members = 0;
function row(i, m, kind) {{
  var label = i + "." + m;
  var object = desc(G, i).value;
  if (kind === "constructor") {{
    var d = desc(G, i);
    check(typeof d.value === "function" && d.writable && !d.enumerable && d.configurable, label);
    return;
  }}
  if (kind === "iterable") {{
    check(object.prototype[Symbol.iterator] === Array.prototype.values, label);
    return;
  }}
  members++;
  var holder = i === "Window" ? G : object.prototype;
  if (kind === "constant") {{
    var on = [object.prototype, object];
    for (var k = 0; k < 2; k++) {{
      var c = desc(on[k], m);
      check(c !== undefined && typeof c.value === "number" && c.writable === false
        && c.enumerable === true && c.configurable === false, label + ": constant " + k);
    }}
    return;
  }}
  var d = desc(holder, m);
  check(d !== undefined, label + ": missing");
  var forged = unforgeable[label] !== true;
  if ("get" in d) {{
    check(typeof d.get === "function" && d.enumerable === true && d.configurable === forged,
      label + ": accessor");
    check((typeof d.set === "function") === (writable[label] === true), label + ": setter");
  }} else {{
    check(typeof d.value === "function" && d.writable === forged && d.enumerable === true
      && d.configurable === forged, label + ": operation");
  }}
}}
{cases}
check(members === {count}, "cases: " + members);
check(Object.getPrototypeOf(Element.prototype) === Node.prototype, "Element.prototype");
check(Object.getPrototypeOf(Node.prototype) === EventTarget.prototype, "Node.prototype");
check(Object.getPrototypeOf(EventTarget.prototype) === Object.prototype, "EventTarget.prototype");
check(Object.getPrototypeOf(Element) === Node, "the interface object Element");
check(Object.getPrototypeOf(G) === Window.prototype, "the global");
check(Object.getPrototypeOf(Text.prototype) === CharacterData.prototype, "Text.prototype");
"#,
        writable = js_set(WRITABLE),
        unforgeable = js_set(UNFORGEABLE),
        count = members.len(),
    );
    with_document(
        limits(),
        test_document(Writes::None, Record::default()),
        |runner| {
            ran(runner, &source);
        },
    );
}

#[test]
fn a_member_checks_the_brand_of_this() {
    let source = r#"
function typeError(f, what) {
  try { f(); } catch (e) { check(e instanceof TypeError, what + ": " + e); return; }
  throw new Error(what + ": no throw");
}
var hasChildNodes = Node.prototype.hasChildNodes;
typeError(function () { hasChildNodes.call(new DOMException("x")); }, "another brand");
typeError(function () { hasChildNodes.call(Object.create(Node.prototype)); }, "made by script");
typeError(function () { hasChildNodes.call({}); }, "a plain object");
typeError(function () { desc(Node.prototype, "nodeType").get.call(new DOMException()); },
  "a getter on another brand");
typeError(function () { Element.prototype.getAttribute.call(document, "a"); },
  "a Document is no Element");
typeError(function () { NodeList.prototype.item.call(el, 0); }, "an Element is no NodeList");
typeError(function () { new Node(); }, "new Node()");
typeError(function () { Node(); }, "Node()");
typeError(function () { DOMException(); }, "DOMException()");
"#;
    with_document(
        limits(),
        test_document(Writes::None, Record::default()),
        |runner| {
            ran(runner, source);
        },
    );
}

#[test]
fn the_members_of_throws_throw_not_supported_error() {
    let rows = selection();
    let throws: Vec<&Row> = rows.iter().filter(|row| row.slice == "throws").collect();
    assert_eq!(throws.len(), 32, "30 members and 2 constructors");
    let mut cases = String::new();
    for row in &throws {
        let reason = row.note.strip_prefix("reason: ").expect("a reason");
        let this = match row.interface.as_str() {
            "EventTarget" => "G",
            "Element" => "el",
            "Document" => "document",
            "ProcessingInstruction" => "pi",
            other => panic!("no object of {other}"),
        };
        if row.member == "constructor" {
            cases.push_str(&format!(
                "notSupported(function () {{ new {i}(); }}, {label:?});\n",
                i = row.interface,
                label = format!("{} constructor is not supported: {reason}", row.interface),
            ));
        } else {
            cases.push_str(&format!(
                "call({i:?}, {m:?}, {this}, {label:?});\n",
                i = row.interface,
                m = row.member,
                label = format!(
                    "{}.{} is not supported: {reason}",
                    row.interface, row.member
                ),
            ));
        }
    }
    let source = format!(
        r#"
var cases = 0;
function notSupported(f, message) {{
  cases++;
  try {{ f(); }} catch (e) {{
    check(e instanceof DOMException && e.name === "NotSupportedError" && e.code === 9
      && e.message === message, message + ": " + e.name + ": " + e.message);
    return;
  }}
  throw new Error(message + ": no throw");
}}
function call(i, m, object, message) {{
  var d = desc(G[i].prototype, m);
  if (d.get) {{
    notSupported(function () {{ d.get.call(object); }}, message);
    if (d.set) {{ notSupported(function () {{ d.set.call(object, "x"); }}, message); cases--; }}
  }} else {{
    notSupported(function () {{ d.value.call(object, "a", "b"); }}, message);
  }}
}}
{cases}
check(cases === {count}, "cases: " + cases);
// Web IDL converts the arguments first; a member of throws converts none.
notSupported(function () {{
  el.getAttributeNode({{ toString: function () {{ throw new Error("converted"); }} }});
}}, "Element.getAttributeNode is not supported: attribute nodes");
"#,
        count = throws.len(),
    );
    with_document(
        limits(),
        test_document(Writes::None, Record::default()),
        |runner| {
            ran(runner, &source);
        },
    );
}

#[test]
fn a_member_of_a_slice_throws_until_its_slice() {
    let source = r#"
var G = this;
function notSupported(f, message) {
  try { f(); } catch (e) {
    if (!(e instanceof DOMException) || e.name !== "NotSupportedError" || e.message !== message) {
      throw new Error(message + ": " + e.name + ": " + e.message);
    }
    return;
  }
  throw new Error(message + ": no throw");
}
notSupported(function () { return G.document.createComment("x"); },
  "Document.createComment is not supported");
notSupported(function () { return G.document.appendChild(G.document.firstChild); },
  "Node.appendChild is not supported");
notSupported(function () { return new Text("a"); }, "Text constructor is not supported");
notSupported(function () { return new DocumentFragment(); },
  "DocumentFragment constructor is not supported");
"#;
    with_document(limits(), surf_document, |runner| {
        let end = runner.evaluate(source);
        assert_eq!(end, ScriptEnd::Ran, "{end}");
    });
}

#[test]
fn dom_exception_is_constructible() {
    let rows = selection();
    let constants: Vec<String> = rows
        .iter()
        .filter(|row| row.interface == "DOMException" && row.member.ends_with("_ERR"))
        .map(|row| format!("{:?}", row.member))
        .collect();
    assert_eq!(constants.len(), 25);
    let source = format!(
        r#"
var G = this;
function check(holds, what) {{ if (!holds) {{ throw new Error(what); }} }}
var names = [{names}];
for (var k = 0; k < names.length; k++) {{
  check(DOMException[names[k]] === k + 1 && DOMException.prototype[names[k]] === k + 1, names[k]);
}}
var e = new DOMException("m", "SyntaxError");
check(e.name === "SyntaxError" && e.message === "m" && e.code === 12, "SyntaxError: " + e.code);
check(e instanceof DOMException && e instanceof Error, "instanceof");
check(Object.getPrototypeOf(DOMException.prototype) === Error.prototype, "%Error.prototype%");
check(Object.prototype.toString.call(e) === "[object DOMException]", "@@toStringTag");
var d = new DOMException();
check(d.name === "Error" && d.message === "" && d.code === 0, "the defaults");
check(new DOMException("x", "NotSupportedError").code === 9, "NotSupportedError's code");
check(new DOMException("x", "NoSuchName").code === 0, "a name without a code");
check(new DOMException("x", "NoModificationAllowedError").code === 7, "the longest legacy name");
var threw = false;
try {{ Object.getOwnPropertyDescriptor(DOMException.prototype, "name").get.call({{}}); }}
catch (x) {{ threw = x instanceof TypeError; }}
check(threw, "name on another object");
"#,
        names = constants.join(", "),
    );
    with_document(limits(), surf_document, |runner| {
        let end = runner.evaluate(&source);
        assert_eq!(end, ScriptEnd::Ran, "{end}");
    });
}

/// Must 1 of the second review, as ruled: a `DOMException` holds its `name` and `message` as
/// the engine's Strings, listed by the `trace` of its data; no other handle keeps them, and they
/// outlive a collection, every code unit kept, for one a script made and one the bindings threw.
#[test]
fn the_strings_of_a_dom_exception_outlive_a_collection() {
    with_document(limits(), surf_document, |runner| {
        let end = runner.evaluate(
            r#"var G = this;
               G.e = new DOMException(String.fromCharCode(0x6D, 0xD800), String.fromCharCode(0x4E));
               try { G.document.createComment("x"); } catch (x) { G.thrown = x; }"#,
        );
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        let collected = runner.agent_and_realm().0.collect();
        assert!(
            matches!(collected, Ok(swell::Outcome::Normal(_))),
            "{collected:?}"
        );
        let end = runner.evaluate(
            r#"function check(holds, what) { if (!holds) { throw new Error(what); } }
               check(G.e.message === "m\uD800" && G.e.name === "N" && G.e.code === 0, "made");
               check(G.thrown.name === "NotSupportedError" && G.thrown.code === 9, "thrown");
               check(G.thrown.message === "Document.createComment is not supported",
                     G.thrown.message);"#,
        );
        assert_eq!(end, ScriptEnd::Ran, "{end}");
    });
}

/// The texts of a `DOMException` the bindings make from a slice's answer are cut at
/// `EXCEPTION_TEXT_BYTES` bytes, at a character's boundary: a name of 2,000 ASCII letters keeps
/// 1,024; a message of one letter and 1,000 two-byte characters keeps 1,023 bytes, 512
/// characters.
#[test]
fn the_texts_of_a_thrown_dom_exception_are_bounded() {
    assert_eq!(EXCEPTION_TEXT_BYTES, 1024);
    with_document(
        limits(),
        test_document(Writes::None, Record::default()),
        |runner| {
            ran(
                runner,
                r#"var caught = null;
                   try { document.getElementById("throw-long"); } catch (x) { caught = x; }
                   check(caught instanceof DOMException, "a DOMException");
                   check(caught.name.length === 1024, "name: " + caught.name.length);
                   check(caught.message.length === 512 && caught.message[511] === "é",
                         "message: " + caught.message.length);"#,
            );
        },
    );
}

/// The peak resident bytes of this process (`VmHWM` of `/proc/self/status`, Linux).
fn peak_resident_bytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("/proc/self/status");
    let kib: u64 = status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|field| field.parse().ok())
        .expect("VmHWM");
    kib * 1024
}

/// Note 5 of the third review, as ruled: the record of an uncaught `DOMException` copies no
/// more of its name and message than its 1,024 characters need. A message of 2^22 code units
/// of U+0800 (8 MiB on swell's heap) thrown uncaught raises the process's peak by less than
/// 4 MiB, and the record is its first 1,024 characters; one of 1,100 characters outside the
/// BMP (2,200 code units) keeps 1,021 of them whole.
#[test]
fn the_record_of_a_long_dom_exception_copies_only_its_prefix() {
    let mut large = limits();
    large.engine.memory_bytes = 32 << 20;
    large.engine.string_length = 1 << 24;
    let (end, grown, astral, long_name) = with_document(large, surf_document, |runner| {
        let end = runner.evaluate(r#"this.e = new DOMException("ࠀ".repeat(1 << 22), "N");"#);
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        let collected = runner.agent_and_realm().0.collect();
        assert!(
            matches!(collected, Ok(swell::Outcome::Normal(_))),
            "{collected:?}"
        );
        let before = peak_resident_bytes();
        let end = runner.evaluate("throw this.e;");
        let grown = peak_resident_bytes().saturating_sub(before);
        let astral = runner.evaluate(r#"throw new DOMException("\u{1F600}".repeat(1100), "N");"#);
        let long_name = runner.evaluate(r#"throw new DOMException("m", "x".repeat(3000));"#);
        (end, grown, astral, long_name)
    });
    match end {
        ScriptEnd::Threw(error) => {
            let message = error.message.expect("a message");
            assert_eq!(message.chars().count(), 1024);
            assert_eq!(message, format!("N: {}", "\u{800}".repeat(1_021)));
        }
        other => panic!("{other}"),
    }
    let mib = grown >> 20;
    assert!(mib < 4, "the record raised the peak by {mib} MiB");
    match astral {
        ScriptEnd::Threw(error) => assert_eq!(
            error.message.as_deref(),
            Some(format!("N: {}", "\u{1F600}".repeat(1_021)).as_str())
        ),
        other => panic!("{other}"),
    }
    match long_name {
        ScriptEnd::Threw(error) => {
            assert_eq!(error.message.as_deref(), Some("x".repeat(1_024).as_str()));
        }
        other => panic!("{other}"),
    }
}

#[test]
fn a_realm_with_the_bindings_stays_within_its_heap() {
    let started = Instant::now();
    let (heap, made) = with_document(limits(), surf_document, |runner| {
        let made = started.elapsed();
        let heap = runner.agent_and_realm().0.heap_bytes().expect("heap_bytes");
        (heap, made)
    });
    let started = Instant::now();
    let (plain, plain_made) = surf_script::run(limits(), Mode::Test, |runner| {
        let made = started.elapsed();
        (
            runner.agent_and_realm().0.heap_bytes().expect("heap_bytes"),
            made,
        )
    })
    .expect("a runner");
    eprintln!(
        "bindings realm: heap {heap} bytes (a realm without them {plain}), made in {made:?} \
         (without them {plain_made:?}); the bound {BINDINGS_REALM_HEAP_BYTES}"
    );
    assert!(
        heap <= BINDINGS_REALM_HEAP_BYTES,
        "{heap} > {BINDINGS_REALM_HEAP_BYTES}"
    );
}

#[test]
fn trusted_type_as_domstring() {
    let record = Record::default();
    let handed = record.clone();
    with_document(limits(), test_document(Writes::None, record), |runner| {
        ran(
            runner,
            r#"el.setAttribute("a", { toString: function () { return "x"; } });"#,
        );
    });
    assert_eq!(handed.taken(), [("a".to_string(), "x".to_string())]);
}

/// A named deviation (should 7 of the review, ruled): a lone surrogate in a `DOMString`
/// argument reaches the slice as U+FFFD, since the tree holds Rust strings (the head of
/// `selection.txt`, `docs/design/scripts.md` Q17). Pinned for an attribute's setter and an
/// operation.
#[test]
fn a_lone_surrogate_reaches_the_slice_as_u_fffd() {
    let record = Record::default();
    let handed = record.clone();
    with_document(limits(), test_document(Writes::None, record), |runner| {
        ran(
            runner,
            r#"el.id = "a\uD800b"; el.setAttribute("\uDC00", "c\uDBFF");"#,
        );
    });
    assert_eq!(
        handed.taken(),
        [
            ("id".to_string(), "a\u{FFFD}b".to_string()),
            ("\u{FFFD}".to_string(), "c\u{FFFD}".to_string()),
        ]
    );
}

/// Note 11 of the review, as ruled: an uncaught `DOMException` ends the script `threw Error`
/// with the message `<name>: <message>`, read from the exception's own data without running
/// script and cut to 1,024 characters: one the bindings threw, and two a script made.
#[test]
fn an_uncaught_dom_exception_names_its_name_and_message() {
    let ends = with_document(limits(), surf_document, |runner| {
        [
            runner.evaluate(r#"this.document.createComment("x");"#),
            runner.evaluate(r#"throw new DOMException("m", "SyntaxError");"#),
            runner.evaluate(
                r#"var s = ""; for (var i = 0; i < 2000; i++) { s += "x"; }
                   throw new DOMException(s, "E");"#,
            ),
        ]
    });
    let long = format!("E: {}", "x".repeat(1_021));
    let expected = [
        "NotSupportedError: Document.createComment is not supported",
        "SyntaxError: m",
        long.as_str(),
    ];
    for (end, message) in ends.iter().zip(expected) {
        match end {
            ScriptEnd::Threw(error) => {
                assert_eq!(error.kind, ScriptErrorKind::Error, "{end}");
                assert_eq!(error.message.as_deref(), Some(message), "{end}");
            }
            other => panic!("not a throw: {other}"),
        }
    }
}

/// The record of the document, read between two scripts.
fn recorded(runner: &mut Runner) -> (Option<Limit>, usize) {
    let host: &ScriptHost = runner.agent_and_realm().0.host();
    let state = host.document().expect("a document");
    (state.limit(), state.tree().node_count())
}

#[test]
fn limit_end() {
    with_document(
        limits(),
        test_document(Writes::LimitNodes, Record::default()),
        |runner| {
            let (_, before) = recorded(runner);
            let end = runner.evaluate(&format!(
                "{PRELUDE}try {{ document.createElement(\"p\"); }} catch (e) {{}}"
            ));
            assert_eq!(end, ScriptEnd::Limit(Reason::Nodes), "{end}");
            assert_eq!(
                recorded(runner),
                (None, before),
                "the record cleared, the tree unchanged"
            );
            assert_eq!(runner.evaluate("1"), ScriptEnd::Ran);
        },
    );
}

/// A DOM limit ends the script with no checkpoint and clears the microtask queue: the job the
/// script queued before the limit never runs, neither after it nor in the next script's
/// checkpoint.
#[test]
fn dom_limit_drops_the_jobs() {
    with_document(
        limits(),
        test_document(Writes::LimitNodes, Record::default()),
        |runner| {
            let end = runner.evaluate(&format!(
                "{PRELUDE}Promise.resolve().then(function () {{ globalThis.q = 1; }});
                 document.createElement(\"p\");"
            ));
            assert_eq!(end, ScriptEnd::Limit(Reason::Nodes), "{end}");
            assert_eq!(runner.evaluate("1"), ScriptEnd::Ran);
            let after = runner.evaluate("if (typeof q !== \"undefined\") throw new Error(\"q\")");
            assert_eq!(after, ScriptEnd::Ran, "{after}");
        },
    );
}

/// `limit_end` with the write in a promise job: the checkpoint's end is the script's, so the
/// limit the job reached ends the script `limit: nodes`, and the next script runs.
#[test]
fn limit_end_in_a_then_callback() {
    with_document(
        limits(),
        test_document(Writes::LimitNodes, Record::default()),
        |runner| {
            let (_, before) = recorded(runner);
            let end = runner.evaluate(&format!(
                "{PRELUDE}Promise.resolve().then(function () {{ document.createElement(\"p\"); }});"
            ));
            assert_eq!(end, ScriptEnd::Limit(Reason::Nodes), "{end}");
            assert_eq!(
                recorded(runner),
                (None, before),
                "the record cleared, the tree unchanged"
            );
            assert_eq!(runner.evaluate("1"), ScriptEnd::Ran);
        },
    );
}

/// Decision 9 as ruled in fix round 1: a limit ends the script where it is reached; no
/// `catch` and no `finally` of the page runs after it, the interrupt flag is clear after the
/// entry and the record is cleared, and the next script runs.
#[test]
fn no_catch_and_no_finally_runs_after_a_limit() {
    with_document(
        limits(),
        test_document(Writes::LimitNodes, Record::default()),
        |runner| {
            let end = runner.evaluate(&format!(
                "{PRELUDE}G.ran = [];
                 try {{ document.createElement(\"p\"); G.ran.push(\"after\"); }}
                 catch (e) {{ G.ran.push(\"catch\"); }}
                 finally {{ G.ran.push(\"finally\"); }}
                 G.ran.push(\"end\");"
            ));
            assert_eq!(end, ScriptEnd::Limit(Reason::Nodes), "{end}");
            let flag = runner.agent_and_realm().0.interrupt_handle().is_set();
            assert!(!flag, "the interrupt flag after the entry");
            assert_eq!(recorded(runner).0, None, "the record cleared");
            let after = runner.evaluate(
                "if (G.ran.length !== 0) { throw new Error(\"ran: \" + G.ran.join(\" \")); }",
            );
            assert_eq!(after, ScriptEnd::Ran, "{after}");
        },
    );
}

#[test]
fn wall_clock_without_record() {
    let mut bounded = limits();
    bounded.wall_clock = Duration::from_millis(100);
    bounded.engine.steps_per_entry = u64::MAX;
    let started = Instant::now();
    let end = with_document(
        bounded,
        test_document(Writes::None, Record::default()),
        |runner| runner.evaluate("while (true) {}"),
    );
    let elapsed = started.elapsed();
    assert_eq!(end, ScriptEnd::Limit(Reason::WallClock), "{end}");
    assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
}

/// The test's `uncharged()`: the count of decision 12 now.
fn uncharged(_cx: &mut Cx<'_, ScriptHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Number(uncharged_copies() as f64))
}

fn define_uncharged(runner: &mut Runner) {
    let (agent, realm) = runner.agent_and_realm();
    let global = agent.global_object(realm).expect("global");
    agent
        .with_cx(realm, |cx| {
            let realm_ref = cx.realm_ref(realm).expect("realm ref");
            let function = cx
                .new_host_function(realm_ref, "uncharged", 0, uncharged, false, None)
                .expect("host function")
                .expect("host function");
            let Value::Object(global) = cx.get(&global).expect("global") else {
                panic!("the global is no object");
            };
            let key = PropertyKey::String(cx.new_string("uncharged").expect("name"));
            cx.set(global, key, Value::Object(function), true)
                .expect("set")
                .expect("set");
        })
        .expect("with_cx");
}

#[test]
fn convert_before_copy() {
    let record = Record::default();
    let handed = record.clone();
    with_document(limits(), test_document(Writes::None, record), |runner| {
        define_uncharged(runner);
        ran(
            runner,
            r#"
el.setAttribute("a", { toString: function () { return String(uncharged()); } });
el.setAttribute({ toString: function () { return "n" + uncharged(); } }, "v");
"#,
        );
    });
    assert_eq!(
        handed.taken(),
        [
            ("a".to_string(), "0".to_string()),
            ("n0".to_string(), "v".to_string())
        ]
    );
}

#[test]
fn node_charge() {
    // The heap a call of `createTextNode("abcd")` adds, with the charge and without it.
    let grows = |writes: Writes| {
        with_document(
            limits(),
            test_document(writes, Record::default()),
            |runner| {
                ran(runner, "");
                let before = runner.agent_and_realm().0.heap_bytes().expect("heap");
                ran(runner, r#"document.createTextNode("abcd");"#);
                let after = runner.agent_and_realm().0.heap_bytes().expect("heap");
                after.saturating_sub(before)
            },
        )
    };
    let (charged, uncharged) = (grows(Writes::Charge), grows(Writes::Uncharged));
    // Decision 13: 3 × `NODE_BYTES` (3 × 136 = 408) and twice the four bytes stored, 416 bytes,
    // the node term of the tree's heap bound (`crates/html/src/limits.rs:45-51`). Acceptance 12
    // of the story says 1,224 + 8 (3 × 408); the test holds the bound the decision names.
    let node_term = 3 * NODE_BYTES as u64 + 2 * 4;
    assert_eq!(node_term, 416);
    assert!(
        charged >= uncharged + node_term,
        "the charge adds {charged} - {uncharged} bytes"
    );
    let mut small = limits();
    small.engine.memory_bytes = 1024 * 1024;
    with_document(
        small,
        test_document(Writes::Charge, Record::default()),
        |runner| {
            let end = runner.evaluate(&format!(
                "{PRELUDE}for (;;) document.createTextNode(\"x\");"
            ));
            assert_eq!(end, ScriptEnd::Limit(Reason::Memory), "{end}");
        },
    );
}
