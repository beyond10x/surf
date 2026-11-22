//! The limits of `docs/design/html-parser.md` item 7 (`story:html-parser-limits`): each limit
//! stops a parse at the point the crate documentation names, with the tree built before it;
//! adversarial inputs and 10,000 seeded ones end inside the node and stored-byte limits, within
//! the bytes [`common::allocation_bound`] allows and without a panic. Every test parses with the
//! default `Limits` unless it says otherwise, and builds its own input (each test runs in a
//! process of its own). The tests of the section "Steps" assert the steps each scan adds.
//!
//! No test asserts a time: on a loaded machine 10 s bounds failed for inputs that were not
//! slower (the fix unit of the review). What catches an input that turns quadratic again: in
//! the scans, the step limit, so each test asserts the limit that stops it (one that ended
//! without one, or at the depth limit, then names `Steps`); in copies of what a token does not
//! pay for, the bytes allocated; a scan that stops counting its visits, the tests of "Steps".
//! The runner's 60 s end is the only time bound left. A shape that ran over 5 s at 16 MiB in the
//! test profile runs at 1 MiB with `max_steps` 2^23 ([`one_mib_form`]), the same visits per
//! input byte; the report of the story gives what each measured in both forms.

mod common;

use std::time::Instant;

use surf_html::limits::{ATTRIBUTE_BYTES, Limit, Limits};
use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document, parse_document_bytes};

const MIB: usize = 1 << 20;

/// The node and stored-byte limits of the defaults.
const MAX_NODES: usize = 1_048_576;
const MAX_STORED_BYTES: usize = 67_108_864;

/// The 1 MiB form of a shape that runs over 5 s at 16 MiB in the test profile: the default
/// limits but `max_steps` 2^23, the same steps per input byte as 2^27 for 16 MiB.
fn one_mib_form() -> ParseOptions {
    with_limits(Limits {
        max_steps: 1 << 23,
        ..Limits::default()
    })
}

fn with_limits(limits: Limits) -> ParseOptions {
    ParseOptions {
        limits,
        ..ParseOptions::default()
    }
}

/// `prefix`, then `unit` as many times as fit in `total` bytes.
fn fill(prefix: &str, unit: &str, total: usize) -> String {
    let mut s = String::with_capacity(total);
    s.push_str(prefix);
    while s.len() + unit.len() <= total {
        s.push_str(unit);
    }
    s
}

/// ` a0 a1 …`: `n` attributes with different names and no value.
fn attributes(n: usize) -> String {
    (0..n).map(|k| format!(" a{k}")).collect()
}

/// `<b id=0><b id=1>…`: `n` formatting elements that Noah's Ark keeps apart.
fn b_with_ids(n: usize) -> String {
    (0..n).map(|k| format!("<b id={k}>")).collect()
}

/// Parses `input` with `options` and asserts that it ends stopped by `expected`, inside the node
/// and stored-byte limits of the defaults, within the bytes [`common::allocation_bound`] allows,
/// and past the step limit only when that limit stopped it. Prints the time it took, which no
/// assertion reads.
fn bounded(name: &str, input: &str, options: &ParseOptions, expected: Option<Limit>) -> Tree {
    let start = Instant::now();
    let (tree, allocated) = common::allocated_while(|| parse_document(input, options));
    let took = start.elapsed();
    let bound = common::allocation_bound(input.len(), &tree);
    println!(
        "measured: {name} {:.3} s, {} input bytes, stopped by {:?}, {} steps, {} nodes, {} stored \
         bytes, {allocated} bytes allocated (bound {bound})",
        took.as_secs_f64(),
        input.len(),
        tree.stopped_by(),
        tree.steps(),
        tree.node_count(),
        tree.stored_bytes()
    );
    assert_eq!(tree.stopped_by(), expected, "{name}");
    assert!(tree.node_count() <= MAX_NODES);
    assert!(tree.stored_bytes() <= MAX_STORED_BYTES);
    assert!(
        allocated <= bound,
        "{name}: {allocated} bytes allocated, bound {bound}"
    );
    assert!(
        tree.steps() <= options.limits.max_steps || tree.stopped_by() == Some(Limit::Steps),
        "{name}: {} steps past the limit without stopping",
        tree.steps()
    );
    tree
}

