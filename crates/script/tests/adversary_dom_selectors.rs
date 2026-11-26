//! Adversarial cases of the slice `selectors` (`story:scripts-dom-selectors`), first review:
//! texts the grammar takes or refuses against CSS Syntax Level 3 and Selectors Level 4, and
//! the statements of the story that no case of `dom_selectors` holds. Each case runs its own
//! runner over a page of a few lines; the cases of steps print their steps and their time and
//! assert steps, never a time.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{ParseOptions, Tree, parse_document};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{Document, Mode, Runner, RunnerLimits, ScriptEnd, run_document};
use swell::Limits;

fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 10_000_000,
            memory_bytes: 64 * 1024 * 1024,
            frame_depth: 1_000,
            reentry_depth: 64,
            parser_nesting: 100,
            string_length: 1 << 22,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 1_000_000_000,
        scripts_per_runner: 100,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(50),
    }
}

/// Runs `body` with a runner over `page`, parsed under the parser's default bounds.
fn with_page<R: Send>(page: &str, body: impl FnOnce(&mut Runner) -> R + Send) -> R {
    let page = page.to_string();
    run_document(
        limits(),
        Mode::Test,
        move || Document {
            tree: parsed(&page),
            limits: ParserLimits::default(),
            slices: dom::slices(DocumentInfo::default()),
        },
        body,
    )
    .expect("a runner with the bindings")
}

fn parsed(page: &str) -> Tree {
    parse_document(
        page,
        &ParseOptions {
            limits: ParserLimits::default(),
            ..ParseOptions::default()
        },
    )
}

/// The functions every script starts with: `check`, and `err`, the name, code and message of
/// what a call throws.
const CHECK: &str = r##"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
function err(f) {
  try { f(); } catch (e) { return e.name + " " + e.code + " " + e.message; }
  return "no throw";
}
var INVALID = "SyntaxError 12 selector is not valid";
var UNSUPPORTED = "SyntaxError 12 selector is not supported by surf: ";
function rep(s, n) {
  var out = "";
  while (n > 0) { if (n % 2 === 1) { out += s; } s += s; n = (n - n % 2) / 2; }
  return out;
}
"##;

/// Runs [`CHECK`] then `source`, and asserts it ran.
fn checked(runner: &mut Runner, source: &str) {
    let end = runner.evaluate(&format!("{CHECK}\n{source}"));
    assert_eq!(end, ScriptEnd::Ran, "{end}");
}

/// CSS Syntax Level 3 (the Editor's Draft of 20 October 2026 the unit cites), 4.2: an ident
/// code point beyond ASCII is a "non-ASCII ident code point", a list of ranges (U+00B7,
/// U+00C0 to U+00D6, U+00D8 to U+00F6, U+00F8 to U+037D, ... U+FDF0 to U+FFFD, U+10000 and
/// above). U+00A0, U+00D7, U+00F7, U+200B, U+2028 and U+E000 are outside it, so each is a
/// delim token (4.3.1), and a delim where a selector wants an identifier, or two compounds
/// without whitespace between them, is a selector that is not valid (Selectors 3.9 and 16).
/// The unit's `is_name_start` takes every non-ASCII code point as a name code point, so it
/// accepts each text below as one identifier and throws nothing. The last two lines are the
/// ranges' edges inside the set, which run.
#[test]
fn code_points_outside_the_ident_set_are_not_valid() {
    with_page("<!DOCTYPE html><div><span></span></div>", |runner| {
        checked(
            runner,
            r##"
var outside = ["div span", "a×", ".a÷b", "#x​", " ", "p",
  "# ", "[a ]"];
var wrong = [];
for (var i = 0; i < outside.length; i++) {
  var got = err(function () { document.querySelector(outside[i]); });
  if (got !== INVALID) { wrong.push(JSON.stringify(outside[i]) + ": " + got); }
}
check(wrong.length === 0, wrong.join("; "));
check(document.querySelector("Øö") === null, "U+00D8 and U+00F6 are ident code points");
check(document.querySelector(".·") === null, "U+00B7 is an ident code point");
"##,
        );
    });
}

