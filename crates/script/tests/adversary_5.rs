//! Third review of `story:webidl-bindings`, on its fix round 2: a `DOMException` holds its
//! `name` and `message` as the engine's Strings, listed by the `trace` of its data (swell's host
//! interface, section 4 and decision 15 @ 0.8.0). Collections between the making and the
//! reading, a subclass, an exception read by a later script, the exceptions the bindings make,
//! the copy `DOMException.code` makes of the name, and the cut of the texts the bindings make
//! at `EXCEPTION_TEXT_BYTES`.
//!
//! swell collects at a safe point once 1 MiB was allocated since the last collection
//! (`MIN_TRIGGER_BYTES`, `crates/swell/src/heap/mod.rs:266-271 @ 0.8.0`): `churn(n)` allocates
//! n MiB or more of garbage, so a script that calls it collects while it runs.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeData, NodeId, ParseOptions, parse_document};
use surf_script::bindings::{
    Attributes, DomCx, DomEnd, DomException, DomResult, EXCEPTION_TEXT_BYTES, Interface, Mutation,
    Reading, Selectors, Slices, TheWindow, Wrap, new_wrapper, wrapper_node,
};
use surf_script::{Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, run_document};
use swell::{Completion, Limits, Outcome, Value};

const PAGE: &str = "<!DOCTYPE html><html><head></head><body><p>a</p></body></html>";

pub(crate) const PRELUDE: &str = r#"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
var document = G.document;
var hoarded = null;
function churn(mib) {
  for (var r = 0; r < mib * 8; r++) {
    var junk = "y".repeat(1 << 17);
    if (hoarded) { hoarded.push(junk); }
  }
}
"#;

/// The memory budget of the cases that collect: [`garbage_past_the_budget`] shows that four
/// `churn(1)` kept alive end the script at it: each makes about 1 MiB, eight Strings of 2^17
/// code units.
const BUDGET: u64 = 4 << 20;

pub(crate) fn limits(memory_bytes: u64, string_length: u64) -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 200_000_000,
            memory_bytes,
            frame_depth: 200,
            reentry_depth: 64,
            parser_nesting: 100,
            string_length,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 2_000_000_000,
        scripts_per_runner: 1_000,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(40),
    }
}

/// `getElementById("throw:<n>")` answers the exception number `n` of [`thrown`].
struct TestReading;

/// The exceptions the slice answers: texts longer than `EXCEPTION_TEXT_BYTES`, made of
/// characters of three and four bytes, so that the cut falls inside one.
fn thrown(n: u32) -> DomException {
    match n {
        // 2 + 4 × 300 bytes; a cut at 1,024 falls in the 256th emoji: 255 kept, 1,022 bytes.
        0 => DomException::new(&format!("ab{}", "\u{1F600}".repeat(300)), "€".repeat(400)),
        // 1 + 3 × 400 bytes; 1,024 is 1 + 3 × 341: the cut falls on a boundary.
        1 => DomException::new(
            &format!("a{}", "€".repeat(400)),
            format!("ab{}", "€".repeat(400)),
        ),
        // A name under the bound, a message of 2,000 bytes of four-byte characters only.
        _ => DomException::new("ShortName", "\u{10348}".repeat(500)),
    }
}

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
        // The fourth pass: the interrupt set by the slice that throws, as a watchdog may set
        // it between the throw and the record (adversary_6).
        if element_id == "interrupt" {
            cx.interrupt().interrupt();
            return Err(DomEnd::Throw(DomException::new("N", "m")));
        }
        match element_id
            .strip_prefix("throw:")
            .and_then(|n| n.parse().ok())
        {
            Some(n) => Err(DomEnd::Throw(thrown(n))),
            None => Ok(None),
        }
    }
}

struct TestAttributes;

impl Attributes for TestAttributes {}

struct TestMutation;

impl Mutation for TestMutation {}

struct TestSelectors;

impl Selectors for TestSelectors {}

struct TestWrap;