/// The steps a parse of `input` with the default limits used.
fn steps(input: &str) -> usize {
    let tree = parse_document(input, &ParseOptions::default());
    assert_eq!(tree.stopped_by(), None, "{input}");
    tree.steps()
}

/// The elements under the root named `local_name`, in tree order.
fn elements(tree: &Tree, local_name: &str) -> Vec<NodeId> {
    tree.descendants(tree.root())
        .into_iter()
        .filter(
            |&n| matches!(tree.data(n), NodeData::Element { local_name: l, .. } if l == local_name),
        )
        .collect()
}

/// The data of every node under the root, in tree order.
fn data(tree: &Tree) -> Vec<NodeData> {
    tree.descendants(tree.root())
        .into_iter()
        .map(|n| tree.data(n).clone())
        .collect()
}

/// The UTF-8 bytes of every string of the nodes under the root and `ATTRIBUTE_BYTES` for each
/// attribute, counted from what they hold: `stored_bytes()` for a tree with no node out of it.
fn stored_bytes_of_nodes(tree: &Tree) -> usize {
    tree.descendants(tree.root())
        .into_iter()
        .map(|n| match tree.data(n) {
            NodeData::Document | NodeData::DocumentFragment => 0,
            NodeData::Doctype {
                name,
                public_id,
                system_id,
            } => name.len() + public_id.len() + system_id.len(),
            NodeData::Element {
                local_name,
                attributes,
                ..
            } => {
                local_name.len()
                    + attributes
                        .iter()
                        .map(|a| {
                            ATTRIBUTE_BYTES
                                + a.prefix.as_ref().map_or(0, String::len)
                                + a.local_name.len()
                                + a.value.len()
                        })
                        .sum::<usize>()
            }
            NodeData::Text(text) => text.len(),
            NodeData::Comment(data) => data.len(),
            NodeData::ProcessingInstruction { target, data } => target.len() + data.len(),
        })
        .sum()
}

fn attribute_count(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes.len(),
        _ => panic!("not an element"),
    }
}

// One test per limit.

#[test]
fn input_of_16_777_217_bytes_stops_at_the_input_limit() {
    check_cut_comment(&parse_document(
        &comment_of_16_777_217_bytes(),
        &ParseOptions::default(),
    ));
}

#[test]
fn input_of_16_777_217_bytes_as_bytes_stops_at_the_input_limit() {
    let parsed = parse_document_bytes(
        comment_of_16_777_217_bytes().as_bytes(),
        &ParseOptions::default(),
    );
    check_cut_comment(&parsed.tree);
}

/// `<!--xxx…`: one comment of 16,777,217 bytes, one byte past the default input limit.
fn comment_of_16_777_217_bytes() -> String {
    let mut input = String::from("<!--");
    input.push_str(&"x".repeat(16_777_217 - 4));
    assert_eq!(input.len(), 16_777_217);
    input
}

/// The tree of [`comment_of_16_777_217_bytes`]: stopped by the input limit, with the comment of
/// the first 16 MiB.
fn check_cut_comment(tree: &Tree) {
    assert_eq!(Limits::default().max_input_bytes, 16 * MIB);
    assert_eq!(tree.stopped_by(), Some(Limit::InputBytes));
    let comments: Vec<usize> = tree
        .children(tree.root())
        .iter()
        .filter_map(|&c| match tree.data(c) {
            NodeData::Comment(data) => Some(data.len()),
            _ => None,
        })
        .collect();
    assert_eq!(comments, [16 * MIB - 4]);
}

