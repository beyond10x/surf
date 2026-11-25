//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`), third review: the
//! refill a memory end cuts, swept over every point of five shapes; the attribute charges
//! against the time of the work they pay for; `substringData` at and past its bounds. A hostile
//! page chooses the markup and every call; the writes go through the test's slices.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeData, NodeId, ParseOptions, Tree, WriteError, parse_document};
use surf_script::bindings::{DomCx, DomEnd, DomException, DomResult, Mutation, Slices};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{Document, Mode, Runner, RunnerLimits, ScriptEnd, run_document};
use surf_url::Url;
use swell::Limits;

const ADDRESS: &str = "https://example.com/dir/page";

fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 10_000_000,
            memory_bytes: 64 * 1024 * 1024,
            frame_depth: 1_000,
            reentry_depth: 64,
            parser_nesting: 1_000,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 1_000_000_000,
        scripts_per_runner: 100,
        source_bytes: 1 << 22,
        wall_clock: Duration::from_secs(50),
    }
}

fn info() -> DocumentInfo {
    DocumentInfo {
        address: Url::parse(ADDRESS, None).expect("the address"),
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

/// The test's `mutation` slice: `appendChild`, `insertBefore`, `removeChild`, `cloneNode` and
/// `createDocumentFragment`, through the tree's public writes.
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

    fn node_insert_before(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
        child: Option<NodeId>,
    ) -> DomResult<NodeId> {
        cx.tree_mut()
            .insert_before(this, node, child)
            .map_err(refused)?;
        Ok(node)
    }

    fn node_remove_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        child: NodeId,
    ) -> DomResult<NodeId> {
        if cx.tree().parent(child) != Some(this) {
            return Err(DomEnd::Throw(DomException::new(
                "NotFoundError",
                "not a child",
            )));
        }
        cx.tree_mut().remove_node(child).map_err(refused)?;
        Ok(child)
    }

    fn node_clone_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        subtree: bool,
    ) -> DomResult<NodeId> {
        cx.tree_mut().clone_node(this, subtree).map_err(refused)
    }

    fn document_create_document_fragment(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<NodeId> {
        cx.tree_mut().create_document_fragment().map_err(refused)
    }
}

fn slices() -> Slices {
    let mut slices = dom::slices(info());
    slices.mutation = Box::new(TestMutation);
    slices
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
            slices: slices(),
        },
        body,
    )
    .expect("a runner with the bindings")
}

fn with_page<R: Send>(
    limits: RunnerLimits,
    page: String,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    with_tree(
        limits,
        move || parse_document(&page, &ParseOptions::default()),
        body,
    )
}

/// `p` elements of the tree, in tree order.
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

/// The steps and the time of `rounds` calls of `call` in one script, after `setup`: the least
/// time of three such scripts, so that the load of the machine weighs less.
fn timed(runner: &mut Runner, setup: &str, call: &str, rounds: u32) -> (u64, Duration) {
    let end = runner.evaluate(setup);
    assert_eq!(end, ScriptEnd::Ran, "{setup}: {end}");
    let source = format!("for (var round = 0; round < {rounds}; round++) {{ {call} }}");
    let mut least = Duration::MAX;
    let mut spent = 0;
    for _ in 0..3 {
        let before = runner.steps();
        let at = Instant::now();
        let end = runner.evaluate(&source);
        least = least.min(at.elapsed());
        assert_eq!(end, ScriptEnd::Ran, "{call}: {end}");
        spent = runner.steps() - before;
    }
    (spent, least)
}

