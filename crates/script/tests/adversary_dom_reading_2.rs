//! Adversarial cases of the slice `reading` (`story:scripts-dom-reading`), second review: the
//! refill of a kept collection after its fixes, the charges, `substringData` in code units,
//! `baseURI`, the sibling hints, the namespace lookups and the rows the first review left. A
//! hostile page chooses the markup and every call; the writes go through the test's slices.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeId, ParseOptions, WriteError, parse_document};
use surf_script::bindings::{Attributes, DomCx, DomEnd, DomException, DomResult, Mutation, Slices};
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

fn info(address: &str) -> DocumentInfo {
    DocumentInfo {
        address: Url::parse(address, None).expect("the address"),
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

/// The test's `mutation` slice, through the tree's public write surface: `appendChild` (a
/// fragment gives its children), `insertBefore`, `removeChild`, `cloneNode`,
/// `createDocumentFragment` and the `nodeValue` setter of character data.
struct TestMutation;

impl Mutation for TestMutation {
    fn set_node_node_value(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: Option<String>,
    ) -> DomResult<()> {
        cx.tree_mut()
            .set_data(this, &value.unwrap_or_default())
            .map_err(refused)
    }

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

/// The test's `attributes` slice: `setAttribute` and `removeAttribute` through the tree.
struct TestAttributes;

impl Attributes for TestAttributes {
    fn element_set_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
        value: String,
    ) -> DomResult<()> {
        cx.tree_mut()
            .set_attribute(this, &qualified_name, &value)
            .map_err(refused)
    }

    fn element_remove_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<()> {
        cx.tree_mut()
            .remove_attribute(this, &qualified_name)
            .map_err(refused)
    }
}

fn slices(info: DocumentInfo) -> Slices {
    let mut slices = dom::slices(info);
    slices.mutation = Box::new(TestMutation);
    slices.attributes = Box::new(TestAttributes);
    slices
}

fn with_page<R: Send>(
    limits: RunnerLimits,
    address: &str,
    page: String,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    let info = info(address);
    run_document(
        limits,
        Mode::Test,
        move || Document {
            tree: parse_document(&page, &ParseOptions::default()),
            limits: ParserLimits::default(),
            slices: slices(info),
        },
        body,
    )
    .expect("a runner with the bindings")
}

fn recorded(runner: &mut Runner) -> Option<surf_html::limits::Limit> {
    runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .and_then(|dom| dom.limit())
}

/// Runs `source` and asserts it ran and recorded no limit.
fn ran(runner: &mut Runner, source: &str) {
    let end = runner.evaluate(source);
    assert_eq!(end, ScriptEnd::Ran, "{end}");
    assert_eq!(recorded(runner), None);
}

fn steps(runner: &mut Runner, source: &str) -> u64 {
    let before = runner.steps();
    ran(runner, source);
    runner.steps() - before
}

/// The message of the error a script threw, or the end's text.
fn outcome(runner: &mut Runner, source: &str) -> String {
    match runner.evaluate(source) {
        ScriptEnd::Threw(error) => error.message.unwrap_or_default(),
        other => format!("{other}"),
    }
}

const CHECK: &str = r#"
var G = this;
function check(holds, what) { if (!holds) { throw new Error(what); } }
function threw(f) { try { f(); } catch (e) { return e.name; } return "no throw"; }
var fails = [];
function expect(got, want, what) { if (got !== want) fails.push(what + ": " + String(got) + " against " + String(want)); }
function done(what) { if (fails.length) throw new Error(fails.length + " wrong: " + fails.slice(0, 12).join(" / ")); throw new Error("held " + what); }
"#;

/// A refill that the memory limit ends in its middle: the getter of a kept `childNodes` makes a
/// wrapper for each new node while it defines the indices, and the run ends there. The script
/// ends, the runner collects, and a later script of the same document reads the same kept
/// object without its getter. The rule: the object is the old snapshot whole, or the new one
/// whole; no index at or past `length` answers a node the glue put there. `refresh_collection`
/// propagates a `Terminated` from `wrap` and `index_key` in its step 2 with `?`, after it
/// defined the first indices and before it wrote the data.
#[test]
fn a_refill_ended_by_the_memory_limit_leaves_one_list() {
    let started = Instant::now();
    let mut small = limits();
    small.engine.memory_bytes = 1024 * 1024;
    let page = format!(
        "<div id=\"p\"><b></b></div><div id=\"q\">{}</div>",
        "<i></i>".repeat(50)
    );
    let (refill, seen, wrong) = with_page(small, ADDRESS, page, |runner| {
        ran(
            runner,
            "var p = document.getElementById('p'), q = document.getElementById('q');\
             var c = p.childNodes, b = c[0];\
             var f = document.createDocumentFragment();\
             while (q.firstChild) f.appendChild(q.firstChild);\
             for (var k = 0; k < 120; k++) p.insertBefore(f.cloneNode(true), b);",
        );
        let refill = runner.evaluate("p.childNodes;");
        let checks = [
            ("length is the old 1", "if (c.length !== 1) throw 1;"),
            ("item(0) is the old child", "if (c.item(0) !== b) throw 1;"),
            ("c[0] is the old child", "if (c[0] !== b) throw 1;"),
            ("c[1] is undefined", "if (c[1] !== undefined) throw 1;"),
            ("c[100] is undefined", "if (c[100] !== undefined) throw 1;"),
        ];
        let mut seen = vec![format!("refill: {refill}")];
        let mut wrong = Vec::new();
        for (what, source) in checks {
            let end = runner.evaluate(source);
            seen.push(format!("{what}: {end}"));
            if end != ScriptEnd::Ran {
                wrong.push(what);
            }
        }
        (refill, seen, wrong)
    });
    eprintln!(
        "memory end in a refill ({:?}):\n{}",
        started.elapsed(),
        seen.join("\n")
    );
    assert_eq!(refill, ScriptEnd::Limit(Reason::Memory), "{seen:#?}");
    assert!(
        wrong.is_empty(),
        "a kept collection shows two lists after a refill the memory limit ended: {seen:#?}"
    );
}

/// The charges of the walks that read attributes, against acceptance 17: per element one step
/// and one per 64 bytes of its `class` attribute for `getElementsByClassName`. A page of 10,000
/// elements with 10 attributes of 100 bytes each and `class="c"`.
#[test]
fn attribute_walks_charge_what_they_read() {
    let started = Instant::now();
    let value = "v".repeat(100);
    let mut attrs = String::new();
    for index in 0..10 {
        attrs.push_str(&format!(" data-a{index}=\"{value}\""));
    }
    let element = format!("<p class=\"c\"{attrs}></p>");
    let mut lines = Vec::new();
    let mut class_steps = 0;
    for count in [1_000_usize, 10_000] {
        let page = format!(
            "<div>{}</div><div>{}</div>",
            element.repeat(count / 2),
            element.repeat(count / 2)
        );
        let (seen, class) = with_page(limits(), ADDRESS, page, |runner| {
            ran(
                runner,
                "var G = this; G.all = document.getElementsByTagName('p');\
                 G.t1 = document.getElementsByTagName('div')[0]; G.t2 = t1.nextElementSibling;\
                 document.getElementsByTagName('*');",
            );
            let mut seen = Vec::new();
            let mut class = 0;
            for (name, source) in [
                (
                    "getElementsByClassName('c')",
                    "document.getElementsByClassName('c');",
                ),
                (
                    "getElementsByTagName('*')",
                    "document.getElementsByTagName('*');",
                ),
                ("getElementById('none')", "document.getElementById('none');"),
                ("namedItem('none')", "all.namedItem('none');"),
                (
                    "isEqualNode of two equal trees",
                    "if (!t1.isEqualNode(t2)) throw 1;",
                ),
                ("textContent", "document.documentElement.textContent;"),
            ] {
                let at = Instant::now();
                let spent = steps(runner, source);
                seen.push(format!(
                    "{count} elements, {name}: {spent} steps, {:?}",
                    at.elapsed()
                ));
                if name.starts_with("getElementsByClassName") {
                    class = spent;
                }
            }
            (seen, class)
        });
        lines.extend(seen);
        class_steps = class;
    }
    lines.push(format!("all: {:?}", started.elapsed()));
    eprintln!("{}", lines.join("\n"));
    // Acceptance 17: one step per element and one per 64 bytes of its one-byte `class`, the
    // walk's step per other node, the collection's step per node: about 30,000. The bound
    // leaves twice that.
    assert!(
        class_steps < 60_000,
        "getElementsByClassName charges {class_steps} steps for 10,000 elements whose class is \
         one byte: it charges every attribute's bytes, not its class attribute's\n{}",
        lines.join("\n")
    );
}