/// CSS Syntax Level 3, 5.4 (consume a function, consume a simple block): the argument of a
/// function is a list of component values, and a `[` or `{` in it opens a simple block that
/// runs to its own `]` or `}`, so a `)` inside that block does not close the function. Each
/// text below is a pseudo-class by its shape, `:is(` or `:not(`, a block holding a `)`, and
/// the `)` that closes the function: the story's not-supported message, `pseudo-class`. The
/// unit balances parentheses alone, closes the function at the `)` inside the block, then
/// meets the block's `]` or `}` as a fault, and answers the not-valid message.
#[test]
fn a_block_in_a_pseudo_class_argument_holds_its_parenthesis() {
    with_page("<!DOCTYPE html><a></a>", |runner| {
        checked(
            runner,
            r##"
var texts = ["a:is([)])", "a:not({)})", "a:is([)]), b"];
var wrong = [];
for (var i = 0; i < texts.length; i++) {
  var got = err(function () { document.querySelector(texts[i]); });
  if (got !== UNSUPPORTED + "pseudo-class") { wrong.push(texts[i] + ": " + got); }
}
check(wrong.length === 0, wrong.join("; "));
"##,
        );
    });
}

/// The steps and the time of `source`, which must run.
fn timed(runner: &mut Runner, source: &str) -> (u64, Duration) {
    let before = runner.steps();
    let at = Instant::now();
    let end = runner.evaluate(source);
    let took = at.elapsed();
    assert_eq!(end, ScriptEnd::Ran, "{end}");
    (runner.steps() - before, took)
}

/// 500 nested `div`, no doctype, and `G.d`, the innermost: its chain is 502 elements (itself,
/// 499 `div`, `body`, `html`).
const NESTED_SETUP: &str =
    "var G = this; var all = document.getElementsByTagName(\"div\"); G.d = all[all.length - 1];";

/// Decision 8 of the story: "a selector of one compound pays no memo (acceptance 15g)"; decision
/// 7: "A complex selector of one compound is answered here" (at the gate). Acceptance 15g as
/// built measures `zzz` and `x y`, which fail the gate, so no case of `dom_selectors` has an
/// element pass the gate of a one-compound selector under a ceiling: with the early answer of
/// `matches_complex` taken out, the answers stay right, the chain and a memo are made for each
/// such element, and every case of `dom_selectors` stays green. Here the innermost of 500
/// nested `div` matches `div`, and `closest("div")` answers it as its first candidate: the
/// glue, the parse and the gate, a few steps; the chain and the memo would add 502 + 62 + 502.
#[test]
fn a_selector_of_one_compound_reads_no_chain() {
    with_page(&"<div>".repeat(500), |runner| {
        checked(runner, NESTED_SETUP);
        let matches = timed(runner, "G.r = G.d.matches(\"div\");");
        let closest = timed(runner, "G.c = G.d.closest(\"div\");");
        checked(
            runner,
            "check(G.r === true && G.c === G.d, \"the innermost\");",
        );
        eprintln!("matches(div): {matches:?}; closest(div): {closest:?}");
        assert!(matches.0 < 100, "{matches:?}");
        assert!(closest.0 < 100, "{closest:?}");
    });
}

/// Decision 7 of the story, the chain: "read whole and once for an element (a later selector of
/// the list at the same element uses it)". No case of `dom_selectors` has two complex selectors
/// of a list pass their gates at one element under a ceiling, so a matcher that reads the chain
/// again for each of them stays green there. Here four selectors of two compounds each pass the
/// gate at the innermost of 500 nested `div`, the fourth matches: the chain once, 502, and for
/// each selector its gate, its memo of 2 x 2 x 502 bytes (125 steps) and 2 x 502 cells, 4 x
/// 1,130; 5,022 and a few tens for the glue, the parse and the script. Read four times the
/// chain adds 1,506.
#[test]
fn a_list_reads_the_chain_once() {
    with_page(&"<div>".repeat(500), |runner| {
        checked(runner, NESTED_SETUP);
        let list = timed(
            runner,
            "G.r = G.d.matches(\"x div, y div, z div, div div\");",
        );
        checked(runner, "check(G.r === true, \"matches\");");
        eprintln!("matches of four selectors: {list:?}");
        assert!((5_022..5_500).contains(&list.0), "{list:?}");
    });
}