#[test]
fn a_cut_input_parses_as_its_prefix() {
    // `é` is two bytes, at bytes 99 and 100: a limit of 100 bytes cuts before it.
    let mut input = String::from("<p title=t>");
    input.push_str(&"a".repeat(99 - input.len()));
    input.push_str("é<b>after</b>");
    assert!(!input.is_char_boundary(100));
    let options = with_limits(Limits {
        max_input_bytes: 100,
        ..Limits::default()
    });
    let cut = parse_document(&input, &options);
    let prefix = parse_document(&input[..99], &ParseOptions::default());
    assert_eq!(cut.stopped_by(), Some(Limit::InputBytes));
    assert_eq!(prefix.stopped_by(), None);
    assert_eq!(data(&cut), data(&prefix));
    // Bytes are cut before decoding: after a byte order mark of 3 bytes, the first byte of `é`
    // alone decodes as UTF-8 decodes an end of input inside a sequence, one U+FFFD.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(input.as_bytes());
    let bytes = parse_document_bytes(
        &bytes,
        &with_limits(Limits {
            max_input_bytes: 103,
            ..Limits::default()
        }),
    );
    assert_eq!(bytes.tree.stopped_by(), Some(Limit::InputBytes));
    let mut expected = input[..99].to_string();
    expected.push('\u{FFFD}');
    assert_eq!(
        data(&bytes.tree),
        data(&parse_document(&expected, &ParseOptions::default()))
    );
    // An input of exactly the limit is not cut.
    let exact = parse_document(
        &input[..99],
        &with_limits(Limits {
            max_input_bytes: 99,
            ..Limits::default()
        }),
    );
    assert_eq!(exact.stopped_by(), None);
}

#[test]
fn element_at_depth_513_stops_at_the_depth_limit() {
    let tree = parse_document(&"<div>".repeat(600), &ParseOptions::default());
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
    // `html`, `body` and 510 `div` are the 512 entries of the stack; `head` was popped.
    assert_eq!(elements(&tree, "div").len(), 510);
    assert_eq!(tree.node_count(), 514);
    assert_eq!(tree.descendants(tree.root()).len(), 514);
    // One `div` fewer stays inside the limit.
    let tree = parse_document(&"<div>".repeat(510), &ParseOptions::default());
    assert_eq!(tree.stopped_by(), None);
}

#[test]
fn node_1_048_577_stops_at_the_node_limit() {
    let tree = parse_document(&"<!---->".repeat(1_048_580), &ParseOptions::default());
    assert_eq!(tree.stopped_by(), Some(Limit::Nodes));
    assert_eq!(tree.node_count(), 1_048_576);
    assert_eq!(tree.descendants(tree.root()).len(), 1_048_576);
}

#[test]
fn attribute_1_025_on_one_element_stops_at_the_attribute_limit() {
    let input = format!("<p>x</p><span{}>y", attributes(1_025));
    let tree = parse_document(&input, &ParseOptions::default());
    assert_eq!(tree.stopped_by(), Some(Limit::Attributes));
    assert_eq!(elements(&tree, "p").len(), 1);
    assert!(elements(&tree, "span").is_empty());
    let p = elements(&tree, "p")[0];
    assert_eq!(tree.children(p).len(), 1);
    assert_eq!(
        *tree.data(tree.children(p)[0]),
        NodeData::Text("x".to_string())
    );
    // 1,024 attributes and 1,000 duplicates the tokenizer drops: the element is created.
    let input = format!("<span{}{}>y", attributes(1_024), " a0".repeat(1_000));
    let tree = parse_document(&input, &ParseOptions::default());
    assert_eq!(tree.stopped_by(), None);
    assert_eq!(attribute_count(&tree, elements(&tree, "span")[0]), 1_024);
    // Every attribute name and value counts toward the stored bytes.
    assert_eq!(tree.stored_bytes(), stored_bytes_of_nodes(&tree));
}

