//! Review of `story:webidl-bindings`: the generated bindings against the story's decisions 9,
//! 10 and 12, Q4's limit words and Web IDL, with test slices that record what reaches them.
//!
//! The test `Wrap` makes a wrapper of the interface its element's local name names (`pi` a
//! `ProcessingInstruction`, `nodebrand` a `Node`, `targetbrand` an `EventTarget`, `chardata` a
//! `CharacterData`), so a member can be called on a sibling or parent brand; it counts its
//! `unwrap` calls and can answer `None` for one element.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use surf_html::limits::{Limit, Limits as ParserLimits};
use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_script::bindings::{
    Attributes, DomCx, DomEnd, DomResult, Interface, Mutation, Reading, Selectors, Slices,
    StringOrElementCreationOptions, TheWindow, Wrap, new_wrapper, not_supported, uncharged_copies,
    wrapper_node,
};
use surf_script::{
    Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, ScriptErrorKind, ScriptHost,
    run_document,
};
use swell::{Completion, Cx, HostCall, Limits, PropertyKey, Value};

const PAGE: &str = "<!DOCTYPE html><html><head></head><body><p>a</p><pi></pi>\
     <nodebrand></nodebrand><targetbrand></targetbrand><chardata></chardata><!--c--></body></html>";

const PRELUDE: &str = r#"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
function desc(object, key) { return Object.getOwnPropertyDescriptor(object, key); }
var document = G.document;
var el = document.getElementById("p");
var pi = document.getElementById("pi");
"#;

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
        wall_clock: Duration::from_secs(20),
    }
}

/// What the test slices saw.
#[derive(Clone, Default)]
struct Log {
    unwraps: Arc<AtomicUsize>,
    creates: Arc<AtomicUsize>,
    attributes: Arc<Mutex<Vec<(String, String)>>>,
    /// `uncharged_copies()` read by `setAttribute`'s slice method, for each call.
    copies_seen: Arc<Mutex<Vec<usize>>>,
}

impl Log {
    fn attributes(&self) -> Vec<(String, String)> {
        self.attributes.lock().expect("the log").clone()
    }

    fn push(&self, name: &str, value: String) {
        self.attributes
            .lock()
            .expect("the log")
            .push((name.to_string(), value));
    }
}

/// How the test slices answer.
#[derive(Clone, Copy, Default)]
struct Answers {
    /// The n-th `createElement` answers the n-th limit (the last one after that); empty: the
    /// trait's default.
    create_limits: &'static [Limit],
    /// `unwrap` answers `None` for the element of this local name.
    refuse_unwrap_of: Option<&'static str>,
    /// `createTextNode` answers without the node charge.
    text_uncharged: bool,
}

fn element(tree: &Tree, name: &str) -> Option<NodeId> {
    tree.descendants(tree.root())
        .into_iter()
        .find(|&node| matches!(tree.data(node), NodeData::Element { local_name, .. } if local_name == name))
}

fn local_name(tree: &Tree, node: NodeId) -> Option<String> {
    match tree.data(node) {
        NodeData::Element { local_name, .. } => Some(local_name.to_string()),
        _ => None,
    }
}

struct TestReading;

impl Reading for TestReading {
    fn window_document(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<NodeId> {
        Ok(cx.tree().root())
    }

    fn window_window(&mut self, _cx: &mut DomCx<'_, '_>) -> DomResult<TheWindow> {
        Ok(TheWindow)
    }

    fn document_get_element_by_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        element_id: String,
    ) -> DomResult<Option<NodeId>> {
        Ok(element(cx.tree(), &element_id))
    }
}

struct TestAttributes(Log);

impl Attributes for TestAttributes {
    fn element_set_attribute(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        qualified_name: String,
        value: String,
    ) -> DomResult<()> {
        self.0
            .copies_seen
            .lock()
            .expect("the log")
            .push(uncharged_copies());
        self.0.push(&qualified_name, value);
        Ok(())
    }

    fn set_element_id(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        value: String,
    ) -> DomResult<()> {
        self.0.push("id", value);
        Ok(())
    }

    fn element_toggle_attribute(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        qualified_name: String,
        force: Option<bool>,
    ) -> DomResult<bool> {
        self.0
            .push(&format!("toggle {qualified_name}"), format!("{force:?}"));
        Ok(true)
    }

    fn element_get_attribute_names(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<Vec<String>> {
        Ok(vec!["x".to_string(), "y".to_string()])
    }
}

struct TestMutation(Log, Answers);

impl Mutation for TestMutation {
    fn document_create_element(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        _local_name: String,
        options: StringOrElementCreationOptions,
    ) -> DomResult<NodeId> {
        let made = self.0.creates.fetch_add(1, Ordering::SeqCst);
        let limits = self.1.create_limits;
        match limits.get(made).or(limits.last()) {
            Some(limit) => Err(DomEnd::Limit(*limit)),
            None => {
                self.0.push("create options", format!("{options:?}"));
                Err(not_supported("Document.createElement"))
            }
        }
    }

