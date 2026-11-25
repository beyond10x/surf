//! Adversarial cases of the sixth review of `story:html-script-hook`, against fix round 5
//! (ruling D423): the step limit is read before each entry of every loop over the stack of open
//! elements, its scope boundaries and the list of active formatting elements, before every
//! write of the parser to the tree and before a token is processed again; a loop the limit
//! ends answers "not found"; an adoption agency iteration the limit ends puts the clones it
//! made back at the place of step 4.14 and stops before step 4.24. A parse ends fewer than
//! 4 x `max_nodes` steps past `max_steps` (`src/limits.rs`, `max_steps`). Each test names what
//! it holds the unit to. The cases above the line near the end that says otherwise held on the
//! tip they were written against (`5d9b011`).

use std::collections::{HashMap, HashSet};

use surf_html::limits::{Limit, Limits};
use surf_html::{
    Context, Namespace, NodeData, NodeId, NotRun, ParseOptions, ScriptHook, Tree, parse_document,
    parse_document_bytes, parse_document_with_hook, parse_fragment,
};

fn with(max_steps: usize, max_nodes: usize) -> ParseOptions {
    ParseOptions {
        scripting_disabled: false,
        limits: Limits {
            max_steps,
            max_nodes,
            ..Limits::default()
        },
    }
}

fn steps(max_steps: usize) -> ParseOptions {
    with(max_steps, Limits::default().max_nodes)
}

fn contents_of(tree: &Tree, node: NodeId) -> Option<NodeId> {
    match tree.data(node) {
        NodeData::Element {
            template_contents: Some(contents),
            ..
        } => Some(*contents),
        _ => None,
    }
}

fn name_of(tree: &Tree, node: NodeId) -> Option<&str> {
    match tree.data(node) {
        NodeData::Element { local_name, .. } => Some(local_name),
        _ => None,
    }
}

/// The nodes reached from `roots` through children and template contents.
fn reach(tree: &Tree, roots: &[NodeId]) -> HashSet<NodeId> {
    let mut seen = HashSet::new();
    let mut pending = roots.to_vec();
    while let Some(node) = pending.pop() {
        if seen.insert(node) {
            pending.extend(tree.children(node).iter().copied());
            pending.extend(contents_of(tree, node));
        }
    }
    seen
}

/// What keeps the nodes reached from `roots` from being a tree: a child that names another
/// parent, a child listed twice, a node its parent does not list, a template's contents with a
/// parent, a document that is not the root, and a cycle of parents and hosts.
fn faults(tree: &Tree, roots: &[NodeId]) -> Vec<String> {
    let bound = tree.node_count();
    let seen = reach(tree, roots);
    let mut found = Vec::new();
    let mut hosts = HashMap::new();
    for &node in &seen {
        let mut listed = HashSet::new();
        for &child in tree.children(node) {
            if !listed.insert(child) {
                found.push(format!("{child:?} listed twice by {node:?}"));
            }
            if tree.parent(child) != Some(node) {
                found.push(format!("{child:?} listed by {node:?} names another parent"));
            }
        }
        if let Some(contents) = contents_of(tree, node) {
            hosts.insert(contents, node);
            if tree.parent(contents).is_some() {
                found.push(format!("the contents {contents:?} have a parent"));
            }
        }
        if let Some(parent) = tree.parent(node)
            && !tree.children(parent).contains(&node)
        {
            found.push(format!("{node:?} names {parent:?}, which does not list it"));
        }
        if node != tree.root() && *tree.data(node) == NodeData::Document {
            found.push(format!("{node:?} is a document and not the root"));
        }
    }
    for &node in &seen {
        let mut at = node;
        let mut up = 0;
        while let Some(next) = tree.parent(at).or_else(|| hosts.get(&at).copied()) {
            at = next;
            up += 1;
            if up > bound {
                found.push(format!("{node:?} is on a cycle"));
                break;
            }
        }
    }
    found
}

/// What a hook does in its runs.
#[derive(Clone, Copy, Debug)]
enum Write {
    /// Nothing.
    Nothing,
    /// Moves the script's parent, the node the parser stands on, to the end of the `body`.
    Lift,
    /// Takes the script's parent out of the tree.
    Remove,
    /// Moves the script's parent into the contents of a new `template` at the end of the `body`.
    IntoTemplate,
}