#[test]
fn attribute_1_025_merged_into_html_stops_at_the_attribute_limit() {
    // 1,000 attributes on `html`, then a tag with 20 attributes already there and 25 new ones:
    // the merge would give `html` attribute 1,025, so nothing of it is made.
    let first = attributes(1_000);
    let again: String = (980..1_025).map(|k| format!(" a{k}")).collect();
    let input = format!("<html{first}><p>x<html{again}><b>y");
    let tree = parse_document(&input, &ParseOptions::default());
    assert_eq!(tree.stopped_by(), Some(Limit::Attributes));
    assert_eq!(attribute_count(&tree, elements(&tree, "html")[0]), 1_000);
    assert!(elements(&tree, "b").is_empty());
    // With 24 new ones the merge gives exactly 1,024.
    let again: String = (980..1_024).map(|k| format!(" a{k}")).collect();
    let input = format!("<html{first}><p>x<html{again}><b>y");
    let tree = parse_document(&input, &ParseOptions::default());
    assert_eq!(tree.stopped_by(), None);
    assert_eq!(attribute_count(&tree, elements(&tree, "html")[0]), 1_024);
    assert_eq!(elements(&tree, "b").len(), 1);
    // `<body>` after an implied `body` with no attribute: 1,024 merge, one more stops.
    let input = format!("<p>x<body{}><body b9999>", attributes(1_024));
    let tree = parse_document(&input, &ParseOptions::default());
    assert_eq!(tree.stopped_by(), Some(Limit::Attributes));
    assert_eq!(attribute_count(&tree, elements(&tree, "body")[0]), 1_024);
}

#[test]
fn cloned_attributes_past_64_mib_stop_at_the_stored_bytes_limit() {
    let value = "v".repeat(1_024);
    let mut prefix = String::from("<p>");
    for k in 0..500 {
        prefix.push_str(&format!("<b v={value} id={k}>"));
    }
    let input = fill(&prefix, "<p>x", 16 * MIB);
    let tree = bounded(
        "cloned_attributes_past_64_mib",
        &input,
        &ParseOptions::default(),
        Some(Limit::StoredBytes),
    );
    assert!(tree.stored_bytes() > MAX_STORED_BYTES - 2_000);
    assert_eq!(tree.stored_bytes(), stored_bytes_of_nodes(&tree));
}

#[test]
fn stack_visits_past_2_27_stop_at_the_step_limit() {
    // 2^27 steps cost 3.6 to 4.9 s in the test profile and over 10 s on a loaded machine: this
    // test lowers the step limit to 2^25, the one change from the defaults. Each `</q>` visits
    // more than 500 open elements, so 2 MiB of them pass 2^27 steps as well.
    assert_eq!(Limits::default().max_steps, 1 << 27);
    let input = fill(&"<span>".repeat(505), "</q>", 2 * MIB);
    assert!((input.len() - 505 * 6) / 4 * 500 > 1 << 27);
    let tree = bounded(
        "stack_visits_past_2_27",
        &input,
        &with_limits(Limits {
            max_steps: 1 << 25,
            ..Limits::default()
        }),
        Some(Limit::Steps),
    );
    // The `span` elements stay: the step limit stopped a parse that created nothing more.
    assert_eq!(elements(&tree, "span").len(), 505);
    // A thousand `</q>` stay far inside the limit.
    let input = format!("{}{}", "<span>".repeat(505), "</q>".repeat(1_000));
    let tree = parse_document(&input, &ParseOptions::default());
    assert_eq!(tree.stopped_by(), None);
}

#[test]
fn a_node_limit_inside_the_adoption_agency_leaves_no_element_out_of_the_tree() {
    // Before `</a>` the tree has 9 nodes; its first outer iteration clones `i` and `b` and
    // creates a new `a`. With every limit of nodes from 1 to 16, every node created is in the
    // tree: an iteration that could pass the limit is not started.
    let input = "<a><b><i><div>x</a>y";
    let unlimited = parse_document(input, &ParseOptions::default());
    assert_eq!(unlimited.node_count(), 13);
    for max_nodes in 1..=16 {
        let tree = parse_document(
            input,
            &with_limits(Limits {
                max_nodes,
                ..Limits::default()
            }),
        );
        assert!(tree.node_count() <= max_nodes, "max_nodes {max_nodes}");
        assert_eq!(
            tree.node_count(),
            tree.descendants(tree.root()).len(),
            "max_nodes {max_nodes}"
        );
        if max_nodes < 13 {
            assert_eq!(
                tree.stopped_by(),
                Some(Limit::Nodes),
                "max_nodes {max_nodes}"
            );
        } else {
            assert_eq!(tree.stopped_by(), None, "max_nodes {max_nodes}");
            assert_eq!(data(&tree), data(&unlimited));
        }
    }
    // With 10 or 11 the limit falls inside the `</a>`: it creates nothing.
    for max_nodes in [10, 11] {
        let tree = parse_document(
            input,
            &with_limits(Limits {
                max_nodes,
                ..Limits::default()
            }),
        );
        assert_eq!(tree.node_count(), 9, "max_nodes {max_nodes}");
    }
}

