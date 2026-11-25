//! Second review of `story:webidl-bindings`, on its fix round 1: the limit as the script's end
//! (decision 9 as ruled), `@@iterator` (Web IDL 3.7.9), the record of an uncaught
//! `DOMException` (Q16 as ruled), the setter without an argument (Web IDL 3.7.6), the
//! `TypeError` of a wrapper the hook does not know, and the copies a `DOMException` keeps.
//!
//! The test slices record what reaches them; `createElement` answers a limit, after spending
//! the memory budget or the step budget first when a case asks for it.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use surf_html::limits::{Limit, Limits as ParserLimits};
use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_script::bindings::{
    Attributes, DomCx, DomEnd, DomResult, Interface, Mutation, Reading, Selectors, Slices,
    StringOrElementCreationOptions, TheWindow, Wrap, new_wrapper, not_supported, wrapper_node,
};
use surf_script::{
    Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, ScriptErrorKind, ScriptHost,
    run_document,
};
use swell::{Completion, Limits, Value};

const PAGE: &str = "<!DOCTYPE html><html><head></head><body><p>a</p><pi></pi>\
     <chardata></chardata><!--c--></body></html>";

const PRELUDE: &str = r#"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
function desc(object, key) { return Object.getOwnPropertyDescriptor(object, key); }
var document = G.document;
var el = document.getElementById("p");
var pi = document.getElementById("pi");
var cd = document.getElementById("chardata");
"#;

fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 10_000_000,
            memory_bytes: 64 * 1024 * 1024,
            frame_depth: 200,
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
    creates: Arc<AtomicUsize>,
    writes: Arc<Mutex<Vec<(String, String)>>>,
}

impl Log {
    fn writes(&self) -> Vec<(String, String)> {
        self.writes.lock().expect("the log").clone()
    }

    fn push(&self, name: &str, value: String) {
        self.writes
            .lock()
            .expect("the log")
            .push((name.to_string(), value));
    }
}

/// What `createElement` does before it answers its limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum First {
    /// Nothing.
    Nothing,
    /// A node charge over the memory budget: the run ends `Terminated(MemoryBudget)`.
    Memory,
    /// Steps over the step budget: the run ends `Terminated(StepBudget)`.
    Steps,
}

/// How the test slices answer.
#[derive(Clone, Copy)]
struct Answers {
    /// The limit `createElement` answers, `None` for the trait's default.
    create: Option<Limit>,
    first: First,
    /// `unwrap` answers `None` for the element of this local name.
    refuse_unwrap_of: Option<&'static str>,
}

const NODES: Answers = Answers {
    create: Some(Limit::Nodes),
    first: First::Nothing,
    refuse_unwrap_of: None,
};

const PLAIN: Answers = Answers {
    create: None,
    first: First::Nothing,
    refuse_unwrap_of: None,
};

fn local_name(tree: &Tree, node: NodeId) -> Option<String> {
    match tree.data(node) {
        NodeData::Element { local_name, .. } => Some(local_name.to_string()),
        _ => None,
    }
}

fn elements(tree: &Tree) -> Vec<NodeId> {
    tree.descendants(tree.root())
        .into_iter()
        .filter(|&node| matches!(tree.data(node), NodeData::Element { .. }))
        .collect()
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
        let tree = cx.tree();
        Ok(elements(tree)
            .into_iter()
            .find(|&node| local_name(tree, node).as_deref() == Some(element_id.as_str())))
    }

    fn node_child_nodes(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<Vec<NodeId>> {
        Ok(cx.tree().children(this).to_vec())
    }

    fn document_get_elements_by_tag_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        _qualified_name: String,
    ) -> DomResult<Vec<NodeId>> {
        Ok(elements(cx.tree()))
    }

    /// The length of the snapshot, which `story:scripts-dom-reading` gives the real slice.
    fn node_list_length(&mut self, _cx: &mut DomCx<'_, '_>, this: &[NodeId]) -> DomResult<u32> {
        Ok(u32::try_from(this.len()).unwrap_or(u32::MAX))
    }

    fn html_collection_length(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        this: &[NodeId],
    ) -> DomResult<u32> {
        Ok(u32::try_from(this.len()).unwrap_or(u32::MAX))
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
}