/// A hook that makes `write` in each run and records each call: the script, whether it was a
/// run, and the limit the tree had recorded at that moment.
struct Recorder {
    write: Write,
    calls: Vec<(NodeId, bool, Option<Limit>)>,
    removed: Vec<NodeId>,
}

impl Recorder {
    fn new(write: Write) -> Recorder {
        Recorder {
            write,
            calls: Vec::new(),
            removed: Vec::new(),
        }
    }
}

fn body_of(tree: &Tree) -> Option<NodeId> {
    let html = *tree
        .children(tree.root())
        .iter()
        .find(|&&c| name_of(tree, c) == Some("html"))?;
    tree.children(html)
        .iter()
        .copied()
        .find(|&c| name_of(tree, c) == Some("body"))
}

impl ScriptHook for Recorder {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        self.calls.push((script, true, tree.stopped_by()));
        let Some(parent) = tree.parent(script) else {
            return;
        };
        if !matches!(tree.data(parent), NodeData::Element { .. }) {
            return;
        }
        match self.write {
            Write::Nothing => {}
            Write::Lift => {
                if let Some(body) = body_of(tree) {
                    let _ = tree.append_child(body, parent);
                }
            }
            Write::Remove => {
                if tree.remove_node(parent).is_ok() {
                    self.removed.push(parent);
                }
            }
            Write::IntoTemplate => {
                if let Some(body) = body_of(tree)
                    && let Ok(template) = tree.create_element_ns(Namespace::Html, "template")
                    && tree.append_child(body, template).is_ok()
                    && let Some(contents) = contents_of(tree, template)
                {
                    let _ = tree.append_child(contents, parent);
                }
            }
        }
    }

    fn skip(&mut self, tree: &Tree, script: NodeId, _: NotRun) {
        self.calls.push((script, false, tree.stopped_by()));
    }
}

/// The pages of the sweep: what each reaches, and the page. Each has at least one script.
fn pages() -> Vec<(&'static str, String)> {
    let span = "<span>".repeat(12);
    let bs: String = (0..6).map(|i| format!("<b id={i}>")).collect();
    vec![
        (
            "adoption agency with a furthest block and clones",
            format!("<body><script>x</script><b><i><u><s>{span}<div>y</b>z"),
        ),
        (
            "adoption agency with no furthest block",
            format!("<body><b>{span}<script>x</script>y</b>z"),
        ),
        (
            "adoption agency over an open object",
            format!("<body><object><b><script>x</script>{span}<p>y</b></p>"),
        ),
        (
            "foster parenting inside a table",
            format!("<table><tr><td>x</td></tr>{bs}y<script>s</script>{span}z</table>w"),
        ),
        (
            "a template in a table",
            format!("<table><template><b>{span}<div>x</b><script>s</script></template><tr><td>y"),
        ),
        (
            "select",
            format!(
                "<select><option>a<script>s</script><b>{span}x</select><input><select><optgroup><option>b"
            ),
        ),
        (
            "foreign content",
            format!(
                "<svg><g><desc><b><script>s</script>{span}<div>x</b></desc></g><p>y</svg><math><mi><i>z</math>"
            ),
        ),
        (
            "frameset",
            format!("<script>s</script><div>{span}<frameset><frame></frameset>"),
        ),
        (
            "an a start tag over an a left in the list",
            format!("<a><p><b></p><table><script>s</script><a>{span}<a>x"),
        ),
        (
            "the entries read again after a hook run",
            format!("<body>{span}<div><script>s</script>{bs}<p>x</b>y"),
        ),
        (
            "end tags over a long stack",
            format!("<body><ul><li><dl><dd>{span}<script>s</script><li><dt>x</dl></ul></q></body>"),
        ),
        (
            "nested formatting with markers",
            format!(
                "<body><b><table><caption><i>{span}<script>s</script>x</caption></table>y</b><nobr><nobr>z"
            ),
        ),
    ]
}