// Steps: each scan adds the entries it visits. Each test compares two inputs that differ only in
// what one scan visits, so that a scan that stops counting turns it red; the end-of-file token
// costs no step in "in body", "after body" and "in frameset".

#[test]
fn steps_reads_the_count_the_step_limit_checks() {
    // `<p>` visits `body` and `html` for "has a p element in button scope"; `x` visits nothing.
    assert_eq!(steps("<p>x"), 2);
    // The token that passes the limit is processed whole, and its steps are counted.
    let tree = parse_document(
        "<p>x",
        &with_limits(Limits {
            max_steps: 1,
            ..Limits::default()
        }),
    );
    assert_eq!(tree.stopped_by(), Some(Limit::Steps));
    assert_eq!(tree.steps(), 2);
}

#[test]
fn steps_count_each_child_the_adoption_agency_moves() {
    // `</a>` moves the `div`'s children to the new `a` (step 4.30): 8 comments more are 8 steps
    // more; a comment is inserted at the end of its parent and visits nothing.
    let input = |k: usize| format!("<a><div>{}</a>", "<!---->".repeat(k));
    assert_eq!(steps(&input(8)) - steps(&input(0)), 8);
}

#[test]
fn steps_count_each_child_visited_to_remove_the_body() {
    // A `frameset` start tag removes the `body` from the `html` element, scanning its children
    // from the last: the comments after `</body>` went into the `html` element after the `body`.
    let input = |k: usize| format!("</body>{}<frameset>", "<!---->".repeat(k));
    assert_eq!(steps(&input(8)) - steps(&input(0)), 8);
}

#[test]
fn steps_count_each_document_child_visited_before_the_html_element() {
    // The `html` element is inserted into the document only when it has no element child: the
    // comments before it are visited.
    let input = |k: usize| format!("{}<html>", "<!---->".repeat(k));
    assert_eq!(steps(&input(8)) - steps(&input(0)), 8);
}

#[test]
fn steps_count_each_open_element_visited_for_a_template() {
    // An `html` start tag in body asks whether a `template` is on the stack: `html`, `body` and
    // 5 `div`, 7 steps for each `html` start tag.
    let input = |m: usize| format!("{}{}", "<div>".repeat(5), "<html>".repeat(m));
    assert_eq!(steps(&input(2)) - steps(&input(1)), 7);
}

#[test]
fn steps_count_each_inner_loop_iteration_of_the_adoption_agency() {
    // `</a>` over `html`, `body`, `a`, `span`, `div` (the list holds the `a`). First outer
    // iteration: the list's entry (4.3) 1, the stack down to the `a` (4.4) 3, the scope (4.5) 3,
    // the furthest block (4.7) 2, the entries `iteration_creates` looks up 1, the inner loop 2
    // iterations and the `span`'s missing entry 1 (4.18), removing the `div` from the `span`
    // (4.22) 1, the walk from the `body` to the root (4.23) 3, the formatting element's entry
    // (4.25) 1. Second outer iteration: the entry, the stack and the scope, 1 each; no furthest
    // block. 21 in all, 2 of them the inner loop's own.
    assert_eq!(steps("<a><span><div></a>") - steps("<a><span><div>"), 21);
}

#[test]
fn steps_count_each_entry_noahs_ark_compares() {
    // Each formatting start tag compares the entries after the last marker: the tenth of ten
    // `b` with different ids visits 9, so the nine after the first visit 1 + 2 + … + 9.
    let input = |k: usize| (0..k).map(|j| format!("<b id={j}>")).collect::<String>();
    assert_eq!(steps(&input(10)) - steps(&input(1)), 45);
}

#[test]
fn steps_count_each_entry_the_reconstruction_rewinds_over() {
    // After `</p>` the five `b` are closed and stay in the list. `x` rewinds over 4 entries and
    // advances over 5, creating an element for each.
    let prefix = format!(
        "<p>{}</p>",
        (0..5).map(|j| format!("<b id={j}>")).collect::<String>()
    );
    assert_eq!(steps(&format!("{prefix}x")) - steps(&prefix), 9);
}