/// A scan for one attribute by name over elements whose attributes it passes or whose long
/// values it compares, against the plain walk of the same member over elements without
/// attributes: the time per step of each, printed beside the plain walk's. The mark is a factor
/// of 20; the class of many one-letter classes passes it, since the walk charges one step per
/// 64 bytes of the value and makes one set lookup per class, one per 2 bytes.
#[test]
fn a_class_of_many_short_classes_costs_time_out_of_proportion_to_its_steps() {
    let started = Instant::now();
    let raised = ParserLimits {
        max_attributes: 200_000,
        max_stored_bytes: 256 * 1024 * 1024,
        ..ParserLimits::default()
    };
    let mut many = limits();
    many.engine.steps_per_entry = 50_000_000;
    let mut cases: Vec<(String, u64, Duration)> = Vec::new();
    // The plain walk: 20,000 elements without attributes.
    let page = format!("<!DOCTYPE html><div>{}</div>", "<p></p>".repeat(20_000));
    let plain = with_page(many, page, |runner| {
        timed(runner, "var x = 1;", "document.getElementById('none');", 20)
    });
    cases.push((
        "plain walk, getElementById('none')".to_string(),
        plain.0,
        plain.1,
    ));
    // 50 elements of 1,024 attributes (the parser's default bound), none of them `id`,
    // `class` or `name`; then one element of 8,192 attributes under a raised bound (the
    // tree's own write of an attribute scans the list, so the setup is quadratic in it).
    for (count, width) in [(50_usize, 1_024_usize), (1, 8_192)] {
        let tree = move || {
            let options = ParseOptions {
                limits: raised,
                ..ParseOptions::default()
            };
            let mut tree = parse_document(
                &format!("<!DOCTYPE html><div>{}</div>", "<p></p>".repeat(count)),
                &options,
            );
            for element in elements(&tree, "p") {
                for index in 0..width {
                    tree.set_attribute(element, &format!("a{index}"), "")
                        .expect("an attribute");
                }
            }
            tree
        };
        let found = with_tree(many, tree, |runner| {
            let mut found = Vec::new();
            for (what, call) in [
                ("getElementById('none')", "document.getElementById('none');"),
                (
                    "getElementsByClassName('zz')",
                    "document.getElementsByClassName('zz');",
                ),
                ("namedItem('none')", "all.namedItem('none');"),
            ] {
                let (steps, took) = timed(
                    runner,
                    "var G = this; G.all = document.getElementsByTagName('p');",
                    call,
                    3,
                );
                found.push((format!("{count} x {width} attributes, {what}"), steps, took));
            }
            found
        });
        cases.extend(found);
    }
    // Long values read or compared: 4 elements whose `class` is 131,072 one-letter classes
    // (256 KiB), in no-quirks and in quirks mode; `id` and `name` of 512 Ki units that differ
    // from the argument in the last unit; an SVG element's `name` (namedItem reads it for
    // HTML elements only).
    for (doctype, mode) in [("<!DOCTYPE html>", "no-quirks"), ("", "quirks")] {
        let classes = "a ".repeat(131_072);
        let tree = move || {
            let mut tree = parse_document(
                &format!("{doctype}<div>{}</div>", "<p></p>".repeat(4)),
                &ParseOptions::default(),
            );
            for element in elements(&tree, "p") {
                tree.set_attribute(element, "class", &classes)
                    .expect("a class");
            }
            tree
        };
        let found = with_tree(many, tree, |runner| {
            let (steps, took) = timed(
                runner,
                "var x = 1;",
                "document.getElementsByClassName('b');",
                1,
            );
            (steps, took)
        });
        cases.push((
            format!("4 classes of 256 KiB in {mode}, getElementsByClassName('b')"),
            found.0,
            found.1,
        ));
    }
    let long = format!("{}y", "x".repeat((1 << 19) - 1));
    let tree = move || {
        let mut tree = parse_document(
            &format!(
                "<!DOCTYPE html><div>{}</div><svg><g></g></svg>",
                "<p></p>".repeat(10)
            ),
            &ParseOptions::default(),
        );
        for element in elements(&tree, "p") {
            tree.set_attribute(element, "id", &long).expect("an id");
            tree.set_attribute(element, "name", &long).expect("a name");
        }
        for element in elements(&tree, "g") {
            tree.set_attribute(element, "name", &long).expect("a name");
        }
        tree
    };
    let found = with_tree(many, tree, |runner| {
        let setup = "var G = this; G.arg = 'x'; while (arg.length < 524288) arg = arg + arg;\
                     G.all = document.getElementsByTagName('*');";
        let mut found = Vec::new();
        for (what, call) in [
            (
                "getElementById(512 Ki units)",
                "document.getElementById(arg);",
            ),
            (
                "namedItem(512 Ki units), id and name",
                "all.namedItem(arg);",
            ),
        ] {
            let (steps, took) = timed(runner, setup, call, 1);
            found.push((
                format!("10 ids and names of 512 Ki units, {what}"),
                steps,
                took,
            ));
        }
        found
    });
    cases.extend(found);
    let per_step = |steps: u64, took: Duration| took.as_nanos() as f64 / steps.max(1) as f64;
    let plain_per_step = per_step(plain.0, plain.1);
    let mut lines = Vec::new();
    let mut over = Vec::new();
    for (what, steps, took) in &cases {
        let ratio = per_step(*steps, *took) / plain_per_step;
        let line = format!(
            "{what}: {steps} steps, {took:?}, {:.1} ns a step, {ratio:.1} x the plain walk's",
            per_step(*steps, *took)
        );
        if ratio > 20.0 && what.contains("classes of 256 KiB") {
            over.push(line.clone());
        }
        lines.push(line);
    }
    lines.push(format!("all: {:?}", started.elapsed()));
    eprintln!("{}", lines.join("\n"));
    assert!(over.is_empty(), "{over:#?}");
}

