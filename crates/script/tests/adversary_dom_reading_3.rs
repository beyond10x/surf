//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`), third review: the
//! refill a memory end cuts, swept over every point of five shapes; the attribute charges
//! against the time of the work they pay for; `substringData` at and past its bounds. A hostile
//! page chooses the markup and every call; the writes go through the test's slices.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeData, NodeId, ParseOptions, Tree, WriteError, parse_document};
use surf_script::bindings::{DomCx, DomEnd, DomException, DomResult, Mutation, Slices};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, run_document};
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

fn cut_refill(runner: &mut Runner) -> bool {
    runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .is_some_and(|dom| dom.cut_refill())
}

/// One shape of a refill: the page, the script that keeps the collections and changes the
/// lists, the script that refills, and the names of the kept collections.
struct Shape {
    name: &'static str,
    page: String,
    prepare: &'static str,
    refill: &'static str,
}

/// Defines `whole(c)`: throws unless every index below `length` holds the node `item()`
/// answers and no index at or past it holds anything.
const WHOLE: &str = "var G = this;\
    G.whole = function (c) {\
      var n = c.length;\
      for (var i = 0; i < n; i++) { if (c[i] === undefined || c[i] !== c.item(i)) throw new Error('index ' + i + ' of ' + n); }\
      for (var j = n; j < n + 40; j++) { if (c[j] !== undefined) throw new Error('past the length ' + j + ' of ' + n); }\
    };";

struct Probe {
    filler: ScriptEnd,
    refill: ScriptEnd,
    cut: bool,
    check: ScriptEnd,
    again: ScriptEnd,
    counts: [(u32, u64); 3],
    collections: u32,
}

/// One document: the shape prepared, a chain of `units` objects `{ n: s, <link> }` held, then
/// the refill under 1 MiB of memory with the chain held by its function only (so the one
/// collection frees it), then the check of every kept collection, twice. The chain grows by
/// one object at a time, so no allocation of the filler is larger than one object.
fn probe(shape: &Shape, link: &str, units: usize) -> Probe {
    let mut small = limits();
    small.engine.memory_bytes = 1024 * 1024;
    let filler = format!("var s = null; for (var i = 0; i < {units}; i++) s = {{ n: s{link} }};");
    let page = shape.page.clone();
    let prepare = format!("{WHOLE}{}", shape.prepare);
    with_page(small, page, |runner| {
        let prepared = runner.evaluate(&prepare);
        assert_eq!(
            prepared,
            ScriptEnd::Ran,
            "{}: prepare: {prepared}",
            shape.name
        );
        let filler = runner.evaluate(&filler);
        if filler != ScriptEnd::Ran {
            return Probe {
                filler,
                refill: ScriptEnd::NotRun(Reason::Memory),
                cut: false,
                check: ScriptEnd::NotRun(Reason::Memory),
                again: ScriptEnd::NotRun(Reason::Memory),
                counts: [(0, 0); 3],
                collections: 0,
            };
        }
        let refill = runner.evaluate(&format!(
            "(function () {{ var t = s; G.s = null; {} return t; }})();",
            shape.refill
        ));
        let cut = cut_refill(runner);
        let before = (runner.scripts(), runner.steps());
        let check = runner.evaluate("for (var k = 0; k < kept.length; k++) whole(kept[k]);");
        let between = (runner.scripts(), runner.steps());
        let again = runner.evaluate("for (var k = 0; k < kept.length; k++) whole(kept[k]);");
        let after = (runner.scripts(), runner.steps());
        Probe {
            filler,
            refill,
            cut,
            check,
            again,
            counts: [before, between, after],
            collections: runner.collections(),
        }
    })
}

/// Sweeps the held String from the largest size at which the refill ends as it ends without
/// a limit, up in steps of `step` units, until the memory end has come before the first
/// define eight times in a row: every point of the refill at which the memory limit can end
/// it. At each: a refill the record names ends `limit: memory`, every later script of the
/// document ends `not run: memory` and adds nothing to the totals, and the one collection ran;
/// without the record every kept collection is whole.
fn sweep(shape: &Shape, step: usize) -> usize {
    let mut cuts = 0;
    for link in ["", ", k: 'k' + i"] {
        cuts += sweep_with(shape, link, step);
    }
    cuts
}

