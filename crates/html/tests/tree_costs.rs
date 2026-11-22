//! The cost of the `selectedcontent` pass and of foster parenting, proved by the steps a parse
//! ends with (no time is read), and the limits the pass is held to. The fixes after the
//! adversary pass on the tree stories of round 1.

use surf_html::limits::{Limit, Limits};
use surf_html::{NodeData, ParseOptions, Tree, parse_document};

fn with_limits(limits: Limits) -> ParseOptions {
    ParseOptions {
        limits,
        ..ParseOptions::default()
    }
}

fn steps(input: &str) -> usize {
    let tree = parse_document(input, &ParseOptions::default());
    assert_eq!(tree.stopped_by(), None, "{input:.80}");
    tree.steps()
}

/// Steps grow linearly: twice the input costs at most twice the steps, and a little more for
/// what every parse costs once. A cost that grows with the square of the input would cost
/// about four times as much.
fn assert_linear(shape: impl Fn(usize) -> String, n: usize) {
    let (one, two) = (steps(&shape(n)), steps(&shape(2 * n)));
    assert!(two <= 2 * one + 50, "{one} steps at n = {n}, {two} at 2n");
}

/// The `select` of `<select><button><selectedcontent></selectedcontent></button>` and one
/// option `a`, repeated.
const SELECT: &str =
    "<select><button><selectedcontent></selectedcontent></button><option>a</option></select>";

/// The text of every node under `node`.
fn text_of(tree: &Tree, node: surf_html::NodeId) -> String {
    tree.descendants(node)
        .into_iter()
        .filter_map(|n| match tree.data(n) {
            NodeData::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

/// One `select`, many `selectedcontent` elements deep below it: one walk of the document
/// decides each one's disabledness, so none walks up to the root.
#[test]
fn the_selectedcontent_pass_is_linear_for_one_select_with_many_selectedcontents() {
    assert_linear(
        |n| {
            format!(
                "<select>{}{}<option>a",
                "<span>".repeat(n),
                "<selectedcontent></selectedcontent>".repeat(n)
            )
        },
        200,
    );
}

/// Many `select` elements, one `selectedcontent` each.
#[test]
fn the_selectedcontent_pass_is_linear_for_many_selects() {
    assert_linear(|n| SELECT.repeat(n), 500);
}

/// The options of a disabled `optgroup` deep below it: each option's disabledness comes from
/// the walk of the list of options, not from a walk up. The parse's own cost of the shape
/// (each `option` tag scans the open elements) is taken away: the pass's is linear.
#[test]
fn the_selectedcontent_pass_is_linear_for_options_deep_in_an_optgroup() {
    let shape = |n: usize, sc: &str| {
        format!(
            "<select><optgroup disabled>{}{}{sc}",
            "<div>".repeat(n),
            "<option>a</option>".repeat(n)
        )
    };
    let pass = |n: usize| {
        steps(&shape(n, "<selectedcontent></selectedcontent>")) - steps(&shape(n, "<p></p>"))
    };
    let (one, two) = (pass(100), pass(200));
    assert!(two <= 2 * one + 50, "{one} steps at n = 100, {two} at 2n");
}

/// The copies are held to the node limit: a limit one node short of the parse with its copies
/// stops the pass at the last copy, with the limit named.
#[test]
fn the_selectedcontent_copies_are_held_to_the_node_limit() {
    let input = SELECT.repeat(20);
    let whole = parse_document(&input, &ParseOptions::default());
    let tree = parse_document(
        &input,
        &with_limits(Limits {
            max_nodes: whole.node_count() - 1,
            ..Limits::default()
        }),
    );
    assert_eq!(tree.stopped_by(), Some(Limit::Nodes));
    assert_eq!(tree.node_count(), whole.node_count() - 1);
}

/// The copies are held to the stored-byte limit the same way.
#[test]
fn the_selectedcontent_copies_are_held_to_the_stored_byte_limit() {
    let input = SELECT.repeat(20);
    let whole = parse_document(&input, &ParseOptions::default());
    let tree = parse_document(
        &input,
        &with_limits(Limits {
            max_stored_bytes: whole.stored_bytes() - 1,
            ..Limits::default()
        }),
    );
    assert_eq!(tree.stopped_by(), Some(Limit::StoredBytes));
    assert!(tree.stored_bytes() < whole.stored_bytes());
}

/// The pass is checked against the step limit while it copies: a limit between the steps of
/// the parse without the pass and the steps with it stops the pass, with the limit named, and
/// the steps end at most one past it.
#[test]
fn the_selectedcontent_copies_are_held_to_the_step_limit() {
    let input = format!(
        "<select><button><selectedcontent></selectedcontent></button><option>{}</option>",
        "<b>x</b>".repeat(200)
    );
    let whole = parse_document(&input, &ParseOptions::default());
    let max_steps = whole.steps() - 100;
    let tree = parse_document(
        &input,
        &with_limits(Limits {
            max_steps,
            ..Limits::default()
        }),
    );
    assert_eq!(tree.stopped_by(), Some(Limit::Steps));
    assert_eq!(tree.steps(), max_steps + 1);
    let copied = tree
        .descendants(tree.root())
        .into_iter()
        .find(|&n| {
            matches!(tree.data(n), NodeData::Element { local_name, .. }
                if local_name == "selectedcontent")
        })
        .map(|n| text_of(&tree, n))
        .unwrap_or_default();
    assert!(copied.len() < 200, "{} characters copied", copied.len());
}

/// Foster parenting in front of a `table` with many siblings in front of it.
#[test]
fn foster_parenting_after_many_siblings_is_linear() {
    assert_linear(
        |n| format!("{}<table>{}", "<p>x</p>".repeat(n), "<img>y".repeat(n)),
        1_000,
    );
}

/// Foster parenting into a `table` in a cell of another table, after siblings in the cell.
#[test]
fn foster_parenting_in_a_nested_table_is_linear() {
    assert_linear(
        |n| {
            format!(
                "<table><tr><td>{}<table>{}",
                "<p>x</p>".repeat(n),
                "<img>y".repeat(n)
            )
        },
        1_000,
    );
}

/// Foster parenting in front of a `table` in template contents.
#[test]
fn foster_parenting_in_template_contents_is_linear() {
    assert_linear(
        |n| {
            format!(
                "<template>{}<table>{}",
                "<p>x</p>".repeat(n),
                "<img>y".repeat(n)
            )
        },
        1_000,
    );
}

/// A `selectedcontent` inside the selected option is disabled, so the option is never copied
/// into its own descendant: that copy would grow until a limit ended it. The small node limit
/// makes such a copy end at once, by the limit; the right parse stays far inside it.
#[test]
fn a_selectedcontent_inside_the_selected_option_is_not_copied_into_itself() {
    let tree = parse_document(
        "<select><button><selectedcontent></selectedcontent></button>\
         <option>a<selectedcontent></selectedcontent></option></select>",
        &with_limits(Limits {
            max_nodes: 1_000,
            ..Limits::default()
        }),
    );
    assert_eq!(tree.stopped_by(), None);
    assert!(tree.node_count() < 20, "{} nodes", tree.node_count());
}

/// Foster parenting after many closed tables in the same parent.
#[test]
fn foster_parenting_after_closed_tables_is_linear() {
    assert_linear(
        |n| {
            format!(
                "{}<table>{}",
                "<table></table>".repeat(n),
                "y<img>".repeat(n)
            )
        },
        1_000,
    );
}
