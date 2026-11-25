//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`), sixth review: the
//! compare charged for both of its sides, `substringData` charged for the bytes its walk can
//! pass, a String argument charged before its copy, and the tests that hold them. Each case that
//! runs a call prints its steps, its time and the time a step against the plain walk's
//! (`getElementById('none')` over 20,000 elements without attributes), and asserts steps,
//! counts and ends, never a time. The source cases read the code of the module of reads and of
//! the generated bindings, comment lines stripped.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{ParseOptions, Tree, parse_document};
use surf_script::bindings::COLLECTION_NODE_STEPS;
use surf_script::dom::{self, DocumentInfo};
use surf_script::{Document, Mode, Runner, RunnerLimits, ScriptEnd, run_document};
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
    let charge = body.find("charge_steps(").expect("the charge in Copies::copy");
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
    assert!(functions.len() > 50, "reads.rs cut into {}", functions.len());
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
    let check = body.find("longer_than(").expect("`copied` measures the data");
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
