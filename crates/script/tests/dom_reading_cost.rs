//! What a step of the slice `reading` buys (`story:scripts-dom-reading`): for each member that
//! reads more than its own node, the worst shape of a tree the parser admits for it (many short
//! units: one-letter classes, one-byte texts, 1,024 to 16,384 attributes under a raised bound, a
//! chain as deep as the depth bound, a parent of 100,000 children), one call, its steps and its
//! time, and the time a step against the plain walk's (`getElementById('none')` over 20,000
//! elements without attributes). The lines are printed; the tests assert the steps (a floor of
//! one step per unit of work the call does), never a time: the machine's load moves those. One
//! run a shape, no loop to steady a number.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeData, NodeId, ParseOptions, Tree, WriteError, parse_document};
use surf_script::bindings::{DomCx, DomEnd, DomException, DomResult, Mutation, Slices};
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
            string_length: 1 << 22,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 1_000_000_000,
        scripts_per_runner: 1_000,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(50),
    }
}

/// The parser's bounds, with 16,384 attributes an element and room for long values.
fn raised() -> ParserLimits {
    ParserLimits {
        max_attributes: 16_384,
        max_stored_bytes: 256 * 1024 * 1024,
        ..ParserLimits::default()
    }
}

fn info() -> DocumentInfo {
    DocumentInfo {
        address: Url::parse("https://example.com/dir/page", None).expect("the address"),
        content_type: "text/html".to_string(),
        encoding: "UTF-8".to_string(),
    }
}

fn refused(error: WriteError) -> DomEnd {
    DomEnd::Throw(DomException::new(
        "HierarchyRequestError",
        format!("{error:?}"),
    ))
}

/// The writes a refill needs: `appendChild` and `cloneNode`.
struct TestMutation;

impl Mutation for TestMutation {
    fn node_append_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
    ) -> DomResult<NodeId> {
        cx.tree_mut().append_child(this, node).map_err(refused)?;
        Ok(node)
    }

    fn node_clone_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        subtree: bool,
    ) -> DomResult<NodeId> {
        cx.tree_mut().clone_node(this, subtree).map_err(refused)
    }
}

fn slices() -> Slices {
    let mut slices = dom::slices(info());
    slices.mutation = Box::new(TestMutation);
    slices
}

fn with_tree<R: Send>(
    tree: impl FnOnce() -> Tree + Send,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(
        limits(),
        Mode::Test,
        move || Document {
            tree: tree(),
            limits: raised(),
            slices: slices(),
        },
        body,
    )
    .expect("a runner with the bindings")
}

