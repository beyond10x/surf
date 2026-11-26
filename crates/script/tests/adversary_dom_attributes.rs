//! An adversary's tests of the slice `attributes` (`story:scripts-dom-attributes`): what the
//! unit's own tests leave open. Each test names the property it holds; a test that fails at the
//! unit's tip names the defect in its message.

use std::time::Duration;

use surf_html::limits::{ATTRIBUTE_BYTES, Limits as ParserLimits};
use surf_html::{Attribute, NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{
    Document, Mode, Runner, RunnerLimits, SESSION_ENGINE_LIMITS, ScriptEnd, run_document,
};
use swell::Limits;

/// The limits of these tests: a memory budget of 64 MiB and a `string_length` of 1 Mi units.
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

/// The engine's limits of a session, with the runner's own room of [`limits`].
fn session() -> RunnerLimits {
    RunnerLimits {
        engine: SESSION_ENGINE_LIMITS,
        ..limits()
    }
}

/// Runs `body` with a runner under `limits` over `page`, parsed under the default limits.
fn with_page<R: Send>(
    limits: RunnerLimits,
    page: String,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(
        limits,
        Mode::Test,
        move || Document {
            tree: parse_document(&page, &ParseOptions::default()),
            limits: ParserLimits::default(),
            slices: dom::slices(DocumentInfo::default()),
        },
        body,
    )
    .expect("a runner with the bindings")
}

/// The functions every script starts with.
const CHECK: &str = r#"
function check(holds, what) { if (!holds) { throw new Error(what); } }
function threw(f) { try { f(); } catch (e) { return e.name; } return "no throw"; }
var SVG = "http://www.w3.org/2000/svg";
"#;

/// Runs `source` and asserts it ran and recorded no limit.
fn ran(runner: &mut Runner, source: &str) {
    let end = runner.evaluate(&format!("{CHECK}{source}"));
    assert_eq!(end, ScriptEnd::Ran, "{end}");
    let limit = runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .and_then(|dom| dom.limit());
    assert_eq!(limit, None);
}

/// The steps `source` adds to the runner's total.
fn steps(runner: &mut Runner, source: &str) -> u64 {
    let before = runner.steps();
    ran(runner, source);
    runner.steps() - before
}

/// What the bindings and the tree hold, read between two scripts.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Figures {
    live: u64,
    charged: u64,
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
    Figures {
        live: state.stored_live(),
        charged: state.stored_charged(),
        stored: state.tree().stored_bytes(),
        writes: state.tree().writes(),
    }
}

/// The bytes the stored handle holds, read between two scripts.
fn charged(runner: &mut Runner) -> u64 {
    runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .expect("the bindings")
        .stored_charged()
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

/// The first element of `tree` whose local name is `name`.
fn element(tree: &Tree, name: &str) -> NodeId {
    tree.descendants(tree.root())
        .into_iter()
        .find(|&node| matches!(tree.data(node), NodeData::Element { local_name, .. } if local_name == name))
        .expect("the element")
}

/// The attribute list of `node`, with its room.
fn list(tree: &Tree, node: NodeId) -> &Vec<Attribute> {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes,
        _ => panic!("an element"),
    }
}

/// A push into an attribute list the parser built takes room for twice its records; the bytes
/// the tree holds because of that one write are those records, not the one the tree counts.
/// The story's decision 8 says the factor of two covers the growth of a list; it covers a list
/// a script grew from empty, not a list the parser built at its exact length.
#[test]
fn a_push_into_a_list_the_parser_built_is_charged_for_the_room_it_takes() {
    let names: String = (0..512).map(|i| format!(" a{i}=\"\"")).collect();
    let page = format!("<html{names}><body></body></html>");
    with_page(session(), page, |runner| {
        ran(runner, "var e = document.documentElement;");
        let html = element(tree(runner), "html");
        let before = list(tree(runner), html).capacity();
        assert_eq!(list(tree(runner), html).len(), 512);
        ran(runner, "e.setAttribute(\"z\", \"\");");
        let after = list(tree(runner), html).capacity();
        let held = (after - before) * ATTRIBUTE_BYTES + 1;
        let charged = charged(runner);
        eprintln!(
            "capacity {before} -> {after}, held {held} bytes for the write, charged {charged}"
        );
        assert!(
            u64::try_from(held).unwrap() <= charged,
            "one setAttribute grew the parsed list from {before} to {after} records: the tree \
             holds {held} more bytes for it, the stored handle charges {charged}"
        );
    });
}

