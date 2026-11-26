//! Adversarial cases of the slice `selectors` (`story:scripts-dom-selectors`), second review,
//! after the fix round: the argument of a pseudo-class against the tokens of CSS Syntax Level 3
//! (the Editor's Draft of 20 October 2026 the unit cites), and the properties no case held.
//! Each case that runs a script runs its own runner over a page of a few lines.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeId, ParseOptions, Tree, WriteError, parse_document};
use surf_script::bindings::{DomCx, DomEnd, DomException, DomResult, Mutation, Slices};
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

fn refused(error: WriteError) -> DomEnd {
    DomEnd::Throw(DomException::new(
        "HierarchyRequestError",
        format!("{error:?}"),
    ))
}

/// A `mutation` slice for the cases that write between two reads: `appendChild`,
/// `removeChild` and `createDocumentFragment`, through the tree's public write surface.
struct Writes;

impl Mutation for Writes {
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

/// Runs `body` with a runner over `page` whose slices write through [`Writes`].
fn with_writes<R: Send>(page: &str, body: impl FnOnce(&mut Runner) -> R + Send) -> R {
    let page = page.to_string();
    run_document(
        limits(),
        Mode::Test,
        move || {
            let mut slices: Slices = dom::slices(DocumentInfo::default());
            slices.mutation = Box::new(Writes);
            Document {
                tree: parsed(&page),
                limits: ParserLimits::default(),
                slices,
            }
        },
        body,
    )
    .expect("a runner with the bindings")
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

/// CSS Syntax 4.3.4: an identifier `url` (in any ASCII case, escapes decoded) followed by `(`
/// and no quote is not a function token: the rest up to the first `)` is one url token (4.3.6),
/// or a bad-url token whose remnants run to the first `)` (4.3.15). A `[`, `{`, `/*` or quote
/// in it opens no block, no comment and no string. The fix reads the argument of a pseudo-class
/// by blocks (5.5.9, 5.5.10) over a lexer that makes every `url(` a function token, so inside
/// a url it opens what CSS Syntax does not open and closes the argument elsewhere:
///
/// - `a:url(x)` is a colon and a url token, no pseudo-class by its shape: not valid;
/// - `a:is(url([))b` closes `:is(` at its second `)`, and `b` follows with no combinator:
///   not valid; the unit keeps `[` open to the end of the text;
/// - `a:is(url({)) )` leaves a `)` with nothing open: not valid;
/// - `a:is(url(/*)) [x="*/))"]` is `:is(` closed, then a descendant attribute selector:
///   a pseudo-class, not supported; the unit opens a comment in the url, ends `:is(` inside
///   the attribute's string and meets an unclosed string where a compound goes: not valid.
///
/// `a:is(url("x)"))` is a function token with a string in it, which both read alike.
#[test]
fn a_url_in_a_pseudo_class_is_one_token() {
    with_page("<!DOCTYPE html><a></a>", |runner| {
        checked(
            runner,
            r##"
var cases = [
  ["a:url(x)", INVALID], ["a::url(x)", INVALID], ["a:URL(x)", INVALID],
  ["a:is(url([))b", INVALID], ["a:is(URL([))b", INVALID], ["a:is(u\\72l([))b", INVALID],
  ["a:is(url({)) )", INVALID],
  ['a:is(url(/*)) [x="*/))"]', UNSUPPORTED + "pseudo-class"],
  ['a:is(url("x)"))', UNSUPPORTED + "pseudo-class"]
];
var wrong = [];
for (var i = 0; i < cases.length; i++) {
  var got = err(function () { document.querySelector(cases[i][0]); });
  if (got !== cases[i][1]) { wrong.push(cases[i][0] + ": " + got); }
}
check(wrong.length === 0, wrong.join("; "));
"##,
        );
    });
}

/// CSS Syntax Level 3 (the Editor's Draft of 20 October 2026), 4.2, "non-ASCII ident code
/// point", typed from the text for this file: each range inclusive.
const NAME_RANGES: [(u32, u32); 15] = [
    (0x00B7, 0x00B7),
    (0x00C0, 0x00D6),
    (0x00D8, 0x00F6),
    (0x00F8, 0x037D),
    (0x037F, 0x1FFF),
    (0x200C, 0x200C),
    (0x200D, 0x200D),
    (0x203F, 0x203F),
    (0x2040, 0x2040),
    (0x2070, 0x218F),
    (0x2C00, 0x2FEF),
    (0x3001, 0xD7FF),
    (0xF900, 0xFDCF),
    (0xFDF0, 0xFFFD),
    (0x10000, 0x10FFFF),
];

fn in_the_list(c: u32) -> bool {
    NAME_RANGES.iter().any(|&(lo, hi)| (lo..=hi).contains(&c))
}

/// The code points from U+0080 to U+FFFF outside the list, surrogates left out, as ranges.
fn gaps() -> Vec<(u32, u32)> {
    let mut gaps: Vec<(u32, u32)> = Vec::new();
    for c in 0x80..=0xFFFF_u32 {
        if (0xD800..=0xDFFF).contains(&c) || in_the_list(c) {
            continue;
        }
        match gaps.last_mut() {
            Some(last) if last.1 + 1 == c => last.1 = c,
            _ => gaps.push((c, c)),
        }
    }
    gaps
}

fn js_ranges(ranges: &[(u32, u32)]) -> String {
    let items: Vec<String> = ranges
        .iter()
        .map(|(lo, hi)| format!("[{lo}, {hi}]"))
        .collect();
    format!("[{}]", items.join(", "))
}

/// The script of a case of the name code points: the gaps (`GAPS`), the list (`LIST`), the
/// code points tried in every place (`POINTS`, each with whether it is read as in the list),
/// `want`, which counts a text whose answer is not the one expected, then `part`, then the
/// check that no answer was wrong.
fn name_script(part: &str) -> String {
    let gaps = gaps();
    let mut points: Vec<(u32, bool)> = Vec::new();
    for (lo, hi) in NAME_RANGES {
        for c in [lo - 1, lo, hi, hi + 1] {
            if c <= 0x10FFFF && !(0xD800..=0xDFFF).contains(&c) {
                points.push((c, in_the_list(c)));
            }
        }
    }
    let mut seen = 0;
    for (lo, hi) in &gaps {
        for c in *lo..=*hi {
            if seen % 800 == 0 {
                points.push((c, false));
            }
            seen += 1;
        }
    }
    for c in [0x0000, 0xD800, 0xDBFF, 0xDC00, 0xDFFF, 0xFFFD] {
        points.push((c, true));
    }
    points.sort_unstable();
    points.dedup();
    let points: Vec<String> = points
        .iter()
        .map(|(c, inside)| format!("[{c}, {inside}]"))
        .collect();
    format!(
        r##"
var GAPS = {gaps};
var LIST = {list};
var POINTS = [{points}];
var bad = 0;
var wrong = [];
function want(text, expected, what) {{
  var got = err(function () {{ document.querySelector(text); }});
  if (got !== expected) {{ bad++; if (wrong.length < 20) {{ wrong.push(what + ": " + got); }} }}
}}
function hex(c) {{ return "U+" + c.toString(16).toUpperCase(); }}
function ch(c) {{ return c > 0xFFFF ? String.fromCodePoint(c) : String.fromCharCode(c); }}
{part}
check(bad === 0, bad + " wrong: " + wrong.join("; "));
"##,
        gaps = js_ranges(&gaps),
        list = js_ranges(&NAME_RANGES),
        points = points.join(", "),
    )
}

/// Runs the case `name` of the name code points over a page of one `a`, and prints its time.
fn names(name: &str, part: &str) {
    let script = name_script(part);
    with_page("<!DOCTYPE html><a></a>", |runner| {
        let at = Instant::now();
        checked(runner, &script);
        eprintln!("{name}: {:?}", at.elapsed());
    });
}

/// Held. The gaps of CSS Syntax 4.2's list in the Basic Multilingual Plane, surrogates left
/// out, are 13 ranges of 9,297 code points; every 11th of them, 846 code points, is a delim
/// after a letter, one call each, each not valid. The fix's case tries the 46 ends of the
/// ranges, so a range added inside a gap (such as U+2500 to U+257F) leaves it green.
#[test]
fn the_gaps_end_a_name() {
    let gaps = gaps();
    let total: u32 = gaps.iter().map(|(lo, hi)| hi - lo + 1).sum();
    assert_eq!((gaps.len(), total), (13, 9_297));
    names(
        "the gaps",
        r##"
var count = 0;
var at = 0;
for (var g = 0; g < GAPS.length; g++) {
  for (var c = GAPS[g][0]; c <= GAPS[g][1]; c++) {
    if (at % 11 === 0) { want("a" + String.fromCharCode(c), INVALID, "a then " + hex(c)); count++; }
    at++;
  }
}
check(at === 9297 && count === 846, "gaps: " + at + ", tried " + count);
"##,
    );
}

/// Held. In each range of the list, every 53rd code point of the Basic Multilingual Plane
/// from the first, every 4,099th of U+10000 to U+10FFFF, and the last of the range continue a
/// name: in names of `a` and up to 1,000 of them, each name a type selector that runs.
#[test]
fn the_list_continues_a_name() {
    let expected: u32 = NAME_RANGES
        .iter()
        .map(|&(lo, hi)| (hi - lo) / if lo >= 0x10000 { 4_099 } else { 53 } + 2)
        .sum();
    names(
        "the list",
        &format!(
            r##"
var parts = [];
var inside = 0;
var last = 0;
function flush() {{
  if (parts.length > 0) {{ want("a" + parts.join(""), "no throw", "a name up to " + hex(last)); parts = []; }}
}}
for (var r = 0; r < LIST.length; r++) {{
  var step = LIST[r][0] >= 0x10000 ? 4099 : 53;
  for (var c = LIST[r][0]; c <= LIST[r][1]; c += step) {{
    parts.push(ch(c)); inside++; last = c;
    if (parts.length === 1000) {{ flush(); }}
  }}
  parts.push(ch(LIST[r][1])); inside++; last = LIST[r][1];
}}
flush();
check(inside === {expected}, "the list: " + inside);
"##
        ),
    );
}

/// Held. At the ends of the list's ranges and the code points beside them, at every 800th code
/// point of the gaps, and at U+0000, the four ends of the surrogates and U+FFFD (each read as
/// U+FFFD, 3.3, so in the list), a code point is tried in 17 places: the start of a name,
/// inside one, after `#` and `.`, inside `[ ]`, an unquoted value, after an escape with and
/// without its space, after `-`, as an escape, in a string and in the argument of a
/// pseudo-class. Outside the list it is a delim in the first 14 (not valid), and in the last
/// three the same as in the list.
#[test]
fn a_code_point_in_seventeen_places() {
    names(
        "seventeen places",
        r##"
var PLACES = [
  ["", "a", INVALID, "no throw"], ["a", "", INVALID, "no throw"], ["a", "b", INVALID, "no throw"],
  ["#", "", INVALID, "no throw"], ["#a", "", INVALID, "no throw"],
  [".", "", INVALID, "no throw"], [".a", "", INVALID, "no throw"],
  ["[", "]", INVALID, "no throw"], ["[a", "]", INVALID, "no throw"],
  ["[a=", "]", INVALID, "no throw"], ["[a=b", "]", INVALID, "no throw"],
  ["\\41", "", INVALID, "no throw"], ["\\41 ", "", INVALID, "no throw"],
  ["-", "", INVALID, "no throw"],
  ["a\\", "", "no throw", "no throw"], ["[a=\"", "\"]", "no throw", "no throw"],
  ["a:is(", ")", UNSUPPORTED + "pseudo-class", UNSUPPORTED + "pseudo-class"]
];
for (var p = 0; p < POINTS.length; p++) {
  var c = POINTS[p][0];
  for (var k = 0; k < PLACES.length; k++) {
    var place = PLACES[k];
    want(place[0] + ch(c) + place[1], POINTS[p][1] ? place[3] : place[2],
      hex(c) + " in " + JSON.stringify(place[0] + "_" + place[1]));
  }
}
check(POINTS.length >= 60, "points: " + POINTS.length);
"##,
    );
}

/// The steps of `source`, which must run.
fn steps_of(runner: &mut Runner, source: &str) -> u64 {
    let before = runner.steps();
    let end = runner.evaluate(source);
    assert_eq!(end, ScriptEnd::Ran, "{end}");
    runner.steps() - before
}

/// Held. The fix's `pseudo` says each token of the argument is taken through `take` and
/// charged a step, as every token before it is (the story: one step a token, charged before
/// the lexer takes it). No case of the crate runs a pseudo-class over more than a few tokens,
/// so a `pseudo` that skips the argument through the lexer uncharged leaves every case green.
/// Two texts of 3,996 bytes, both refused as a pseudo-class: `:is(` and 3,990 tokens (`b` and a
/// space, 1,995 times), and `:is(` and one string. The glue's copy and the text's charge are
/// the same; the tokens differ by 3,994 - 5 = 3,989.
#[test]
fn the_tokens_of_a_pseudo_class_argument_are_charged() {
    with_page("<!DOCTYPE html><a></a>", |runner| {
        checked(
            runner,
            r##"
G.many = "a:is(" + rep("b ", 1995) + ")";
G.one = "a:is(\"" + rep("b", 3988) + "\")";
check(G.many.length === 3996 && G.one.length === 3996, "two texts of 3,996 bytes");
G.refused = function (text) {
  try { document.querySelector(text); } catch (e) { return e.message; }
  return "no throw";
};
"##,
        );
        let many = steps_of(runner, "G.m = G.refused(G.many);");
        let one = steps_of(runner, "G.o = G.refused(G.one);");
        checked(
            runner,
            r##"var P = "selector is not supported by surf: pseudo-class";
check(G.m === P && G.o === P, G.m + "; " + G.o);"##,
        );
        eprintln!("3,994 tokens: {many} steps; 5 tokens: {one} steps");
        assert_eq!(many - one, 3_989, "{many} - {one}");
    });
}

/// The code of a Rust source: each comment dropped, and each string literal and character
/// literal made empty, so that a word inside a literal is not read and a `//` inside one does
/// not hide the rest of its line.
fn code_of(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let mut code = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let after_name = i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
        if c == '/' && at(i + 1) == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && at(i + 1) == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && at(i + 1) == Some('*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && at(i + 1) == Some('/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            code.push(' ');
        } else if c == 'r' && !after_name && matches!(at(i + 1), Some('"' | '#')) {
            let mut j = i + 1;
            let mut hashes = 0;
            while at(j) == Some('#') {
                hashes += 1;
                j += 1;
            }
            if at(j) == Some('"') {
                j += 1;
                while j < chars.len()
                    && !(chars[j] == '"' && (1..=hashes).all(|k| at(j + k) == Some('#')))
                {
                    j += 1;
                }
                i = j + 1 + hashes;
                code.push_str("\"\"");
            } else {
                code.push(c);
                i += 1;
            }
        } else if c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                i += if chars[i] == '\\' { 2 } else { 1 };
            }
            i += 1;
            code.push_str("\"\"");
        } else if c == '\'' && at(i + 1) == Some('\\') {
            let mut j = i + 3;
            while j < chars.len() && chars[j] != '\'' {
                j += 1;
            }
            i = j + 1;
            code.push_str("' '");
        } else if c == '\'' && at(i + 2) == Some('\'') {
            i += 3;
            code.push_str("' '");
        } else {
            code.push(c);
            i += 1;
        }
    }
    code
}