/// The faults of `page` parsed with a hook that makes `write`, at every step limit from 0 to
/// the steps of the whole parse and two more: no panic (a debug build checks the parser's
/// asserts); the nodes reached from the document, and from the nodes the hook took out, are a
/// tree; when the whole parse leaves every node it made in the tree, so does every stopped one
/// (a page with a `frameset`, whose rule takes the `body` out, is left out of that check); the
/// parse ends at `Steps` past its limit or whole inside it; each script element the tree holds
/// has one call (an HTML or SVG one: a MathML `script` gets none); and no `run` happens after
/// the parse stopped.
fn sweep(page: &str, write: Write) -> Vec<String> {
    let mut wrong = Vec::new();
    let mut hook = Recorder::new(write);
    let full = parse_document_with_hook(page, &steps(usize::MAX), &mut hook);
    let all_held = !page.contains("<frameset") && {
        let mut roots = vec![full.root()];
        roots.extend(&hook.removed);
        reach(&full, &roots).len() == full.node_count()
    };
    for max in 0..=full.steps() + 2 {
        let mut hook = Recorder::new(write);
        let tree = parse_document_with_hook(page, &steps(max), &mut hook);
        let mut roots = vec![tree.root()];
        roots.extend(&hook.removed);
        let mut found = faults(&tree, &roots);
        let held = reach(&tree, &roots);
        if all_held && held.len() != tree.node_count() {
            found.push(format!(
                "{} of {} nodes held",
                held.len(),
                tree.node_count()
            ));
        }
        let ended = match tree.stopped_by() {
            Some(Limit::Steps) => tree.steps() > max,
            None => tree.steps() <= max,
            Some(_) => false,
        };
        if !ended {
            found.push(format!(
                "{} steps, stopped by {:?}",
                tree.steps(),
                tree.stopped_by()
            ));
        }
        let mut calls = HashMap::new();
        for &(script, run, stopped) in &hook.calls {
            *calls.entry(script).or_insert(0) += 1;
            if run && stopped.is_some() {
                found.push(format!("a run of {script:?} after a stop at {stopped:?}"));
            }
        }
        for &node in &held {
            let watched = matches!(tree.data(node), NodeData::Element {
                namespace: Namespace::Html | Namespace::Svg,
                local_name,
                ..
            } if local_name == "script");
            if watched && calls.get(&node) != Some(&1) {
                found.push(format!("{node:?} had {:?} calls", calls.get(&node)));
            }
        }
        if !found.is_empty() {
            wrong.push(format!("{page:?}, {write:?}, limit {max}: {found:?}"));
        }
    }
    wrong
}

const WRITES: [Write; 4] = [
    Write::Nothing,
    Write::Lift,
    Write::Remove,
    Write::IntoTemplate,
];

/// Claims 3 and 4 of fix round 5, and its claim about the hook (`docs/design/scripts.md`, Q1;
/// the crate documentation, "Limits"): [`sweep`] over each page of [`pages`] with each hook
/// write.
#[test]
fn every_limit_over_these_pages_leaves_a_tree_and_one_call_per_script() {
    let mut wrong = Vec::new();
    for (_, page) in pages() {
        for write in WRITES {
            wrong.extend(sweep(&page, write));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} cases: {:#?}",
        wrong.len(),
        &wrong[..wrong.len().min(12)]
    );
}

/// The same as [`every_limit_over_these_pages_leaves_a_tree_and_one_call_per_script`] over
/// 1,500 random pages of 6 to 30 pieces (formatting elements and their end tags, scope
/// boundaries, tables and their parts, `template`, `select`, foreign content, `frameset`, lists
/// and scripts), each with one hook write in turn: every limit of each.
#[test]
fn every_limit_over_random_pages_leaves_a_tree_and_one_call_per_script() {
    const PIECES: &[&str] = &[
        "<a>",
        "</a>",
        "<b>",
        "</b>",
        "<i>",
        "</i>",
        "<nobr>",
        "<p>",
        "</p>",
        "<div>",
        "</div>",
        "<span>",
        "<object>",
        "</object>",
        "<table>",
        "</table>",
        "<tr>",
        "<td>",
        "</td>",
        "<caption>",
        "<template>",
        "</template>",
        "<select>",
        "</select>",
        "<option>",
        "<svg>",
        "<desc>",
        "</svg>",
        "<math>",
        "<mi>",
        "</math>",
        "<li>",
        "<dd>",
        "<frameset>",
        "x",
        "<!--c-->",
        "</q>",
        "<html id=h>",
        "<body id=b>",
        "<script>s</script>",
        "<script>s</script>",
        "<svg><script>v</script>",
    ];
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut below = |n: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n as u64) as usize
    };
    let mut wrong = Vec::new();
    for round in 0..1_500 {
        let mut page = String::new();
        for _ in 0..6 + below(25) {
            page.push_str(PIECES[below(PIECES.len())]);
        }
        wrong.extend(sweep(&page, WRITES[round % WRITES.len()]));
    }
    assert!(
        wrong.is_empty(),
        "{} cases: {:#?}",
        wrong.len(),
        &wrong[..wrong.len().min(12)]
    );
}

