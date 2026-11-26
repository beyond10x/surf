//! Adversarial cases of the slice `selectors` (`story:scripts-dom-selectors`), third review:
//! the url token of CSS Syntax Level 3 (the Editor's Draft of 20 October 2026 the unit cites),
//! 4.3.4 "consume an ident-like token", 4.3.6 "consume a url token" and 4.3.15 "consume the
//! remnants of a bad url", as the second fix round added it to the lexer.
//!
//! The unit's own url cases end most texts with ` ]`, so their right answer is "not valid"; a
//! url that ends too early leaves a stray `)` and is "not valid" too, and a function token
//! read where a url goes, or a url read where a function goes, answers alike in a pseudo-class
//! argument. The cases here choose texts whose right answer is "not supported" where a wrong
//! end or a wrong token answers "not valid", and the place after a single colon, where a url
//! token is no pseudo-class by its shape and a function token is one.

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

fn parsed(page: &str) -> Tree {
    parse_document(
        page,
        &ParseOptions {
            limits: ParserLimits::default(),
            ..ParseOptions::default()
        },
    )
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

/// The functions every script starts with: `check`; `err`, the name, code and message of what
/// a call throws; `rep`, `n` copies of `s` in few steps; and `classes`, which runs each text
/// of a list of `[text, answer]` through `querySelector` and lists the texts answered
/// otherwise.
const CHECK: &str = r##"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
function err(f) {
  try { f(); } catch (e) { return e.name + " " + e.code + " " + e.message; }
  return "no throw";
}
var INVALID = "SyntaxError 12 selector is not valid";
var UNSUPPORTED = "SyntaxError 12 selector is not supported by surf: ";
var PSEUDO = UNSUPPORTED + "pseudo-class";
var ELEMENT = UNSUPPORTED + "pseudo-element";
function rep(s, n) {
  var out = "";
  while (n > 0) { if (n % 2 === 1) { out += s; } s += s; n = (n - n % 2) / 2; }
  return out;
}
function classes(texts) {
  var wrong = [];
  for (var i = 0; i < texts.length; i++) {
    var text = texts[i][0];
    var got = err(function () { document.querySelector(text); });
    var shown = text.length > 40 ? text.slice(0, 20) + "..." + text.slice(-12) : text;
    if (got !== texts[i][1]) { wrong.push(JSON.stringify(shown) + " (" + text.length + "): " + got); }
  }
  return wrong;
}
"##;

/// Runs [`CHECK`] then `source`, and asserts it ran.
fn checked(runner: &mut Runner, source: &str) {
    let end = runner.evaluate(&format!("{CHECK}\n{source}"));
    assert_eq!(end, ScriptEnd::Ran, "{end}");
}

const PAGE: &str = "<!DOCTYPE html><a></a>";

/// 4.3.6 and 4.3.15: where a url ends. Each text is a pseudo-class whose argument holds one
/// url and then the `)` that closes `:is(`, so the text is valid and not supported; a url that
/// ended one `)` too early would leave a `)` where a compound goes, not valid.
///
/// - `url(x\)y)`: `\)` is a valid escape, part of the url (4.3.6, 4.3.7);
/// - `url(x"\)y)`: a `"` makes a bad url, whose remnants consume `\)` as an escape (4.3.15);
/// - `url(x\` LF `y\)z)`: a backslash before a newline is no valid escape (4.3.8): a bad url,
///   and its remnants keep the escaped `)`;
/// - `url(x y\)z)`: whitespace then a code point other than `)`: a bad url, the same;
/// - `url( x )`, `url(x` LF TAB `)`, `url(x` CR LF `)`: whitespace then `)` ends the url and
///   consumes the `)` (4.3.6), CR LF being one newline (3.3);
/// - `url(\29 )`: the escape `\29 ` is `)` with its space, part of the url;
/// - `url(x\` at the end: a valid escape (the end is no newline), the end ends the url.
#[test]
fn a_url_ends_at_its_own_paren() {
    with_page(PAGE, |runner| {
        checked(
            runner,
            r##"
var wrong = classes([
  ["a:is(url(x\\)y))", PSEUDO], ["a:is(url(x\"\\)y))", PSEUDO],
  ["a:is(url(x\\\ny\\)z))", PSEUDO], ["a:is(url(x y\\)z))", PSEUDO],
  ["a:is(url( x ))", PSEUDO], ["a:is(url(x\n\t))", PSEUDO], ["a:is(url(x\r\n))", PSEUDO],
  ["a:is(url(\\29 ))", PSEUDO], ["a:is(url(x\\", PSEUDO],
  ["a:is(url(x'\\))) b", PSEUDO], ["a:is(url(x(\\))) b", PSEUDO],
  ["a:is(url(x\u0001\\)))", PSEUDO], ["a:is(url(x\u007f\\)))", PSEUDO]
]);
check(wrong.length === 0, wrong.join("; "));
"##,
        );
    });
}

/// 4.3.4: after `url(`, a quote, or whitespace and then a quote, makes a function token, and
/// anything else a url token. After a single colon a function token is a pseudo-class by its
/// shape (not supported) and a url token is no pseudo-class (not valid), so the two answers
/// tell the tokens apart where a pseudo-class argument would not. The whitespace before the
/// quote is any run of space, tab and newline (CR, FF and CR LF are newlines, 3.3), up to the
/// text bound: 4,086 spaces in a text of 4,096 bytes.
#[test]
fn a_quote_after_url_makes_a_function() {
    with_page(PAGE, |runner| {
        checked(
            runner,
            r##"
var wrong = classes([
  ["a:url(\"x\")", PSEUDO], ["a:url('x')", PSEUDO], ["a:url( \"x\")", PSEUDO],
  ["a:url(\n'x')", PSEUDO], ["a:URL(\t\"x\")", PSEUDO], ["a:url(\r\n\"x\")", PSEUDO],
  ["a:url(\f \"x\")", PSEUDO], ["a:url(  \t\n 'x')", PSEUDO], ["a::url( \"x\")", ELEMENT],
  ["a:url(" + rep(" ", 4086) + "\"x\")", PSEUDO],
  ["a:url(" + rep(" ", 4088) + "x)", INVALID], ["a:url(" + rep(" ", 4090), INVALID],
  ["a:url()", INVALID], ["a:url( )", INVALID], ["a:url(", INVALID], ["a:url( x\"y)", INVALID],
  ["a:url(x\"y\")", INVALID], ["a::url(x)", INVALID]
]);
check(wrong.length === 0, wrong.join("; "));
"##,
        );
    });
}

/// 4.3.4: the name is compared after its escapes are decoded and in any ASCII case; any other
/// name before `(` is a function token. After a single colon: `\75 rl(`, `U\52 L(`, `ur\l(`
/// and `u\000072l(` (six hex digits) are `url(`, a url token, not valid; `urls(`,
/// `url-prefix(`, `xurl(`, `-url(` and a `url` that only ends a longer name are functions,
/// not supported; `url\28 x)` is the identifier `url(x` and a stray `)`, not valid.
#[test]
fn the_name_url_is_read_decoded() {
    with_page(PAGE, |runner| {
        checked(
            runner,
            r##"
var wrong = classes([
  ["a:\\75 rl(x)", INVALID], ["a:U\\52 L(x)", INVALID], ["a:ur\\l(x)", INVALID],
  ["a:u\\000072l(x)", INVALID], ["a:\\55\\52\\4c(x)", INVALID],
  ["a:urls(x)", PSEUDO], ["a:url-prefix(x)", PSEUDO], ["a:xurl(x)", PSEUDO],
  ["a:-url(x)", PSEUDO], ["a:my-url(x)", PSEUDO], ["a:url\\28 x)", INVALID],
  ["a:\\75 rl('x')", PSEUDO]
]);
check(wrong.length === 0, wrong.join("; "));
"##,
        );
    });
}

/// The class of a url in each place the lexer meets one: where a compound goes, after a
/// combinator, as an attribute's name or value (no production takes a url: not valid); in a
/// pseudo-class argument and in a block nested in it (skipped: not supported); and the token
/// after a url's own `)`: `a:is(url(x))b` puts a type selector after a pseudo-class (not
/// valid), `a:is(url(x) )`, `a:is(url(x)) b` and `a:is(url(x)),b` are valid.
#[test]
fn a_url_in_each_place() {
    with_page(PAGE, |runner| {
        checked(
            runner,
            r##"
var wrong = classes([
  ["url(x)", INVALID], [" url(x) ", INVALID], ["a>url(x)", INVALID], ["a > url( x )", INVALID],
  ["a,url(x)", INVALID], ["[url(x)]", INVALID], ["[a=url(x)]", INVALID],
  ["[a=url(\"x\")]", INVALID], ["[a=url(x]", INVALID], ["a.url(x)", INVALID],
  ["a#url(x)", INVALID],
  ["a:is(url(x))b", INVALID], ["a:is(url(x) )", PSEUDO], ["a:is(url(x)) b", PSEUDO],
  ["a:is(url(x)),b", PSEUDO], ["a:is([url(x)])", PSEUDO], ["a:is({url(x]})", PSEUDO],
  ["a:is(((url(x\")) b)) c", PSEUDO], ["a:not(b:is(url(x\\)) c)) d", PSEUDO],
  ["a::part(url(x\\)))", ELEMENT]
]);
check(wrong.length === 0, wrong.join("; "));
"##,
        );
    });
}

/// The sentence the round changed in `docs/design/scripts.md`: a code point outside the name
/// list of 4.2 (U+00A0 here) makes the text not valid where a name goes, and a string, an
/// escape or a pseudo-class argument that holds it leaves the text valid. So do a comment, a
/// url in a pseudo-class argument and a pseudo-element's argument, which it does not name;
/// an unquoted attribute value is a name.
#[test]
fn a_code_point_outside_the_name_list_in_each_holder() {
    with_page(PAGE, |runner| {
        checked(
            runner,
            r##"
var wrong = classes([
  ["div   span", INVALID], ["a ", INVALID], ["[a= ]", INVALID],
  ["[a=\" \"]", "no throw"], ["a\\a0 b", "no throw"], ["a:is( )", PSEUDO],
  ["a/* */", "no throw"], ["a:is(url( ))", PSEUDO], ["a::part( )", ELEMENT]
]);
check(wrong.length === 0, wrong.join("; "));
"##,
        );
    });
}

/// The steps of one call of `querySelector(G[name])`, after a first call warmed the runner.
fn steps_of(runner: &mut Runner, name: &str) -> u64 {
    let before = runner.steps();
    let end = runner.evaluate(&format!(
        "try {{ document.querySelector(G.{name}); }} catch (e) {{ G.e = e.message; }}"
    ));
    assert_eq!(end, ScriptEnd::Ran, "{name}: {end}");
    runner.steps() - before
}

/// One token, one step, the bytes paid by the text's charge: a url costs what a string of its
/// length costs in its place, for many urls in one text (681 `url(x)` in a pseudo-class
/// argument against as many `"xxxx"`), a bad url whose remnants hold 1,020 `url(` against one
/// string of its length, and a url in each of 584 nested blocks against a string in each.
/// Each pair is two texts of one length and one count of tokens; the steps of each pair are
/// equal, so no url byte is read under a charge a string does not pay.
#[test]
fn urls_cost_what_strings_of_their_length_cost() {
    with_page(PAGE, |runner| {
        checked(
            runner,
            r##"
G.u1 = "a:is(" + rep("url(x)", 681) + ")";
G.s1 = "a:is(" + rep("\"xxxx\"", 681) + ")";
G.u2 = "a:is(url(x\"" + rep("url(", 1020) + "))";
G.s2 = "a:is(\"" + rep("x", 4085) + "\")";
G.u3 = "a:is(" + rep("(url(x)", 584);
G.s3 = "a:is(" + rep("(\"xxxx\"", 584);
check(G.u1.length === G.s1.length && G.u2.length === G.s2.length && G.u3.length === G.s3.length,
  "one length a pair");
check(G.u1.length <= 4096 && G.u2.length <= 4096 && G.u3.length <= 4096, "within the bound");

"##,
        );
        // The first call of this shape costs 3 steps more (the unit's own cost case says so of
        // the runner's first call); one call before the pairs keeps it out of them.
        steps_of(runner, "s1");
        let mut lines = Vec::new();
        for (url, string) in [("u1", "s1"), ("u2", "s2"), ("u3", "s3")] {
            let u = steps_of(runner, url);
            let s = steps_of(runner, string);
            lines.push((url, u, string, s));
        }
        eprintln!("{lines:?}");
        for (url, u, string, s) in &lines {
            assert_eq!(u, s, "{url} {u} steps, {string} {s} steps");
        }
        checked(
            runner,
            "check(G.e === \"selector is not supported by surf: pseudo-class\", G.e);",
        );
    });
}
