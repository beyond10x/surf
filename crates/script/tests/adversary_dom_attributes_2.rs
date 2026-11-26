//! A second adversary's tests of the slice `attributes` (`story:scripts-dom-attributes`), after
//! the two fix rounds of its first review: the engine's side of the stored handle, the source
//! rule of the members' files, the DOM's "validate and extract" row by row, and the charge of
//! writes on two elements in turn. Each test names the property it holds; a test that fails at
//! the unit's tip names the defect in its message.

use std::time::Duration;

use surf_html::limits::{ATTRIBUTE_BYTES, Limits as ParserLimits};
use surf_html::{Attribute, NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{
    Document, Mode, Reason, Runner, RunnerLimits, SESSION_ENGINE_LIMITS, ScriptEnd, run_document,
};
use swell::Limits;

/// The limits of these tests: the engine's limits of a session (a memory budget of 64 MiB),
/// with the runner's room of the slice's own tests.
fn session() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            string_length: 1 << 20,
            ..SESSION_ENGINE_LIMITS
        },
        steps_per_runner: 1_000_000_000,
        scripts_per_runner: 10_000,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(50),
    }
}

/// Runs `body` with a runner over `page`, parsed under `parse`.
fn with_page<R: Send>(
    page: String,
    parse: ParserLimits,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(
        session(),
        Mode::Test,
        move || Document {
            tree: parse_document(
                &page,
                &ParseOptions {
                    limits: parse,
                    ..ParseOptions::default()
                },
            ),
            limits: parse,
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

/// The bytes the engine's heap is charged now: its arenas, its records and every host charge,
/// the stored handle among them.
fn heap(runner: &mut Runner) -> i64 {
    let bytes = runner.agent_and_realm().0.heap_bytes().expect("heap_bytes");
    i64::try_from(bytes).expect("a heap under 2^63 bytes")
}

/// The stored handle as the bindings record it: the live bytes and the bytes charged.
fn stored(runner: &mut Runner) -> (u64, u64) {
    let state = runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .expect("the bindings");
    (state.stored_live(), state.stored_charged())
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

fn attributes(tree: &Tree, node: NodeId) -> &[Attribute] {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes,
        _ => panic!("an element"),
    }
}

/// The room of the attribute list of `node` in records, read from the list's allocation.
fn room_of(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes.capacity(),
        _ => panic!("an element"),
    }
}

/// The bytes of the strings of the attributes of `node`, summed here from their lengths.
fn strings_of(tree: &Tree, node: NodeId) -> usize {
    attributes(tree, node)
        .iter()
        .map(|a| a.prefix.as_ref().map_or(0, String::len) + a.local_name.len() + a.value.len())
        .sum()
}

/// How far the engine's heap may move by itself between two scripts that make no charge of
/// their own (a script's source and code, an arena's growth): far below the 2 MiB a stored
/// value of 1 MiB is charged.
const NOISE: i64 = 256 * 1024;

/// The engine gives back what a removal refunds. The bindings record the stored handle in
/// `stored_charged`, and every test of the slice reads that record; this one reads the engine's
/// own count, `heap_bytes`, which the handle is part of. A set of 1 MiB on an empty list raises
/// it by twice the 80 bytes of the record, the name and the value; the removal empties the list
/// and lowers it by as much, as the record does.
#[test]
fn the_engine_gives_back_what_a_removal_refunds() {
    with_page("<p></p>".to_string(), ParserLimits::default(), |runner| {
        ran(
            runner,
            "var e = document.documentElement; var v = \"x\".repeat(1048576);",
        );
        let set = (ATTRIBUTE_BYTES + 1 + (1 << 20)) as u64;
        let before = heap(runner);
        ran(runner, "e.setAttribute(\"a\", v);");
        let after_set = heap(runner);
        assert_eq!(stored(runner), (set, 2 * set), "the record after the set");
        let rose = after_set - before;
        assert!(
            (rose - 2 * set as i64).abs() < NOISE,
            "the set raised the engine's heap by {rose}, the record says {}",
            2 * set
        );
        ran(runner, "e.removeAttribute(\"a\");");
        let after_removal = heap(runner);
        assert_eq!(stored(runner), (0, 0), "the record after the removal");
        let fell = after_set - after_removal;
        assert!(
            (fell - 2 * set as i64).abs() < NOISE,
            "the removal gave the engine back {fell} bytes; the record gave back {}",
            2 * set
        );
        eprintln!("engine: set +{rose}, removal -{fell}, record {}", 2 * set);
    });
}

/// A write the tree refuses leaves the engine's count as it was: the charge taken before the
/// tree's limit check is put back in the engine, not only in the record. An element of three
/// attributes where three are allowed; `setAttribute` of a fourth with a value of 1 MiB ends
/// `limit: attributes`.
#[test]
fn a_write_the_tree_refuses_leaves_the_engine_as_before() {
    let parse = ParserLimits {
        max_attributes: 3,
        ..ParserLimits::default()
    };
    let page = "<p a=\"1\" b=\"2\" c=\"3\"></p>".to_string();
    with_page(page, parse, |runner| {
        ran(
            runner,
            "var p = document.getElementsByTagName(\"p\")[0]; var v = \"x\".repeat(1048576);",
        );
        let before = heap(runner);
        let end = runner.evaluate("p.setAttribute(\"d\", v);");
        assert_eq!(end, ScriptEnd::Limit(Reason::Attributes));
        assert_eq!(stored(runner), (0, 0), "the record after the refusal");
        let moved = heap(runner) - before;
        assert!(
            moved.abs() < NOISE,
            "after the refused write the engine's heap moved by {moved}; the record by 0"
        );
        eprintln!("engine after a refused write of 1 MiB: {moved}");
    });
}

/// The code of a Rust source: each line up to a `//` that is outside a string literal. A `//`
/// inside a string (a URL, a message) leaves the rest of the line code.
fn code(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let bytes = line.as_bytes();
            let mut in_string = false;
            let mut at = 0;
            while at < bytes.len() {
                match bytes[at] {
                    b'\\' if in_string => at += 1,
                    b'"' => in_string = !in_string,
                    b'/' if !in_string && bytes.get(at + 1) == Some(&b'/') => {
                        return &line[..at];
                    }
                    _ => {}
                }
                at += 1;
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The members' files name no tree and no charge, read with the comments cut where a comment
/// starts: the members' file names no tree, no engine and no charge; the reads' file names no
/// write and no memory charge; the writes' file reaches the tree to write and the memory
/// budget only inside `charged_write`. The first review's source test cuts each line at its
/// first `//`, so a call after a string that holds `//` on the same line is not seen.
#[test]
fn the_members_files_hide_no_call_behind_a_slash_pair_in_a_string() {
    assert_eq!(code("a(\"x//y\"); b(); // c"), "a(\"x//y\"); b(); ");
    assert_eq!(code("/// doc"), "");
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
    let after = &writes[start..];
    let next = after[1..].find("\nfn ").map_or(after.len(), |at| at + 1);
    for token in ["tree_mut", "charge_stored", "settle_stored"] {
        assert!(
            !writes[..start].contains(token),
            "attribute_writes.rs names `{token}` before charged_write"
        );
        assert!(
            !after[next..].contains(token),
            "attribute_writes.rs names `{token}` after charged_write"
        );
    }
}

/// The DOM's "validate and extract" for `setAttributeNS`, row by row: the split at the first
/// colon (a second colon is the local name's), the prefix held to a valid namespace prefix
/// (`=` admitted) at its start, inside and at its end, the local name to a valid attribute
/// local name (`=` refused), an empty prefix or local name, a code point outside ASCII, the
/// `InvalidCharacterError` before the `NamespaceError`, each namespace rule, and a namespace
/// outside the six the tree stores answering `NotSupportedError` after them. A name admitted is
/// stored with its prefix and local name and read back by both reads.
#[test]
fn validate_and_extract_row_by_row() {
    with_page("<p></p>".to_string(), ParserLimits::default(), |runner| {
        ran(
            runner,
            r#"var e = document.documentElement;
var SVG = "http://www.w3.org/2000/svg", XML = "http://www.w3.org/XML/1998/namespace",
    XMLNS = "http://www.w3.org/2000/xmlns/", OTHER = "http://example.com/";
var ICE = "InvalidCharacterError", NE = "NamespaceError";
var rows = [
  [SVG, "a:b:c", "b:c"], [SVG, "a::", ":"], [SVG, "a=b:c", "c"], [SVG, "=:c", "c"],
  [SVG, "a=:c", "c"], [SVG, "a:b=", ICE], [SVG, "a:=b", ICE], [SVG, "a:b:=", ICE],
  [SVG, "a:b:c=d", ICE], [SVG, ":a", ICE], [SVG, "::", ICE], [SVG, "a:", ICE], [SVG, ":", ICE],
  [SVG, "", ICE], [SVG, "\u0000:a", ICE], [SVG, "a\t:b", ICE], [SVG, "a/:b", ICE],
  [SVG, "a>:b", ICE], [SVG, " a:b", ICE], [SVG, "a:b\n", ICE], [SVG, "a:\u000cb", ICE],
  [SVG, "\u00a0:b", "b"], [SVG, "a:\u3000", "\u3000"], [SVG, "a:\u00e9=", ICE],
  [null, "=:a", NE], [null, ">:a", ICE], [null, "a=", ICE], [null, "=a", ICE], ["", "a:b", NE],
  [null, "xmlns", NE], [SVG, "xmlns", NE], [XMLNS, "xmlns", "xmlns"], [XMLNS, "xmlns:a", "a"],
  [SVG, "xmlns:a", NE], [XMLNS, "a", NE], [XMLNS, "a:b", NE], [XMLNS, "a:xmlns", NE],
  [SVG, "a:xmlns", "xmlns"], [XML, "xml:lang", "lang"], [SVG, "xml:a", NE],
  [XML, "a:b", "b"], [XML, "xmlns:a", NE], [XML, "xml", "xml"], [null, "xml", "xml"],
  [OTHER, "a=:b", "NotSupportedError"], [OTHER, ">:b", ICE], [OTHER, "xmlns", NE],
  [OTHER, "a:b", "NotSupportedError"]
];
rows.forEach(function (row, i) {
  var ns = row[0], name = row[1], want = row[2];
  var error = want === ICE || want === NE || want === "NotSupportedError";
  var got = threw(function () { e.setAttributeNS(ns, name, "v" + i); });
  if (error) {
    check(got === want, "row " + i + ": " + name + " threw " + got + ", not " + want);
    check(!e.hasAttributes(), "row " + i + ": an attribute after the throw");
    return;
  }
  check(got === "no throw", "row " + i + ": " + name + " threw " + got);
  check(e.getAttributeNS(ns, want) === "v" + i, "row " + i + ": getAttributeNS of " + want);
  check(e.getAttribute(name) === "v" + i, "row " + i + ": getAttribute of " + name);
  var names = e.getAttributeNames();
  check(names.length === 1 && names[0] === name, "row " + i + ": names " + names.join(","));
  e.removeAttributeNS(ns, want);
  check(!e.hasAttributes(), "row " + i + ": removed");
});"#,
        );
        assert_eq!(stored(runner), (0, 0));
    });
}

/// The arguments (part 4 of the first review, where it named no case): `undefined` for the
/// nullable namespace is null, the string `"null"` is a URL outside the six (nothing found,
/// nothing removed, `NotSupportedError` to set); a `toString` that throws or a Symbol in any
/// argument throws before anything is written or charged; the name is converted before the
/// value.
#[test]
fn the_arguments_convert_before_anything_is_written() {
    with_page("<p></p>".to_string(), ParserLimits::default(), |runner| {
        ran(
            runner,
            r#"var e = document.documentElement;
e.setAttributeNS(undefined, "u", "1");
check(e.getAttributeNS(null, "u") === "1", "undefined is null");
check(e.hasAttributeNS(undefined, "u") && e.hasAttributeNS("", "u"), "undefined and the empty string");
check(threw(function () { e.setAttributeNS("null", "n", "1"); }) === "NotSupportedError", "set in null");
check(e.getAttributeNS("null", "u") === null && !e.hasAttributeNS("null", "u"), "read in null");
e.removeAttributeNS("null", "u");
check(e.hasAttribute("u"), "the string null removes nothing");
e.removeAttributeNS(undefined, "u");
check(!e.hasAttributes(), "undefined removes");
var bad = { toString: function () { throw new RangeError("converted"); } };
check(threw(function () { e.setAttribute("w", bad); }) === "RangeError", "a value that throws");
check(threw(function () { e.setAttribute(bad, "w"); }) === "RangeError", "a name that throws");
check(threw(function () { e.setAttributeNS(null, "w", bad); }) === "RangeError", "NS value");
check(threw(function () { e.setAttributeNS(bad, "w", "v"); }) === "RangeError", "NS namespace");
check(threw(function () { e.toggleAttribute(bad); }) === "RangeError", "toggle name");
check(threw(function () { e.className = bad; }) === "RangeError", "className");
check(threw(function () { e.setAttribute("w", Symbol("v")); }) === "TypeError", "a Symbol value");
check(threw(function () { e.id = Symbol("v"); }) === "TypeError", "a Symbol id");
check(!e.hasAttributes() && e.id === "" && e.className === "", "nothing written");
var order = [];
e.setAttribute({ toString: function () { order.push("n"); return "o"; } },
               { toString: function () { order.push("v"); return "x"; } });
check(order.join() === "n,v" && e.getAttribute("o") === "x", "the name first: " + order.join());
e.removeAttribute("o");
check(!e.hasAttributes(), "removed");"#,
        );
        assert_eq!(stored(runner), (0, 0));
    });
}

/// The next number of a xorshift generator: the same sequence for the same seed, no clock.
fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// The write for the random number `r` on the element named `target`: an add or replacement
/// by each writer, a removal by each remover, a toggle either way; 16 names, values of 0 to 9
/// bytes.
fn seeded_write(r: u64, target: &str) -> String {
    let name = format!("n{}", (r >> 8) % 16);
    let value = "v".repeat(((r >> 16) % 10) as usize);
    match r % 12 {
        0..=3 => format!("{target}.setAttribute(\"{name}\", \"{value}\");"),
        4 => format!("{target}.setAttributeNS(S, \"p:{name}\", \"{value}\");"),
        5 => format!("{target}.toggleAttribute(\"{name}\");"),
        6 => format!("{target}.className = \"{value}\";"),
        7 | 8 => format!("{target}.removeAttribute(\"{name}\");"),
        9 => format!("{target}.removeAttributeNS(S, \"{name}\");"),
        10 => format!("{target}.toggleAttribute(\"{name}\", false);"),
        _ => format!("{target}.removeAttribute(\"class\");"),
    }
}

/// The same writes on two elements in turn: the live bytes are the sum over both lists of the
/// record slots each gained since the start, times 80, and the strings' difference, both read
/// from the lists, and the charge is twice them, after each of 800 seeded writes. Both lists
/// start empty, so no refund meets the stop at 0.
#[test]
fn writes_on_two_elements_in_turn_are_charged_by_both_lists() {
    with_page("<p></p>".to_string(), ParserLimits::default(), |runner| {
        ran(
            runner,
            "var H = document.documentElement; var B = document.getElementsByTagName(\"body\")[0];\nvar S = \"http://www.w3.org/2000/svg\";",
        );
        let html = element(tree(runner), "html");
        let body = element(tree(runner), "body");
        let start = (
            room_of(tree(runner), html) + room_of(tree(runner), body),
            strings_of(tree(runner), html) + strings_of(tree(runner), body),
        );
        assert_eq!(start, (0, 0));
        let mut seed: u64 = 0x2e1e_ca7e_0002_0800;
        let mut most = 0;
        for k in 0..800 {
            let r = next_random(&mut seed);
            let target = if (r >> 32) % 2 == 0 { "H" } else { "B" };
            let write = seeded_write(r, target);
            ran(runner, &write);
            let tree = tree(runner);
            let rooms = room_of(tree, html) + room_of(tree, body);
            let strings = strings_of(tree, html) + strings_of(tree, body);
            most = most.max(room_of(tree, html)).max(room_of(tree, body));
            let live = (rooms * ATTRIBUTE_BYTES + strings) as u64;
            assert_eq!(stored(runner), (live, 2 * live), "write {k}: {write}");
        }
        assert!(most >= 8, "a list doubled: {most} records");
    });
}