/// Claim 4 for a fragment, and the second pass after an encoding `meta` (the crate
/// documentation, "Limits": the steps are counted per parse pass). The fragment pages of
/// [`pages`] in a `div`, a `table` and a `template`, and a page of bytes whose `meta` past the
/// prescan changes the encoding, at every step limit: a tree, every node held (but what a
/// fragment of no input holds outside its root, the context element), ended at `Steps` or
/// whole.
#[test]
fn a_fragment_and_the_second_pass_stop_inside_a_token_with_a_tree() {
    let mut wrong = Vec::new();
    let check = |what: &str, max: usize, tree: &Tree, outside: usize, wrong: &mut Vec<String>| {
        let mut found = faults(tree, &[tree.root()]);
        let held = reach(tree, &[tree.root()]).len();
        if held + outside != tree.node_count() {
            found.push(format!("{held} of {} nodes held", tree.node_count()));
        }
        let ended = match tree.stopped_by() {
            Some(Limit::Steps) => tree.steps() > max,
            None => tree.steps() <= max,
            Some(_) => false,
        };
        if !ended {
            found.push(format!(
                "{} steps, stopped by {:?}",
                tree.steps(),
                tree.stopped_by()
            ));
        }
        if !found.is_empty() {
            wrong.push(format!("{what}, limit {max}: {found:?}"));
        }
    };
    for context in ["div", "table", "template", "tr", "select"] {
        let context = Context {
            namespace: Namespace::Html,
            local_name: context.to_string(),
        };
        let empty = parse_fragment("", &context, &steps(usize::MAX));
        let outside = empty.node_count() - reach(&empty, &[empty.root()]).len();
        for (what, page) in pages() {
            if what == "frameset" {
                continue;
            }
            let full = parse_fragment(&page, &context, &steps(usize::MAX));
            for max in 0..=full.steps() + 1 {
                let tree = parse_fragment(&page, &context, &steps(max));
                check(what, max, &tree, outside, &mut wrong);
            }
        }
    }
    let mut bytes = b"<!DOCTYPE html><head><title>".to_vec();
    bytes.extend(std::iter::repeat_n(b'a', 1100));
    bytes.extend_from_slice(b"</title><meta charset=utf-8><body><b><i><u>");
    bytes.extend_from_slice("<span>".repeat(12).as_bytes());
    bytes.extend_from_slice(b"<div>\xc3\xa9</b><table><tr>x<td>y</table>");
    let full = parse_document_bytes(&bytes, &steps(usize::MAX));
    assert_eq!(
        full.encoding,
        Some("UTF-8"),
        "the meta changes the encoding"
    );
    let one_pass = parse_document(&String::from_utf8_lossy(&bytes), &steps(usize::MAX)).steps();
    assert!(full.tree.steps() <= one_pass + 2, "the steps of one pass");
    for max in 0..=full.tree.steps() + 1 {
        let parsed = parse_document_bytes(&bytes, &steps(max));
        check("the second pass", max, &parsed.tree, 0, &mut wrong);
    }
    assert!(
        wrong.is_empty(),
        "{} cases: {:#?}",
        wrong.len(),
        &wrong[..wrong.len().min(12)]
    );
}

/// For every limit from the steps of `page` to those of `page` and `token` (no script runs):
/// how far past the limit the parse ends, by the limit's offset from the steps of `page`. Each
/// ends at `Steps`.
fn overshoots(page: &str, token: &str) -> Vec<usize> {
    let options = |max_steps| ParseOptions {
        scripting_disabled: true,
        limits: Limits {
            max_steps,
            ..Limits::default()
        },
    };
    let before = parse_document(page, &options(usize::MAX)).steps();
    let whole = format!("{page}{token}");
    let after = parse_document(&whole, &options(usize::MAX)).steps();
    (before..after)
        .map(|max| {
            let tree = parse_document(&whole, &options(max));
            assert_eq!(tree.stopped_by(), Some(Limit::Steps), "limit {max}");
            tree.steps() - max
        })
        .collect()
}