/// "Validate and extract" (DOM, 1.4) tests the prefix against "valid namespace prefix", which
/// admits `=` (only ASCII whitespace, NUL, `/` and `>` are out), and only the local name after
/// the first colon against "valid attribute local name". `setAttributeNS(SVG, "a=b:c", "1")`
/// stores the attribute `c` with the prefix `a=b`.
#[test]
fn a_prefix_may_hold_an_equals_sign() {
    let page = "<html><body></body></html>".to_string();
    with_page(limits(), page, |runner| {
        ran(
            runner,
            r#"
var e = document.documentElement;
var got = threw(function () { e.setAttributeNS(SVG, "a=b:c", "1"); });
check(got === "no throw", "setAttributeNS(SVG, \"a=b:c\", \"1\") threw " + got);
check(e.getAttributeNS(SVG, "c") === "1" && e.getAttribute("a=b:c") === "1", "the attribute");"#,
        );
    });
}

/// `count` distinct names of `len` bytes: letters and a four-digit number.
fn long_names(count: usize, len: usize) -> Vec<String> {
    (0..count)
        .map(|i| format!("{}{i:04}", "a".repeat(len - 4)))
        .collect()
}

/// A write that puts a value pays the tree's own scan with both of its terms, `c / A` and
/// `k * n / S`, beside the scan of `find`: on an element of 64 attributes whose names all have
/// the wanted name's 1,000 bytes, each scan reads up to 64,000 bytes of names. Floors, decision
/// 7: the glue's copy of the name (1,000 x 2 / 16 = 125), the pass over it (1,000 / 16 = 62),
/// two scans of 64 / 4 + 64 x 1,000 / 16 = 4,016 each and the tree's copy (1,000 / 16 = 62):
/// 8,281. The `id` setter on 935 names of two bytes: two scans of 935 / 4 + 935 x 2 / 16 =
/// 233 + 116 = 349, 698 in all.
#[test]
fn a_put_pays_the_tree_scan_over_names_of_the_wanted_length() {
    let names = long_names(64, 1_000);
    let mut short = Vec::new();
    for first in 'a'..='z' {
        for second in ('a'..='z').chain('0'..='9') {
            let name = format!("{first}{second}");
            if name != "id" {
                short.push(name);
            }
        }
    }
    assert_eq!(short.len(), 935);
    let attributes =
        |names: &[String]| -> String { names.iter().map(|name| format!(" {name}=\"\"")).collect() };
    let page = format!(
        "<html{}><body><b{}></b></body></html>",
        attributes(&names),
        attributes(&short)
    );
    with_page(limits(), page, |runner| {
        ran(
            runner,
            "var e = document.documentElement; var b = document.getElementsByTagName(\"b\")[0]; \
             var w = \"a\".repeat(996) + \"zzzz\";",
        );
        let floor = 125 + 62 + 2 * (64 / 4 + 64 * 1_000 / 16) + 62;
        assert_eq!(floor, 8_281);
        for (call, undo) in [
            ("e.setAttribute(w, \"\");", "e.removeAttribute(w);"),
            (
                "e.setAttributeNS(null, w, \"\");",
                "e.removeAttributeNS(null, w);",
            ),
            ("e.toggleAttribute(w);", "e.removeAttribute(w);"),
        ] {
            let spent = steps(runner, call);
            eprintln!("{call} {spent} steps, floor {floor}");
            assert!(
                spent >= floor,
                "{call}: {spent} steps, under the floor {floor}"
            );
            ran(runner, undo);
        }
        let floor = 2 * (935 / 4 + 935 * 2 / 16);
        assert_eq!(floor, 698);
        for call in ["b.id = \"v\";", "b.id = \"w\";"] {
            let spent = steps(runner, call);
            eprintln!("{call} {spent} steps, floor {floor}");
            assert!(
                spent >= floor,
                "{call}: {spent} steps, under the floor {floor}"
            );
        }
    });
}