/// The lines of `source` that are code: a line comment and the rest of a line after `//` left
/// out (a doc comment is a line comment).
fn code_lines(source: &str) -> Vec<(usize, &str)> {
    source
        .lines()
        .enumerate()
        .map(|(at, line)| (at + 1, line.split("//").next().unwrap_or("")))
        .filter(|(_, code)| !code.trim().is_empty())
        .collect()
}

/// Decision 17 of the story: `selectors.rs` holds no call of the tree and no charge. The
/// unit's guard (`dom_selectors::the_selectors_read_the_tree_only_through_the_reads`) looks
/// for the strings `tree()`, `tree_mut()`, `charge(` and `charge_steps(`, the form the reading
/// slice's review showed to pass `DomCx::tree(cx)`, `cx.tree ()`, an alias of `charge` and
/// `cx.engine()`; that review's guard (`adversary_dom_reading_5`) reads `reading.rs` and
/// `mod.rs` only. With `let _ = DomCx::tree(cx).root();` in `selectors.rs` every case of
/// `dom_selectors` stays green. This guard reads the code of `selectors.rs` for the words
/// `tree`, `Tree`, `engine` and `charge`.
#[test]
fn the_selectors_name_no_tree_no_engine_and_no_charge() {
    let mut found = Vec::new();
    for (line, code) in code_lines(include_str!("../src/dom/selectors.rs")) {
        for word in ["tree", "Tree", "engine", "charge"] {
            if code.contains(word) {
                found.push(format!(
                    "selectors.rs:{line}: `{word}` in {:?}",
                    code.trim()
                ));
            }
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}

/// Decisions 6 and 17 of the story: the walk copies no list of children, and `Tree::descendants`,
/// which answers a list of every node under a node, is not used. The unit's guard cuts
/// `Frames::next` out of `selector_reads.rs` and looks for `children(` in the rest: a walk
/// rewritten over `cx.tree().descendants(scope)` (eight lines of `descendant_elements`, one
/// step charged a node as before) passes it, and every case of `dom_selectors` stays green,
/// since the steps are the same. This guard reads the code of `selector_reads.rs` for
/// `descendants`, `to_vec` and `to_owned`.
#[test]
fn the_selector_reads_copy_no_list_of_the_tree() {
    let mut found = Vec::new();
    for (line, code) in code_lines(include_str!("../src/dom/selector_reads.rs")) {
        for word in ["descendants", "to_vec", "to_owned"] {
            if code.contains(word) {
                found.push(format!(
                    "selector_reads.rs:{line}: `{word}` in {:?}",
                    code.trim()
                ));
            }
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}

/// Held. Matching at the edges the story's cases leave out: an element the parser left with
/// the first of two attributes of one name; a `class` split at TAB, CR and FF and not at
/// U+00A0, its token reached through an escape; an empty `class`; `[a=""]` against an empty
/// and a missing attribute; `null` and `undefined` made Strings by the glue (the type
/// selectors `null` and `undefined`), a Symbol and a missing argument a `TypeError`, a throwing
/// `toString` its own error; a `toString` that calls a member on the same `this` and reads the
/// tree while the outer call converts its argument; the list of `querySelectorAll` in tree
/// order with no node twice for selectors that overlap.
#[test]
fn edges_of_matching_and_of_arguments() {
    let page = "<!DOCTYPE html><body>\
<p id=\"dup\" a=\"1\" a=\"2\"></p>\
<p id=\"ws\" class=\"&#9;x&#13;y&#12;z&nbsp;w\"></p>\
<p id=\"e\" class=\"\"></p><p id=\"em\" b=\"\"></p><p id=\"nb\"></p>\
<null id=\"n\"></null><undefined id=\"u\"></undefined>\
<div id=\"a\"><p id=\"b\" class=\"k\"><span id=\"c\" class=\"k\"></span></p></div>\
</body>";
    with_page(page, |runner| {
        checked(
            runner,
            r##"
function el(id) { return document.getElementById(id); }
check(document.querySelector('[a="1"]') === el("dup") && document.querySelector('[a="2"]') === null,
  "the parser keeps the first of two attributes of one name");
check(document.querySelector(".x") === el("ws") && document.querySelector(".y") === el("ws"),
  "TAB and CR split");
check(document.querySelector(".z") === null && document.querySelector(".w") === null,
  "U+00A0 does not split");
check(document.querySelector(".z\\a0 w") === el("ws"), "the token z U+00A0 w through an escape");
check(document.querySelector('[class=""]') === el("e") && document.querySelector(".e") === null,
  "an empty class");
check(document.querySelector('[b=""]') === el("em") && document.querySelector('#nb[b=""]') === null,
  "[b=\"\"] against an empty and a missing attribute");
check(document.querySelector(null) === el("n") && document.querySelector(undefined) === el("u"),
  "null and undefined are type selectors");
check(err(function () { document.querySelector(); }).indexOf("TypeError") === 0, "no argument");
check(err(function () { document.querySelector(Symbol("s")); }).indexOf("TypeError") === 0,
  "a Symbol");
var thrower = { toString: function () { throw new RangeError("mine"); } };
check(err(function () { document.querySelector(thrower); }).indexOf("RangeError") === 0,
  "a throwing toString");
var a = el("a");
var seen = [];
var reader = { toString: function () {
  seen.push(a.querySelectorAll("*").length,
    a.closest("body") === document.getElementsByTagName("body")[0], a.matches("div"));
  return "span"; } };
check(a.querySelector(reader) === el("c") && seen.join(",") === "2,true,true", "a reentrant toString");
var list = document.querySelectorAll("span, .k, div > p, #a, *");
var order = [];
for (var i = 0; i < list.length; i++) { order.push(list[i]); }
var everything = document.getElementsByTagName("*");
check(order.length === everything.length, "every element once: " + order.length);
for (var i = 0; i < order.length; i++) { check(order[i] === everything[i], "tree order at " + i); }
"##,
        );
    });
}

/// Held. The text bound of 4,096 bytes is of UTF-8 after the glue's copy (decision 6): 2,048
/// two-byte characters run and 2,049 are over it; 1,024 astral characters (2,048 code units)
/// run and 1,025 are over it; a lone surrogate is copied as U+FFFD, three bytes, so 1,365 lone
/// surrogates (4,095 bytes) run and 1,366 are over it.
#[test]
fn the_text_bound_counts_the_bytes_of_the_copy() {
    with_page("<!DOCTYPE html><p></p>", |runner| {
        checked(
            runner,
            r##"
var OVER = UNSUPPORTED + "text over 4096 bytes";
var pairs = [["é", 2048], ["😀", 1024], ["\uD800", 1365]];
for (var i = 0; i < pairs.length; i++) {
  var at = rep(pairs[i][0], pairs[i][1]);
  var over = rep(pairs[i][0], pairs[i][1] + 1);
  check(document.querySelector(at) === null, "at the bound: " + i);
  check(err(function () { document.querySelector(over); }) === OVER, "over the bound: " + i);
}
"##,
        );
    });
}

/// Held. A `NodeList` of `querySelectorAll` keeps its nodes, and each node its one wrapper,
/// across a collection of the agent: a property a script set on a node of the list is there
/// after `collect()`, and the node is the object `querySelector` answers.
#[test]
fn a_list_and_its_wrappers_survive_a_collection() {
    with_page("<!DOCTYPE html><p></p><p></p><p></p>", |runner| {
        checked(
            runner,
            "G.list = document.querySelectorAll(\"p\"); G.list[1].mark = 7;",
        );
        let collected = runner.agent_and_realm().0.collect();
        assert!(
            matches!(collected, Ok(swell::Outcome::Normal(_))),
            "{collected:?}"
        );
        checked(
            runner,
            r##"
var again = document.querySelectorAll("p");
check(G.list.length === 3 && G.list !== again, "a new list");
check(G.list[1] === again[1] && G.list[1].mark === 7, "the wrapper and its property");
check(document.querySelector("p") === G.list[0], "querySelector answers the same object");
"##,
        );
    });
}