// Adversarial inputs: each ends inside the limits, stopped by the limit named.

#[test]
fn nested_div_100_000_ends_within_bounds() {
    let input = "<div>".repeat(100_000);
    bounded(
        "nested_div_100_000",
        &input,
        &ParseOptions::default(),
        Some(Limit::Depth),
    );
}

#[test]
fn formatting_elements_100_000_then_text_after_p_end_within_bounds() {
    let mut input = String::from("<p>");
    for k in 0..100_000 {
        input.push_str(["<b>", "<i>", "<em>", "<code>"][k % 4]);
    }
    input.push_str("</p>");
    input.push_str(&"x".repeat(100_000));
    bounded(
        "formatting_elements_100_000_then_text_after_p",
        &input,
        &ParseOptions::default(),
        Some(Limit::Depth),
    );
}

#[test]
fn table_with_100_000_misnested_a_ends_within_bounds() {
    let input = format!("<table>{}", "<a><div></a>".repeat(100_000));
    bounded(
        "table_with_100_000_misnested_a",
        &input,
        &ParseOptions::default(),
        Some(Limit::Depth),
    );
}

#[test]
fn element_with_1_000_000_attributes_ends_within_bounds() {
    let input = format!("<div{}>x", attributes(1_000_000));
    bounded(
        "element_with_1_000_000_attributes",
        &input,
        &ParseOptions::default(),
        Some(Limit::Attributes),
    );
}

#[test]
fn input_of_16_mib_plus_one_byte_ends_within_bounds() {
    // The 1 MiB form, with the input limit at 1 MiB too.
    let options = with_limits(Limits {
        max_input_bytes: MIB,
        max_steps: 1 << 23,
        ..Limits::default()
    });
    let input = "x".repeat(MIB + 1);
    let tree = bounded(
        "input_of_16_mib_plus_one_byte",
        &input,
        &options,
        Some(Limit::InputBytes),
    );
    assert_eq!(
        tree.stored_bytes(),
        MIB + "html".len() + "head".len() + "body".len()
    );
}

#[test]
fn reconstruction_of_1_000_formatting_elements_ends_within_bounds() {
    let input = format!("<p>{}{}", b_with_ids(1_000), "<p>x".repeat(1_000));
    bounded(
        "reconstruction_of_1_000_formatting_elements",
        &input,
        &ParseOptions::default(),
        Some(Limit::Depth),
    );
}

#[test]
fn nested_object_60_000_then_misnested_a_ends_within_bounds() {
    let unit = format!("<a>{}</a>{}", "<div>".repeat(8), "</div>".repeat(8));
    let input = fill(&"<object>".repeat(60_000), &unit, MIB);
    bounded(
        "nested_object_60_000_then_misnested_a",
        &input,
        &ParseOptions::default(),
        Some(Limit::Depth),
    );
}

#[test]
fn a_then_div_20_000_then_a_end_tags_end_within_bounds() {
    let input = fill(&format!("<a>{}", "<div>".repeat(20_000)), "</a>", MIB);
    bounded(
        "a_then_div_20_000_then_a_end_tags",
        &input,
        &ParseOptions::default(),
        Some(Limit::Depth),
    );
}

#[test]
fn nested_b_30_000_with_different_ids_ends_within_bounds() {
    bounded(
        "nested_b_30_000_with_different_ids",
        &b_with_ids(30_000),
        &ParseOptions::default(),
        Some(Limit::Depth),
    );
}

#[test]
fn nested_fieldset_with_legend_20_000_ends_within_bounds() {
    let input = "<fieldset disabled><legend>".repeat(20_000);
    bounded(
        "nested_fieldset_with_legend_20_000",
        &input,
        &ParseOptions::default(),
        Some(Limit::Depth),
    );
}

#[test]
fn object_490_then_misnested_a_to_16_mib_ends_within_bounds() {
    // The 1 MiB form.
    let unit = format!("<a>{}</a>{}", "<div>".repeat(8), "</div>".repeat(8));
    let input = fill(&"<object>".repeat(490), &unit, MIB);
    bounded(
        "object_490_then_misnested_a_to_16_mib",
        &input,
        &one_mib_form(),
        Some(Limit::Steps),
    );
}