/// Mutant W16 of fix round 5 (the read before each entry of the reconstruction's advance
/// removed), called equivalent (report lines 1017-1018: "each advance entry's one step is
/// followed by the element's room_at read before anything else"). Its read is the first one
/// after a stretch that reads nothing: an `a` start tag over an `a` the adoption agency left in
/// the list (step 4.5: not in scope behind a `table`) finds that `a` on the stack (the scan's
/// last entry is counted after its read) and takes it off, counting the `table` above it and
/// the two scope boundaries (`html`, `table`); then the reconstruction of the closed `b` has no
/// entry to rewind over and reads first in its advance. So a limit there ends at most 4 steps
/// past (1 + 3); without that read the advance visits the `b`'s entry before `room_at` reads,
/// and the parse ends 5 past.
#[test]
fn the_advance_of_the_reconstruction_reads_the_limit_after_a_removal_from_the_stack() {
    let over = overshoots("<a><p><b></p><table>", "<a>");
    assert_eq!(over.iter().max(), Some(&4), "{over:?}");
}

/// Mutant W31 of fix round 5 (the read after step 4.14 removed), called equivalent (report line
/// 1024: "the inner loop's top read comes next with no write between"). Between them the
/// adoption agency walks from the place up to the root (for step 4.23) and checks the depth
/// limit, both counted and unread: a scan for the foster parenting `table` that the limit ends
/// answers the `html` element, and without the read that walk adds 2 steps to every stop
/// inside the scan. With it, a stop inside a scan ends one step past the limit. Over `</b>`
/// after `<table><b><div>` and 200 `span`, the limits before the stretch of steps 4.26 to 4.30
/// (which reads nothing and ends up to 405 past) end one step past, but for 3 in counted walks
/// and the end of a scan; without the read about 200 more do.
#[test]
fn a_limit_inside_the_foster_parenting_scan_of_step_4_14_ends_one_step_past() {
    let page = format!("<table><b><div>{}", "<span>".repeat(200));
    let over = overshoots(&page, "</b>");
    let stretch = over
        .iter()
        .position(|&o| o > 100)
        .expect("the stretch of steps 4.26 to 4.30");
    let past_one: Vec<(usize, usize)> = over[..stretch]
        .iter()
        .enumerate()
        .filter(|&(_, &o)| o > 1)
        .map(|(at, &o)| (at, o))
        .collect();
    assert!(past_one.len() <= 3, "{past_one:?}");
}