/// Held. The story (decision 17): `selectors.rs` holds no call of `tree()` and no charge, and
/// "`reads::quirks` (no charge) is the one other function the members use"; its head comment:
/// "Every read of the tree and every step charged is a function of `selector_reads`; this
/// file holds none." The review's guard reads the code lines of `selectors.rs` cut at the
/// first `//`, so `let _s = "//"; let _ = DomCx::tree(cx).root();` passes it, and it reads no
/// path into `reads.rs`, so `let _ = reads::scan_for(cx, this, reads::Reads::Classes)?;` in
/// `matches` passes it: that scan reads the tree and charges a step per 4 attributes, none
/// over a bare element, and every cost case stays green. This guard reads the code with its
/// comments and literals taken out: no word of the tree, the engine, a charge or the other
/// readers of the context, and the name `reads` only in `use super::reads;` and in
/// `reads::quirks(`.
#[test]
fn the_selectors_reach_the_tree_through_no_other_door() {
    let code = code_of(include_str!("../src/dom/selectors.rs"));
    let mut found = Vec::new();
    for word in [
        "tree",
        "Tree",
        "engine",
        "charge",
        "totals",
        "limit_reached",
        "interrupt",
        "string_length",
    ] {
        if code.contains(word) {
            found.push(format!("`{word}`"));
        }
    }
    let name = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let mut quirks = 0;
    for (at, _) in code.match_indices("reads") {
        if name(code[..at].chars().next_back()) || name(code[at + 5..].chars().next()) {
            continue;
        }
        let rest = &code[at + 5..];
        if rest.starts_with("::quirks(") {
            quirks += 1;
        } else if !(code[..at].ends_with("use super::") && rest.starts_with(';')) {
            let line = code[..at].matches('\n').count() + 1;
            found.push(format!(
                "`reads` at code line {line}: {:?}",
                &rest[..rest.len().min(40)]
            ));
        }
    }
    assert!(quirks >= 1, "reads::quirks is read");
    assert!(found.is_empty(), "{found:#?}");
}