#[test]
fn a_then_div_500_then_a_end_tags_to_16_mib_end_within_bounds() {
    // The 1 MiB form.
    let input = fill(&format!("<a>{}", "<div>".repeat(500)), "</a>", MIB);
    bounded(
        "a_then_div_500_then_a_end_tags_to_16_mib",
        &input,
        &one_mib_form(),
        None,
    );
}

#[test]
fn span_505_then_unmatched_end_tags_to_16_mib_end_within_bounds() {
    // The 1 MiB form.
    let input = fill(&"<span>".repeat(505), "</q>", MIB);
    bounded(
        "span_505_then_unmatched_end_tags_to_16_mib",
        &input,
        &one_mib_form(),
        Some(Limit::Steps),
    );
}

#[test]
fn span_505_then_div_pairs_to_16_mib_end_within_bounds() {
    // The 1 MiB form.
    let input = fill(&"<span>".repeat(505), "<div></div>", MIB);
    bounded(
        "span_505_then_div_pairs_to_16_mib",
        &input,
        &one_mib_form(),
        Some(Limit::Steps),
    );
}

#[test]
fn b_505_with_1_024_attributes_each_ends_within_bounds() {
    // Noah's Ark: each `b` has the same 1,023 attributes and one that differs.
    let common = attributes(1_023);
    let input: String = (0..505).map(|k| format!("<b{common} z={k}>")).collect();
    bounded(
        "b_505_with_1_024_attributes_each",
        &input,
        &ParseOptions::default(),
        None,
    );
}

#[test]
fn tags_with_1_024_attributes_to_16_mib_end_within_bounds() {
    // The 1 MiB form.
    let unit = format!("<span{}></span>", attributes(1_024));
    let input = fill("", &unit, MIB);
    bounded(
        "tags_with_1_024_attributes_to_16_mib",
        &input,
        &one_mib_form(),
        None,
    );
}

#[test]
fn html_start_tags_with_new_attributes_end_within_bounds() {
    let input: String = (0..50_000).map(|k| format!("<html a{k}>")).collect();
    let tree = bounded(
        "html_start_tags_with_new_attributes",
        &input,
        &ParseOptions::default(),
        Some(Limit::Attributes),
    );
    assert_eq!(attribute_count(&tree, elements(&tree, "html")[0]), 1_024);
}

// Seeded inputs.

/// A xorshift64* generator: the same seed gives the same inputs on every machine.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// The fragments seeded inputs are made of: markup of the modes written, the tags of the later
/// tree stories (tables, `select`, `template`, `svg`, `math`), misnested formatting elements,
/// and pieces of markup cut anywhere.
const FRAGMENTS: &[&str] = &[
    "<div>",
    "</div>",
    "<p>",
    "</p>",
    "<a href=x>",
    "</a>",
    "<b>",
    "</b>",
    "<i class=c>",
    "</i>",
    "<nobr>",
    "</nobr>",
    "<font color=red>",
    "</font>",
    "<table>",
    "</table>",
    "<tr>",
    "<td>",
    "</td>",
    "<th>",
    "<caption>",
    "<select>",
    "<option>",
    "</select>",
    "<template>",
    "</template>",
    "<svg>",
    "</svg>",
    "<math>",
    "<mi>",
    "</math>",
    "<li>",
    "<dd>",
    "<dt>",
    "<ul>",
    "</ul>",
    "<object>",
    "</object>",
    "<marquee>",
    "<button>",
    "</button>",
    "<form>",
    "</form>",
    "<h1>",
    "</h3>",
    "<pre>\n",
    "<textarea>",
    "</textarea>",
    "<title>",
    "</title>",
    "<script>",
    "</script>",
    "<style>",
    "<noscript>",
    "<iframe>",
    "<xmp>",
    "<plaintext>",
    "<frameset>",
    "<frame>",
    "</frameset>",
    "<html lang=en>",
    "<body class=b>",
    "</body>",
    "</html>",
    "<html dir=ltr>",
    "<body id=b2>",
    "<p a b c>",
    "<div id=d class=e>",
    "<head>",
    "<meta charset=utf-8>",
    "<br>",
    "</br>",
    "<img src=s>",
    "<image>",
    "<input type=hidden>",
    "<hr>",
    "<!DOCTYPE html>",
    "<!-- c -->",
    "<?pi x?>",
    "<![CDATA[d]]>",
    "x",
    "yz ",
    "\n",
    "&amp;",
    "&#x41;",
    "&notin",
    "&",
    "<",
    "</",
    "<x",
    "a=\"",
    "\"",
    "'",
    "=",
    ">",
    "\0",
    "é",
    "<ruby><rt>",
    "<rb>",
    "<applet>",
    "</p></b></a>",
];

