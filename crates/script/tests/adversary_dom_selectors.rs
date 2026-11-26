//! Adversarial cases of the slice `selectors` (`story:scripts-dom-selectors`), first review:
//! texts the grammar takes or refuses against CSS Syntax Level 3 and Selectors Level 4, and
//! the statements of the story that no case of `dom_selectors` holds. Each case runs its own
//! runner over a page of a few lines; the cases of steps print their steps and their time and
//! assert steps, never a time.

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