/// Held. DOM: the argument is converted before the member reads the tree, so a `toString` that
/// changes the tree is seen by the member: `matches("div b")` and `closest("div")` of a `b`
/// the conversion took out of its `div` answer false and null; `querySelector("p")` after the
/// conversion removed the first `p` answers the second; `querySelectorAll("div > p")` after
/// the conversion moved a `p` into the `div` finds it.
#[test]
fn a_conversion_that_writes_the_tree_comes_first() {
    let page = "<!DOCTYPE html><body><div id=\"d\"><b id=\"b1\"></b><b id=\"b2\"></b></div>\
<p id=\"p1\"></p><p id=\"p2\"></p></body>";
    with_writes(page, |runner| {
        checked(
            runner,
            r##"
function el(id) { return document.getElementById(id); }
var d = el("d"), b1 = el("b1"), b2 = el("b2"), p1 = el("p1"), p2 = el("p2");
check(b1.matches("div b") && b2.closest("div") === d, "before");
check(b1.matches({ toString: function () { d.removeChild(b1); return "div b"; } }) === false,
  "matches after the conversion took this out");
check(b2.closest({ toString: function () { d.removeChild(b2); return "div"; } }) === null,
  "closest after the conversion took this out");
check(document.querySelector({ toString: function () { p1.parentNode.removeChild(p1); return "p"; } })
  === p2, "querySelector after the conversion removed the first p");
var found = document.querySelectorAll({ toString: function () { d.appendChild(p2); return "div > p"; } });
check(found.length === 1 && found[0] === p2, "querySelectorAll after the conversion moved a p");
"##,
        );
    });
}