    /// The node charge of decision 13 and nothing else (or nothing at all).
    fn document_create_text_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        data: String,
    ) -> DomResult<NodeId> {
        if !self.1.text_uncharged {
            cx.charge_node(data.len())?;
        }
        Ok(this)
    }

    fn set_character_data_data(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        value: String,
    ) -> DomResult<()> {
        self.0.push("data", value);
        Ok(())
    }

    fn character_data_delete_data(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        offset: u32,
        count: u32,
    ) -> DomResult<()> {
        self.0.push("delete", format!("{offset} {count}"));
        Ok(())
    }
}

struct TestSelectors;

impl Selectors for TestSelectors {}

struct TestWrap(Log, Answers);

impl Wrap for TestWrap {
    fn wrap(&mut self, cx: &mut DomCx<'_, '_>, node: NodeId) -> Completion<Value> {
        let interface = match cx.tree().data(node) {
            NodeData::Document => Interface::Document,
            NodeData::DocumentFragment => Interface::DocumentFragment,
            NodeData::Doctype { .. } => Interface::DocumentType,
            NodeData::Element { local_name, .. } => match &**local_name {
                "pi" => Interface::ProcessingInstruction,
                "nodebrand" => Interface::Node,
                "targetbrand" => Interface::EventTarget,
                "chardata" => Interface::CharacterData,
                _ => Interface::Element,
            },
            NodeData::Text(_) => Interface::Text,
            NodeData::Comment(_) => Interface::Comment,
            NodeData::ProcessingInstruction { .. } => Interface::ProcessingInstruction,
        };
        new_wrapper(cx, interface, node).map(Value::Object)
    }

    fn unwrap(&self, cx: &DomCx<'_, '_>, value: Value) -> Option<NodeId> {
        self.0.unwraps.fetch_add(1, Ordering::SeqCst);
        let node = wrapper_node(cx, value)?;
        match self.1.refuse_unwrap_of {
            Some(name) if local_name(cx.tree(), node).as_deref() == Some(name) => None,
            Some(_) | None => Some(node),
        }
    }
}

fn document(log: Log, answers: Answers) -> impl FnOnce() -> Document + Send {
    move || Document {
        tree: parse_document(PAGE, &ParseOptions::default()),
        limits: ParserLimits::default(),
        slices: Slices {
            reading: Box::new(TestReading),
            attributes: Box::new(TestAttributes(log.clone())),
            mutation: Box::new(TestMutation(log.clone(), answers)),
            selectors: Box::new(TestSelectors),
            wrap: Box::new(TestWrap(log, answers)),
        },
    }
}

fn with<R: Send>(
    limits: RunnerLimits,
    log: &Log,
    answers: Answers,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(limits, Mode::Test, document(log.clone(), answers), body)
        .expect("a runner with the bindings")
}

fn evaluate(runner: &mut Runner, source: &str) -> ScriptEnd {
    runner.evaluate(&format!("{PRELUDE}{source}"))
}

fn ran(runner: &mut Runner, source: &str) {
    let end = evaluate(runner, source);
    assert_eq!(end, ScriptEnd::Ran, "{end}");
}

fn recorded(runner: &mut Runner) -> Option<Limit> {
    let host: &ScriptHost = runner.agent_and_realm().0.host();
    host.document().expect("a document").limit()
}

const NODES: Answers = Answers {
    create_limits: &[Limit::Nodes],
    refuse_unwrap_of: None,
    text_uncharged: false,
};