struct TestMutation(Log, Answers);

impl Mutation for TestMutation {
    fn document_create_element(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        local_name: String,
        _options: StringOrElementCreationOptions,
    ) -> DomResult<NodeId> {
        self.0.creates.fetch_add(1, Ordering::SeqCst);
        let Some(limit) = self.1.create else {
            self.0.push("create", local_name);
            return Err(not_supported("Document.createElement"));
        };
        match self.1.first {
            First::Nothing => {}
            // Both answers are dropped: the slice goes on to its limit as a slice that checks
            // its own count after the charge would.
            First::Memory => {
                let _ = cx.charge_node(usize::MAX / 8);
            }
            First::Steps => {
                let _ = cx.engine().charge_steps(u64::MAX);
            }
        }
        Err(DomEnd::Limit(limit))
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

struct TestWrap(Answers);

impl Wrap for TestWrap {
    fn wrap(&mut self, cx: &mut DomCx<'_, '_>, node: NodeId) -> Completion<Value> {
        let interface = match cx.tree().data(node) {
            NodeData::Document => Interface::Document,
            NodeData::DocumentFragment => Interface::DocumentFragment,
            NodeData::Doctype { .. } => Interface::DocumentType,
            NodeData::Element { local_name, .. } => match &**local_name {
                "pi" => Interface::ProcessingInstruction,
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
        let node = wrapper_node(cx, value)?;
        match self.0.refuse_unwrap_of {
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
            mutation: Box::new(TestMutation(log, answers)),
            selectors: Box::new(TestSelectors),
            wrap: Box::new(TestWrap(answers)),
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

/// Whether the interrupt flag is set between two scripts.
fn flag(runner: &mut Runner) -> bool {
    runner.agent_and_realm().0.interrupt_handle().is_set()
}

/// `hit()` reaches the limit inside a `try` and, would it go on, writes after it; each source
/// calls it from a place the engine or a member enters, and writes again in the outer code.
const HIT: &str = r#"
function hit(where) {
  try { document.createElement("p"); } catch (e) { el.setAttribute("catch", where); }
  finally { el.id = where; }
  el.setAttribute("after", where);
}
"#;

const NESTED: &[(&str, &str)] = &[
    (
        "toString of an argument",
        r#"el.setAttribute({ toString: function () { hit("toString"); return "n"; } }, "v");"#,
    ),
    (
        "valueOf of an unsigned long",
        r#"cd.deleteData({ valueOf: function () { hit("valueOf"); return 0; } }, 1);"#,
    ),
    (
        "Symbol.toPrimitive of a setter's value",
        r#"el.id = { [Symbol.toPrimitive]: function () { hit("toPrimitive"); return "i"; } };"#,
    ),
    (
        "a getter the engine calls",
        r#"var o = { get x() { hit("getter"); return 1; } }; var y = o.x;
           el.setAttribute("outer", "getter");"#,
    ),
    (
        "a comparator of sort",
        r#"[2, 1, 3].sort(function (a, b) { hit("sort"); return a - b; });
           el.setAttribute("outer", "sort");"#,
    ),
    (
        "a replacer of JSON.stringify",
        r#"JSON.stringify({ a: 1 }, function (k, v) { hit("stringify"); return v; });
           el.setAttribute("outer", "stringify");"#,
    ),
    (
        "a callback of forEach",
        r#"[1, 2].forEach(function () { hit("forEach"); }); el.setAttribute("outer", "forEach");"#,
    ),
    (
        "a generator's next",
        r#"function* g() { hit("generator"); yield 1; } g().next();
           el.setAttribute("outer", "generator");"#,
    ),
    (
        "a tagged template",
        r#"function tag(s) { hit("tag"); return s; } tag`x`; el.setAttribute("outer", "tag");"#,
    ),
    (
        "a callback of replace",
        r#""ab".replace(/a/, function () { hit("replace"); return "c"; });
           el.setAttribute("outer", "replace");"#,
    ),
    (
        "a constructor called with new",
        r#"function F() { hit("new"); } new F(); el.setAttribute("outer", "new");"#,
    ),
    (
        "Function.prototype.call",
        r#"hit.call(null, "call"); el.setAttribute("outer", "call");"#,
    ),
    (
        "a catch and a finally of an outer frame",
        r#"try { [1].map(function () { hit("map"); }); }
           catch (e) { el.setAttribute("outer", "catch"); }
           finally { el.setAttribute("outer", "finally"); }"#,
    ),
    (
        "the DOMException constructor's conversion",
        r#"new DOMException({ toString: function () { hit("exception"); return "m"; } });
           el.setAttribute("outer", "exception");"#,
    ),
    (
        "createElement's own conversion",
        r#"document.createElement({ toString: function () { hit("create"); return "q"; } });"#,
    ),
    (
        "after a RangeError of frame_depth caught",
        r#"function deep() { deep(); }
           try { deep(); } catch (e) { check(e instanceof RangeError, "depth: " + e); hit("depth"); }
           el.setAttribute("outer", "depth");"#,
    ),
];

/// Held. A limit reached in a nested entry (a conversion of an outer member's argument, a
/// getter, a callback of a builtin, a generator, a tagged template, a constructor) ends the
/// script there: no `catch`, `finally` or later code of any frame runs, the outer member does
/// not go on to its slice, `createElement` is reached once, and the end is `limit: nodes`.
#[test]
fn a_limit_in_a_nested_entry_ends_the_script_there() {
    let mut wrong = Vec::new();
    for (what, source) in NESTED {
        let log = Log::default();
        let (end, record, set) = with(limits(), &log, NODES, |runner| {
            let end = evaluate(runner, &format!("{HIT}{source}"));
            (end, recorded(runner), flag(runner))
        });
        let creates = log.creates.load(Ordering::SeqCst);
        let writes = log.writes();
        if end != ScriptEnd::Limit(Reason::Nodes)
            || creates != 1
            || !writes.is_empty()
            || record.is_some()
            || set
        {
            wrong.push(format!(
                "{what}: {end}, createElement {creates}, writes {writes:?}, record {record:?}, \
                 flag {set}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Held; tells the runner's clear of the interrupt in `Runner::document_end` from its absence
/// (R21 of `impl/report-3.txt`, which the implementor found no test for). A slice that ends the
/// run itself (a node charge over the memory budget, or steps over the step budget) and then
/// answers a limit: `end_with_limit` sets the interrupt, and its `charge_steps(0)` answers the
/// termination already set without reading the flag. The flag stays set until the runner
/// withdraws it. The ends: `limit: memory` keeps its word, the step budget gives way to the
/// recorded limit (the ruling); the record is cleared, the flag is clear, and the next script
/// runs and spends steps.
#[test]
fn a_limit_after_swells_own_end_leaves_no_flag_set() {
    let cases = [
        (First::Memory, ScriptEnd::Limit(Reason::Memory)),
        (First::Steps, ScriptEnd::Limit(Reason::Nodes)),
    ];
    for (first, expected) in cases {
        let log = Log::default();
        let answers = Answers { first, ..NODES };
        let (end, record, set, next, steps) = with(limits(), &log, answers, |runner| {
            let end = evaluate(
                runner,
                r#"try { document.createElement("p"); } catch (e) {}"#,
            );
            let record = recorded(runner);
            let set = flag(runner);
            let before = runner.steps();
            let next = runner.evaluate("var i = 0; while (i < 100) { i++; }");
            (end, record, set, next, runner.steps() - before)
        });
        assert_eq!(end, expected, "{first:?}: {end}");
        assert_eq!(record, None, "{first:?}: the record");
        assert!(!set, "{first:?}: the interrupt flag after the entry");
        assert_eq!(next, ScriptEnd::Ran, "{first:?}: the next script: {next}");
        assert!(
            steps > 100,
            "{first:?}: the next script spent {steps} steps"
        );
    }
}

/// Held. Web IDL 3.7.9: `@@iterator` of an interface with an indexed property getter is
/// `%Array.prototype.values%`, the function object of the realm, writable, not enumerable,
/// configurable; with `iterable<T>`, `values` is the same object. A script that replaced
/// `Array.prototype.values` and `Array.prototype[Symbol.iterator]` in an earlier script does
/// not change it; and no other interface has the property.
#[test]
fn the_iterator_is_the_realms_array_values() {
    let log = Log::default();
    with(limits(), &log, PLAIN, |runner| {
        ran(
            runner,
            r#"G.values = Array.prototype.values;
               Array.prototype.values = function () {};
               Array.prototype[Symbol.iterator] = function () {};"#,
        );
        ran(
            runner,
            r#"var names = ["EventTarget", "Node", "Element", "Document", "CharacterData",
                 "Text", "Comment", "ProcessingInstruction", "DocumentType", "DocumentFragment",
                 "NodeList", "HTMLCollection", "DOMException", "Window"];
               for (var i = 0; i < names.length; i++) {
                 var name = names[i];
                 var has = Object.getOwnPropertySymbols(G[name].prototype)
                   .indexOf(Symbol.iterator) >= 0;
                 check(has === (name === "NodeList" || name === "HTMLCollection"),
                   name + " has @@iterator: " + has);
               }
               check(!Object.getOwnPropertySymbols(G).includes(Symbol.iterator), "the global");
               var lists = [NodeList.prototype, HTMLCollection.prototype];
               for (var j = 0; j < lists.length; j++) {
                 var d = desc(lists[j], Symbol.iterator);
                 check(d.value === G.values, "the realm's %Array.prototype.values%");
                 check(d.writable && !d.enumerable && d.configurable, "the attributes");
               }
               check(NodeList.prototype.values === G.values, "NodeList values");"#,
        );
    });
}

/// Held. `for (x of collection)` and `Array.from` of a `NodeList` and an `HTMLCollection` give
/// its nodes in order, each the wrapper its index property holds, also after a script replaced
/// `Array.prototype[Symbol.iterator]`. The test slice answers `length` (the iterator reads it
/// first); with surf's own slices today `length` answers `NotSupportedError`.
#[test]
fn for_of_a_collection_gives_its_nodes() {
    let log = Log::default();
    with(limits(), &log, PLAIN, |runner| {
        ran(
            runner,
            r#"Array.prototype[Symbol.iterator] = function () { throw new Error("replaced"); };
               var lists = [document.getElementsByTagName("*"), document.childNodes,
                 el.childNodes];
               for (var i = 0; i < lists.length; i++) {
                 var list = lists[i];
                 var seen = [];
                 for (var x of list) { seen.push(x); }
                 check(seen.length === list.length, "for-of " + i + ": " + seen.length +
                   " of " + list.length);
                 for (var k = 0; k < seen.length; k++) {
                   check(seen[k] === list[k], "for-of " + i + " at " + k);
                 }
                 check(Array.from(list).length === list.length, "Array.from " + i);
               }
               check(lists[0].length > 3, "elements: " + lists[0].length);"#,
        );
    });
}

/// Held. Q16 as ruled: the record of an uncaught `DOMException` is `<name>: <message>` from its
/// own data, and no script runs while it is read: getters of `DOMException.prototype` replaced,
/// a subclass with getters of its own, an own `message` accessor. A name with a line break and
/// a lone surrogate (U+FFFD, the copy's deviation) are kept as copied; the site is the throw's.
#[test]
fn the_record_of_a_dom_exception_runs_no_script() {
    let log = Log::default();
    let sources = [
        (
            r#"Object.defineProperty(DOMException.prototype, "message",
                 { get: function () { el.setAttribute("ran", "message"); return "x"; } });
               Object.defineProperty(DOMException.prototype, "name",
                 { get: function () { el.setAttribute("ran", "name"); return "y"; } });
               throw new DOMException("m", "N");"#,
            "N: m",
        ),
        (
            r#"function E() {}
               E.prototype = Object.create(DOMException.prototype);
               Object.defineProperty(E.prototype, "message",
                 { get: function () { el.setAttribute("ran", "sub"); return "x"; } });
               Object.defineProperty(E.prototype, "name",
                 { get: function () { el.setAttribute("ran", "sub"); return "y"; } });
               var sub = new DOMException("m2", "N2");
               Object.setPrototypeOf(sub, E.prototype);
               throw sub;"#,
            "N2: m2",
        ),
        (
            r#"var e = new DOMException("m3", "N3");
               Object.defineProperty(e, "message",
                 { get: function () { el.setAttribute("ran", "own"); return "x"; } });
               throw e;"#,
            "N3: m3",
        ),
        (r#"throw new DOMException("m4", "a\nb");"#, "a\nb: m4"),
        (r#"throw new DOMException("\uD800", "N5");"#, "N5: \u{FFFD}"),
    ];
    let ends = with(limits(), &log, PLAIN, |runner| {
        sources
            .iter()
            .map(|(source, _)| evaluate(runner, source))
            .collect::<Vec<_>>()
    });
    for ((source, message), end) in sources.iter().zip(&ends) {
        match end {
            ScriptEnd::Threw(error) => {
                assert_eq!(error.kind, ScriptErrorKind::Error, "{source}");
                assert_eq!(error.message.as_deref(), Some(*message), "{source}");
                assert!(error.site.is_some(), "the site of {source}");
            }
            other => panic!("{source}: {other}"),
        }
    }
    assert_eq!(log.writes(), Vec::new(), "script ran during the read");
}

/// Held. The read of an uncaught `Error`'s data is an entry of its own (`Runner::error`); the
/// steps the script spent still count in the runner's totals.
#[test]
fn the_steps_of_a_script_that_threw_an_error_count() {
    let log = Log::default();
    let steps = with(limits(), &log, PLAIN, |runner| {
        let before = runner.steps();
        let end = runner
            .evaluate(r#"var i = 0; while (i < 1000) { i++; } throw new DOMException("m", "N");"#);
        assert!(matches!(end, ScriptEnd::Threw(_)), "{end}");
        runner.steps() - before
    });
    assert!(steps > 1000, "the runner counted {steps} steps");
}

/// The resident bytes of this process, from `/proc/self/statm` (Linux).
fn resident_bytes() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").expect("/proc/self/statm");
    let pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .and_then(|field| field.parse().ok())
        .expect("the resident pages");
    pages * 4096
}

/// Red. `docs/design/scripts.md` (the rule under Q4): "no uncharged copy of surf's is alive
/// while script runs", and swell's `HOST_OBJECT_LIVE_BYTES` (`host/limits.rs:80-82 @ 0.8.0`):
/// "What the data owns behind its box is the host's to charge". `new DOMException(message,
/// name)` copies both Strings out of the engine into `ExceptionData` (`prelude.rs:1347-1357`,
/// `:520-523`) and charges neither: 64 exceptions of a 256 KiB message hold 16 MiB outside a
/// memory budget of 4 MiB, and the count of exceptions has no bound but the steps. Either the
/// copies are charged (the script ends `limit: memory`) or there are none (the process does
/// not grow by them).
#[test]
fn a_dom_exception_holds_no_uncharged_copy() {
    let log = Log::default();
    let mut small = limits();
    small.engine.memory_bytes = 4 * 1024 * 1024;
    let (end, grown, seconds) = with(small, &log, PLAIN, |runner| {
        ran(runner, r#"G.s = "x".repeat(1 << 18); G.kept = [];"#);
        let before = resident_bytes();
        let started = Instant::now();
        let end =
            runner.evaluate("for (var i = 0; i < 64; i++) { G.kept.push(new DOMException(G.s)); }");
        let grown = resident_bytes().saturating_sub(before);
        (end, grown, started.elapsed().as_secs_f64())
    });
    assert!(seconds < 30.0, "{seconds} s");
    let mib = grown >> 20;
    assert!(
        end == ScriptEnd::Limit(Reason::Memory) || mib < 4,
        "{end}; the process grew by {mib} MiB for 16 MiB of messages, memory_bytes 4 MiB"
    );
}

/// Red. Web IDL's `DOMException` (its `message` and `name` are `DOMString`): a script reads back
/// the message it gave. The deviation the fix round named (a lone surrogate reaches a slice as
/// U+FFFD, `selection.txt`'s head) is about the slices; the `DOMException` constructor has no
/// slice, and its copy (`prelude.rs:1349`) turns `"\uD800"` into `"�"` too, unnamed.
#[test]
fn a_dom_exception_keeps_its_message() {
    let log = Log::default();
    let end = with(limits(), &log, PLAIN, |runner| {
        evaluate(
            runner,
            r#"var e = new DOMException("a\uD800", "N\uDC00");
               check(e.message === "a\uD800", "message: " + e.message.charCodeAt(1).toString(16));
               check(e.name === "N\uDC00", "name: " + e.name.charCodeAt(1).toString(16));"#,
        )
    });
    assert_eq!(end, ScriptEnd::Ran, "{end}");
}

/// Held. Web IDL 3.7.6: the setter of an attribute converts its first argument, `undefined`
/// when none was passed: `[LegacyNullToEmptyString] DOMString` (`CharacterData.data`) takes
/// `undefined` as `"undefined"` and `null` as `""`; a second argument is ignored; the setter
/// still checks `this`.
#[test]
fn a_setter_converts_undefined_and_checks_this() {
    let log = Log::default();
    with(limits(), &log, PLAIN, |runner| {
        ran(
            runner,
            r#"var data = desc(CharacterData.prototype, "data").set;
               data.call(cd);
               data.call(cd, null);
               data.call(cd, "x", "y");
               var id = desc(Element.prototype, "id").set;
               id.call(el);
               var threw = null;
               try { id.call({}); } catch (e) { threw = e; }
               check(threw instanceof TypeError, "this: " + threw);"#,
        );
    });
    assert_eq!(
        log.writes(),
        [
            ("data".to_string(), "undefined".to_string()),
            ("data".to_string(), String::new()),
            ("data".to_string(), "x".to_string()),
            ("id".to_string(), "undefined".to_string()),
        ]
    );
}

/// Held. The `TypeError` of a wrapper the hook does not know, raised once the bindings are back
/// in the host (should 4 as fixed): from a variadic `(Node or DOMString)...` argument after a
/// String, and from `this`; the script catches each, the next member's `DOMException` is not
/// turned into a `TypeError` by a note left behind, and the agent is not faulted.
#[test]
fn the_type_error_of_an_unknown_wrapper_leaves_no_note() {
    let log = Log::default();
    let answers = Answers {
        refuse_unwrap_of: Some("pi"),
        ..PLAIN
    };
    let ends = with(limits(), &log, answers, |runner| {
        [
            evaluate(
                runner,
                r#"var a = null;
                   try { el.append("s", pi); } catch (e) { a = e; }
                   check(a instanceof TypeError, "append: " + a);
                   var b = null;
                   try { Element.prototype.remove.call(pi); } catch (e) { b = e; }
                   check(b instanceof TypeError, "this: " + b);
                   var c = null;
                   try { el.remove(); } catch (e) { c = e; }
                   check(c instanceof DOMException && c.name === "NotSupportedError",
                     "the next member: " + c);"#,
            ),
            runner.evaluate("1"),
        ]
    });
    assert_eq!(ends, [ScriptEnd::Ran, ScriptEnd::Ran]);
}
