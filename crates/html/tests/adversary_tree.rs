//! Adversarial cases against the tree stories of round 1 (`story:html-tree-tables`,
//! `story:html-tree-template`, `story:html-tree-foreign`, `story:html-tree-complete`). Each test
//! names the sentence or the rule of the standard it holds the code to. No test asserts a time:
//! the steps a parse ends with are the count.

use surf_html::limits::{Limit, Limits};
use surf_html::{
    Context, Namespace, NodeData, NodeId, ParseOptions, Tree, parse_document, parse_fragment,
};

/// The size of a worst input, as `tests/limits.rs` has it: 64 KiB.
const SIZE: usize = 1 << 16;

fn with_limits(limits: Limits) -> ParseOptions {
    ParseOptions {
        limits,
        ..ParseOptions::default()
    }
}

/// The first HTML element named `name` in tree order.
fn first_element(tree: &Tree, name: &str) -> NodeId {
    tree.descendants(tree.root())
        .into_iter()
        .find(
            |&n| matches!(tree.data(n), NodeData::Element { local_name, .. } if local_name == name),
        )
        .unwrap_or_else(|| panic!("no {name} element"))
}

/// The text under `node`.
fn text_of(tree: &Tree, node: NodeId) -> String {
    tree.descendants(node)
        .into_iter()
        .filter_map(|n| match tree.data(n) {
            NodeData::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

/// Holds `src/lib.rs` ("Steps": "every limit below ends a parse with a defined result"; "the
/// token that takes it past the limit is processed whole, then parsing stops") and
/// `tree_builder/selectedcontent.rs` ("Each node the pass visits or creates is a step") together:
/// the pass after the end-of-file token checks the step count only before each `select`, so the
/// work of the last (here the only) `select` is never checked. 100 `selectedcontent` elements
/// 100 elements below the `select` each walk to the root: about 11,000 steps, where the parse
/// itself costs a few hundred. With the default limits the same shape (one `select`, 16 MiB of
/// `selectedcontent` pairs 500 elements down) ends far past 2^27 steps with no limit named.
#[test]
fn the_selectedcontent_pass_ends_within_the_step_limit() {
    let max_steps = 5_000;
    let input = format!(
        "<select>{}{}<option>a",
        "<span>".repeat(100),
        "<selectedcontent></selectedcontent>".repeat(100)
    );
    let tree = parse_document(
        &input,
        &with_limits(Limits {
            max_steps,
            ..Limits::default()
        }),
    );
    assert!(
        tree.steps() <= max_steps || tree.stopped_by() == Some(Limit::Steps),
        "{} steps of {max_steps}, stopped by {:?}",
        tree.steps(),
        tree.stopped_by()
    );
}

/// Holds `tree_builder/selectedcontent.rs` ("the first one that is not disabled when the display
/// size is 1") to the HTML Standard: the display size is the `size` attribute "by the rules for
/// parsing non-negative integers", and those rules read `-0` as zero (the rules for parsing
/// integers return 0 − 0, which is not less than zero). A display size of 0 is not 1, so no
/// option is selected and the `selectedcontent` stays empty. `+1` is the control: display size
/// 1, the option is copied.
#[test]
fn a_select_of_size_minus_zero_copies_no_option_into_selectedcontent() {
    let parse = |size: &str| {
        parse_document(
            &format!(
                "<!DOCTYPE html><select size={size}><button><selectedcontent></selectedcontent>\
                 </button><option>a</option></select>"
            ),
            &ParseOptions::default(),
        )
    };
    let control = parse("+1");
    assert_eq!(
        text_of(&control, first_element(&control, "selectedcontent")),
        "a"
    );
    let tree = parse("-0");
    assert_eq!(text_of(&tree, first_element(&tree, "selectedcontent")), "");
}

// The `selectedcontent` pass (`tree_builder/selectedcontent.rs`) against the rules of 4.10.17
// and 4.10.7 it names. Four conformance cases (`webkit02.dat` 45 to 48) hold it; each test
// below is the one that turns red when the rule it names is dropped from that module.

/// The text of each HTML `selectedcontent` of the document parsed from `input`, in tree order.
fn selectedcontents(input: &str, options: &ParseOptions) -> Vec<String> {
    let tree = parse_document(input, options);
    tree.descendants(tree.root())
        .into_iter()
        .filter(|&n| matches!(tree.data(n), NodeData::Element { local_name, .. } if local_name == "selectedcontent"))
        .map(|n| text_of(&tree, n))
        .collect()
}

fn sc(input: &str) -> Vec<String> {
    selectedcontents(&format!("<!DOCTYPE html>{input}"), &ParseOptions::default())
}

const BUTTON: &str = "<button><selectedcontent></selectedcontent></button>";

/// "Recalculate a selectedcontent element's disabledness": a second `select` among the
/// ancestors disables it, and a disabled one is never updated.
#[test]
fn a_selectedcontent_below_two_selects_stays_empty() {
    assert_eq!(
        sc(&format!(
            "<select>{BUTTON}<option>a</option><object><select>{BUTTON}<option>b</option>\
             </select></object></select>"
        )),
        ["a", ""]
    );
}

/// The same algorithm: an `option` (or `selectedcontent`) ancestor disables it.
#[test]
fn a_selectedcontent_inside_an_option_stays_empty() {
    assert_eq!(
        sc(&format!(
            "<select>{BUTTON}<option>a<selectedcontent></selectedcontent></option></select>"
        )),
        ["a", "", ""]
    );
}

/// "If select has the multiple attribute, then return."
#[test]
fn a_select_with_multiple_updates_no_selectedcontent() {
    assert_eq!(
        sc(&format!(
            "<select multiple>{BUTTON}<option selected>a</option></select>"
        )),
        [""]
    );
}

/// "Clone an option into a selectedcontent": `script`, `frame`, `iframe`, `embed` and `object`
/// elements are removed from the copy with their subtrees.
#[test]
fn the_copy_leaves_out_script_iframe_embed_and_object() {
    assert_eq!(
        sc(&format!(
            "<select>{BUTTON}<option>a<script>s</script><iframe>i</iframe><embed>\
             <object>o</object>b</option></select>"
        )),
        ["ab"]
    );
}

/// "Replace all with documentFragment within selectedcontent": what the element held goes.
#[test]
fn the_copy_replaces_what_the_selectedcontent_held() {
    assert_eq!(
        sc(
            "<select><button><selectedcontent>old</selectedcontent></button><option>a</option></select>"
        ),
        ["a"]
    );
}

/// An option is disabled when the nearest `optgroup` above it has `disabled`; the first option
/// that is not disabled is selected.
#[test]
fn an_option_in_a_disabled_optgroup_is_not_selected_by_default() {
    assert_eq!(
        sc(&format!(
            "<select>{BUTTON}<optgroup disabled><option>a</option></optgroup><option>b</option></select>"
        )),
        ["b"]
    );
}

/// "Get the list of options": an `optgroup` with an ancestor `optgroup` below the `select` is
/// skipped with its options.
#[test]
fn an_option_in_a_nested_optgroup_is_not_in_the_list_of_options() {
    assert_eq!(
        sc(&format!(
            "<select>{BUTTON}<optgroup><div><optgroup><option>a</option></optgroup></div>\
             </optgroup><option>b</option></select>"
        )),
        ["b"]
    );
}

/// The selectedness setting algorithm keeps the last selected option.
#[test]
fn the_last_selected_option_is_copied() {
    assert_eq!(
        sc(&format!(
            "<select>{BUTTON}<option selected>a</option><option selected>b</option></select>"
        )),
        ["b"]
    );
}

/// A display size other than 1 selects no option by default.
#[test]
fn a_select_of_size_two_copies_no_option() {
    assert_eq!(
        sc(&format!(
            "<select size=2>{BUTTON}<option>a</option></select>"
        )),
        [""]
    );
}

/// `tree_builder/selectedcontent.rs`: "neither does a parse that a limit stopped" get the
/// update. The depth limit stops this parse after the `select` is closed.
#[test]
fn a_parse_stopped_by_a_limit_updates_no_selectedcontent() {
    let input = format!(
        "<!DOCTYPE html><select>{BUTTON}<option>a</option></select>{}",
        "<div>".repeat(20)
    );
    let options = with_limits(Limits {
        max_depth: 10,
        ..Limits::default()
    });
    let tree = parse_document(&input, &options);
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
    assert_eq!(selectedcontents(&input, &options), [""]);
}

/// `run` in `src/tree_builder.rs`: the step limit is checked "also [after] the end-of-file token,
/// whose rules scan too (in template pops down to each open `template`)". 500 open templates
/// cost no step while they open and about three each at the end of the file.
#[test]
fn the_end_of_file_token_is_held_to_the_step_limit() {
    let tree = parse_document(
        &"<template>".repeat(500),
        &with_limits(Limits {
            max_steps: 1_000,
            ..Limits::default()
        }),
    );
    assert!(tree.steps() > 1_000, "{} steps", tree.steps());
    assert_eq!(tree.stopped_by(), Some(Limit::Steps));
}

/// 13.4, the fragment parsing algorithm: "Set the parser's form element pointer to the nearest
/// node to the context element that is a form element (… including the element itself …)", so
/// in body ignores a `form` start tag in a fragment for a `form` context.
#[test]
fn a_fragment_for_a_form_ignores_a_form_start_tag() {
    let context = Context {
        namespace: Namespace::Html,
        local_name: "form".to_string(),
    };
    let tree = parse_fragment("<form><input>", &context, &ParseOptions::default());
    let names: Vec<String> = tree
        .children(tree.root())
        .iter()
        .map(|&n| match tree.data(n) {
            NodeData::Element { local_name, .. } => local_name.clone(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(names, ["input"]);
}

/// Holds `docs/design/html-parser.md` item 7 and `tests/limits.rs`' small form (`max_steps` 8 ×
/// the input's size, the steps per input byte of the default 2^27 for 16 MiB), as
/// `text_twenty_elements_below_an_unclosed_b_is_not_cut_by_the_step_limit` does for in body:
/// text in a `table` is foster parented a character at a time, and each character looks for
/// the `table` among its parent's children from the first child, so each costs one step per
/// sibling in front of the table. 2,048 paragraphs, then a table holding the rest of a 64 KiB
/// page as stray text: a sloppy page, cut by the step limit.
#[test]
fn foster_parented_text_after_many_siblings_is_not_cut_by_the_step_limit() {
    let mut input = "<p>x</p>".repeat(2_048);
    input.push_str("<table>");
    input.push_str(&"y".repeat(SIZE - input.len()));
    let tree = parse_document(
        &input,
        &with_limits(Limits {
            max_steps: 8 * SIZE,
            ..Limits::default()
        }),
    );
    assert_eq!(
        tree.stopped_by(),
        None,
        "{} steps for {} bytes",
        tree.steps(),
        input.len()
    );
}

/// The same budget for foster parented elements: each `img` in a `table` is inserted in front
/// of the table, and the place is found by a scan of the parent's children from the first, which
/// holds every `img` before it. 64 KiB of `<img>` in one table.
#[test]
fn foster_parented_elements_are_not_cut_by_the_step_limit() {
    let mut input = String::from("<table>");
    input.push_str(&"<img>".repeat((SIZE - input.len()) / 5));
    let tree = parse_document(
        &input,
        &with_limits(Limits {
            max_steps: 8 * SIZE,
            ..Limits::default()
        }),
    );
    assert_eq!(
        tree.stopped_by(),
        None,
        "{} steps for {} bytes",
        tree.steps(),
        input.len()
    );
}