/// "Valid attribute local name" and "valid namespace prefix" (DOM, 1.4) for each ASCII
/// whitespace, NUL, `/`, `=` and `>`, in a name, in a prefix and in a local name after a
/// prefix; a prefix may hold `=`, a local name may hold a colon after the first, and a
/// capital outside ASCII is not lowered. A throw writes nothing and charges nothing for memory.
#[test]
fn every_character_the_names_refuse_and_admit() {
    let page = "<html><body></body></html>".to_string();
    with_page(limits(), page, |runner| {
        ran(runner, "var e = document.documentElement;");
        let before = figures(runner);
        ran(
            runner,
            r#"
var bad = ["a\tb", "a\nb", "a\fb", "a\rb", "a b", "a\u0000b", "a/b", "a=b", "a>b", ""];
for (var i = 0; i < bad.length; i++) {
  check(threw(function () { e.setAttribute(bad[i], "v"); }) === "InvalidCharacterError", "setAttribute " + i);
  check(threw(function () { e.toggleAttribute(bad[i]); }) === "InvalidCharacterError", "toggleAttribute " + i);
  check(threw(function () { e.toggleAttribute(bad[i], false); }) === "InvalidCharacterError", "toggleAttribute false " + i);
  check(threw(function () { e.setAttributeNS(SVG, bad[i], "v"); }) === "InvalidCharacterError", "setAttributeNS " + i);
  check(threw(function () { e.setAttributeNS(SVG, "p:" + bad[i], "v"); }) === "InvalidCharacterError", "setAttributeNS local " + i);
}
var badPrefix = ["a\tb", "a\nb", "a\fb", "a\rb", "a b", "a\u0000b", "a/b", "a>b", ""];
for (var j = 0; j < badPrefix.length; j++) {
  check(threw(function () { e.setAttributeNS(SVG, badPrefix[j] + ":c", "v"); }) === "InvalidCharacterError", "prefix " + j);
}
check(e.getAttributeNames().length === 0, "nothing written");"#,
        );
        assert_eq!(figures(runner), before, "a throw moves nothing");
        ran(
            runner,
            r#"
e.setAttributeNS(SVG, "x:a:b", "2");
check(e.getAttributeNS(SVG, "a:b") === "2", "the local name is what follows the first colon");
check(e.getAttribute("x:a:b") === "2", "the qualified name");
e.setAttribute("É", "3");
check(e.getAttribute("é") === null && e.getAttribute("É") === "3", "no lowering outside ASCII");
e.setAttribute("a b", "4");
check(e.getAttribute("a b") === "4", "U+00A0 is no ASCII whitespace");
var names = e.getAttributeNames().join("|");
check(names === "x:a:b|É|a b", names);"#,
        );
        let after = figures(runner);
        assert_eq!(after.live, (after.stored - before.stored) as u64);
        assert_eq!(after.charged, 2 * after.live);
    });
}

/// A value of `string_length` units of three bytes each, `3 x string_length` bytes, is answered
/// whole: only a value of more than three bytes a unit is over the limit without a count.
#[test]
fn a_value_of_three_bytes_a_unit_at_string_length_is_answered() {
    let page = "<html><body></body></html>".to_string();
    with_page(limits(), page, |runner| {
        ran(
            runner,
            r#"
var e = document.documentElement;
e.setAttribute("a", "€".repeat(1048576));
var got = e.getAttribute("a");
check(got.length === 1048576 && got.charCodeAt(1048575) === 0x20ac, "getAttribute");
got = null;
got = e.getAttributeNS(null, "a");
check(got.length === 1048576, "getAttributeNS");
got = null;
e.id = "€".repeat(1048576);
check(e.id.length === 1048576, "the id getter");"#,
        );
        let html = element(tree(runner), "html");
        assert_eq!(list(tree(runner), html)[0].value.len(), 3 * 1_048_576);
    });
}

/// `toggleAttribute`'s `force`: `undefined` is no `force` (Web IDL, an optional argument), and
/// any other value is its ToBoolean.
#[test]
fn toggle_reads_undefined_as_no_force() {
    let page = "<html><body></body></html>".to_string();
    with_page(limits(), page, |runner| {
        ran(
            runner,
            r#"
var e = document.documentElement;
check(e.toggleAttribute("t", undefined) === true && e.hasAttribute("t"), "undefined adds");
check(e.toggleAttribute("t", undefined) === false && !e.hasAttribute("t"), "undefined removes");
check(e.toggleAttribute("t", null) === false && !e.hasAttribute("t"), "null is false");
check(e.toggleAttribute("t", 0) === false && !e.hasAttribute("t"), "0 is false");
check(e.toggleAttribute("t", "") === false && !e.hasAttribute("t"), "the empty string is false");
check(e.toggleAttribute("t", {}) === true && e.getAttribute("t") === "", "an object is true");
check(e.toggleAttribute("t", 1) === true && e.getAttribute("t") === "", "true keeps it");
check(e.toggleAttribute("t", NaN) === false && !e.hasAttribute("t"), "NaN is false");"#,
        );
        let now = figures(runner);
        assert_eq!((now.live, now.charged), (0, 0));
    });
}