fn sweep_with(shape: &Shape, link: &str, step: usize) -> usize {
    let started = Instant::now();
    let ok = |units: usize| {
        let probe = probe(shape, link, units);
        probe.filler == ScriptEnd::Ran && probe.refill == ScriptEnd::Ran
    };
    assert!(ok(0), "{}: the refill without a filler", shape.name);
    let (mut low, mut high) = (0_usize, 2_048);
    assert!(
        !ok(high),
        "{}: the refill under the largest filler",
        shape.name
    );
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if ok(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    let mut seen = Vec::new();
    let mut wrong = Vec::new();
    let (mut recorded, mut whole, mut before_define) = (0, 0, 0);
    let mut units = low + 1;
    let mut probes = 0;
    while before_define < 8 && probes < 400 {
        probes += 1;
        let probe = probe(shape, link, units);
        let line = format!(
            "{units}: filler {}, refill {}, record {}, check {}, again {}, totals {:?}, collections {}",
            probe.filler,
            probe.refill,
            probe.cut,
            probe.check,
            probe.again,
            probe.counts,
            probe.collections
        );
        if probe.filler != ScriptEnd::Ran {
            seen.push(line);
            break;
        }
        if probe.cut {
            recorded += 1;
            before_define = 0;
            let not_run = ScriptEnd::NotRun(Reason::Memory);
            if probe.refill != ScriptEnd::Limit(Reason::Memory)
                || probe.check != not_run
                || probe.again != not_run
                || probe.counts[0] != probe.counts[1]
                || probe.counts[1] != probe.counts[2]
                || probe.collections != 1
            {
                wrong.push(line.clone());
            }
        } else {
            whole += 1;
            if probe.refill == ScriptEnd::Limit(Reason::Memory) {
                before_define += 1;
            }
            // The check may meet the limit itself: then it shows nothing; it never throws.
            if !matches!(
                probe.check,
                ScriptEnd::Ran | ScriptEnd::Limit(Reason::Memory)
            ) {
                wrong.push(line.clone());
            }
        }
        seen.push(line);
        units += step;
    }
    eprintln!(
        "{} (link {link:?}): refill ends as without a limit up to {low} units; {probes} probes from {}: {recorded} \
         recorded, {whole} not recorded ({:?})\n{}",
        shape.name,
        low + 1,
        started.elapsed(),
        seen.join("\n")
    );
    assert!(wrong.is_empty(), "{}: {wrong:#?}", shape.name);
    assert!(
        before_define > 0,
        "{}: no probe ended before the first define",
        shape.name
    );
    recorded
}

fn grow_page() -> String {
    format!(
        "<div id=\"p\">{}</div><div id=\"q\">{}</div><div id=\"r\">{}</div>",
        "<b></b>".repeat(4),
        "<i></i>".repeat(20),
        "<b></b>".repeat(4)
    )
}

/// Moves `q`'s children into a fragment `f`; `fresh()` answers a deep clone, whose children
/// have no wrapper yet.
const FRAGMENT: &str = "var p = document.getElementById('p'), q = document.getElementById('q'),\
    r = document.getElementById('r');\
    var f = document.createDocumentFragment();\
    while (q.firstChild) f.appendChild(q.firstChild);\
    G.fresh = function () { return f.cloneNode(true); };";

#[test]
fn a_memory_end_at_every_point_of_a_growing_child_nodes_refill() {
    let prepare: &'static str = Box::leak(
        format!("{FRAGMENT} var c = p.childNodes; G.kept = [c]; p.insertBefore(fresh(), c[0]);")
            .into_boxed_str(),
    );
    let cuts = sweep(
        &Shape {
            name: "childNodes 4 -> 24",
            page: grow_page(),
            prepare,
            refill: "p.childNodes;",
        },
        1,
    );
    eprintln!("records: {cuts}");
}

#[test]
fn a_memory_end_at_every_point_of_a_growing_children_refill() {
    let prepare: &'static str = Box::leak(
        format!("{FRAGMENT} var c = p.children; G.kept = [c]; p.insertBefore(fresh(), c[0]);")
            .into_boxed_str(),
    );
    let cuts = sweep(
        &Shape {
            name: "children 4 -> 24",
            page: grow_page(),
            prepare,
            refill: "p.children;",
        },
        1,
    );
    eprintln!("records: {cuts}");
}

#[test]
fn a_memory_end_at_every_point_of_a_shrinking_refill() {
    // To empty: step 1 defines the old tail again, step 3 deletes it. To 4: step 2 defines
    // over the kept wrappers.
    for (name, prepare) in [
        (
            "childNodes 24 -> 0",
            "var p = document.getElementById('p'), c = p.childNodes; G.kept = [c];\
             while (p.firstChild) p.removeChild(p.firstChild);",
        ),
        (
            "childNodes 24 -> 8",
            "var p = document.getElementById('p'), c = p.childNodes; G.kept = [c];\
             for (var k = 0; k < 16; k++) p.removeChild(p.firstChild);",
        ),
    ] {
        let cuts = sweep(
            &Shape {
                name,
                page: format!("<div id=\"p\">{}</div>", "<b></b>".repeat(24)),
                prepare,
                refill: "p.childNodes;",
            },
            1,
        );
        eprintln!("records: {cuts}");
    }
}

#[test]
fn a_memory_end_at_every_point_of_a_put_back() {
    // The object takes no new index: step 2 redefines 0..4 with new wrappers, index 4 is
    // refused, the put-back defines the old 4 again and the getter throws its TypeError, which
    // the script catches.
    let prepare: &'static str = Box::leak(
        format!(
            "{FRAGMENT} var c = p.childNodes; G.kept = [c];\
             Object.preventExtensions(c); p.insertBefore(fresh(), c[0]);"
        )
        .into_boxed_str(),
    );
    let cuts = sweep(
        &Shape {
            name: "non-extensible childNodes 4 -> 24",
            page: grow_page(),
            prepare,
            refill: "try { p.childNodes; } catch (e) { if (!(e instanceof TypeError)) throw e; }",
        },
        1,
    );
    eprintln!("records: {cuts}");
}

