//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`), fifth review: the
//! module of reads that charges before each read, one price for a byte (one step per 16), and
//! walks that copy no child list. Each case runs one call on a shape the parser admits (or the
//! tree's own writes make), prints its steps, its time and the time a step against the plain
//! walk's (`getElementById('none')` over 20,000 elements without attributes), and asserts
//! steps, counts and ends, never a time.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{Namespace, NodeData, NodeId, ParseOptions, Tree, parse_document};
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

/// The parser's bounds with room for 2,097,152 nodes; the others are its defaults.
fn many_nodes() -> ParserLimits {
    ParserLimits {
        max_nodes: 1 << 21,
        ..ParserLimits::default()
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

/// The lines of `source` that are code: a line comment and the rest of a line after `//` left
/// out.
fn code_lines(source: &str) -> Vec<(usize, &str)> {
    source
        .lines()
        .enumerate()
        .map(|(at, line)| (at + 1, line.split("//").next().unwrap_or("")))
        .filter(|(_, code)| !code.trim().is_empty())
        .collect()
}

/// Held. The round's guard of its structure reads `reading.rs` for the text `tree()`,
/// `tree_mut()` and `charge(` (`dom_reading_cost.rs:596-601`). These spellings pass it and
/// still read the tree or charge outside the module of reads: `DomCx::tree(cx)`, `cx.tree ()`,
/// a closure or a `use` alias of the method, `cx.engine().charge_steps(n)`, and a function of
/// `reads.rs` that hands the tree (or the engine) out. This guard reads the code of `reading.rs`
/// and of `mod.rs` for the words `tree`, `Tree`, `charge` and `engine`, and the signatures of
/// the functions `reads.rs` gives the members for a return type that lends a reference.
#[test]
fn the_guard_of_the_reads_holds_against_other_spellings() {
    let mut found = Vec::new();
    for (file, source) in [
        ("reading.rs", include_str!("../src/dom/reading.rs")),
        ("mod.rs", include_str!("../src/dom/mod.rs")),
    ] {
        for (line, code) in code_lines(source) {
            for word in ["tree", "Tree", "charge", "engine"] {
                if code.contains(word) {
                    found.push(format!("{file}:{line}: `{word}` in {:?}", code.trim()));
                }
            }
        }
    }
    let reads = include_str!("../src/dom/reads.rs");
    for (at, _) in reads.match_indices("pub(super) fn ") {
        let rest = &reads[at..];
        let signature = &rest[..rest.find('{').unwrap_or(rest.len())];
        let returns = signature.split("->").nth(1).unwrap_or("");
        for word in ["&", "Tree", "Cx"] {
            if returns.contains(word) {
                found.push(format!(
                    "reads.rs: `{word}` in the return type of {:?}",
                    signature.split_whitespace().collect::<Vec<_>>().join(" ")
                ));
            }
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}

/// Held, and what holds charges no other test holds. Long names (markup the parser admits
/// under its default bounds): `tagName`, `localName` and `nodeName` of an element named by
/// 1 MiB (the member's copy and the glue's, one step per 16 bytes each); `isEqualNode` of two
/// such elements (the name compared); `getElementsByTagName` with a name of 16 KiB against 64
/// elements whose names are as long and differ in the last byte (each compare reads the name);
/// and a doctype's public ID of 1 MiB.
#[test]
fn long_names_are_charged_where_they_are_read_or_compared() {
    const LONG: usize = 1 << 20;
    const SHORT: usize = 16 * 1024;
    let long = "a".repeat(LONG);
    let short = format!("{}b", "a".repeat(SHORT - 1));
    let wanted = format!("{}c", "a".repeat(SHORT - 1));
    let shorts = format!("<{short}></{short}>").repeat(64);
    let public = "p".repeat(LONG);
    let page = format!(
        "<!DOCTYPE html PUBLIC \"{public}\"><body><{long}></{long}><{long}></{long}>{shorts}"
    );
    let calls = with_tree(limits(), ParserLimits::default(), parsed(page), |runner| {
        ran(
            runner,
            "var G = this; G.x = document.documentElement.lastChild.firstChild; \
             G.y = x.nextSibling;",
        );
        let copied = 2 * LONG as u64 / 16;
        vec![
            call(runner, "tagName of a name of 1 MiB", "x.tagName;", copied),
            call(
                runner,
                "localName of a name of 1 MiB",
                "x.localName;",
                copied,
            ),
            call(runner, "nodeName of a name of 1 MiB", "x.nodeName;", copied),
            call(
                runner,
                "isEqualNode of two elements of a name of 1 MiB",
                "x.isEqualNode(y);",
                LONG as u64 / 16,
            ),
            call(
                runner,
                "getElementsByTagName(16 KiB) over 64 names of 16 KiB",
                &format!("document.getElementsByTagName('{wanted}');"),
                64 * SHORT as u64 / 16,
            ),
            call(
                runner,
                "doctype.publicId of 1 MiB",
                "document.doctype.publicId;",
                copied,
            ),
        ]
    });
    report("long names", &calls);
}

/// Held. A String argument the member reads only for a compare it ends at once: the glue's
/// copy is charged, one step per 8 code units, for ASCII and for a 2-byte character (whose
/// copy into UTF-8 writes 2 bytes a unit); `getElementsByTagName` also folds its argument to
/// ASCII lower case once, one step per 16 bytes.
#[test]
fn a_string_argument_of_4_mi_units_that_the_member_hardly_reads() {
    let calls = with_tree(
        limits(),
        ParserLimits::default(),
        parsed("<!DOCTYPE html><p></p>".to_string()),
        |runner| {
            ran(
                runner,
                "var G = this; G.big = 'a'; while (big.length < 4194304) big = big + big; \
                 G.wide = '\\u00e9'; while (wide.length < 4194304) wide = wide + wide; \
                 G.p = document.documentElement;",
            );
            let floor = (4 << 20) * 2 / 16;
            vec![
                call(
                    runner,
                    "isDefaultNamespace(4 Mi ASCII units)",
                    "p.isDefaultNamespace(big);",
                    floor,
                ),
                call(
                    runner,
                    "isDefaultNamespace(4 Mi units of a 2-byte character)",
                    "p.isDefaultNamespace(wide);",
                    floor,
                ),
                call(
                    runner,
                    "getElementsByTagName(4 Mi ASCII units), folded once",
                    "document.getElementsByTagName(big);",
                    floor + (4 << 20) / 16,
                ),
            ]
        },
    );
    report("a String argument the member hardly reads", &calls);
}

/// Held. `string_length` counts UTF-16 code units, not bytes: under a bound of 4,096 units, a
/// value of exactly 4,096 units of a 2-, 3- or 4-byte character (8,192 to 12,288 bytes, past the
/// bound in bytes) is answered whole by `data`, `textContent`, `wholeText` and `title`, and one
/// character more is the realm's `RangeError`.
#[test]
fn string_length_in_code_units_at_and_past_its_bound() {
    const UNITS: usize = 4_096;
    let mut bounded = limits();
    bounded.engine.string_length = UNITS as u64;
    for (character, units) in [("a", 1), ("\u{e9}", 1), ("\u{20ac}", 1), ("\u{1f600}", 2)] {
        for extra in [0, 1] {
            let count = UNITS / units + extra;
            let text = character.repeat(count);
            let page = format!("<!DOCTYPE html><title>{text}</title><p id=\"l\">{text}</p>");
            let source = if extra == 0 {
                format!(
                    "{CHECK} var l = document.getElementById('l'); var t = l.firstChild; \
                     check(t.data.length === {UNITS}, 'data ' + t.data.length); \
                     check(l.textContent.length === {UNITS}, 'textContent'); \
                     check(t.wholeText.length === {UNITS}, 'wholeText'); \
                     check(document.title.length === {UNITS}, 'title'); \
                     check(t.length === {UNITS}, 'length');"
                )
            } else {
                format!(
                    "{CHECK} var l = document.getElementById('l'); var t = l.firstChild; \
                     rangeError(function () {{ return t.data; }}, 'data'); \
                     rangeError(function () {{ return l.textContent; }}, 'textContent'); \
                     rangeError(function () {{ return t.wholeText; }}, 'wholeText'); \
                     rangeError(function () {{ return document.title; }}, 'title'); \
                     check(t.length === {}, 'length');",
                    UNITS + units
                )
            };
            let end = with_tree(
                bounded,
                ParserLimits::default(),
                parsed(page),
                move |runner| runner.evaluate(&source),
            );
            assert_eq!(end, ScriptEnd::Ran, "{character:?} x {count}: {end}");
        }
    }
}

/// Held. `title` and the shapes of its element: an SVG `title` is not the document's; a `title`
/// in the body is; no `title` answers the empty String after a walk of the whole tree; a `title`
/// of 50,000 text children (made through the tree's writes; the parser merges a title's text)
/// charges a step a child and a step a word; and one word of 2 MiB is charged at one step per
/// 16 bytes where the member reads it and again where the glue copies it.
#[test]
fn title_over_its_shapes() {
    let answers = with_tree(
        limits(),
        ParserLimits::default(),
        parsed(
            "<!DOCTYPE html><svg><title>s</title></svg><p>x</p><title> b \t\n c </title>"
                .to_string(),
        ),
        |runner| {
            runner.evaluate(&format!(
                "{CHECK} check(document.title === 'b c', 'title: ' + document.title);"
            ))
        },
    );
    assert_eq!(answers, ScriptEnd::Ran, "{answers}");
    let none = with_tree(
        limits(),
        ParserLimits::default(),
        parsed("<!DOCTYPE html><svg><title>s</title></svg>".to_string()),
        |runner| {
            runner.evaluate(&format!(
                "{CHECK} check(document.title === '', 'title: ' + document.title);"
            ))
        },
    );
    assert_eq!(none, ScriptEnd::Ran, "{none}");
    let mut calls = Vec::new();
    let tree = || {
        let mut tree = parsed("<!DOCTYPE html><title></title>".to_string())();
        let title = elements(&tree, "title")[0];
        for _ in 0..50_000 {
            let text = tree.create_text_node(" a").expect("a text");
            tree.append_child(title, text).expect("appended");
        }
        tree
    };
    calls.push(with_tree(
        limits(),
        ParserLimits::default(),
        tree,
        |runner| {
            call(
                runner,
                "title of 50,000 text children of one word",
                "document.title;",
                50_000 + 50_000,
            )
        },
    ));
    let page = format!("<!DOCTYPE html><title>{}</title>", "a".repeat(2 << 20));
    calls.push(with_tree(
        limits(),
        ParserLimits::default(),
        parsed(page),
        |runner| {
            call(
                runner,
                "title of one word of 2 MiB",
                "document.title;",
                2 * (2 << 20) / 16,
            )
        },
    ));
    report("title over its shapes", &calls);
}

/// Held. `baseURI` of a `base` whose `href` is a path of 1 MiB of `é`: the URL parser writes
/// each as `%C3%A9`, so the answer is 3 MiB. The `href` is charged before it is read, the answer
/// where the member makes it and where the glue copies it.
#[test]
fn base_uri_of_an_href_that_percent_encoding_triples() {
    let href = format!("/{}", "\u{e9}".repeat(1 << 19));
    let page = format!("<!DOCTYPE html><base href=\"{href}\"><p></p>");
    let answer = "https://example.com".len() as u64 + 1 + 3 * (1 << 20);
    let calls = with_tree(limits(), ParserLimits::default(), parsed(page), |runner| {
        vec![call(
            runner,
            "baseURI of an href of 1 MiB of a 2-byte character",
            &format!(
                "{CHECK} var u = document.documentElement.baseURI; \
                 check(u.length === {answer}, 'length ' + u.length);"
            ),
            href.len() as u64 / 16 + 2 * (answer / 16),
        )]
    });
    report("baseURI of an href that percent-encoding triples", &calls);
}

/// Held, by steps; the walk's copy of a child list is not held by any test (the time is
/// printed only). A tree of a parent of 1,048,576 children (made through the tree's writes,
/// under a raised node bound) beside a chain of elements as deep as the depth bound: the calls
/// that stop at the first child, or reach the deepest element of the chain, charge a step a node
/// they visit and visit no child of the large parent past the first.
#[test]
fn a_parent_of_a_mebi_children_beside_a_chain_at_the_depth_bound() {
    const CHILDREN: usize = 1 << 20;
    let depth = ParserLimits::default().max_depth;
    let tree = move || {
        let mut tree = parsed_under(
            "<!DOCTYPE html><div id=\"c\"></div><div id=\"p\"></div>".to_string(),
            many_nodes(),
        )();
        let found = elements(&tree, "div");
        let mut at = found[0];
        for _ in 0..depth {
            let Ok(next) = tree.create_element_ns(Namespace::Html, "i") else {
                break;
            };
            if tree.append_child(at, next).is_err() {
                break;
            }
            at = next;
        }
        for _ in 0..CHILDREN {
            let child = tree
                .create_element_ns(Namespace::Html, "b")
                .expect("a child");
            tree.append_child(found[1], child).expect("appended");
        }
        tree
    };
    let calls = with_tree(limits(), many_nodes(), tree, |runner| {
        ran(
            runner,
            "var G = this; G.p = document.getElementById('p'); \
             G.e = document.getElementById('c'); while (e.lastChild) e = e.lastChild; \
             G.d = document.documentElement;",
        );
        vec![
            call(
                runner,
                "p.contains(p.firstChild)",
                "p.contains(p.firstChild);",
                2,
            ),
            call(runner, "p.firstElementChild", "p.firstElementChild;", 1),
            call(
                runner,
                "d.contains(the deepest of the chain)",
                "d.contains(e);",
                400,
            ),
            call(
                runner,
                "the deepest.compareDocumentPosition(p.lastChild)",
                "e.compareDocumentPosition(p.lastChild);",
                400,
            ),
            call(
                runner,
                "getElementById('c'), the chain's top before the parent",
                "document.getElementById('c');",
                4,
            ),
        ]
    });
    let printed = lines(&calls);
    eprintln!("a parent of 1,048,576 children:\n{}", printed.join("\n"));
    // A script of one call costs about ten steps of its own (the entry, the reads of `G`'s
    // properties, the getters), beside the member's.
    let most = [64, 64, 2_000, 2_000, 64];
    for (call, most) in calls.iter().zip(most) {
        assert!(
            call.floor <= call.steps && call.steps <= most,
            "{}: {} steps, not in {}..={most}",
            call.what,
            call.steps,
            call.floor
        );
    }
}
