//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`), fourth review: what a
//! step buys after the per-token, per-attribute and glue weights. Each case runs one call on a
//! shape the parser admits (or the tree's own writes make), prints its steps, its time and the
//! time a step against the plain walk's (`getElementById('none')` over 20,000 elements without
//! attributes), and asserts steps, counts and ends, never a time.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};
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
    tree: impl FnOnce() -> Tree + Send,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(
        limits,
        Mode::Test,
        move || Document {
            tree: tree(),
            limits: ParserLimits::default(),
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

/// The elements named `name`, in tree order.
fn elements(tree: &Tree, name: &str) -> Vec<NodeId> {
    let mut found = Vec::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if let NodeData::Element { local_name, .. } = tree.data(node)
            && local_name == name
        {
            found.push(node);
        }
        pending.extend(tree.children(node).iter().rev());
    }
    found
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
    let plain = with_tree(limits(), parsed(page), |runner| {
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

/// `document.title` collapses the ASCII whitespace of its `title`'s text: it passes every byte
/// of that text, and charges only the children of the `title` and the bytes it keeps. A `title`
/// of 2 and 4 MiB of spaces around one letter (markup the parser admits under its default
/// bounds) answers "x": the walk to the `title`, then a pass over every byte, for a handful of
/// steps that do not grow with the bytes. The floor is the slice's own rule: one step per 64
/// bytes of a String it reads.
#[test]
fn a_title_of_whitespace_is_read_without_a_charge_for_its_bytes() {
    let mut calls = Vec::new();
    for mib in [2_usize, 4] {
        let bytes = mib << 20;
        let page = format!(
            "<!DOCTYPE html><title>{}x{}</title>",
            " ".repeat(bytes / 2),
            "\n".repeat(bytes / 2)
        );
        calls.push(with_tree(limits(), parsed(page), |runner| {
            ran(
                runner,
                "if (document.title !== 'x') throw new Error(document.title);",
            );
            call(
                runner,
                &format!("title of {mib} MiB of whitespace around one letter"),
                "document.title;",
                (bytes / 64) as u64,
            )
        }));
    }
    report("a title of whitespace", &calls);
}

/// `document.title` of one-letter words: each word is trimmed, found, admitted twice (the space
/// and the letter) and appended twice, for one step per 64 bytes kept and the glue's one per 16
/// bytes copied: about a sixth of a step a word. A word is a unit of work smaller than 64
/// bytes, and the rule of the slice charges one step for each such unit (as a class token
/// costs one). 1 and 2 MiB of `a ` (512 Ki and 1 Mi words); the floor is a step a word.
#[test]
fn a_title_of_one_letter_words_charges_less_than_a_step_a_word() {
    let mut calls = Vec::new();
    for mib in [1_usize, 2] {
        let bytes = mib << 20;
        let page = format!("<!DOCTYPE html><title>{}</title>", "a ".repeat(bytes / 2));
        calls.push(with_tree(limits(), parsed(page), |runner| {
            call(
                runner,
                &format!("title of {mib} MiB of one-letter words"),
                "document.title;",
                (bytes / 2) as u64,
            )
        }));
    }
    report("a title of one-letter words", &calls);
}

/// The class walk over 40 elements whose `class` is one token of 256 KiB: one step per 64 bytes
/// and one for the token, and a set lookup that hashes (in quirks mode, folds first) all 256
/// KiB. Wanted: `b`, and the same 256 KiB token, which every element holds. The floor of an
/// element is one step and one per `bytes_a_step` bytes of its `class`.
fn one_long_token(doctype: &'static str, mode: &str, bytes_a_step: u64) -> Vec<Call> {
    let mut calls = Vec::new();
    let tree = move || {
        let token = "a".repeat(1 << 18);
        let page = format!("{doctype}<div>{}</div>", "<p></p>".repeat(40));
        let mut tree = parsed(page)();
        for element in elements(&tree, "p") {
            tree.set_attribute(element, "class", &token)
                .expect("a class");
        }
        tree
    };
    let per_element = 1 + (1 << 18) / bytes_a_step;
    calls.extend(with_tree(limits(), tree, |runner| {
        ran(
            runner,
            "var G = this; G.big = 'a'; while (big.length < 262144) big = big + big;",
        );
        vec![
            call(
                runner,
                &format!("40 classes of one 256 KiB token in {mode}, getElementsByClassName('b')"),
                "document.getElementsByClassName('b');",
                40 * per_element,
            ),
            call(
                runner,
                &format!("the same, getElementsByClassName(the token) in {mode}"),
                "if (document.getElementsByClassName(big).length !== 40) throw 1;",
                40 * per_element + 40 * 8,
            ),
        ]
    }));
    calls
}

/// In quirks mode each token is folded into ASCII lower case character by character
/// (`folded.extend(class.chars().map(..))`) before its lookup: a conversion of every byte, the
/// work the glue prices at one step per 16 bytes of a String it copies (`STRING_BYTES_A_STEP`),
/// charged here at one per 64 and the token's one step. The floor is the glue's price.
#[test]
fn a_long_class_token_is_folded_in_quirks_mode_below_the_price_of_a_copy() {
    let calls = one_long_token("", "quirks", 16);
    report("one long class token, quirks", &calls);
}

/// An argument of 4 Mi ASCII code units: the glue copies it out of the engine
/// (`Copies::copy`: the engine's `to_utf16`, then UTF-16 to UTF-8) at one step per 64 bytes,
/// where it copies a String answer into the engine at one per 16. A document of one element,
/// so the member itself compares nothing of it. The floor is the price of an answer's copy.
#[test]
fn an_argument_is_copied_at_a_quarter_of_the_price_of_an_answer() {
    let calls = with_tree(
        limits(),
        parsed("<!DOCTYPE html><p></p>".to_string()),
        |runner| {
            ran(
                runner,
                "var G = this; G.big = 'a'; while (big.length < 4194304) big = big + big;",
            );
            vec![call(
                runner,
                "getElementById(4 Mi ASCII units)",
                "document.getElementById(big);",
                (4 << 20) / 16,
            )]
        },
    );
    report("an argument of 4 Mi units", &calls);
}