/// The arguments are converted before the member reads the tree (Web IDL): a conversion that
/// writes the same element is seen whole by the call, and the charge follows the tree.
#[test]
fn a_conversion_that_writes_the_same_element() {
    let page = "<html><body></body></html>".to_string();
    with_page(session(), page, |runner| {
        ran(runner, "var e = document.documentElement;");
        let start = figures(runner);
        ran(
            runner,
            r#"
e.setAttribute("a", "old");
e.setAttribute("a", { toString: function () { e.removeAttribute("a"); return "new"; } });
check(e.getAttribute("a") === "new" && e.getAttributeNames().join() === "a", "removed while converted");
e.setAttribute({ toString: function () { e.setAttribute("b", "1"); return "c"; } },
  { toString: function () { e.removeAttribute("b"); return "w"; } });
check(e.getAttributeNames().join() === "a,c" && e.getAttribute("c") === "w", e.getAttributeNames().join());
check(e.toggleAttribute({ toString: function () { e.toggleAttribute("t"); return "t"; } }) === false
  && !e.hasAttribute("t"), "the nested toggle is seen");
check(threw(function () {
  e.setAttribute("d", { toString: function () { e.setAttribute("d", "1"); throw new Error("x"); } });
}) === "Error" && e.getAttribute("d") === "1", "a throw in the conversion keeps the nested write");
check(threw(function () { e.setAttribute(Symbol("s"), "v"); }) === "TypeError", "a Symbol");
e.setAttributeNS({ toString: function () { e.id = "n"; return SVG; } }, "q:r",
  { toString: function () { e.removeAttribute("id"); return "v"; } });
check(e.getAttributeNS(SVG, "r") === "v" && !e.hasAttribute("id"), "setAttributeNS");
check(e.getAttributeNames().join() === "a,c,d,q:r", e.getAttributeNames().join());"#,
        );
        let now = figures(runner);
        assert_eq!(now.live, (now.stored - start.stored) as u64);
        assert_eq!(now.charged, 2 * now.live);
        assert_eq!(now.writes, start.writes);
    });
}

/// Each of the 17 methods checks the brand of `this` before anything else: `undefined`,
/// `null`, a plain object, an object whose prototype is an element, a primitive, a `Text` and
/// the `Document` each throw a `TypeError` and change nothing. swell 0.8.0 has no `Proxy`
/// ("no proxy exists before slice 9"), so no case of one is made.
#[test]
fn every_method_checks_its_brand() {
    let page = "<html><body><p id=\"t\">x</p></body></html>".to_string();
    with_page(limits(), page, |runner| {
        ran(
            runner,
            "var e = document.documentElement; var text = document.getElementById(\"t\").firstChild;",
        );
        let before = figures(runner);
        let tree_before = tree(runner).clone();
        ran(
            runner,
            r#"
check(typeof Proxy === "undefined", "a Proxy exists: add it to the values");
var P = Element.prototype;
function accessor(name, set) {
  var d = Object.getOwnPropertyDescriptor(P, name);
  return set ? function (t) { d.set.call(t, "x"); } : function (t) { d.get.call(t); };
}
var calls = {
  getAttribute: function (t) { P.getAttribute.call(t, "id"); },
  getAttributeNS: function (t) { P.getAttributeNS.call(t, null, "id"); },
  getAttributeNames: function (t) { P.getAttributeNames.call(t); },
  hasAttribute: function (t) { P.hasAttribute.call(t, "id"); },
  hasAttributeNS: function (t) { P.hasAttributeNS.call(t, null, "id"); },
  hasAttributes: function (t) { P.hasAttributes.call(t); },
  setAttribute: function (t) { P.setAttribute.call(t, "a", "b"); },
  setAttributeNS: function (t) { P.setAttributeNS.call(t, null, "a", "b"); },
  removeAttribute: function (t) { P.removeAttribute.call(t, "id"); },
  removeAttributeNS: function (t) { P.removeAttributeNS.call(t, null, "id"); },
  toggleAttribute: function (t) { P.toggleAttribute.call(t, "a"); },
  getId: accessor("id", false), setId: accessor("id", true),
  getClassName: accessor("className", false), setClassName: accessor("className", true),
  getSlot: accessor("slot", false), setSlot: accessor("slot", true)
};
var values = [undefined, null, {}, Object.create(e), 1, "s", text, document];
var count = 0;
for (var name in calls) {
  count++;
  for (var i = 0; i < values.length; i++) {
    var got;
    try { calls[name](values[i]); got = "no throw"; }
    catch (x) { got = x instanceof TypeError ? "TypeError" : String(x); }
    check(got === "TypeError", name + " on value " + i + ": " + got);
  }
}
check(count === 17, "17 methods");"#,
        );
        assert_eq!(figures(runner), before);
        assert_eq!(tree(runner), &tree_before);
    });
}