fn parsed(page: String) -> impl FnOnce() -> Tree + Send {
    move || {
        parse_document(
            &page,
            &ParseOptions {
                limits: raised(),
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

/// The plain walk's time a step, in nanoseconds: one call of `getElementById('none')` over
/// 20,000 elements without attributes.
fn plain() -> f64 {
    let page = format!("<!DOCTYPE html><div>{}</div>", "<p></p>".repeat(20_000));
    let plain = with_tree(parsed(page), |runner| {
        call(runner, "plain", "document.getElementById('none');", 20_000)
    });
    per_step(&plain)
}

fn per_step(call: &Call) -> f64 {
    call.took.as_nanos() as f64 / call.steps.max(1) as f64
}

/// Prints a line a call, against the plain walk, and asserts each call's floor of steps.
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

const N: u64 = 100_000;

/// The children of the parent a collection's own loop runs over: half of `N`; the time a step
/// is the same, and the test stays short.
const H: u64 = 50_000;

/// Runs `calls` on a parent of `children` children, the parent as `p`, the document element as
/// `d`, and prints them under `name`.
fn on_a_parent_of_children(
    children: u64,
    name: &str,
    calls: impl FnOnce(&mut Runner) -> Vec<Call> + Send,
) {
    let page = format!(
        "<!DOCTYPE html><div id=\"p\">{}</div>",
        "<b></b>".repeat(children as usize)
    );
    let calls = with_tree(parsed(page), |runner| {
        let end = runner.evaluate(
            "var G = this; G.p = document.getElementById('p'); G.d = document.documentElement;",
        );
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        calls(runner)
    });
    report(name, &calls);
}

/// A parent of 100,000 children: the walks over every node, the list of element children, the
/// sibling read without a hint, the order of two children, and the text of the whole tree.
#[test]
fn walks_over_a_parent_of_100_000_children() {
    on_a_parent_of_children(N, "walks over a parent of 100,000 children", |runner| {
        vec![
            call(
                runner,
                "getElementsByTagName('*')",
                "document.getElementsByTagName('*');",
                8 * N,
            ),
            call(
                runner,
                "getElementsByTagNameNS('*', '*')",
                "document.getElementsByTagNameNS('*', '*');",
                8 * N,
            ),
            call(
                runner,
                "contains(the last child)",
                "d.contains(p.lastChild);",
                N,
            ),
            call(runner, "children", "p.children;", 8 * N),
            call(
                runner,
                "previousSibling of the last child, no hint",
                "p.lastChild.previousSibling;",
                N - 1,
            ),
            call(
                runner,
                "compareDocumentPosition of the two last children",
                "G.l = p.lastChild; G.k = p.children[99998]; l.compareDocumentPosition(k);",
                N - 1,
            ),
            call(
                runner,
                "textContent of 100,001 elements",
                "d.textContent;",
                N,
            ),
        ]
    });
}

/// The `childNodes` of a parent of 100,000 children: its first read, its refill after an
/// append, and one item; and a script of one number, whose time is what a call of a few steps
/// costs before its member: the entry, the parse, the answer.
#[test]
fn the_child_list_of_100_000_children() {
    on_a_parent_of_children(N, "the child list of 100,000 children", |runner| {
        vec![
            call(
                runner,
                "childNodes, first read",
                "G.c = p.childNodes;",
                8 * N,
            ),
            call(
                runner,
                "childNodes refill after an append",
                "p.appendChild(p.lastChild.cloneNode(false)); p.childNodes;",
                16 * N,
            ),
            call(runner, "item(100,000)", "c.item(100000);", 1),
            call(runner, "a script of one number, for its own cost", "0;", 0),
        ]
    });
}

/// `forEach` over the `childNodes` of 50,000 children, the read of the list with it.
#[test]
fn for_each_over_50_000_children() {
    on_a_parent_of_children(H, "forEach over 50,000 children", |runner| {
        vec![call(
            runner,
            "forEach",
            "var c = p.childNodes; var n = 0; c.forEach(function () { n++; });",
            8 * H,
        )]
    });
}

/// The iterator over the `childNodes` of 50,000 children, the read of the list with it.
#[test]
fn the_iterator_over_50_000_children() {
    on_a_parent_of_children(H, "the iterator over 50,000 children", |runner| {
        vec![call(
            runner,
            "the iterator",
            "var c = p.childNodes; var m = 0; for (var x of c) m++;",
            8 * H,
        )]
    });
}

/// An element of 16,384 attributes (under a raised bound; the tree's own write of an attribute
/// scans its list, so two such elements are made), each attribute one byte of name and none of
/// value: the scans for one attribute by name pass them at one step per 4, a compare of whole
/// lists reads each.
#[test]
fn elements_of_16_384_attributes() {
    const N: u64 = 16_384;
    let tree = || {
        let mut tree = parsed("<!DOCTYPE html><p></p><p></p>".to_string())();
        for element in elements(&tree, "p") {
            for index in 0..16_384 {
                tree.set_attribute(element, &format!("a{index}"), "")
                    .expect("an attribute");
            }
        }
        tree
    };
    let calls = with_tree(tree, |runner| {
        let end = runner.evaluate(
            "var G = this; G.all = document.getElementsByTagName('p'); \
             G.a = all[0]; G.b = all[1];",
        );
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        vec![
            call(
                runner,
                "getElementById('none')",
                "document.getElementById('none');",
                2 * N / 4,
            ),
            call(
                runner,
                "getElementsByClassName('zz')",
                "document.getElementsByClassName('zz');",
                2 * N / 4,
            ),
            call(
                runner,
                "namedItem('none')",
                "all.namedItem('none');",
                2 * N / 4,
            ),
            call(runner, "baseURI", "a.baseURI;", 2 * N / 4),
            call(
                runner,
                "lookupNamespaceURI('zz')",
                "a.lookupNamespaceURI('zz');",
                N / 4,
            ),
            call(
                runner,
                "lookupPrefix('urn:x')",
                "a.lookupPrefix('urn:x');",
                N / 4,
            ),
            call(runner, "isEqualNode", "a.isEqualNode(b);", N),
        ]
    });
    report("elements of 16,384 attributes", &calls);
}

/// 50 and 200 elements of 1,024 attributes and one of 8,192 (under a raised bound), each
/// attribute one byte of name and none of value: the walks for one attribute by name.
#[test]
fn many_elements_of_many_attributes() {
    let mut calls = Vec::new();
    for (count, attributes) in [(50_u64, 1_024_u64), (200, 1_024), (1, 8_192)] {
        let tree = move || {
            let page = format!("<!DOCTYPE html>{}", "<p></p>".repeat(count as usize));
            let mut tree = parsed(page)();
            for element in elements(&tree, "p") {
                for index in 0..attributes {
                    tree.set_attribute(element, &format!("a{index}"), "")
                        .expect("an attribute");
                }
            }
            tree
        };
        let floor = count * attributes / 4;
        calls.extend(with_tree(tree, |runner| {
            let end = runner.evaluate("var G = this; G.all = document.getElementsByTagName('p');");
            assert_eq!(end, ScriptEnd::Ran, "{end}");
            let shape = format!("{count} x {attributes}");
            vec![
                call(
                    runner,
                    &format!("{shape}: getElementById('none')"),
                    "document.getElementById('none');",
                    floor,
                ),
                call(
                    runner,
                    &format!("{shape}: getElementsByClassName('zz')"),
                    "document.getElementsByClassName('zz');",
                    floor,
                ),
                call(
                    runner,
                    &format!("{shape}: namedItem('none')"),
                    "all.namedItem('none');",
                    floor,
                ),
            ]
        }));
    }
    report("many elements of many attributes", &calls);
}

/// The class walk over elements whose `class` is 131,072 one-letter classes (256 KiB), 4 and
/// 40 of them, in `mode`: one step per class token it reads.
fn class_tokens(doctype: &'static str, mode: &str) {
    let mut calls = Vec::new();
    for count in [4_u64, 40] {
        let tree = move || {
            let classes = "a ".repeat(131_072);
            let page = format!("{doctype}<div>{}</div>", "<p></p>".repeat(count as usize));
            let mut tree = parsed(page)();
            for element in elements(&tree, "p") {
                tree.set_attribute(element, "class", &classes)
                    .expect("a class");
            }
            tree
        };
        calls.push(with_tree(tree, |runner| {
            call(
                runner,
                &format!("{count} classes of 256 KiB in {mode}, getElementsByClassName('b')"),
                "document.getElementsByClassName('b');",
                count * 131_072,
            )
        }));
    }
    report(&format!("class tokens, {mode}"), &calls);
}

#[test]
fn class_tokens_in_no_quirks_mode() {
    class_tokens("<!DOCTYPE html>", "no-quirks");
}

#[test]
fn class_tokens_in_quirks_mode() {
    class_tokens("", "quirks");
}

/// The wanted set built from an argument of 131,072 one-letter classes (256 KiB), over a
/// document of one element without a class: one step per token of the argument.
#[test]
fn an_argument_of_131_072_classes() {
    let calls = with_tree(parsed("<!DOCTYPE html><p></p>".to_string()), |runner| {
        let end = runner
            .evaluate("var G = this; G.big = 'a '; while (big.length < 262144) big = big + big;");
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        vec![call(
            runner,
            "getElementsByClassName of 131,072 classes",
            "document.getElementsByClassName(big);",
            131_072,
        )]
    });
    report("an argument of 131,072 classes", &calls);
}

/// Texts: 50,000 adjacent texts of one byte (made through the tree's writes, since the parser
/// joins adjacent text), and one text of 1 MiB, whose String answer the glue copies at one step
/// per 16 bytes (per 8 UTF-16 units) beside the member's step per 64 bytes.
#[test]
fn many_short_texts_and_one_long_text() {
    const COPIED: u64 = (1 << 20) / 64 + (1 << 20) / 16;
    let tree = || {
        let mut tree = parsed("<!DOCTYPE html><p id=\"t\"></p><p id=\"l\"></p>".to_string())();
        let found = elements(&tree, "p");
        for _ in 0..50_000 {
            let text = tree.create_text_node("a").expect("a text");
            tree.append_child(found[0], text).expect("appended");
        }
        let long = tree.create_text_node(&"a".repeat(1 << 20)).expect("a text");
        tree.append_child(found[1], long).expect("appended");
        tree
    };
    let calls = with_tree(tree, |runner| {
        let end = runner.evaluate(
            "var G = this; G.t = document.getElementById('t'); \
             G.l = document.getElementById('l').firstChild;",
        );
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        vec![
            call(
                runner,
                "wholeText of 50,000 texts of one byte",
                "t.firstChild.wholeText;",
                50_000,
            ),
            call(
                runner,
                "textContent of 50,000 texts of one byte",
                "t.textContent;",
                50_000,
            ),
            call(runner, "data of 1 MiB", "l.data;", COPIED),
            call(runner, "nodeValue of 1 MiB", "l.nodeValue;", COPIED),
            call(
                runner,
                "substringData(0, 1 Mi) of 1 MiB",
                "l.substringData(0, 1048576);",
                (1 << 20) / 64 + (2 << 20) / 16,
            ),
        ]
    });
    report("many short texts and one long text", &calls);
}

/// A chain of elements as deep as the parser's depth bound, read from its deepest element: the
/// namespace lookups, the root, and the order against the document element.
#[test]
fn a_chain_as_deep_as_the_depth_bound() {
    let depth = ParserLimits::default().max_depth;
    let page = format!("<!DOCTYPE html>{}", "<div>".repeat(depth));
    let calls = with_tree(parsed(page), |runner| {
        let end = runner.evaluate(
            "var G = this; G.e = document.documentElement; \
             while (e.lastChild) e = e.lastChild;",
        );
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        let deep = runner.evaluate(
            "var k = 0; for (var x = e; x; x = x.parentNode) k++; \
             if (k < 400) throw new Error('k ' + k);",
        );
        assert_eq!(deep, ScriptEnd::Ran, "{deep}");
        let floor = 400;
        vec![
            call(
                runner,
                "lookupPrefix('urn:x')",
                "e.lookupPrefix('urn:x');",
                floor,
            ),
            call(
                runner,
                "lookupNamespaceURI('zz')",
                "e.lookupNamespaceURI('zz');",
                floor,
            ),
            call(
                runner,
                "isDefaultNamespace(null)",
                "e.isDefaultNamespace(null);",
                1,
            ),
            call(runner, "getRootNode()", "e.getRootNode();", floor),
            call(runner, "a script of one number, for its own cost", "0;", 0),
            call(
                runner,
                "compareDocumentPosition(documentElement)",
                "e.compareDocumentPosition(document.documentElement);",
                floor,
            ),
            call(
                runner,
                "contains(the deepest)",
                "document.contains(e);",
                floor,
            ),
        ]
    });
    report("a chain as deep as the depth bound", &calls);
}

/// The members of `reading` read the tree only through its module of reads, whose functions
/// charge before their loops: the source of `reading.rs` calls neither `tree()` nor
/// `tree_mut()`, and charges nothing of its own.
#[test]
fn the_members_read_the_tree_only_through_the_reads() {
    let source = include_str!("../src/dom/reading.rs");
    for call in ["tree()", "tree_mut()", "charge("] {
        assert!(!source.contains(call), "reading.rs holds `{call}`");
    }
}