/// Red. Q4 (`docs/design/scripts.md`, "parser: nodes"): a write that would make one more node
/// is refused and the script ends `limit: nodes`; decision 9 ends it "even under a `catch`".
/// Without a `catch` the throw of `end_with_limit` leaves the entry first: the engine reads the
/// interrupt only after a host call that returned normally, so the entry ends `Throw`, and
/// `Runner::document_end` clears the record and the interrupt and answers `threw`.
#[test]
fn a_limit_not_caught_ends_the_script_at_the_limit() {
    let log = Log::default();
    let (end, after, record) = with(limits(), &log, NODES, |runner| {
        let end = evaluate(runner, r#"document.createElement("p");"#);
        let record = recorded(runner);
        (end, runner.evaluate("1"), record)
    });
    assert_eq!(after, ScriptEnd::Ran, "the next script");
    assert_eq!(record, None, "the record cleared");
    assert_eq!(end, ScriptEnd::Limit(Reason::Nodes), "{end}");
}

/// Red. The same limit, caught and then left by another throw before the engine's next read
/// of the interrupt: rethrown, a `finally` over another throw, and a new error in the `catch`.
#[test]
fn a_limit_left_by_another_throw_ends_the_script_at_the_limit() {
    let sources = [
        r#"try { document.createElement("p"); } catch (e) { throw e; }"#,
        r#"try { throw 1; } finally { try { document.createElement("p"); } catch (e) {} }"#,
        r#"try { document.createElement("p"); } catch (e) { throw new Error("after"); }"#,
    ];
    let mut ends = Vec::new();
    for source in sources {
        let log = Log::default();
        ends.push(with(limits(), &log, NODES, |runner| {
            evaluate(runner, source)
        }));
    }
    let wrong: Vec<String> = sources
        .iter()
        .zip(&ends)
        .filter(|(_, end)| **end != ScriptEnd::Limit(Reason::Nodes))
        .map(|(source, end)| format!("{source} -> {end}"))
        .collect();
    assert!(wrong.is_empty(), "not `limit: nodes`: {wrong:#?}");
}

/// Held. After a member reported a limit under a `catch`, a call that follows the `catch` does
/// not reach the slice again (the engine reads the interrupt first), so a second limit cannot
/// replace the first in the record.
#[test]
fn no_member_runs_after_a_limit_in_the_same_script() {
    let log = Log::default();
    let answers = Answers {
        create_limits: &[Limit::Nodes, Limit::Depth],
        refuse_unwrap_of: None,
        text_uncharged: false,
    };
    let end = with(limits(), &log, answers, |runner| {
        evaluate(
            runner,
            r#"try { document.createElement("p"); } catch (e) {}
               try { document.createElement("q"); } catch (e) {}"#,
        )
    });
    let creates = log.creates.load(Ordering::SeqCst);
    assert_eq!(
        (end.clone(), creates),
        (ScriptEnd::Limit(Reason::Nodes), 1),
        "{end}, createElement reached {creates} times"
    );
}

/// Held. A limit reported inside a conversion (a `toString` the engine calls for an argument)
/// ends the script at the limit, and the outer member's slice is not called.
#[test]
fn a_limit_inside_a_conversion_ends_the_outer_call() {
    let log = Log::default();
    let end = with(limits(), &log, NODES, |runner| {
        evaluate(
            runner,
            r#"el.setAttribute({ toString: function () {
                 try { document.createElement("p"); } catch (e) {}
                 return "a"; } }, "v");"#,
        )
    });
    assert_eq!(end, ScriptEnd::Limit(Reason::Nodes), "{end}");
    assert_eq!(log.attributes(), Vec::<(String, String)>::new());
}

/// Held. A script after one that recorded a limit and ended otherwise inherits neither the
/// record nor the interrupt.
#[test]
fn the_next_script_inherits_no_limit() {
    let log = Log::default();
    let (first, second, record) = with(limits(), &log, NODES, |runner| {
        let first = evaluate(
            runner,
            r#"try { document.createElement("p"); } catch (e) { throw e; }"#,
        );
        let record = recorded(runner);
        (
            first,
            runner.evaluate("var n = 0; for (var i = 0; i < 10; i++) { n++; }"),
            record,
        )
    });
    assert_ne!(first, ScriptEnd::Ran);
    assert_eq!(record, None);
    assert_eq!(second, ScriptEnd::Ran, "{second}");
}

/// Red (a state this unit's own `Wrap` does not reach). `unwrap_node` throws its `TypeError`
/// from inside `with_dom`, while the bindings are out of the host: `type_error` finds no realm
/// (`realm_of`), answers `Terminated(EngineFault)`, and the agent is faulted for good. The
/// message "an object that is no node of this document" can never reach a script.
#[test]
fn a_wrapper_the_hook_does_not_know_is_a_type_error() {
    let log = Log::default();
    let answers = Answers {
        create_limits: &[],
        refuse_unwrap_of: Some("pi"),
        text_uncharged: false,
    };
    let (end, next) = with(limits(), &log, answers, |runner| {
        let end = evaluate(
            runner,
            r#"var caught = null;
               try { pi.hasChildNodes(); } catch (e) { caught = e; }
               check(caught instanceof TypeError, "a TypeError: " + caught);"#,
        );
        (end, runner.evaluate("1"))
    });
    assert_eq!(
        (end.clone(), next.clone()),
        (ScriptEnd::Ran, ScriptEnd::Ran),
        "{end}; {next}"
    );
}

/// Red. Web IDL 3.7.9 (vendored `webidl.html`, "If definition has an indexed property
/// getter, then: Perform DefineMethodProperty(target, %Symbol.iterator%,
/// %Array.prototype.values%, false)"): `HTMLCollection` has `getter Element? item(unsigned
/// long index)`, so its prototype has `@@iterator`. The generated spec says `iterable: false`.
#[test]
fn html_collection_has_the_array_iterator() {
    let log = Log::default();
    with(limits(), &log, Answers::default(), |runner| {
        ran(
            runner,
            r#"var d = desc(HTMLCollection.prototype, Symbol.iterator);
               check(d !== undefined, "HTMLCollection.prototype[@@iterator] is missing");
               check(d.value === Array.prototype.values && d.writable && !d.enumerable
                 && d.configurable, "its attributes");"#,
        );
    });
}