/// The stored bytes of `tree` counted afresh from the nodes under its root, as the tree counts
/// them.
fn fresh_count(tree: &Tree) -> usize {
    tree.descendants(tree.root())
        .into_iter()
        .map(|node| match tree.data(node) {
            NodeData::Document | NodeData::DocumentFragment => 0,
            NodeData::Doctype {
                name,
                public_id,
                system_id,
            } => name.len() + public_id.len() + system_id.len(),
            NodeData::Element {
                local_name,
                attributes,
                ..
            } => {
                local_name.len()
                    + attributes
                        .iter()
                        .map(Attribute::stored_bytes)
                        .sum::<usize>()
            }
            NodeData::Text(text) => text.len(),
            NodeData::Comment(data) => data.len(),
            NodeData::ProcessingInstruction { target, data } => target.len() + data.len(),
        })
        .sum()
}

/// After every path of every writer, the tree's total equals a fresh count of the tree, the
/// live bytes its rise since the parse, the charge twice them, the write counter does not move,
/// and the reading members see each write.
#[test]
fn every_path_of_every_writer_keeps_the_counts() {
    let page = "<!DOCTYPE html><html><body><p id=\"a\" class=\"k\">x</p><p id=\"b\"></p>\
                <svg id=\"g\" viewBox=\"0 0 1 1\"><use id=\"u\" xlink:href=\"#a\"></use></svg>\
                </body></html>"
        .to_string();
    with_page(session(), page, |runner| {
        ran(
            runner,
            r#"var a = document.getElementById("a"); var b = document.getElementById("b");
var g = document.getElementById("g"); var u = document.getElementById("u");
var XLINK = "http://www.w3.org/1999/xlink"; var XML = "http://www.w3.org/XML/1998/namespace";
var XMLNS = "http://www.w3.org/2000/xmlns/";"#,
        );
        let start = figures(runner);
        assert_eq!(fresh_count(tree(runner)), start.stored, "the parse's count");
        let writes = [
            "a.id = \"z\"; check(document.getElementById(\"z\") === a && document.getElementById(\"a\") === null, \"id\");",
            "b.id = \"z\"; check(document.getElementById(\"z\") === a, \"the first in tree order\");",
            "a.id = \"\"; check(document.getElementById(\"z\") === b && document.getElementById(\"\") === null, \"empty id\");",
            "a.removeAttribute(\"id\"); a.id = \"a\"; check(a.getAttributeNames().join() === \"class,id\", a.getAttributeNames().join());",
            "a.className = \"m n\"; check(document.getElementsByClassName(\"n\")[0] === a && document.getElementsByClassName(\"k\").length === 0, \"class\");",
            "a.setAttribute(\"DATA-X\", \"1\"); a.setAttribute(\"data-x\", \"22\"); check(a.getAttribute(\"Data-X\") === \"22\", \"lowered\");",
            "g.setAttribute(\"ViewBox\", \"2\"); check(g.getAttribute(\"viewBox\") === \"0 0 1 1\" && g.getAttribute(\"ViewBox\") === \"2\", \"svg case\");",
            "g.setAttributeNS(XLINK, \"xlink:href\", \"#b\"); g.setAttributeNS(XLINK, \"l:href\", \"#c\"); check(g.getAttribute(\"xlink:href\") === \"#c\", \"replaced, prefix kept\");",
            "u.setAttributeNS(XML, \"xml:lang\", \"en\"); u.setAttributeNS(XMLNS, \"xmlns\", \"urn:a\"); u.setAttributeNS(XMLNS, \"xmlns:p\", \"urn:b\");",
            "a.setAttributeNS(null, \"ID\", \"caps\"); check(a.id === \"a\" && a.getAttributeNS(null, \"ID\") === \"caps\", \"two names\");",
            "a.setAttribute(\"a:b\", \"1\"); a.setAttributeNS(XLINK, \"a:b\", \"2\"); check(a.getAttribute(\"a:b\") === \"1\", \"the first of two qualified names\");",
            "a.removeAttribute(\"a:b\"); check(a.getAttribute(\"a:b\") === \"2\", \"the second is found now\");",
            "check(a.toggleAttribute(\"hidden\") && !a.toggleAttribute(\"HIDDEN\"), \"both ways\");",
            "a.removeAttributeNS(XLINK, \"b\"); a.removeAttributeNS(null, \"ID\"); u.removeAttributeNS(XMLNS, \"p\");",
            "g.removeAttribute(\"ViewBox\"); g.removeAttribute(\"xlink:href\"); check(g.getAttributeNames().join() === \"id,viewBox\", g.getAttributeNames().join());",
            "a.setAttribute(\"data-x\", \"\"); a.removeAttribute(\"data-x\"); a.className = \"k\"; b.removeAttribute(\"id\"); b.id = \"b\";",
            "u.removeAttributeNS(XML, \"lang\"); u.removeAttributeNS(XMLNS, \"xmlns\");",
        ];
        for write in writes {
            ran(runner, write);
            let now = figures(runner);
            let tree = tree(runner);
            assert_eq!(fresh_count(tree), now.stored, "{write}");
            assert_eq!(now.writes, start.writes, "{write}");
            // A removal of the parser's bytes while nothing is live stops at 0 (decision 8),
            // so the live bytes are at least the rise, never under it.
            let rise = now.stored as i64 - start.stored as i64;
            assert!(
                now.live as i64 >= rise,
                "{write}: live {} under the rise {rise}",
                now.live
            );
            assert_eq!(now.charged, 2 * now.live, "{write}");
            for node in tree.descendants(tree.root()) {
                if let NodeData::Element { attributes, .. } = tree.data(node) {
                    assert!(
                        attributes.capacity() <= (2 * attributes.len()).max(4),
                        "{write}: a list of {} holds room for {}",
                        attributes.len(),
                        attributes.capacity()
                    );
                    let names = tree.attribute_names(node);
                    for (name, attribute) in names.iter().zip(attributes) {
                        assert_eq!(name.len(), attribute.qualified_len(), "{write}");
                    }
                }
            }
        }
    });
}

