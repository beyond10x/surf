//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`), sixth review: the
//! compare charged for both of its sides, `substringData` charged for the bytes its walk can
//! pass, a String argument charged before its copy, and the tests that hold them. Each case that
//! runs a call prints its steps, its time and the time a step against the plain walk's
//! (`getElementById('none')` over 20,000 elements without attributes), and asserts steps,
//! counts and ends, never a time. The source cases read the code of the module of reads and of
//! the generated bindings, comment lines stripped.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_script::bindings::COLLECTION_NODE_STEPS;
use surf_script::dom::{self, DocumentInfo};
use surf_script::{Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, run_document};
use surf_url::Url;
use swell::Limits;

fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 50_000_000,
            memory_bytes: 256 * 1024 * 1024,
            frame_depth: 1_000,
            reentry_depth: 64,
            parser_nesting: 1_000,
            string_length: 1 << 23,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 1_000_000_000,
        scripts_per_runner: 1_000,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(50),
    }
}

fn info() -> DocumentInfo {
    DocumentInfo {
        address: Url::parse("https://example.com/dir/page", None).expect("the address"),
        content_type: "text/html".to_string(),
        encoding: "UTF-8".to_string(),
    }
}

fn with_tree<R: Send>(
    limits: RunnerLimits,
    parser: ParserLimits,
    tree: impl FnOnce() -> Tree + Send,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(
        limits,
        Mode::Test,
        move || Document {
            tree: tree(),
            limits: parser,
            slices: dom::slices(info()),
        },
        body,
    )
    .expect("a runner with the bindings")
}

/// `page` parsed under the parser's own default bounds.
fn parsed(page: String) -> impl FnOnce() -> Tree + Send {
    move || {
        parse_document(
            &page,
            &ParseOptions {
                limits: ParserLimits::default(),
                ..ParseOptions::default()
            },
        )
    }
}

/// One measured call: what it is, its steps, its time, and the floor of its steps.
struct Call {
    what: String,
    steps: u64,
    took: Duration,
    floor: u64,
}

/// Runs `source` once and measures it; the script must run to its end.
fn call(runner: &mut Runner, what: &str, source: &str, floor: u64) -> Call {
    let before = runner.steps();
    let at = Instant::now();
    let end = runner.evaluate(source);
    let took = at.elapsed();
    assert_eq!(end, ScriptEnd::Ran, "{what}: {end}");
    Call {
        what: what.to_string(),
        steps: runner.steps() - before,
        took,
        floor,
    }
}

fn ran(runner: &mut Runner, source: &str) {
    let end = runner.evaluate(source);
    assert_eq!(end, ScriptEnd::Ran, "{source}: {end}");
}

fn per_step(call: &Call) -> f64 {
    call.took.as_nanos() as f64 / call.steps.max(1) as f64
}

/// The plain walk's time a step, in nanoseconds: one call of `getElementById('none')` over
/// 20,000 elements without attributes.
fn plain() -> f64 {
    let page = format!("<!DOCTYPE html><div>{}</div>", "<p></p>".repeat(20_000));
    let plain = with_tree(limits(), ParserLimits::default(), parsed(page), |runner| {
        call(runner, "plain", "document.getElementById('none');", 20_000)
    });
    per_step(&plain)
}

/// Prints a line a call, against the plain walk, then asserts each call's floor of steps.
fn report(name: &str, calls: &[Call]) {
    let plain = plain();
    let lines: Vec<String> = calls
        .iter()
        .map(|call| {
            format!(
                "{}: {} steps (floor {}), {:?}, {:.1} ns a step, {:.1} x the plain walk's {:.1} ns",
                call.what,
                call.steps,
                call.floor,
                call.took,
                per_step(call),
                per_step(call) / plain,
                plain
            )
        })
        .collect();
    eprintln!("{name}:\n{}", lines.join("\n"));
    for call in calls {
        assert!(call.steps >= call.floor, "{}: {lines:#?}", call.what);
    }
}

const CHECK: &str = "function check(c, m) { if (!c) throw new Error(m); } \
                     function indexSize(f, m) { try { f(); } catch (e) { \
                     if (e.name === 'IndexSizeError') return; throw e; } \
                     throw new Error('no IndexSizeError: ' + m); }";