/// One seeded input of at most 4,096 bytes: one in eight up to 4,096 bytes long, the others up to
/// 512, so that the 10,000 stay within the time of the test step.
fn seeded_input(random: &mut Seeded) -> String {
    let len = if random.below(8) == 0 {
        random.below(4_097)
    } else {
        random.below(513)
    };
    let mut s = String::new();
    loop {
        let fragment = FRAGMENTS[random.below(FRAGMENTS.len())];
        if s.len() + fragment.len() > len {
            return s;
        }
        s.push_str(fragment);
    }
}

#[test]
fn seeded_10_000_inputs_end_within_bounds() {
    // One input in four parses with the default limits; each of the others with one limit made
    // small, in turn, so that every limit stops some of them at a place of its own.
    let d = Limits::default();
    let small = [
        Limits {
            max_input_bytes: 1_000,
            ..d
        },
        Limits { max_depth: 12, ..d },
        Limits { max_nodes: 40, ..d },
        Limits {
            max_attributes: 1,
            ..d
        },
        Limits {
            max_stored_bytes: 800,
            ..d
        },
        Limits {
            max_steps: 100,
            ..d
        },
    ];
    let mut random = Seeded(0x5EED_0000_2026_1122);
    let mut stopped = std::collections::HashMap::new();
    let (mut most_steps, mut most_allocated) = (0, 0);
    let start = Instant::now();
    for n in 0..10_000 {
        let input = seeded_input(&mut random);
        let limits = if n % 4 == 0 {
            d
        } else {
            small[n % small.len()]
        };
        let options = with_limits(limits);
        let parsed = std::panic::catch_unwind(|| {
            common::allocated_while(|| {
                if n % 5 == 0 {
                    parse_document_bytes(input.as_bytes(), &options).tree
                } else {
                    parse_document(&input, &options)
                }
            })
        });
        let (tree, allocated) = parsed.unwrap_or_else(|_| panic!("input {n} panicked: {input:?}"));
        assert!(
            tree.node_count() <= limits.max_nodes,
            "input {n}: {input:?}"
        );
        assert!(
            tree.stored_bytes() <= limits.max_stored_bytes,
            "input {n}: {input:?}"
        );
        // `parse_document_bytes` decodes the input first, and may parse twice.
        let bound = 3 * common::allocation_bound(input.len(), &tree);
        assert!(
            allocated <= bound,
            "input {n}: {allocated} bytes allocated, bound {bound}: {input:?}"
        );
        assert!(
            tree.steps() <= limits.max_steps || tree.stopped_by() == Some(Limit::Steps),
            "input {n}: {} steps: {input:?}",
            tree.steps()
        );
        most_steps = most_steps.max(tree.steps());
        most_allocated = most_allocated.max(allocated);
        *stopped.entry(tree.stopped_by()).or_insert(0) += 1;
    }
    let took = start.elapsed();
    println!(
        "measured: seeded_10_000_inputs {:.3} s, at most {most_steps} steps and {most_allocated} \
         bytes allocated for one input, stopped by {stopped:?}",
        took.as_secs_f64()
    );
    for limit in [
        Limit::Depth,
        Limit::Nodes,
        Limit::Attributes,
        Limit::StoredBytes,
        Limit::Steps,
        Limit::InputBytes,
    ] {
        assert!(
            stopped.contains_key(&Some(limit)),
            "no input stopped by {limit:?}"
        );
    }
}