/// The members of the slice in any spelling: `attributes.rs` names no tree, no engine and no
/// charge, not through `DomCx::tree_mut(cx)` nor `cx.charge_node(..)`; neither reads nor writes
/// reach the memory budget but through `charged_write`; the reads name no `tree_mut`.
#[test]
fn the_members_file_names_no_tree_and_no_charge_in_any_spelling() {
    let code = |source: &str| -> String {
        source
            .lines()
            .map(|line| line.find("//").map_or(line, |at| &line[..at]))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let members = code(include_str!("../src/dom/attributes.rs"));
    for token in ["tree", "charge", "engine", "settle", "Tree", "super::reads"] {
        assert!(!members.contains(token), "attributes.rs names `{token}`");
    }
    let reads = code(include_str!("../src/dom/attribute_reads.rs"));
    for token in [
        "tree_mut",
        "charge_node",
        "charge_host_bytes",
        "resize_host_bytes",
        "charge_stored",
        "settle_stored",
    ] {
        assert!(!reads.contains(token), "attribute_reads.rs names `{token}`");
    }
    let writes = code(include_str!("../src/dom/attribute_writes.rs"));
    for token in [
        "tree(",
        "charge_node",
        "charge_host_bytes",
        "resize_host_bytes",
        "engine",
    ] {
        assert!(
            !writes.contains(token),
            "attribute_writes.rs names `{token}`"
        );
    }
    let start = writes.find("fn charged_write").expect("charged_write");
    for token in ["tree_mut", "charge_stored", "settle_stored"] {
        assert!(
            !writes[..start].contains(token),
            "attribute_writes.rs names `{token}` before charged_write"
        );
        let after = &writes[start..];
        let next = after[1..].find("\nfn ").map_or(after.len(), |at| at + 1);
        assert!(
            !after[next..].contains(token),
            "attribute_writes.rs names `{token}` after charged_write"
        );
    }
}