/// `source` without its comment lines (a line whose text starts with `//`).
fn code_of(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The code of `reads.rs` cut at each `fn` line into its functions: (name, body).
fn functions_of(code: &str) -> Vec<(String, String)> {
    let mut functions: Vec<(String, String)> = Vec::new();
    for line in code.lines() {
        let head = line.trim_start();
        let head = head.strip_prefix("pub(super) ").unwrap_or(head);
        let head = head.strip_prefix("pub ").unwrap_or(head);
        if let Some(rest) = head.strip_prefix("fn ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            functions.push((name, String::new()));
        }
        if let Some((_, body)) = functions.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    functions
}

/// The names of the functions `text` calls: each identifier right before a `(`.
fn calls_in(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = text.as_bytes();
    for (at, byte) in bytes.iter().enumerate() {
        if *byte != b'(' {
            continue;
        }
        let start = text[..at]
            .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
            .map_or(0, |found| found + 1);
        if start < at {
            names.push(text[start..at].to_string());
        }
    }
    names
}

/// The first read of `childNodes` copies the child list for one step a child before the copy,
/// beside the collection's `COLLECTION_NODE_STEPS` a node. The round's test of that step
/// compares 1,000 children with 999, a difference of one child: a step charged for every
/// second, fourth, eighth or thousandth child gives the same difference (1000/k - 999/k is 1 for
/// every k that divides 1,000). Here 1,000 children against 1: 999 children more, each one step
/// and its collection's steps.
#[test]
fn every_child_of_a_first_child_nodes_read_is_charged() {
    let first_read = |count: usize| {
        let page = format!("<!DOCTYPE html><p id=\"p\">{}</p>", "<b></b>".repeat(count));
        with_tree(limits(), ParserLimits::default(), parsed(page), |runner| {
            ran(runner, "var G = this; G.p = document.getElementById('p');");
            let before = runner.steps();
            ran(runner, "p.childNodes;");
            runner.steps() - before
        })
    };
    let (many, one) = (first_read(1_000), first_read(1));
    eprintln!("childNodes, first read: 1,000 children {many} steps, 1 child {one}");
    assert_eq!(
        many - one,
        999 * (1 + COLLECTION_NODE_STEPS),
        "{many} against {one}"
    );
}

/// `isEqualNode` charges both nodes of every pair before it reads either, whatever their kinds
/// and on whichever side the long thing is: a long comment on either side, a long text three
/// levels down on the other side only, a doctype's long public ID against itself and against an
/// element (kinds that differ: both charged before the kinds are compared), two attribute
/// lists that differ at their first attribute and hold a long name beside it, and the document
/// against itself. The floor is one step per 16 bytes of every side the pair holds.
#[test]
fn is_equal_node_charges_both_sides_of_every_kind_of_pair() {
    const LONG: usize = 1 << 20;
    let long = "y".repeat(LONG);
    let name = "n".repeat(LONG / 2);
    let page = format!(
        "<!DOCTYPE html PUBLIC \"{long}\"><body>\
         <!--x--><!--{long}-->\
         <div><p><b>x</b></p></div><div><p><b>{long}</b></p></div>\
         <p id=\"c\" v=\"1\" {name}></p><p id=\"d\" v=\"2\" {name}></p>"
    );
    let floor = LONG as u64 / 16;
    let calls = with_tree(limits(), ParserLimits::default(), parsed(page), |runner| {
        ran(
            runner,
            &format!(
                "{CHECK} var G = this; G.body = document.getElementsByTagName('body')[0]; \
                 G.short = body.childNodes[0]; G.long = body.childNodes[1]; \
                 check(short.nodeType === 8 && long.nodeType === 8, 'comments'); \
                 G.a = body.childNodes[2]; G.b = body.childNodes[3]; \
                 check(a.localName === 'div' && b.localName === 'div', 'the two divs'); \
                 G.c = document.getElementById('c'); G.d = document.getElementById('d'); \
                 G.t = document.doctype;"
            ),
        );
        vec![
            call(
                runner,
                "short.isEqualNode(long), a comment of 1 MiB on the other side",
                "short.isEqualNode(long);",
                floor,
            ),
            call(
                runner,
                "long.isEqualNode(short), a comment of 1 MiB on this side",
                "long.isEqualNode(short);",
                floor,
            ),
            call(
                runner,
                "a.isEqualNode(b), a text of 1 MiB three levels down on the other side",
                "a.isEqualNode(b);",
                floor,
            ),
            call(
                runner,
                "t.isEqualNode(t), a public ID of 1 MiB on both sides",
                "t.isEqualNode(t);",
                2 * floor,
            ),
            call(
                runner,
                "body.isEqualNode(t), a doctype of 1 MiB against an element",
                "body.isEqualNode(t);",
                floor,
            ),
            call(
                runner,
                "c.isEqualNode(d), lists that differ at the first attribute, a name of 512 KiB each",
                "c.isEqualNode(d);",
                floor,
            ),
            call(
                runner,
                "document.isEqualNode(document), every byte of the page twice",
                "document.isEqualNode(document);",
                2 * (3 * floor + floor / 2 * 2),
            ),
        ]
    });
    report("isEqualNode over every kind of pair", &calls);
}

/// `substringData` at its bounds: the answers and the `IndexSizeError` of the DOM for offsets
/// and counts at 0, at the end, past it, at `u32` max and where `offset + count` passes 2^32
/// (which the DOM adds without wrapping), over data of 1-, 2-, 3- and 4-byte characters, an
/// offset inside a surrogate pair answering its lone half; then the charge of the walk against
/// the bytes it passes over 1 MiB of each width, with an offset at the end, past it and a count
/// of `u32` max.
#[test]
fn substring_data_at_its_bounds() {
    let twos = 524_288_usize;
    let threes = 349_525_usize;
    let fours = 262_144_usize;
    let page = format!(
        "<!DOCTYPE html><p id=\"s\">a\u{e9}\u{20ac}\u{1f600}</p><p id=\"one\">{}</p>\
         <p id=\"two\">{}</p><p id=\"three\">{}</p><p id=\"four\">{}</p>",
        "a".repeat(1 << 20),
        "\u{e9}".repeat(twos),
        "\u{20ac}".repeat(threes),
        "\u{1f600}".repeat(fours)
    );
    let calls = with_tree(limits(), ParserLimits::default(), parsed(page), |runner| {
        ran(
            runner,
            &format!(
                "{CHECK} var t = document.getElementById('s').firstChild; \
                 check(t.length === 5, 'length ' + t.length); \
                 check(t.substringData(0, 0) === '', '0, 0'); \
                 check(t.substringData(5, 0) === '', 'the end, 0'); \
                 check(t.substringData(5, 4294967295) === '', 'the end, u32 max'); \
                 check(t.substringData(0, 4294967295) === 'a\\u00e9\\u20ac\\ud83d\\ude00', 'all'); \
                 check(t.substringData(1, -1) === '\\u00e9\\u20ac\\ud83d\\ude00', 'count -1'); \
                 check(t.substringData(2, 4294967294) === '\\u20ac\\ud83d\\ude00', 'sum 2^32'); \
                 check(t.substringData(4294967297, 1) === '\\u00e9', 'offset 2^32 + 1'); \
                 var high = t.substringData(3, 1); \
                 check(high.length === 1 && high.charCodeAt(0) === 0xd83d, 'the high half'); \
                 var low = t.substringData(4, 7); \
                 check(low.length === 1 && low.charCodeAt(0) === 0xde00, 'the low half'); \
                 indexSize(function () {{ t.substringData(6, 0); }}, 'past the end'); \
                 indexSize(function () {{ t.substringData(4294967295, 1); }}, 'u32 max'); \
                 indexSize(function () {{ t.substringData(-1, 0); }}, '-1'); \
                 var G = this; G.one = document.getElementById('one').firstChild; \
                 G.two = document.getElementById('two').firstChild; \
                 G.three = document.getElementById('three').firstChild; \
                 G.four = document.getElementById('four').firstChild;"
            ),
        );
        let mib = 1_u64 << 20;
        vec![
            call(
                runner,
                "substringData(the last unit, 1) of 1 MiB of a 1-byte character",
                &format!("one.substringData({}, 1);", (1 << 20) - 1),
                mib / 16,
            ),
            call(
                runner,
                "substringData(the last unit, 1) of 1 MiB of a 2-byte character",
                &format!("two.substringData({}, 1);", twos - 1),
                mib / 16,
            ),
            call(
                runner,
                "substringData(0, u32 max) of 1 MiB of a 3-byte character",
                "three.substringData(0, 4294967295);",
                (3 * threes as u64 + 2 * threes as u64) / 16,
            ),
            call(
                runner,
                "substringData(u32 max, 0) of 1 MiB of a 4-byte character, past the end",
                "try { four.substringData(4294967295, 0); } catch (e) {}",
                mib / 16,
            ),
            call(
                runner,
                "substringData(the length, 1) of 1 MiB of a 4-byte character, at the end",
                &format!("four.substringData({}, 1);", 2 * fours),
                mib / 16,
            ),
        ]
    });
    report("substringData at its bounds", &calls);
}

/// A String argument is charged before any copy of it: in `Copies::copy` of the generated
/// bindings, the code before the charge calls only `string_length` (which copies nothing) and
/// `is_finite`, and the charge's failure ends the call (`?`). The round's source test finds the
/// first `to_utf16` after the charge; a copy by another name before the charge (the prelude's own
/// `prefix_units`, which calls `to_utf16` inside, or `to_rust_string_lossy`) or a charge whose
/// failure is dropped (`let _ = cx.charge_steps(..);`) passes it.
#[test]
fn nothing_but_the_length_is_read_before_the_argument_is_charged() {
    let code = code_of(include_str!("../src/bindings.rs"));
    let start = code
        .find("fn copy(&mut self, cx: &mut Cx<'_, ScriptHost>, string: StringRef)")
        .expect("Copies::copy in bindings.rs");
    let body = &code[start..];
    let body = &body[..body.find("\n    }\n").expect("the end of Copies::copy")];
    let open = body.find('{').expect("the body of Copies::copy");
    let charge = body
        .find("charge_steps(")
        .expect("the charge in Copies::copy");
    let before = &body[open..charge];
    let called: Vec<String> = calls_in(before)
        .into_iter()
        .filter(|name| !["string_length", "is_finite"].contains(&name.as_str()))
        .collect();
    assert!(
        called.is_empty(),
        "Copies::copy calls {called:?} before its charge:\n{body}"
    );
    let line = body[charge..].lines().next().unwrap_or_default();
    assert!(
        line.trim_end().ends_with(")?;"),
        "the charge of Copies::copy does not end the call when it fails: {line}"
    );
    let line_start = body[..charge].rfind('\n').map_or(0, |at| at + 1);
    let head = body[line_start..charge].trim();
    assert!(
        head == "cx." || head == "cx.engine.",
        "the charge of Copies::copy is not a statement of its own: `{head}`"
    );
}

/// No walk of the module of reads copies a child list, by any spelling of a copy the standard
/// library has for a slice: beside the round's four (`.to_vec()`, `.collect`, `extend(`,
/// `Vec::from(`) also `.to_owned()`, `from_iter(`, `extend_from_slice(`, `extend_from_within(`,
/// `.into_vec()`, `Box::from(`, `Rc::from(` and `Arc::from(`; only the three functions that
/// answer a list or compare attribute lists may hold one.
#[test]
fn no_walk_of_the_reads_copies_a_child_list_by_another_spelling() {
    const ANSWERS: [&str; 3] = ["element_children", "shallow_equal", "children"];
    const COPIES: [&str; 12] = [
        ".to_vec()",
        ".collect",
        "extend(",
        "Vec::from(",
        ".to_owned()",
        "from_iter(",
        "extend_from_slice(",
        "extend_from_within(",
        ".into_vec()",
        "Box::from(",
        "Rc::from(",
        "Arc::from(",
    ];
    let functions = functions_of(&code_of(include_str!("../src/dom/reads.rs")));
    assert!(
        functions.len() > 50,
        "reads.rs cut into {}",
        functions.len()
    );
    let copying: Vec<String> = functions
        .iter()
        .filter(|(name, _)| !ANSWERS.contains(&name.as_str()))
        .flat_map(|(name, body)| {
            COPIES
                .iter()
                .filter(|copy| body.contains(*copy))
                .map(move |copy| format!("`{name}` holds `{copy}`"))
        })
        .collect();
    assert!(copying.is_empty(), "{copying:#?}");
}

/// `data`, `nodeValue` and a text's `textContent` copy nothing before the data is measured
/// against `string_length`: the code of `copied` before its `longer_than(` holds no copy of a
/// String by any spelling. The round's count (`dom::data_copies`) sees only `counted_copy`; a
/// copy before the check by `to_string`, `to_owned`, `String::from` or `format!` leaves it at 0.
#[test]
fn the_data_is_not_copied_before_it_is_measured_by_any_spelling() {
    const COPIES: [&str; 8] = [
        ".to_string()",
        ".to_owned()",
        "String::from",
        "format!",
        ".into()",
        "counted_copy(",
        "push_str(",
        ".clone()",
    ];
    let functions = functions_of(&code_of(include_str!("../src/dom/reads.rs")));
    let (_, body) = functions
        .iter()
        .find(|(name, _)| name == "copied")
        .expect("reads.rs holds `copied`");
    let check = body
        .find("longer_than(")
        .expect("`copied` measures the data");
    let before = &body[..check];
    let found: Vec<&str> = COPIES
        .iter()
        .copied()
        .filter(|copy| before.contains(copy))
        .collect();
    assert!(
        found.is_empty(),
        "`copied` copies before its check: {found:?}\n{before}"
    );
    assert!(
        body[check..].contains("counted_copy("),
        "`copied` makes its copy by `counted_copy` after the check:\n{body}"
    );
}

/// `limits()` with its own steps a script and `string_length`.
fn limits_of(steps_per_entry: u64, string_length: u64) -> RunnerLimits {
    let mut limits = limits();
    limits.engine.steps_per_entry = steps_per_entry;
    limits.engine.string_length = string_length;
    limits
}

/// The steps of one script that must run to its end.
fn steps_of(runner: &mut Runner, source: &str) -> u64 {
    let before = runner.steps();
    ran(runner, source);
    runner.steps() - before
}

/// The end and the steps of one script.
fn ended(runner: &mut Runner, source: &str) -> (ScriptEnd, u64) {
    let before = runner.steps();
    let end = runner.evaluate(source);
    (end, runner.steps() - before)
}

/// `isEqualNode` of two elements of 257 attributes (an `id` and 256 of 65 bytes) charges in two
/// parts before it reads either list: first what is known from the counts (a step a node, a
/// step per 16 bytes of its name, a step an attribute: 2 x 258 here), then the bytes of both
/// lists (2 x (256 x 65 + 3) bytes / 16).
/// A script whose step limit falls inside the second part ends there with the first part spent
/// and the second not; one whose limit falls inside the first part ends with neither spent.
/// The steps that would pass the limit are not spent, so the count of the ended script says
/// which charge refused.
#[test]
fn a_compare_ended_between_its_two_charges_spent_the_first_only() {
    const ATTRIBUTES: usize = 256;
    let list = |value: &str| {
        (0..ATTRIBUTES)
            .map(|at| format!("a{at:063}=\"{value}\""))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let page = format!(
        "<!DOCTYPE html><p id=\"c\" {}></p><p id=\"d\" {}></p>",
        list("1"),
        list("2")
    );
    let known = 2 * (1 + 1 + ATTRIBUTES as u64);
    let bytes = 2 * ((ATTRIBUTES as u64 * 65 + 3) / 16);
    let setup = [
        "var G = this; G.n = null; G.c = document.getElementById('c');",
        "G.d = document.getElementById('d');",
    ];
    let (none, full) = with_tree(
        limits(),
        ParserLimits::default(),
        parsed(page.clone()),
        |runner| {
            for part in setup {
                ran(runner, part);
            }
            (
                steps_of(runner, "c.isEqualNode(n);"),
                steps_of(runner, "c.isEqualNode(d);"),
            )
        },
    );
    eprintln!("isEqualNode of 257 attributes each: null {none} steps, the pair {full}");
    assert_eq!(full - none, known + bytes, "{full} against {none}");
    let at_limit = |limit: u64| {
        let page = page.clone();
        with_tree(
            limits_of(limit, 1 << 23),
            ParserLimits::default(),
            parsed(page),
            |runner| {
                for part in setup {
                    ran(runner, part);
                }
                let ended = ended(runner, "c.isEqualNode(d);");
                let after = steps_of(runner, "c.isEqualNode(n);");
                (ended, after)
            },
        )
    };
    let inside_bytes = none + known + bytes / 2;
    let ((end, spent), after) = at_limit(inside_bytes);
    eprintln!("limit {inside_bytes}, inside the bytes' part: {end}, {spent} steps");
    assert_eq!(end, ScriptEnd::Limit(Reason::StepsPerScript), "{end}");
    assert!(
        spent + bytes > inside_bytes && spent <= none + known && spent + 8 >= none + known,
        "the bytes' part refused: {spent} steps, the known part ends at {}",
        none + known
    );
    assert_eq!(after, none, "a script after the end");
    let inside_known = none + known / 2;
    let ((end, spent), _) = at_limit(inside_known);
    eprintln!("limit {inside_known}, inside the known part: {end}, {spent} steps");
    assert_eq!(end, ScriptEnd::Limit(Reason::StepsPerScript), "{end}");
    assert!(
        spent <= none && spent + 8 >= none,
        "the known part refused: {spent} steps against {none} before the charges"
    );
}

/// The members of `CharacterData` and `Text` that answer the data, or a count of it, charge its
/// bytes and not its code units: over 1 MiB of a 3-byte character (349,525 code units,
/// 1,048,575 bytes) `data`, `nodeValue`, `textContent` and `wholeText` of the text, `data` and
/// `textContent` of a comment of the same, and `textContent` of the text's parent charge a step
/// per 16 bytes for the read and again for the answer's copy (a charge in units would give
/// 21,845 for the read); `length` a step per 16 bytes for its count.
#[test]
fn the_data_members_charge_bytes_not_code_units() {
    const UNITS: usize = 349_525;
    let data = "\u{20ac}".repeat(UNITS);
    let bytes = data.len() as u64;
    let page = format!("<!DOCTYPE html><p id=\"t\">{data}</p><div id=\"c\"><!--{data}--></div>");
    let read = bytes / 16;
    let calls = with_tree(limits(), ParserLimits::default(), parsed(page), |runner| {
        ran(
            runner,
            &format!(
                "{CHECK} var G = this; G.p = document.getElementById('t'); G.t = p.firstChild; \
                 G.k = document.getElementById('c').firstChild; \
                 check(t.nodeType === 3 && k.nodeType === 8, 'a text and a comment'); \
                 check(t.length === {UNITS} && k.length === {UNITS}, 'the lengths');"
            ),
        );
        vec![
            call(runner, "t.data", "t.data;", 2 * read),
            call(runner, "t.nodeValue", "t.nodeValue;", 2 * read),
            call(runner, "t.textContent", "t.textContent;", 2 * read),
            call(runner, "t.wholeText", "t.wholeText;", 2 * read),
            call(runner, "t.length", "t.length;", read),
            call(runner, "k.data", "k.data;", 2 * read),
            call(runner, "k.textContent", "k.textContent;", 2 * read),
            call(runner, "k.length", "k.length;", read),
            call(runner, "p.textContent", "p.textContent;", 2 * read),
        ]
    });
    report("the data members over 1 MiB of a 3-byte character", &calls);
}

/// A String argument at the session's `string_length` (1,024 here) is copied for its charge,
/// two bytes a code unit at one step per 16 (128 steps for 1,024 units of any width), and
/// `getElementsByTagName` folds its bytes beside it (192 steps for 1,024 units of 3 bytes). A
/// String one unit over is never made by the engine's own operations tried here: each throws
/// before a member sees it; any that makes one, the member charges its units before its copy.
#[test]
fn an_argument_at_the_string_length_and_one_over() {
    const LIMIT: u64 = 1_024;
    let page = "<!DOCTYPE html><p></p>".to_string();
    with_tree(
        limits_of(50_000_000, LIMIT),
        ParserLimits::default(),
        parsed(page),
        |runner| {
            ran(
                runner,
                "var G = this; G.one = 'a'; G.at = 'a'.repeat(1024); \
                 G.atE = '\\u00e9'.repeat(1024); G.atEuro = '\\u20ac'.repeat(1024);",
            );
            let base = steps_of(runner, "document.getElementById(one);");
            let tag_base = steps_of(runner, "document.getElementsByTagName(one);");
            for (what, name) in [("a", "at"), ("\u{e9}", "atE"), ("\u{20ac}", "atEuro")] {
                let id = steps_of(runner, &format!("document.getElementById({name});"));
                eprintln!("getElementById(1,024 x {what}): {id} steps, 1 x a {base}");
                assert_eq!(id - base, LIMIT * 2 / 16, "getElementById(1,024 x {what})");
            }
            let tag = steps_of(runner, "document.getElementsByTagName(atEuro);");
            eprintln!("getElementsByTagName(1,024 x \u{20ac}): {tag} steps, 1 x a {tag_base}");
            assert_eq!(tag - tag_base, LIMIT * 2 / 16 + 3 * LIMIT / 16);
            let makers = [
                "at + 'a'",
                "'a'.repeat(1025)",
                "[at, 'a'].join('')",
                "at.padEnd(1025, 'a')",
                "at.concat('a')",
                "JSON.stringify(at)",
                "'\\u00df'.repeat(1024).toUpperCase()",
                "encodeURIComponent(atEuro)",
                "String.fromCharCode.apply(null, Array(1025).fill(97))",
                "`${at}a`",
                "at.replace(/a/g, 'aa')",
                "Array(1100).join('a')",
                "'e\\u0301'.repeat(512).normalize('NFC') + atE.normalize('NFD')",
                "'\\u0130'.repeat(1024).toLowerCase()",
                "String(Array(600).fill('ab'))",
            ];
            let mut over = Vec::new();
            for (index, maker) in makers.iter().enumerate() {
                let end = runner.evaluate(&format!(
                    "var G = this; G.s = undefined; G.e = 'made'; \
                     try {{ G.s = {maker}; }} catch (x) {{ G.e = x.name; }} \
                     if (s !== undefined && s.length > {LIMIT}) throw new Error('over ' + s.length); \
                     throw new Error(e);"
                ));
                let text = end.to_string();
                eprintln!("maker {index} `{maker}`: {text}");
                if let Some(at) = text.find("\"over ") {
                    let units: u64 = text[at + 6..]
                        .chars()
                        .take_while(char::is_ascii_digit)
                        .collect::<String>()
                        .parse()
                        .expect("the length of the String");
                    let id = steps_of(runner, "document.getElementById(s);");
                    eprintln!("  getElementById of its {units} units: {id} steps, 1 x a {base}");
                    assert_eq!(id - base, units * 2 / 16, "maker {index} `{maker}`");
                    over.push((index, units));
                }
            }
            eprintln!("makers that made a String over {LIMIT} units: {over:?}");
        },
    );
}

/// A call whose step limit falls inside the charge of its String argument (256 Ki code units,
/// 32,768 steps) ends there with nothing of the charge spent; the next script runs as before.
/// The script spends most of its steps in an empty loop first, so that the limit (400,000 a
/// script, room for the doublings that build the String) falls in the middle of the charge.
/// Whether the engine's String was copied before the end is not counted anywhere a test can
/// read (the ledger of copies is dropped with the call): the time of the ended script against
/// the same script with an argument of one unit is printed, and the order in `Copies::copy` is
/// held by `nothing_but_the_length_is_read_before_the_argument_is_charged`.
#[test]
fn a_call_ended_at_its_arguments_charge() {
    const UNITS: u64 = 1 << 18;
    const LIMIT: u64 = 400_000;
    let charge = UNITS * 2 / 16;
    let page = "<!DOCTYPE html><p></p>".to_string();
    let burn = |turns: u64, argument: &str| {
        format!("for (var i = 0; i < {turns}; i++) {{}} document.getElementById({argument});")
    };
    with_tree(
        limits_of(LIMIT, 1 << 23),
        ParserLimits::default(),
        parsed(page),
        |runner| {
            ran(
                runner,
                "var G = this; G.one = 'a'; G.big = 'a'.repeat(1024);",
            );
            for _ in 0..8 {
                ran(runner, "G.big = big + big;");
            }
            ran(
                runner,
                &format!("if (big.length !== {UNITS}) throw new Error('length ' + big.length);"),
            );
            let base = steps_of(runner, "document.getElementById(one);");
            let full = steps_of(runner, "document.getElementById(big);");
            assert_eq!(full - base, charge, "the argument's charge");
            let (a, b) = (
                steps_of(runner, &burn(1_000, "one")),
                steps_of(runner, &burn(2_000, "one")),
            );
            let turn = (b - a) / 1_000;
            let turns = 1_000 + (LIMIT - charge / 2 - a) / turn;
            let at = Instant::now();
            let below = steps_of(runner, &burn(turns, "one"));
            let below_took = at.elapsed();
            assert!(
                below + charge / 4 <= LIMIT && below + charge >= LIMIT + charge / 4,
                "{turns} turns: {below} steps with one unit, the limit {LIMIT}"
            );
            let at = Instant::now();
            let (end, spent) = ended(runner, &burn(turns, "big"));
            let ended_took = at.elapsed();
            let after = steps_of(runner, "document.getElementById(one);");
            eprintln!(
                "{turns} turns and getElementById: one unit {below} steps, {below_took:?}; \
                 256 Ki units {end}, {spent} steps, {ended_took:?}; the next script {after} steps \
                 (alone {base}, with 256 Ki units {full})"
            );
            assert_eq!(end, ScriptEnd::Limit(Reason::StepsPerScript), "{end}");
            assert!(
                spent <= below && spent + base >= below,
                "{spent} steps spent, {below} for the whole script with one unit"
            );
            assert_eq!(after, base, "the next script");
        },
    );
}

/// The first element named `name` in tree order.
fn first_element(tree: &Tree, name: &str) -> NodeId {
    let mut stack = vec![tree.root()];
    while let Some(node) = stack.pop() {
        if let NodeData::Element { local_name, .. } = tree.data(node)
            && local_name == name
        {
            return node;
        }
        stack.extend(tree.children(node).iter().rev().copied());
    }
    panic!("no element `{name}`");
}

/// `wholeText` charges a step before each sibling it reads on either side. A parent of 1,000
/// adjacent texts of one byte (made through the tree's writes, since the parser joins adjacent
/// text): for the text at index i the scan for its index costs i + 1, the siblings before it i
/// and the siblings after it 999 - i, the bytes the same for each; so the last text costs 999
/// steps more than the first. Without the step a sibling before, both cost the same.
#[test]
fn whole_text_charges_the_siblings_before_the_text() {
    const TEXTS: u64 = 1_000;
    let tree = || {
        let mut tree = parsed("<!DOCTYPE html><p id=\"p\"></p>".to_string())();
        let p = first_element(&tree, "p");
        for _ in 0..TEXTS {
            let text = tree.create_text_node("a").expect("a text");
            tree.append_child(p, text).expect("appended");
        }
        tree
    };
    with_tree(limits(), ParserLimits::default(), tree, |runner| {
        ran(
            runner,
            &format!(
                "{CHECK} var G = this; G.p = document.getElementById('p'); \
                 G.first = p.firstChild; G.last = p.lastChild; \
                 check(p.childNodes.length === {TEXTS}, 'the texts'); \
                 check(first.wholeText.length === {TEXTS}, 'the whole text');"
            ),
        );
        let first = steps_of(runner, "first.wholeText;");
        let last = steps_of(runner, "last.wholeText;");
        eprintln!("wholeText of 1,000 texts: the first {first} steps, the last {last}");
        assert_eq!(last - first, TEXTS - 1, "{last} against {first}");
    });
}

/// `getElementsByClassName` charges the bytes of its argument, one step per 16, beside its
/// copy (two bytes a code unit) and its tokens, before it builds the wanted set: one class of
/// 1 Mi letters costs, over a page without a class, 131,072 + 65,536 steps more than a class of
/// one letter (one token each). The round's case of 131,072 classes asserts the step a token
/// only, which the steps of the copy cover without the bytes' charge.
#[test]
fn a_class_argument_is_charged_for_its_bytes() {
    let page = "<!DOCTYPE html><p></p>".to_string();
    with_tree(limits(), ParserLimits::default(), parsed(page), |runner| {
        ran(
            runner,
            "var G = this; G.one = 'x'; G.big = 'x'; \
             while (big.length < 1048576) big = big + big;",
        );
        let short = steps_of(runner, "document.getElementsByClassName(one);");
        let long = steps_of(runner, "document.getElementsByClassName(big);");
        eprintln!("getElementsByClassName: 1 Mi letters {long} steps, 1 letter {short}");
        assert_eq!(long - short, (1 << 20) * 2 / 16 + (1 << 20) / 16);
    });
}