/// Red. Web IDL's attribute setter (3.7.6, vendored `webidl.html`: "Let V be undefined. If any
/// arguments were passed, then set V to the value of the first argument passed.") throws no
/// `TypeError` for a call without an argument; the generated setters call `required(.., 1,
/// ..)`. `Element.id`'s setter called with none converts `undefined` to "undefined".
#[test]
fn an_attribute_setter_without_an_argument_converts_undefined() {
    let log = Log::default();
    with(limits(), &log, Answers::default(), |runner| {
        ran(runner, r#"desc(Element.prototype, "id").set.call(el);"#);
    });
    assert_eq!(
        log.attributes(),
        [("id".to_string(), "undefined".to_string())]
    );
}

const INTERFACES: &str = r#"["EventTarget", "Node", "Element", "Document", "CharacterData",
  "Text", "Comment", "ProcessingInstruction", "DocumentType", "DocumentFragment", "NodeList",
  "HTMLCollection", "DOMException", "Window"]"#;

/// Held. Every generated getter, setter and operation of every prototype, and the members of
/// the global, called with a `this` that does not implement its interface (primitives, plain
/// objects, functions, objects made by script on the right prototype, prototype objects, a
/// `DOMException`, the global, wrappers of sibling and parent brands, a `Proxy` of a wrapper)
/// in four ways (`call`, `Function.prototype.apply.call`, `Function.prototype.call.call`, a bound
/// function; swell 0.8.0 has no `Reflect`)
/// throws the brand check's `TypeError`, and the wrapper hook's `unwrap` is never asked.
/// Every interface object called without `new` throws a `TypeError`.
#[test]
fn every_generated_function_checks_the_brand_of_this() {
    let log = Log::default();
    let setup = format!(
        r#"
var interfaces = {INTERFACES};
var nodeBrand = document.getElementById("nodebrand");
var targetBrand = document.getElementById("targetbrand");
var charBrand = document.getElementById("chardata");
var branded = [G, el, pi, document, nodeBrand, targetBrand, charBrand, new DOMException("x")];
var others = [undefined, null, 0, "s", true, Symbol("s"), {{}}, function () {{}},
  Object.create(Node.prototype), Object.create(Element.prototype),
  Object.create(Window.prototype), Node.prototype, Element.prototype, Window.prototype,
  DOMException.prototype];
if (typeof Proxy === "function") {{ others.push(new Proxy(el, {{}})); others.push(new Proxy(G, {{}})); }}
var arrayMethods = [Array.prototype.values, Array.prototype.keys, Array.prototype.entries,
  Array.prototype.forEach];
G.wrong = [];
G.cases = 0;
G.brandCase = function (f, t, label) {{
  var ways = [
    function () {{ return f.call(t, "a", "b", "c"); }},
    function () {{ return Function.prototype.apply.call(f, t, ["a", "b", "c"]); }},
    function () {{ return Function.prototype.call.call(f, t, "a", "b", "c"); }},
    function () {{ return f.bind(t)("a", "b", "c"); }}
  ];
  for (var w = 0; w < ways.length; w++) {{
    G.cases++;
    try {{ ways[w](); G.wrong.push(label + " #" + w + ": no throw"); }}
    catch (e) {{
      if (!(e instanceof TypeError) || String(e.message).indexOf("does not implement") < 0) {{
        G.wrong.push(label + " #" + w + ": " + (e && e.name) + ": " + (e && e.message));
      }}
    }}
  }}
}};
G.badThis = function (name) {{
  var I = G[name];
  var out = [];
  for (var k = 0; k < branded.length; k++) {{
    if (!(branded[k] instanceof I)) {{ out.push(branded[k]); }}
  }}
  for (var k = 0; k < others.length; k++) {{
    if (name === "Window" && (others[k] === undefined || others[k] === null)) {{ continue; }}
    out.push(others[k]);
  }}
  return out;
}};
G.brandInterface = function (name) {{
  var I = G[name];
  var holder = name === "Window" ? G : I.prototype;
  var keys = name === "Window" ? ["window", "self", "document", "event"]
    : Object.getOwnPropertyNames(holder);
  var bad = G.badThis(name);
  for (var k = 0; k < keys.length; k++) {{
    if (keys[k] === "constructor") {{ continue; }}
    var d = desc(holder, keys[k]);
    var fs = [];
    if (d.get) {{ fs.push(["get", d.get]); }}
    if (d.set) {{ fs.push(["set", d.set]); }}
    if (typeof d.value === "function" && arrayMethods.indexOf(d.value) < 0) {{
      fs.push(["call", d.value]);
    }}
    for (var j = 0; j < fs.length; j++) {{
      for (var b = 0; b < bad.length; b++) {{
        G.brandCase(fs[j][1], bad[b], name + "." + keys[k] + " " + fs[j][0] + " this #" + b);
      }}
    }}
  }}
  for (var b = 0; b < bad.length; b++) {{
    G.cases++;
    try {{ I.call(bad[b]); G.wrong.push(name + "() this #" + b + ": no throw"); }}
    catch (e) {{ if (!(e instanceof TypeError)) {{ G.wrong.push(name + "(): " + e); }} }}
  }}
}};
"#
    );
    let counter = log.unwraps.clone();
    let (ends, wrong, cases, unwraps) = with(limits(), &log, Answers::default(), |runner| {
        let mut ends = vec![evaluate(runner, &setup)];
        // The setup's own `getElementById` calls unwrap `document`; the matrix starts here.
        let before = counter.load(Ordering::SeqCst);
        for name in [
            "EventTarget",
            "Node",
            "Element",
            "Document",
            "CharacterData",
            "Text",
            "Comment",
            "ProcessingInstruction",
            "DocumentType",
            "DocumentFragment",
            "NodeList",
            "HTMLCollection",
            "DOMException",
            "Window",
        ] {
            ends.push(runner.evaluate(&format!("brandInterface({name:?});")));
        }
        let unwraps = counter.load(Ordering::SeqCst) - before;
        let mut wrong = String::new();
        let mut cases = String::new();
        ends.push(runner.evaluate("G.wrong.length === 0 || (function () { throw new Error(G.wrong.slice(0, 20).join('\\n')); })();"));
        if let ScriptEnd::Threw(error) = ends.last().expect("an end") {
            wrong = error.to_string();
        }
        ends.push(
            runner.evaluate("if (G.cases < 10000) { throw new Error('cases: ' + G.cases); }"),
        );
        if let ScriptEnd::Threw(error) = ends.last().expect("an end") {
            cases = error.to_string();
        }
        eprintln!(
            "{}",
            runner.evaluate("throw new Error('brand cases: ' + G.cases);")
        );
        (ends, wrong, cases, unwraps)
    });
    assert!(wrong.is_empty(), "{wrong}");
    assert!(cases.is_empty(), "{cases}");
    assert!(ends.iter().all(|end| *end == ScriptEnd::Ran), "{ends:?}");
    assert_eq!(unwraps, 0, "unwrap asked by the matrix");
}

