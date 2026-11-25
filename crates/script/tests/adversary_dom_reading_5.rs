//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`), fifth review: the
//! module of reads that charges before each read, one price for a byte (one step per 16), and
//! walks that copy no child list. Each case runs one call on a shape the parser admits (or the
//! tree's own writes make), prints its steps, its time and the time a step against the plain
//! walk's (`getElementById('none')` over 20,000 elements without attributes), and asserts
//! steps, counts and ends, never a time.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{ParseOptions, Tree, parse_document};
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

/// `page` parsed under `parser`.
fn parsed_under(page: String, parser: ParserLimits) -> impl FnOnce() -> Tree + Send {
    move || {
        parse_document(
            &page,
            &ParseOptions {
                limits: parser,
                ..ParseOptions::default()
            },
        )
    }
}

/// `page` parsed under the parser's own default bounds.
fn parsed(page: String) -> impl FnOnce() -> Tree + Send {
    parsed_under(page, ParserLimits::default())
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

/// The lines of `calls` against the plain walk.
fn lines(calls: &[Call]) -> Vec<String> {
    let plain = plain();
    calls
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
        .collect()
}

/// Prints a line a call, against the plain walk, then asserts each call's floor of steps.
fn report(name: &str, calls: &[Call]) {
    let lines = lines(calls);
    eprintln!("{name}:\n{}", lines.join("\n"));
    for call in calls {
        assert!(call.steps >= call.floor, "{}: {lines:#?}", call.what);
    }
}

const CHECK: &str = "function check(c, m) { if (!c) throw new Error(m); } \
                     function rangeError(f, m) { try { f(); } catch (e) { \
                     if (e instanceof RangeError) return; throw e; } \
                     throw new Error('no RangeError: ' + m); }";

/// Red (must). `isEqualNode` charges the pair it compares by `this`'s side only
/// (`reads.rs:937-953`, `equal_cost`), on the ground that a compare reads no further than the
/// shorter string. But the attribute lists are compared through a map (`reads.rs:894-910`):
/// every attribute name of the other element is hashed whole before the lookup, and a hash
/// reads every byte. Two `p` elements with one attribute each, `this`'s named `x` and the
/// other's named by 1 and 2 MiB of `y` (markup the parser admits under its default bounds):
/// `a.isEqualNode(b)` hashes the long name for a handful of steps that do not grow with it,
/// while `b.isEqualNode(a)` charges it. The floor is the slice's one price: a step per 16 bytes
/// the call reads.
#[test]
fn is_equal_node_hashes_a_long_attribute_name_of_the_other_node_unpaid() {
    let mut calls = Vec::new();
    for bytes in [1_usize << 20, 2 << 20] {
        let name = "y".repeat(bytes);
        let page = format!("<!DOCTYPE html><p x></p><p {name}></p>");
        let floor = bytes as u64 / 16;
        calls.extend(with_tree(
            limits(),
            ParserLimits::default(),
            parsed(page),
            |runner| {
                ran(
                    runner,
                    &format!(
                        "{CHECK} var G = this; var ps = document.getElementsByTagName('p'); \
                     G.a = ps[0]; G.b = ps[1]; check(!a.isEqualNode(b), 'equal');"
                    ),
                );
                let mib = bytes >> 20;
                vec![
                    call(
                        runner,
                        &format!("b.isEqualNode(a), b's attribute name of {mib} MiB"),
                        "b.isEqualNode(a);",
                        floor,
                    ),
                    call(
                        runner,
                        &format!("a.isEqualNode(b), b's attribute name of {mib} MiB"),
                        "a.isEqualNode(b);",
                        floor,
                    ),
                ]
            },
        ));
    }
    report("isEqualNode against a long attribute name", &calls);
}

/// Red (should). One price for a byte, the round's rule: every byte a member reads or compares
/// is charged. `isEqualNode` of two equal texts of 1 MiB compares both, 2 MiB read, and charges
/// the bytes of `this`'s text only (`reads.rs:942`, "the bytes of `node`'s side bound it"). The
/// floor: both sides at one step per 16 bytes.
#[test]
fn is_equal_node_compares_two_long_texts_for_the_bytes_of_one() {
    const BYTES: usize = 1 << 20;
    let text = "a".repeat(BYTES);
    let page = format!("<!DOCTYPE html><p>{text}</p><p>{text}</p>");
    let calls = with_tree(limits(), ParserLimits::default(), parsed(page), |runner| {
        ran(
            runner,
            &format!(
                "{CHECK} var G = this; var ps = document.getElementsByTagName('p'); \
                 G.a = ps[0].firstChild; G.b = ps[1].firstChild; check(a.isEqualNode(b), 'equal');"
            ),
        );
        vec![call(
            runner,
            "isEqualNode of two equal texts of 1 MiB",
            "a.isEqualNode(b);",
            2 * BYTES as u64 / 16,
        )]
    });
    report("isEqualNode of two long texts", &calls);
}

/// Red (should). `substringData` charges one step per 16 code units of `offset + count`
/// (`reads.rs:1271-1272`) and walks the data's UTF-8 to the offset: a unit is 3 bytes of `€`
/// and 2 bytes of each half of `😀`'s pair (4 bytes, 2 units). An offset at the last unit reads
/// every byte of the data for a third (`€`) or half (`😀`) of their price; `length` of the same
/// data charges every byte (`reads.rs:1249-1250`). The floor is the one price, a step per 16
/// bytes the call reads.
#[test]
fn substring_data_passes_bytes_at_the_price_of_code_units() {
    let euros = 349_525_usize;
    let faces = 262_144_usize;
    let page = format!(
        "<!DOCTYPE html><p id=\"e\">{}</p><p id=\"f\">{}</p>",
        "\u{20ac}".repeat(euros),
        "\u{1f600}".repeat(faces)
    );
    let euro_bytes = 3 * euros as u64;
    let face_bytes = 4 * faces as u64;
    let calls = with_tree(limits(), ParserLimits::default(), parsed(page), |runner| {
        ran(
            runner,
            "var G = this; G.e = document.getElementById('e').firstChild; \
             G.f = document.getElementById('f').firstChild;",
        );
        vec![
            call(
                runner,
                "length of 1 MiB of a 3-byte character",
                "e.length;",
                euro_bytes / 16,
            ),
            call(
                runner,
                "substringData(the last unit, 1) of 1 MiB of a 3-byte character",
                &format!("e.substringData({}, 1);", euros - 1),
                euro_bytes / 16,
            ),
            call(
                runner,
                "length of 1 MiB of a 4-byte character",
                "f.length;",
                face_bytes / 16,
            ),
            call(
                runner,
                "substringData(the last unit, 1) of 1 MiB of a 4-byte character",
                &format!("f.substringData({}, 1);", 2 * faces - 1),
                face_bytes / 16,
            ),
        ]
    });
    report("substringData of characters of 3 and 4 bytes", &calls);
}