/// `namedItem` reads `name` for an element of the HTML namespace only, and the ruling charges
/// the bytes of the values it reads or compares and no others. An SVG element's `name` of
/// 64,000 bytes is not read: its charge is the element's step. An HTML element's `name` of the
/// same length is read: about 1,000 steps.
#[test]
fn named_item_charges_an_svg_name_it_does_not_read() {
    let long = "n".repeat(64_000);
    let page = format!("<!DOCTYPE html><p name=\"{long}\"></p><svg><g name=\"{long}\"></g></svg>");
    let (html, svg) = with_page(limits(), page, |runner| {
        let end = runner.evaluate(
            "var G = this; G.p = document.getElementsByTagName('p');\
             G.g = document.getElementsByTagName('g');",
        );
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        let mut spent = |source: &str| {
            let before = runner.steps();
            let end = runner.evaluate(source);
            assert_eq!(end, ScriptEnd::Ran, "{source}: {end}");
            runner.steps() - before
        };
        let base = spent("var x = 1;");
        (
            spent("p.namedItem('none');") - base,
            spent("g.namedItem('none');") - base,
        )
    });
    eprintln!("namedItem('none'): HTML element, name read: {html} steps; SVG element: {svg} steps");
    assert!(svg < 50, "the SVG element's name is not read: {svg} steps");
}

/// `lookupNamespaceURI(prefix)` looks for the attribute `xmlns:prefix` by its name on each
/// element up the tree and reads only the value of the one it finds: a scan for one attribute
/// by name. An element with one other attribute of 64,000 bytes and none of `xmlns`: the
/// value is not read, so its bytes are not charged.
#[test]
fn lookup_namespace_uri_charges_values_it_does_not_read() {
    let long = "v".repeat(64_000);
    let page = format!("<!DOCTYPE html><p id=\"t\" data-long=\"{long}\"></p><p id=\"u\"></p>");
    let (long_steps, short_steps) = with_page(limits(), page, |runner| {
        let end = runner.evaluate(
            "var G = this; G.t = document.getElementById('t'); G.u = document.getElementById('u');",
        );
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        let mut spent = |source: &str| {
            let before = runner.steps();
            let end = runner.evaluate(source);
            assert_eq!(end, ScriptEnd::Ran, "{source}: {end}");
            runner.steps() - before
        };
        (
            spent("if (t.lookupNamespaceURI('zz') !== null) throw 1;"),
            spent("if (u.lookupNamespaceURI('zz') !== null) throw 1;"),
        )
    });
    eprintln!(
        "lookupNamespaceURI('zz'): an element with a value of 64,000 bytes {long_steps} steps, \
         without {short_steps} steps"
    );
    assert!(
        long_steps < short_steps + 50,
        "{long_steps} against {short_steps} steps"
    );
}