/// The member rows of `selection.txt`, as `Interface.member`.
fn rows() -> Vec<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../webidl/selection.txt");
    let text = std::fs::read_to_string(path).expect("read selection.txt");
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|line| !line.starts_with("definitions:") && !line.starts_with("global:"))
        .map(|line| {
            let mut words = line.split(' ');
            let interface = words.next().unwrap_or_default();
            let member = words.next().unwrap_or_default();
            format!("{interface}.{member}")
        })
        .collect()
}

/// Held. No prototype or interface object carries a property the selection does not give its
/// interface: the own keys of each prototype are `constructor`, the iteration methods of an
/// iterable and its rows; those of each interface object are `length`, `name`, `prototype`
/// and its constants.
#[test]
fn no_member_outside_the_selection() {
    let rows = rows();
    let list: Vec<String> = rows.iter().map(|row| format!("{row:?}: true")).collect();
    let source = format!(
        r#"
var rows = {{ {rows} }};
var interfaces = {INTERFACES};
var extra = [];
for (var i = 0; i < interfaces.length; i++) {{
  var name = interfaces[i];
  if (name === "Window") {{ continue; }}
  var keys = Object.getOwnPropertyNames(G[name].prototype);
  for (var k = 0; k < keys.length; k++) {{
    var key = keys[k];
    if (key === "constructor") {{ continue; }}
    if (rows[name + ".iterable"] && ["entries", "keys", "values", "forEach"].indexOf(key) >= 0) {{
      continue;
    }}
    if (!rows[name + "." + key]) {{ extra.push(name + ".prototype." + key); }}
  }}
  keys = Object.getOwnPropertyNames(G[name]);
  for (var k = 0; k < keys.length; k++) {{
    var key = keys[k];
    if (key === "length" || key === "name" || key === "prototype") {{ continue; }}
    if (!rows[name + "." + key]) {{ extra.push(name + "." + key); }}
  }}
}}
check(extra.length === 0, "outside the selection: " + extra.join(", "));
"#,
        rows = list.join(", ")
    );
    let log = Log::default();
    with(limits(), &log, Answers::default(), |runner| {
        ran(runner, &source)
    });
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

/// Held. Conversions that run script: left to right and all before the slice (a dictionary's
/// members in lexicographic order, after the argument before it), with no uncharged copy alive
/// while any of them runs; a call of the same member from inside a conversion (reentry) is
/// served first and whole; replacing the prototype's members and the outer `this`'s
/// [[Prototype]] from inside a conversion does not change what the outer call hands its slice;
/// a throw inside a conversion reaches the script and the slice is not called.
#[test]
fn conversions_run_in_order_before_any_copy() {
    let log = Log::default();
    with(limits(), &log, Answers::default(), |runner| {
        define_uncharged(runner);
        ran(
            runner,
            r#"
var order = [];
function v(name) {
  return { toString: function () { order.push(name + ":" + uncharged()); return name; } };
}
el.setAttribute(v("n1"), v("v1"));
try {
  document.createElement(v("p"), {
    get is() { order.push("is:" + uncharged()); return v("x"); },
    get customElementRegistry() { order.push("cer:" + uncharged()); return undefined; }
  });
} catch (e) { check(e.name === "NotSupportedError", "createElement: " + e); }
check(order.join(",") === "n1:0,v1:0,p:0,cer:0,is:0,x:0", "order: " + order.join(","));
el.setAttribute({ toString: function () {
  el.setAttribute("inner", { toString: function () { return "i" + uncharged(); } });
  delete Element.prototype.setAttribute;
  Object.setPrototypeOf(el, null);
  return "outer";
} }, "o");
var threw = null;
try { Function.prototype.apply.call(Element.prototype.toggleAttribute, el,
  [{ toString: function () { throw new RangeError("from toString"); } }]); }
catch (e) { threw = e; }
check(threw instanceof RangeError, "the conversion's throw: " + threw);
"#,
        );
    });
    assert_eq!(
        log.attributes(),
        [
            ("n1".to_string(), "v1".to_string()),
            (
                "create options".to_string(),
                "Dictionary(ElementCreationOptions { is: Some(\"x\") })".to_string()
            ),
            ("inner".to_string(), "i0".to_string()),
            ("outer".to_string(), "o".to_string()),
        ]
    );
}

/// Held. A conversion that loops ends the script at the step budget and the slice is not
/// called.
#[test]
fn a_conversion_at_the_step_budget_ends_the_script() {
    let log = Log::default();
    let mut bounded = limits();
    bounded.engine.steps_per_entry = 200_000;
    let end = with(bounded, &log, Answers::default(), |runner| {
        evaluate(
            runner,
            "el.setAttribute({ toString: function () { for (;;) {} } }, \"v\");",
        )
    });
    assert_eq!(end, ScriptEnd::Limit(Reason::StepsPerScript), "{end}");
    assert!(log.attributes().is_empty());
}

/// Held. The arguments Web IDL converts by type: too few throws a `TypeError` before any
/// conversion; an extra one is ignored; `undefined` and `null` go through ToString for
/// `DOMString`; an optional `boolean` passed `undefined` is absent; the empty String; a String
/// at the engine's length bound (2^20 here) is handed whole, and one past it is a `RangeError`
/// of the engine that never reaches the slice.
#[test]
fn arguments_convert_by_their_types() {
    let log = Log::default();
    with(limits(), &log, Answers::default(), |runner| {
        ran(
            runner,
            r#"
var converted = false;
var threw = null;
try { el.setAttribute({ toString: function () { converted = true; return "a"; } }); }
catch (e) { threw = e; }
check(threw instanceof TypeError && !converted, "too few: " + threw);
el.setAttribute("a", "b", "extra");
el.setAttribute(undefined, null);
el.toggleAttribute("t", undefined);
el.toggleAttribute("u", 0);
el.setAttribute("", "");
el.setAttribute("long", "x".repeat(1048576));
threw = null;
try { el.setAttribute("past", "x".repeat(1048576) + "y"); } catch (e) { threw = e; }
check(threw instanceof RangeError, "one past the bound: " + threw);
"#,
        );
    });
    let got = log.attributes();
    let short: Vec<(String, String)> = got
        .iter()
        .map(|(name, value)| (name.clone(), value.chars().take(8).collect()))
        .collect();
    assert_eq!(
        short,
        [
            ("a", "b"),
            ("undefined", "null"),
            ("toggle t", "None"),
            ("toggle u", "Some(fal"),
            ("", ""),
            ("long", "xxxxxxxx"),
        ]
        .map(|(name, value)| (name.to_string(), value.to_string()))
    );
    assert_eq!(got[5].1.len(), 1 << 20);
}

/// Held. The bindings read no property a script can replace: after a script replaced `Array`,
/// put a setter on `Array.prototype[0]`, cut `Element.prototype` from its chain, froze
/// `Node.prototype` and deleted the interface objects from the global, a later script still
/// gets a real Array of the realm from `getAttributeNames` (the setter not run), and a new
/// wrapper still has the original prototype and passes the brand check.
#[test]
fn a_script_cannot_change_what_the_bindings_read() {
    let log = Log::default();
    with(limits(), &log, Answers::default(), |runner| {
        ran(
            runner,
            r#"
G.saved = { Array: Array, Element: Element, proto: Element.prototype,
  isArray: Array.isArray, hits: 0 };
Object.defineProperty(Array.prototype, "0", { set: function () { G.saved.hits++; },
  configurable: true });
G.Array = function () { G.saved.hits++; return {}; };
Object.setPrototypeOf(Element.prototype, null);
Object.freeze(Node.prototype);
delete G.Element;
delete G.Node;
"#,
        );
        ran(
            runner,
            r#"
var names = el.getAttributeNames();
check(G.saved.isArray(names) && names.length === 2 && desc(names, "0").value === "x"
  && desc(names, "1").value === "y", "the names: " + names);
check(G.saved.hits === 0, "a replaced property ran: " + G.saved.hits);
var fresh = document.getElementById("pi");
var p = document.getElementById("p");
check(Object.getPrototypeOf(p) === G.saved.proto, "a new wrapper's prototype");
var threw = null;
try { G.saved.proto.getAttribute.call(fresh, "a"); } catch (e) { threw = e; }
check(threw instanceof TypeError, "the brand check of a ProcessingInstruction: " + threw);
"#,
        );
    });
}

/// Held. Decision 12's count holds the copies while the slice runs: `setAttribute`'s method
/// reads 2 (its two Strings), and 0 is read again once the member returned.
#[test]
fn the_copy_count_holds_the_copies_while_the_slice_runs() {
    let log = Log::default();
    with(limits(), &log, Answers::default(), |runner| {
        define_uncharged(runner);
        ran(
            runner,
            r#"el.setAttribute("a", "b"); check(uncharged() === 0, "after the member");"#,
        );
    });
    assert_eq!(*log.copies_seen.lock().expect("the log"), [2]);
}

/// Held. What the unit's own tests leave free, against Web IDL (3.7): `@@toStringTag` on each
/// prototype `{ writable: false, enumerable: false, configurable: true }`; `@@unscopables` of
/// the DOM's `[Unscopable]` members on an object with a null prototype; the `name` of getters
/// and setters (`get x`, `set x`) and of operations, a setter's `length` 1 and a getter's 0;
/// the `length` of operations by their required arguments (DOM Standard IDL); each interface
/// object's `name` and `length`; a member of the global called with `this` `undefined`
/// answers for the global.
#[test]
fn generated_property_details_follow_web_idl() {
    let log = Log::default();
    with(limits(), &log, Answers::default(), |runner| {
        ran(
            runner,
            &format!(
                r#"
var interfaces = {INTERFACES};
for (var i = 0; i < interfaces.length; i++) {{
  var name = interfaces[i];
  var I = G[name];
  check(I.name === name, name + ".name");
  check(I.length === (name === "ProcessingInstruction" ? 1 : 0), name + ".length " + I.length);
  var t = desc(I.prototype, Symbol.toStringTag);
  check(t.value === name && !t.writable && !t.enumerable && t.configurable, name + " @@toStringTag");
  if (name === "Window") {{ continue; }}
  var keys = Object.getOwnPropertyNames(I.prototype);
  for (var k = 0; k < keys.length; k++) {{
    var d = desc(I.prototype, keys[k]);
    if (d.get) {{
      check(d.get.name === "get " + keys[k] && d.get.length === 0, name + "." + keys[k] + " getter");
    }}
    if (d.set) {{
      check(d.set.name === "set " + keys[k] && d.set.length === 1, name + "." + keys[k] + " setter");
    }}
    if (typeof d.value === "function" && keys[k] !== "constructor"
        && ["entries", "keys", "values", "forEach"].indexOf(keys[k]) < 0) {{
      check(d.value.name === keys[k], name + "." + keys[k] + " name " + d.value.name);
    }}
  }}
}}
var lengths = {{ "Element.setAttribute": 2, "Element.setAttributeNS": 3,
  "CharacterData.replaceData": 3, "Node.insertBefore": 2, "Document.createElement": 1,
  "Node.cloneNode": 0, "EventTarget.addEventListener": 2, "Element.toggleAttribute": 1,
  "NodeList.item": 1, "Node.getRootNode": 0, "Document.importNode": 1,
  "Document.createElementNS": 2, "Element.prepend": 0, "Node.replaceChild": 2,
  "Document.createProcessingInstruction": 2, "Element.insertAdjacentText": 2 }};
for (var label in lengths) {{
  var parts = label.split(".");
  check(G[parts[0]].prototype[parts[1]].length === lengths[label], label + " length");
}}
var lists = {{ Element: ["slot", "prepend", "append", "replaceChildren", "before", "after",
  "replaceWith", "remove"], Document: ["prepend", "append", "replaceChildren"],
  DocumentFragment: ["prepend", "append", "replaceChildren"],
  CharacterData: ["before", "after", "replaceWith", "remove"],
  DocumentType: ["before", "after", "replaceWith", "remove"] }};
for (var name in lists) {{
  var u = desc(G[name].prototype, Symbol.unscopables);
  check(u !== undefined && !u.writable && !u.enumerable && u.configurable, name + " @@unscopables");
  check(Object.getPrototypeOf(u.value) === null, name + " @@unscopables prototype");
  var own = Object.getOwnPropertyNames(u.value).sort().join(",");
  check(own === lists[name].slice().sort().join(","), name + " @@unscopables: " + own);
  for (var k = 0; k < lists[name].length; k++) {{
    var e = desc(u.value, lists[name][k]);
    check(e.value === true && e.writable && e.enumerable && e.configurable, name + " entry");
  }}
}}
check(desc(Node.prototype, Symbol.unscopables) === undefined, "Node has no @@unscopables");
var it = desc(NodeList.prototype, Symbol.iterator);
check(it.value === Array.prototype.values && it.writable && !it.enumerable && it.configurable,
  "NodeList @@iterator");
for (var i = 0; i < interfaces.length; i++) {{
  var I = G[interfaces[i]];
  var c = desc(I.prototype, "constructor");
  check(c.value === I && c.writable && !c.enumerable && c.configurable, interfaces[i] + " constructor");
  var p = desc(I, "prototype");
  check(p.value === I.prototype && !p.writable && !p.enumerable && !p.configurable,
    interfaces[i] + ".prototype");
}}
check(desc(G, "document").get.call(undefined) instanceof Document, "document with this undefined");
check(desc(G, "window").get.call(null) === G, "window with this null");
"#
            ),
        );
    });
}

/// Held. Decision 13 keeps a node's charge until the agent is dropped: measured after a
/// collection, `createTextNode("abcd")` with the charge (and nothing else) grows the heap by at
/// least the 416 bytes of the charge more than the same call without it.
#[test]
fn the_node_charge_is_kept_across_a_collection() {
    let grows = |text_uncharged: bool| {
        let log = Log::default();
        let answers = Answers {
            text_uncharged,
            ..Answers::default()
        };
        with(limits(), &log, answers, |runner| {
            ran(runner, "");
            let collect = |runner: &mut Runner| {
                runner.agent_and_realm().0.collect().expect("collect");
                runner.agent_and_realm().0.heap_bytes().expect("heap")
            };
            let before = collect(runner);
            ran(runner, r#"document.createTextNode("abcd");"#);
            collect(runner).saturating_sub(before)
        })
    };
    let (charged, uncharged) = (grows(false), grows(true));
    assert!(
        charged >= uncharged + 416,
        "after a collection the charge adds {charged} - {uncharged} bytes"
    );
}

/// Held. Conversions the unit's own tests leave free: `[LegacyNullToEmptyString]` (`null` is
/// the empty string, `undefined` "undefined"); `unsigned long` modulo 2^32 with NaN as 0;
/// ToBoolean of a String by its length; a `(DOMString or ElementCreationOptions)` union takes
/// the dictionary for `null` and the String for a String.
#[test]
fn typed_conversions_the_unit_leaves_free() {
    let log = Log::default();
    with(limits(), &log, Answers::default(), |runner| {
        ran(
            runner,
            r#"
var c = document.getElementById("chardata");
c.data = null;
c.data = undefined;
c.deleteData(-1, 4294967297);
c.deleteData(1.9, NaN);
el.toggleAttribute("e", "");
el.toggleAttribute("f", "0");
try { document.createElement("p", null); } catch (e) {}
try { document.createElement("p", "x"); } catch (e) {}
"#,
        );
    });
    let expected = [
        ("data", ""),
        ("data", "undefined"),
        ("delete", "4294967295 1"),
        ("delete", "1 0"),
        ("toggle e", "Some(false)"),
        ("toggle f", "Some(true)"),
        (
            "create options",
            "Dictionary(ElementCreationOptions { is: None })",
        ),
        ("create options", "String(\"x\")"),
    ]
    .map(|(name, value)| (name.to_string(), value.to_string()));
    assert_eq!(log.attributes(), expected);
}

/// Held. A `DOMException` has an [[ErrorData]] slot (`HostObjectSpec::error_kind`), so the
/// runner reads one a script throws and does not catch as an error, not as a plain value.
#[test]
fn an_uncaught_dom_exception_is_an_error() {
    let log = Log::default();
    let end = with(limits(), &log, Answers::default(), |runner| {
        runner.evaluate(r#"throw new DOMException("m", "SyntaxError");"#)
    });
    match &end {
        ScriptEnd::Threw(error) => assert_eq!(error.kind, ScriptErrorKind::Error, "{end}"),
        other => panic!("not a throw: {other}"),
    }
}