impl Wrap for TestWrap {
    fn wrap(&mut self, cx: &mut DomCx<'_, '_>, node: NodeId) -> Completion<Value> {
        let interface = match cx.tree().data(node) {
            NodeData::Document => Interface::Document,
            NodeData::DocumentFragment => Interface::DocumentFragment,
            NodeData::Doctype { .. } => Interface::DocumentType,
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

fn document() -> Document {
    Document {
        tree: parse_document(PAGE, &ParseOptions::default()),
        limits: ParserLimits::default(),
        slices: Slices {
            reading: Box::new(TestReading),
            attributes: Box::new(TestAttributes),
            mutation: Box::new(TestMutation),
            selectors: Box::new(TestSelectors),
            wrap: Box::new(TestWrap),
        },
    }
}

pub(crate) fn with<R: Send>(limits: RunnerLimits, body: impl FnOnce(&mut Runner) -> R + Send) -> R {
    run_document(limits, Mode::Test, document, body).expect("a runner with the bindings")
}

pub(crate) fn evaluate(runner: &mut Runner, source: &str) -> ScriptEnd {
    runner.evaluate(&format!("{PRELUDE}{source}"))
}

pub(crate) fn ran(runner: &mut Runner, source: &str) {
    let end = evaluate(runner, source);
    assert_eq!(end, ScriptEnd::Ran, "{end}");
}

pub(crate) fn collect(runner: &mut Runner) {
    for _ in 0..2 {
        let collected = runner.agent_and_realm().0.collect();
        assert!(matches!(collected, Ok(Outcome::Normal(_))), "{collected:?}");
    }
}

/// What the first script makes: 64 exceptions whose Strings nothing but the exception holds,
/// made by `String.fromCharCode`, by concatenation in a loop (a rope where swell has ropes), by
/// `repeat`, by `slice` of a longer String, by `toString` of a number and by a lone surrogate;
/// each name and message differs, and the script keeps a recipe to make them again.
const MAKE: &str = r#"
function texts(i) {
  var rope = ""; for (var k = 0; k < 40; k++) { rope = rope + String.fromCharCode(0x61 + (k + i) % 26); }
  var kinds = [
    String.fromCharCode(0x4E, 0x61 + i % 26, 0xD800 + i),
    rope,
    "r" + i + "-".repeat(300 + i),
    ("0123456789abcdef".repeat(8) + i).slice(3, 70 + i),
    (i * 1234567.5).toString(),
    "\uDC00" + i + "\uD800"
  ];
  return [kinds[i % kinds.length], kinds[(i + 1) % kinds.length] + "|" + i];
}
G.made = [];
for (var i = 0; i < 64; i++) {
  var t = texts(i);
  G.made.push(new DOMException(t[0], t[1]));
  t = null;
  if (i % 16 === 15) { churn(1); }
}
"#;

/// The second script: every exception reads back the code units it was given.
const READ: &str = r#"
churn(2);
for (var i = 0; i < 64; i++) {
  var t = texts(i), e = G.made[i];
  check(e.message === t[0], "message " + i);
  check(e.name === t[1], "name " + i);
  check(e.code === 0, "code " + i);
  check(e instanceof DOMException && Object.prototype.toString.call(e) === "[object DOMException]", "kind " + i);
}
"#;

/// Held, the ground of the cases that collect: four `churn(1)` kept alive end the script at the
/// memory budget.
#[test]
fn garbage_past_the_budget() {
    let end = with(limits(BUDGET, 1 << 20), |runner| {
        evaluate(runner, "hoarded = []; churn(4);")
    });
    assert_eq!(end, ScriptEnd::Limit(Reason::Memory), "{end}");
}

/// The steps each script of the next test may charge, `MAKE`, `READ` and `READ` again: 1.5
/// times the steps of its first green run
/// (4,415,946, 2,334,536 and 2,334,536). The steps do not depend on the machine; the margin is
/// for a change of the engine.
const STEPS_CEILINGS: [u64; 3] = [6_623_919, 3_501_804, 3_501_804];

/// Held. Swell's host interface @ 0.8.0, section 4: a handle listed in the `trace` of host data
/// the engine reaches is strong. 64 exceptions of Strings no other value holds, collections
/// forced by garbage in the making script, two `Agent::collect` between the scripts and more
/// garbage in the reading one: every code unit read back, a lone surrogate too. The cost of the
/// three scripts is bounded by the steps each charges (`Runner::steps` around it, at most
/// [`STEPS_CEILINGS`]); the collections charge no step and are not bounded here.
#[test]
fn strings_only_exceptions_hold_survive_collections_inside_and_between_scripts() {
    let started = Instant::now();
    let steps = with(limits(BUDGET, 1 << 20), |runner| {
        let mut steps = [0; 3];
        for (script, source) in [MAKE, READ, READ].into_iter().enumerate() {
            if script > 0 {
                collect(runner);
            }
            let before = runner.steps();
            ran(runner, source);
            steps[script] = runner.steps() - before;
        }
        steps
    });
    println!(
        "steps {steps:?}, ceilings {STEPS_CEILINGS:?}, {:?}",
        started.elapsed()
    );
    for ((script, steps), ceiling) in ["MAKE", "READ", "READ again"]
        .into_iter()
        .zip(steps)
        .zip(STEPS_CEILINGS)
    {
        assert!(
            steps <= ceiling,
            "{script} charged {steps} steps, over its ceiling of {ceiling}"
        );
    }
}

/// Held. The `DOMException` constructor converts its message and then its name, and either
/// conversion may run script that allocates; the converted message lives in the call's scope
/// only while the name's `toString` runs (section 4: "valid for the scope it was handed in").
/// The `toString` of the name makes 1 MiB of garbage or more, so the engine collects
/// inside the constructor; the exception still holds the message, made by concatenation in the
/// message's own `toString` and held by nothing else. A subclass of the ES5 form reads it
/// back. (swell has neither `Reflect` nor `class`: a script cannot give the constructor another
/// `new_target`, and `DOMException.prototype` is not writable, so no script runs at that read.)
#[test]
fn a_collection_inside_the_constructor_keeps_the_converted_strings() {
    with(limits(BUDGET, 1 << 20), |runner| {
        ran(
            runner,
            r#"function Sub() {}
               Sub.prototype = Object.create(DOMException.prototype);
               G.list = [];
               for (var i = 0; i < 8; i++) {
                 var message = { toString: function () { return "m" + i + "x".repeat(50 + i); } };
                 var name = { toString: function () { if (i < 4) { churn(1); } return "Name" + String.fromCharCode(0x41 + i); } };
                 var e = new DOMException(message, name);
                 Object.setPrototypeOf(e, Sub.prototype);
                 G.list.push(e);
               }"#,
        );
        collect(runner);
        ran(
            runner,
            r#"for (var i = 0; i < 8; i++) {
                 var e = G.list[i];
                 check(e.message === "m" + i + "x".repeat(50 + i), "message " + i);
                 check(e.name === "Name" + String.fromCharCode(0x41 + i), "name " + i);
                 check(Object.getPrototypeOf(e) === Sub.prototype, "prototype " + i);
                 check(e instanceof DOMException && e.code === 0, "instance " + i);
               }"#,
        );
    });
}

/// Held. The exceptions the bindings make (a slice's answer, a member of `throws`) survive a
/// collection and are read by a later script; their texts are cut at `EXCEPTION_TEXT_BYTES` at
/// a character's boundary: a cut inside a four-byte character keeps no half of it (no lone
/// surrogate), a cut on a three-byte boundary keeps the whole 1,024 bytes.
#[test]
fn the_exceptions_the_bindings_make_are_cut_at_a_boundary_and_survive_a_collection() {
    assert_eq!(EXCEPTION_TEXT_BYTES, 1024);
    with(limits(BUDGET, 1 << 20), |runner| {
        ran(
            runner,
            r#"G.caught = [];
               for (var n = 0; n < 3; n++) {
                 try { document.getElementById("throw:" + n); } catch (x) { G.caught.push(x); }

               }
               try { new Text(); } catch (x) { G.ctor = x; }"#,
        );
        collect(runner);
        ran(
            runner,
            r#"churn(2);
               var a = G.caught[0], b = G.caught[1], c = G.caught[2];
               check(G.caught.length === 3, "three thrown");
               check(a.name.length === 2 + 2 * 255, "name 0: " + a.name.length);
               check(a.name.codePointAt(a.name.length - 2) === 0x1F600, "name 0 ends in a whole emoji");
               check(a.message.length === 341 && a.message[340] === "€", "message 0: " + a.message.length);
               check(b.name.length === 1 + 341, "name 1: " + b.name.length);
               check(b.message.length === 2 + 340, "message 1: " + b.message.length);
               check(c.name === "ShortName" && c.code === 0, "name 2");
               check(c.message.length === 2 * 256 && c.message.codePointAt(510) === 0x10348, "message 2: " + c.message.length);
               for (var i = 0; i < 3; i++) {
                 var s = G.caught[i].name + G.caught[i].message;
                 for (var k = 0; k < s.length; k++) {
                   var u = s.charCodeAt(k);
                   if (u >= 0xD800 && u <= 0xDBFF) { var v = s.charCodeAt(k + 1); check(v >= 0xDC00 && v <= 0xDFFF, "lone high " + i); k++; }
                   else { check(!(u >= 0xDC00 && u <= 0xDFFF), "lone low " + i); }
                 }
               }
               check(G.ctor === undefined || (G.ctor.name === "NotSupportedError" && G.ctor.code === 9) || G.ctor instanceof TypeError, "ctor");"#,
        );
    });
}

/// Held. An uncaught `DOMException` whose message is 2^20 code units is recorded as
/// `<name>: <message>` cut to 1,024 characters, after the script, by a read that runs none.
#[test]
fn the_record_of_an_uncaught_exception_of_a_long_message_is_cut() {
    let end = with(limits(64 << 20, 1 << 20), |runner| {
        evaluate(
            runner,
            r#"throw new DOMException("\u0800".repeat(1 << 20), "N");"#,
        )
    });
    match end {
        ScriptEnd::Threw(error) => {
            let message = error.message.expect("a message");
            assert_eq!(message.chars().count(), 1024);
            assert!(
                message.starts_with("N: \u{800}\u{800}"),
                "{}",
                &message[..16]
            );
        }
        other => panic!("{other}"),
    }
}

/// The peak resident bytes of this process (`VmHWM` of `/proc/self/status`, Linux). Each test
/// runs in a process of its own under cargo-nextest.
pub(crate) fn peak_resident_bytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("/proc/self/status");
    let line = status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .expect("VmHWM");
    let kib: u64 = line
        .split_whitespace()
        .nth(1)
        .and_then(|field| field.parse().ok())
        .expect("the peak in kB");
    kib * 1024
}

/// Held, rewritten in the fourth pass on ruling 1 of fix round 3: `DOMException.code` copies a
/// name of at most 26 code units (the longest legacy name), so each copy is bounded and
/// dropped when the getter returns; a longer name is not copied (code 0 from its length), and
/// a loop of reads of either ends at the step limit. Each loop counts its rounds: a read of
/// `code` of a name of 2^14 code units of U+0800 costs about what a read of `name` costs (a
/// copy would charge 768 steps per read, one per 64 bytes of its 48 KiB), so the loop of
/// `code` makes more than half the rounds of the loop of `name`.
#[test]
fn the_code_getter_in_a_loop_copies_at_most_a_legacy_name_and_ends_at_the_step_limit() {
    let mut small = limits(64 << 20, 1 << 20);
    small.engine.steps_per_entry = 1_000_000;
    let started = Instant::now();
    let ends = with(small, |runner| {
        ran(
            runner,
            r#"G.e = new DOMException("m", "\u0800".repeat(1 << 14));
               G.f = new DOMException("m", "NoModificationAllowedError");
               G.g = new DOMException("m", "\u0800".repeat(26));
               G.rounds = [0, 0, 0];"#,
        );
        let ends = [
            r#"for (;;) { check(G.f.code === 7 && G.g.code === 0, "code"); G.rounds[0]++; }"#,
            r#"for (;;) { check(G.e.name.length === 16384, "name"); G.rounds[1]++; }"#,
            r#"for (;;) { check(G.e.code === 0, "code"); G.rounds[2]++; }"#,
        ]
        .map(|source| evaluate(runner, source));
        ran(
            runner,
            r#"check(G.rounds[2] * 2 > G.rounds[1] && G.rounds[0] > 1000, "rounds " + G.rounds);"#,
        );
        ends
    });
    for end in ends {
        assert_eq!(end, ScriptEnd::Limit(Reason::StepsPerScript), "{end}");
    }
    println!("three loops to the step limit: {:?}", started.elapsed());
}

/// Red (should). `docs/design/scripts.md:332-337` holds as written (no script runs while the
/// copy lives), but `DOMException.code` copies the whole name out of the engine, uncharged
/// (`prelude.rs:1418-1423` through `Copies::copy`, `:445-455`: `to_utf16` and then
/// `String::from_utf16_lossy`, both alive at once), to compare it with legacy names of at most
/// 26 code units. A name of 2^22 code units of U+0800 is 8 MiB on swell's heap under a budget
/// of 16 MiB; one read of `code` raises the process's peak by the 8 MiB of UTF-16 and the
/// 12 MiB of UTF-8, more than the whole budget. The prelude has a length that copies nothing
/// (`string_length`, `prelude.rs:744-758`): a name longer than 26 code units has code 0
/// without a copy.
#[test]
fn the_code_getter_copies_no_name_longer_than_the_legacy_names() {
    let (end, grown) = with(limits(16 << 20, 1 << 24), |runner| {
        ran(
            runner,
            r#"G.e = new DOMException("m", "\u0800".repeat(1 << 22));"#,
        );
        collect(runner);
        let before = peak_resident_bytes();
        let end = evaluate(runner, r#"check(G.e.code === 0, "code");"#);
        (end, peak_resident_bytes().saturating_sub(before))
    });
    assert_eq!(end, ScriptEnd::Ran, "{end}");
    let mib = grown >> 20;
    assert!(
        mib < 4,
        "one read of DOMException.code raised the peak by {mib} MiB, memory_bytes 16 MiB"
    );
}