#[test]
fn a_memory_end_at_every_point_of_two_refills_in_one_script() {
    let prepare: &'static str = Box::leak(
        format!(
            "{FRAGMENT} var c = p.childNodes, d = r.childNodes; G.kept = [c, d];\
             p.insertBefore(fresh(), c[0]); r.insertBefore(fresh(), d[0]);"
        )
        .into_boxed_str(),
    );
    let cuts = sweep(
        &Shape {
            name: "two parents 4 -> 24",
            page: grow_page(),
            prepare,
            refill: "p.childNodes; r.childNodes;",
        },
        1,
    );
    eprintln!("records: {cuts}");
    assert!(cuts > 0, "no probe cut a refill after a define");
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

/// `substringData` at and past its bounds, each argument as Web IDL converts it to an
/// `unsigned long`: the answer or the exception, and the steps of the call.
#[test]
fn substring_data_at_and_past_its_bounds() {
    let page = format!("<p id=\"t\">{}</p>", "abcdefgh".repeat(125));
    let cases: [(&str, &str, Result<&str, &str>); 13] = [
        ("0", "0", Ok("")),
        ("0", "3", Ok("abc")),
        ("1000", "0", Ok("")),
        ("1000", "1", Ok("")),
        ("1001", "0", Err("IndexSizeError")),
        ("0", "4294967295", Ok("all")),
        ("4294967295", "1", Err("IndexSizeError")),
        ("4294967296", "3", Ok("abc")),
        ("4294967297", "4294967298", Ok("bc")),
        ("998", "-1", Ok("gh")),
        ("NaN", "NaN", Ok("")),
        ("-1", "5", Err("IndexSizeError")),
        ("Infinity", "Infinity", Ok("")),
    ];
    let seen = with_page(limits(), page, |runner| {
        let end = runner.evaluate("var G = this; G.t = document.getElementById('t').firstChild;");
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        let mut seen = Vec::new();
        for (offset, count, want) in cases {
            let check = match want {
                Ok("all") => {
                    "if (got.length !== 1000) throw new Error('got ' + got.length);".to_string()
                }
                Ok(text) => format!("if (got !== '{text}') throw new Error('got ' + got);"),
                Err(name) => format!("throw new Error('no {name}');"),
            };
            let source = format!(
                "var got; try {{ got = t.substringData({offset}, {count}); }} \
                 catch (e) {{ if (e.name !== '{}') throw new Error('threw ' + e.name); got = null; }}\
                 if (got !== null) {{ {check} }}",
                want.err().unwrap_or("none")
            );
            let before = runner.steps();
            let end = runner.evaluate(&source);
            seen.push((format!("({offset}, {count})"), end, runner.steps() - before));
        }
        seen
    });
    let lines: Vec<String> = seen
        .iter()
        .map(|(call, end, steps)| format!("substringData{call}: {end}, {steps} steps"))
        .collect();
    eprintln!("{}", lines.join("\n"));
    for (call, end, steps) in &seen {
        assert_eq!(*end, ScriptEnd::Ran, "{call}: {lines:#?}");
        assert!(*steps < 200, "{call}: {steps} steps: {lines:#?}");
    }
}

/// `namedItem` compares the `name` of an element of the HTML namespace with its key: a `name`
/// of 64,000 bytes is charged, about 1,000 steps (no other test holds the charge of `name`).
#[test]
fn named_item_charges_an_html_name_it_compares() {
    let page = format!("<!DOCTYPE html><p name=\"{}\"></p>", "n".repeat(64_000));
    let spent = with_page(limits(), page, |runner| {
        let end = runner.evaluate("var G = this; G.p = document.getElementsByTagName('p');");
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        let before = runner.steps();
        let end = runner.evaluate("p.namedItem('none');");
        assert_eq!(end, ScriptEnd::Ran, "{end}");
        runner.steps() - before
    });
    eprintln!("namedItem('none') over an HTML name of 64,000 bytes: {spent} steps");
    assert!(spent > 900, "{spent} steps");
}

/// Step 1 of a refill defines again only the old indices past the new length that the object
/// still has: an index the script deleted is passed over, so a non-extensible list with a
/// deleted tail index still shrinks (a define of the absent index would be refused).
#[test]
fn a_non_extensible_list_with_a_deleted_tail_index_shrinks() {
    let page = format!("<div id=\"p\">{}</div>", "<b></b>".repeat(10));
    let end = with_page(limits(), page, |runner| {
        runner.evaluate(
            "var p = document.getElementById('p'), c = p.childNodes;\
             if (!delete c[9]) throw new Error('not deleted');\
             Object.preventExtensions(c);\
             for (var k = 0; k < 5; k++) p.removeChild(p.firstChild);\
             var d = p.childNodes;\
             if (d !== c || c.length !== 5 || c[0] !== p.firstChild || c.item(4) !== c[4]\
                 || c[5] !== undefined || c[9] !== undefined) throw new Error('not the new list');",
        )
    });
    assert_eq!(end, ScriptEnd::Ran, "{end}");
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
