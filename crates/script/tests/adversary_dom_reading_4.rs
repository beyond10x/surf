//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`), fourth review: what a
//! step buys after the per-token, per-attribute and glue weights. Each case runs one call on a
//! shape the parser admits (or the tree's own writes make), prints its steps, its time and the
//! time a step against the plain walk's (`getElementById('none')` over 20,000 elements without
//! attributes), and asserts steps, counts and ends, never a time.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{
    Document, Mode, Runner, RunnerLimits, SESSION_ENGINE_LIMITS, ScriptEnd, run_document,
};
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

/// The session's engine limits (10,000,000 steps an entry), with a wall clock a loaded build
/// node does not reach.
fn session() -> RunnerLimits {
    RunnerLimits {
        engine: SESSION_ENGINE_LIMITS,
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

/// `baseURI` parses the `href` of the first `base` with one, up to the URL parser's 2 MiB, and
/// charges it one step per 64 bytes. A `javascript:` URL is parsed whole and thrown away (the
/// answer is the document's address), so only the read's charge pays for the parse: 1 and 2 MiB
/// of ASCII, and 1 MiB of two-byte letters, each percent-encoded by the parser.
#[test]
fn base_uri_parses_a_long_href() {
    let mut calls = Vec::new();
    for (what, tail) in [
        ("1 MiB of ASCII", "a".repeat((1 << 20) - 11)),
        ("2 MiB of ASCII", "a".repeat((2 << 20) - 11)),
        (
            "1 MiB of two-byte letters",
            "é".repeat(((1 << 20) - 12) / 2),
        ),
    ] {
        let href = format!("javascript:{tail}");
        let bytes = href.len() as u64;
        let page = format!("<!DOCTYPE html><base href=\"{href}\"><p></p>");
        calls.push(with_tree(limits(), parsed(page), move |runner| {
            ran(
                runner,
                "if (document.baseURI !== 'https://example.com/dir/page') \
                 throw new Error(document.baseURI.length);",
            );
            call(
                runner,
                &format!("baseURI, a javascript: href of {what}"),
                "document.baseURI;",
                bytes / 64,
            )
        }));
    }
    report("baseURI of a long href", &calls);
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

/// In no-quirks mode the token is hashed and compared, never rewritten: the rule's one step per
/// 64 bytes holds the mark.
#[test]
fn one_long_class_token_in_no_quirks_mode() {
    let calls = one_long_token("<!DOCTYPE html>", "no-quirks", 64);
    report("one long class token, no-quirks", &calls);
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

/// The wanted set of 65,536 distinct tokens against 4 elements whose `class` holds the same
/// 65,536 tokens: one lookup per token of each element, no work of the product.
#[test]
fn a_wanted_set_of_many_tokens_against_classes_of_many_tokens() {
    let tree = || {
        let classes: Vec<String> = (0..65_536).map(|index| format!("t{index}")).collect();
        let classes = classes.join(" ");
        let mut tree = parsed(format!("<!DOCTYPE html><div>{}</div>", "<p></p>".repeat(4)))();
        for element in elements(&tree, "p") {
            tree.set_attribute(element, "class", &classes)
                .expect("a class");
        }
        tree
    };
    let calls = with_tree(limits(), tree, |runner| {
        ran(
            runner,
            "var G = this; var a = []; for (var i = 0; i < 65536; i++) a.push('t' + i); \
             G.wanted = a.join(' ');",
        );
        vec![call(
            runner,
            "getElementsByClassName(65,536 tokens) over 4 elements of the same 65,536",
            "if (document.getElementsByClassName(wanted).length !== 4) throw 1;",
            4 * 65_536 + 65_536,
        )]
    });
    report("a wanted set against classes of many tokens", &calls);
}

/// One element of 1,024 attributes (the parser's bound), a `class` of 131,072 one-letter
/// tokens, an `id` of 256 KiB and a `name` of 256 KiB: the scans by name, the token charge and
/// the value charges on one element; `getElementById` and `namedItem` with an argument as long
/// as the `id`, different in its last byte.
#[test]
fn a_mixed_element_of_many_attributes_tokens_and_long_values() {
    let tree = || {
        let mut tree = parsed("<!DOCTYPE html><p></p>".to_string())();
        let element = elements(&tree, "p")[0];
        for index in 0..1_021 {
            tree.set_attribute(element, &format!("a{index}"), "v")
                .expect("an attribute");
        }
        tree.set_attribute(element, "class", &"a ".repeat(1 << 17))
            .expect("a class");
        tree.set_attribute(element, "id", &"i".repeat(1 << 18))
            .expect("an id");
        tree.set_attribute(element, "name", &"n".repeat(1 << 18))
            .expect("a name");
        tree
    };
    let calls = with_tree(limits(), tree, |runner| {
        ran(
            runner,
            "var G = this; G.id = 'i'; while (id.length < 262144) id = id + id; \
             G.id = id.slice(1) + 'j'; G.all = document.getElementsByTagName('p');",
        );
        let scan = 1_024 / 4;
        vec![
            call(
                runner,
                "getElementById(256 KiB, last byte differs)",
                "document.getElementById(id);",
                scan + (1 << 18) / 64,
            ),
            call(
                runner,
                "getElementsByClassName('a b')",
                "document.getElementsByClassName('a b');",
                scan + (1 << 18) / 64 + (1 << 17),
            ),
            call(
                runner,
                "namedItem(256 KiB, last byte differs)",
                "all.namedItem(id);",
                scan + 2 * (1 << 18) / 64,
            ),
        ]
    });
    report("a mixed element", &calls);
}

/// `isEqualNode` of two trees of 5,000 elements of 16 attributes that differ only in the last
/// attribute of the last element: every element's attributes are compared, one step each.
#[test]
fn is_equal_node_of_trees_that_differ_in_the_last_attribute_of_the_last_node() {
    let attributes: String = (0..16).map(|index| format!(" a{index}=\"v\"")).collect();
    let mut last = attributes.clone();
    last.replace_range(last.len() - 2..last.len() - 1, "w");
    let one = format!("<p{attributes}></p>").repeat(5_000);
    let other = format!(
        "{}<p{last}></p>",
        format!("<p{attributes}></p>").repeat(4_999)
    );
    let page = format!("<!DOCTYPE html><section>{one}</section><section>{other}</section>");
    let calls = with_tree(limits(), parsed(page), |runner| {
        ran(
            runner,
            "var G = this; G.s = document.getElementsByTagName('section'); \
             G.a = s[0]; G.b = s[1];",
        );
        vec![call(
            runner,
            "isEqualNode, the last attribute of the last of 5,000 elements differs",
            "if (a.isEqualNode(b)) throw 1;",
            5_000 * 17,
        )]
    });
    report("isEqualNode of two trees", &calls);
}

/// `compareDocumentPosition` across a deep and a wide tree: the deepest of a chain at the depth
/// bound under the first of 100,000 children, against the last child.
#[test]
fn compare_document_position_across_a_deep_and_a_wide_tree() {
    let depth = ParserLimits::default().max_depth - 8;
    let page = format!(
        "<!DOCTYPE html><div id=\"p\"><section>{}{}</section>{}</div>",
        "<div>".repeat(depth),
        "</div>".repeat(depth),
        "<span></span>".repeat(99_999)
    );
    let calls = with_tree(limits(), parsed(page), |runner| {
        ran(
            runner,
            "var G = this; G.p = document.getElementById('p'); G.e = p.firstChild; \
             while (e.lastChild) e = e.lastChild; G.l = p.lastChild;",
        );
        vec![call(
            runner,
            "compareDocumentPosition(the deepest, the last of 100,000)",
            "if (e.compareDocumentPosition(l) !== 4) throw 1;",
            100_000,
        )]
    });
    report("compareDocumentPosition, deep and wide", &calls);
}

/// `textContent` of a tree of 100,000 comments (the parser makes them; no text), and of a chain
/// at the depth bound with a text of one byte at each level.
#[test]
fn text_content_of_comments_and_of_short_texts_under_a_chain() {
    let comments = format!(
        "<!DOCTYPE html><div id=\"c\">{}</div>",
        "<!--c-->".repeat(100_000)
    );
    let mut calls = vec![with_tree(limits(), parsed(comments), |runner| {
        ran(runner, "var G = this; G.c = document.getElementById('c');");
        call(
            runner,
            "textContent of 100,000 comments",
            "if (c.textContent !== '') throw 1;",
            100_000,
        )
    })];
    let depth = ParserLimits::default().max_depth - 8;
    let chain = format!(
        "<!DOCTYPE html><div id=\"t\">{}</div>",
        "a<div>".repeat(depth)
    );
    calls.push(with_tree(limits(), parsed(chain), |runner| {
        ran(runner, "var G = this; G.t = document.getElementById('t');");
        call(
            runner,
            "textContent of a chain with a one-byte text at each level",
            "t.textContent;",
            2 * depth as u64,
        )
    }));
    report("textContent of comments and of short texts", &calls);
}

/// An ordinary page under the session's limits: 25,000 paragraphs of a 38-byte text (50,003
/// nodes, 950,000 bytes of text). One read of the document element's `textContent` costs more
/// than a hundredth of the session's 10,000,000 steps an entry through the copy's charge, so a
/// loop of 100 reads ends `limit: steps-per-script`; without that charge (the walk and the
/// 64-byte charge only) the hundred reads fit. The loop itself is not run here: it runs about
/// 5 s in the test profile.
#[test]
fn an_ordinary_page_read_once() {
    let text = "abcdefghijklmnopqrstuvwxyzabcdefghijkl";
    let page = format!(
        "<!DOCTYPE html><body>{}</body>",
        format!("<p>{text}</p>").repeat(25_000)
    );
    let bytes = (25_000 * text.len()) as u64;
    with_tree(session(), parsed(page), |runner| {
        let before = runner.steps();
        ran(runner, "document.documentElement.textContent;");
        let one = runner.steps() - before;
        let without = one - bytes / 16;
        eprintln!(
            "an ordinary page of 50,003 nodes and {bytes} bytes of text: \
             documentElement.textContent {one} steps, {without} without the copy's charge; \
             100 reads {} against {}",
            100 * one,
            SESSION_ENGINE_LIMITS.steps_per_entry
        );
        assert!(100 * one > SESSION_ENGINE_LIMITS.steps_per_entry, "{one}");
        assert!(
            100 * without < SESSION_ENGINE_LIMITS.steps_per_entry,
            "{one}"
        );
    });
}

/// The wanted set's charge of one step per 64 bytes of the argument, held by a floor: one token
/// of 256 KiB, so the per-token charge adds one step and the argument's copy and the set's bytes
/// are 4,096 steps each. The floor of an argument of many short tokens no longer needs the
/// bytes' charge, since each token now costs a step.
#[test]
fn the_wanted_set_charges_the_bytes_of_one_long_token() {
    let calls = with_tree(
        limits(),
        parsed("<!DOCTYPE html><p></p>".to_string()),
        |runner| {
            ran(
                runner,
                "var G = this; G.big = 'a'; while (big.length < 262144) big = big + big;",
            );
            vec![call(
                runner,
                "getElementsByClassName(one token of 256 KiB)",
                "document.getElementsByClassName(big);",
                2 * (1 << 18) / 64 + 1,
            )]
        },
    );
    report("the wanted set of one long token", &calls);
}

/// Members that read more than their own node and have no case in the sweep: `length` of a
/// text of 1 MiB (it counts the UTF-16 units of every character), and the element children
/// members over a parent of 100,000 comments and one element at its end.
#[test]
fn members_the_sweep_left_out() {
    let tree = || {
        let mut tree = parsed(format!(
            "<!DOCTYPE html><div id=\"c\">{}<b></b></div><p id=\"t\"></p>",
            "<!---->".repeat(100_000)
        ))();
        let p = elements(&tree, "p")[0];
        let text = tree.create_text_node(&"a".repeat(1 << 20)).expect("a text");
        tree.append_child(p, text).expect("appended");
        tree
    };
    let calls = with_tree(limits(), tree, |runner| {
        ran(
            runner,
            "var G = this; G.c = document.getElementById('c'); \
             G.t = document.getElementById('t').firstChild;",
        );
        vec![
            call(
                runner,
                "length of a text of 1 MiB",
                "if (t.length !== 1048576) throw 1;",
                (1 << 20) / 64,
            ),
            call(
                runner,
                "firstElementChild past 100,000 comments",
                "c.firstElementChild;",
                100_000,
            ),
            call(
                runner,
                "childElementCount of 100,000 comments and one element",
                "if (c.childElementCount !== 1) throw 1;",
                100_000,
            ),
            call(
                runner,
                "previousElementSibling of the element past 100,000 comments",
                "c.lastChild.previousElementSibling;",
                100_000,
            ),
        ]
    });
    report("members the sweep left out", &calls);
}