/// Mutants W4 (the read of `room_at`) and W8 (the read before a character's write) of fix
/// round 5, called equivalent (report lines 1011-1013: "an element is read for twice with no
/// step between (room_at, then admit)"; "the character's read: admit reads next with no step
/// between"). There are steps between: an element whose place is a template's contents has its
/// parent's depth read by a walk to the template (a step) after `room_at` and before `admit`,
/// and a character foster parented before a `table` scans the table's parent's children for it
/// (a step) after its read and before `admit`. A scan's last visit can end one step past the
/// limit with no stop yet; then each of those reads stops the parse there, and without it the
/// count goes one further. On these pages every limit ends exactly one step past.
#[test]
fn every_limit_of_these_small_pages_ends_one_step_past() {
    let options = |max_steps| ParseOptions {
        scripting_disabled: true,
        limits: Limits {
            max_steps,
            ..Limits::default()
        },
    };
    let mut wrong = Vec::new();
    for page in [
        "<template><p>",
        "<table><tr><template><p>",
        "<table>xy</table>",
        "<template><table><tr>x",
    ] {
        let all = parse_document(page, &options(usize::MAX)).steps();
        for max in 0..all {
            let tree = parse_document(page, &options(max));
            if tree.stopped_by() != Some(Limit::Steps) || tree.steps() != max + 1 {
                wrong.push(format!(
                    "{page}: limit {max}, {} steps, stopped by {:?}",
                    tree.steps(),
                    tree.stopped_by()
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A hook that flattens the elements the parser opened since its last run: each `span` from the
/// script's parent up to the element with the id `fb` goes to the end of that element's
/// children, in order, so every `span` stays open as a child of `fb`.
struct Flatten;

impl ScriptHook for Flatten {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        let mut chain = Vec::new();
        let mut at = tree.parent(script);
        while let Some(node) = at {
            if name_of(tree, node) != Some("span") {
                break;
            }
            chain.push(node);
            at = tree.parent(node);
        }
        let Some(fb) = at else {
            return;
        };
        for &span in chain.iter().rev() {
            tree.append_child(fb, span).unwrap();
        }
    }

    fn skip(&mut self, _: &Tree, _: NodeId, _: NotRun) {}
}

/// Claim 2 of fix round 5 (`limits.rs`, `max_steps`: "A parse ends fewer than 4 × max_nodes
/// steps past max_steps"), its longest stretch: steps 4.26 to 4.30 of the adoption agency read
/// nothing. With a hook that keeps A `span` open as children of the furthest block (`Flatten`),
/// one `</b>` over them removes the `b` from the stack (A + 2 visits), inserts the new `b` below
/// them (A) and moves the furthest block's A children (A): about 3 x A steps past the read of
/// step 4.24. With `max_nodes` 12,000 and A = 11,600, the limit at that read: the parse ends
/// about 2.9 x `max_nodes` past it, under the bound.
#[test]
fn the_longest_stretch_of_the_adoption_agency_stays_under_four_times_the_nodes() {
    const NODES: usize = 12_000;
    const A: usize = 11_600;
    const PERIOD: usize = 200;
    let mut page = String::from("<body><b><div id=fb>");
    for i in 0..A {
        page.push_str("<span>");
        if (i + 1) % PERIOD == 0 {
            page.push_str("<script>x</script>");
        }
    }
    let parse =
        |input: &str, max: usize| parse_document_with_hook(input, &with(max, NODES), &mut Flatten);
    let before = parse(&page, usize::MAX);
    assert_eq!(before.stopped_by(), None);
    let whole = format!("{page}</b>");
    let after = parse(&whole, usize::MAX);
    assert_eq!(after.stopped_by(), None);
    // The limit at which the stretch runs and goes furthest: the least limit whose parse ends
    // more than A past it, found by halving between the steps before the token and A + before.
    let (mut low, mut high) = (before.steps(), before.steps() + 2 * A + 100);
    assert!(parse(&whole, high).steps() > high + A);
    while low + 1 < high {
        let mid = (low + high) / 2;
        if parse(&whole, mid).steps() > mid + A {
            high = mid;
        } else {
            low = mid;
        }
    }
    let tree = parse(&whole, high);
    let over = tree.steps() - high;
    eprintln!(
        "limit {high}: {} steps, {over} past, {:.2} x max_nodes",
        tree.steps(),
        over as f64 / NODES as f64
    );
    assert_eq!(tree.stopped_by(), Some(Limit::Steps));
    assert!(over >= 3 * A, "{over} past");
    assert!(over < 4 * NODES, "{over} past");
}

// The cases from here to the end fail on the tip they were written against (`5d9b011`).

/// Ruling D423 of fix round 5: "A loop that ends at the limit leaves the tree a tree ... and
/// the parse ends with stopped_by Steps, as when the limit is read after a token"; claim 3: "A
/// loop ended at the limit answers 'not found'". The `html` and `body` start tags of "in body"
/// add their attributes only when no `template` is on the stack; `template_on_stack` scans the
/// stack from its first entry, so a `template` near the top is the last entry it reaches. When
/// the limit ends that scan it answers "no template", and `merge_attributes` checks the
/// attribute limit before it reads the step limit (in `admit`): an `html` that already holds
/// 1,024 attributes (the default limit) then stops the parse at `Attributes`, a limit the page
/// never reaches, since with the `template` open the tag is ignored. The whole parse ends with
/// no limit; every parse that stops should name `Steps`. On `5d9b011` the limits inside that
/// scan name `Attributes`, for `<html>` and for `<body>`.
#[test]
fn a_scan_ended_at_the_step_limit_names_no_other_limit() {
    let full: String = (0..1024).map(|i| format!(" a{i}=v")).collect();
    let options = |max_steps| ParseOptions {
        scripting_disabled: true,
        limits: Limits {
            max_steps,
            ..Limits::default()
        },
    };
    let mut wrong = Vec::new();
    for (page, what) in [
        (
            format!("<html{full}>{}<template><html b=2>", "<div>".repeat(40)),
            "html",
        ),
        (
            format!("<body{full}>{}<template><body b=2>", "<div>".repeat(40)),
            "body",
        ),
    ] {
        let whole = parse_document(&page, &options(usize::MAX));
        assert_eq!(whole.stopped_by(), None, "{what}");
        let mut named = Vec::new();
        for max in 0..whole.steps() {
            let tree = parse_document(&page, &options(max));
            if tree.stopped_by() != Some(Limit::Steps) {
                named.push((max, tree.stopped_by()));
            }
        }
        if !named.is_empty() {
            wrong.push(format!(
                "{what}: {} limits of {} name another limit, first {:?}",
                named.len(),
                whole.steps(),
                named[0]
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
