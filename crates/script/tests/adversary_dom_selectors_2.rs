//! Adversarial cases of the slice `selectors` (`story:scripts-dom-selectors`), second review,
//! after the fix round: the argument of a pseudo-class against the tokens of CSS Syntax Level 3
//! (the Editor's Draft of 20 October 2026 the unit cites). Each case runs its own runner over a
//! page of a few lines.

use std::time::Duration;

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