/// Held. Each of the nine, taken from its prototype, refuses a `this` that is not of its
/// interface with a `TypeError` and reads nothing: an object made from the prototype of its own
/// interface, a plain object, `undefined`, `null`, a `Text`, and the wrapper of another of
/// the three interfaces. The engine has no `Proxy` (its name is a `ReferenceError`), so a page
/// cannot wrap one.
#[test]
fn a_foreign_this_is_a_type_error() {
    with_writes("<!DOCTYPE html><body><p id=\"p\">x</p></body>", |runner| {
        checked(
            runner,
            r##"
var p = document.getElementById("p");
var text = p.firstChild;
var fragment = document.createDocumentFragment();
var members = [
  [Element.prototype, "matches", p], [Element.prototype, "webkitMatchesSelector", p],
  [Element.prototype, "closest", p], [Element.prototype, "querySelector", p],
  [Element.prototype, "querySelectorAll", p],
  [Document.prototype, "querySelector", document], [Document.prototype, "querySelectorAll", document],
  [DocumentFragment.prototype, "querySelector", fragment],
  [DocumentFragment.prototype, "querySelectorAll", fragment]
];
var wrong = [];
for (var i = 0; i < members.length; i++) {
  var f = members[i][0][members[i][1]];
  var own = members[i][2];
  var foreign = [Object.create(Object.getPrototypeOf(own)), {}, undefined, null, text,
    own === p ? document : p];
  for (var j = 0; j < foreign.length; j++) {
    var got = err(function () { f.call(foreign[j], "p"); });
    if (got.indexOf("TypeError") !== 0) { wrong.push(members[i][1] + " " + j + ": " + got); }
  }
  var right = err(function () { f.call(own, "p"); });
  if (right !== "no throw") { wrong.push(members[i][1] + " on its own: " + right); }
}
check(wrong.length === 0, wrong.join("; "));
"##,
        );
    });
}
