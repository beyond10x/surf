//! The outline of a document's tree: one line per element and per text node, bounded, with the
//! handles of [`surf_page::handles`] (`docs/design/page-as-text.md` section 5). The rules are in
//! the crate documentation.

use std::collections::{HashMap, HashSet};

use surf_answer::{quote_text, quote_text_cut};
use surf_html::{Attribute, Namespace, NodeData, NodeId, Tree};

use crate::{collapse, count_node_visit};

/// The level, counted from the `html` element at 0, below which the outline shows nothing: an
/// element at this level that has children ends in `deep <n>`.
pub const LEVEL_MAX: usize = 40;

/// The characters of a text or an attribute value the outline shows before it cuts.
pub const VALUE_CHARS_MAX: usize = 80;

/// The attributes the outline shows, in this order.
pub const ATTRIBUTES: [&str; 11] = [
    "id", "lang", "href", "src", "alt", "type", "name", "action", "method", "colspan", "rowspan",
];

/// The lines of the outline of `tree`, without the `> ` prefix of the answer, one at a time, so
/// that a caller keeps only the lines it shows (`surf_answer::text_page`). One walk with its
/// own stack over [`Tree::children`]; the subtree of a hidden element or of one at
/// [`LEVEL_MAX`] is walked once more, only to count it. The time is linear in the number of
/// nodes and of their attributes.
pub fn outline(tree: &Tree) -> impl Iterator<Item = String> + '_ {
    let handles: HashMap<NodeId, String> = surf_page::handles(tree).into_iter().collect();
    let hidden = surf_page::hidden_roots(tree);
    let pending = tree
        .children(tree.root())
        .iter()
        .rev()
        .map(|&child| (child, 0))
        .collect();
    Outline {
        tree,
        handles,
        hidden,
        pending,
    }
}

struct Outline<'a> {
    tree: &'a Tree,
    handles: HashMap<NodeId, String>,
    /// The nodes hidden while their parent is not ([`surf_page::hidden_roots`]): the walk never
    /// enters a hidden subtree, so a node it meets is hidden exactly when it is in the set.
    hidden: HashSet<NodeId>,
    /// The nodes still to write, each with its level, the next on top.
    pending: Vec<(NodeId, usize)>,
}

impl Iterator for Outline<'_> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        while let Some((node, level)) = self.pending.pop() {
            count_node_visit();
            match self.tree.data(node) {
                NodeData::Element {
                    namespace,
                    local_name,
                    attributes,
                    ..
                } => {
                    return Some(
                        self.element_line(node, level, *namespace, local_name, attributes),
                    );
                }
                NodeData::Text(text) if !self.hidden.contains(&node) => {
                    let text = collapse(text);
                    if !text.is_empty() {
                        return Some(format!(
                            "{}{}",
                            indent(level),
                            quote_text_cut(&text, VALUE_CHARS_MAX)
                        ));
                    }
                }
                _ => {}
            }
        }
        None
    }
}

impl Outline<'_> {
    fn element_line(
        &mut self,
        node: NodeId,
        level: usize,
        namespace: Namespace,
        local_name: &str,
        attributes: &[Attribute],
    ) -> String {
        let mut line = indent(level);
        line.push_str(&name(namespace, local_name));
        if let Some(handle) = self.handles.get(&node) {
            line.push_str(" @");
            line.push_str(handle);
        }
        let mut values: [Option<&str>; ATTRIBUTES.len()] = [None; ATTRIBUTES.len()];
        for attribute in attributes.iter().filter(|a| a.namespace.is_none()) {
            if let Some(at) = ATTRIBUTES.iter().position(|&n| n == attribute.local_name) {
                values[at].get_or_insert(attribute.value.as_str());
            }
        }
        for (name, value) in ATTRIBUTES.iter().zip(values) {
            if let Some(value) = value {
                line.push(' ');
                line.push_str(name);
                line.push('=');
                line.push_str(&quote_text_cut(value, VALUE_CHARS_MAX));
            }
        }
        let children = self.tree.children(node);
        if self.hidden.contains(&node) {
            line.push_str(&format!(" hid {}", count_below(self.tree, node)));
        } else if level == LEVEL_MAX && !children.is_empty() {
            line.push_str(&format!(" deep {}", count_below(self.tree, node)));
        } else if !children.is_empty()
            && children
                .iter()
                .all(|&c| matches!(self.tree.data(c), NodeData::Text(_)))
        {
            let mut text = String::new();
            for &child in children {
                count_node_visit();
                if let NodeData::Text(data) = self.tree.data(child)
                    && !self.hidden.contains(&child)
                {
                    text.push_str(data);
                }
            }
            let text = collapse(&text);
            if !text.is_empty() {
                line.push(' ');
                line.push_str(&quote_text_cut(&text, VALUE_CHARS_MAX));
            }
        } else {
            self.pending
                .extend(children.iter().rev().map(|&child| (child, level + 1)));
        }
        line
    }
}

/// Two spaces per level.
fn indent(level: usize) -> String {
    "  ".repeat(level)
}

/// The element's name on its line: `svg:` or `math:` before a name outside the HTML namespace,
/// and the local name, quoted when it is not of the form `[a-z][a-z0-9-]*` or is of the form of
/// a handle, `[lbf][0-9]+`.
fn name(namespace: Namespace, local_name: &str) -> String {
    let prefix = match namespace {
        Namespace::Svg => "svg:",
        Namespace::MathMl => "math:",
        _ => "",
    };
    let mut chars = local_name.chars();
    let plain = chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    let handle_form = local_name.len() > 1
        && local_name.starts_with(['l', 'b', 'f'])
        && local_name[1..].bytes().all(|b| b.is_ascii_digit());
    if plain && !handle_form {
        format!("{prefix}{local_name}")
    } else {
        format!("{prefix}{}", quote_text(local_name))
    }
}

/// The elements and the text nodes that are not whitespace only below `node`, template contents
/// left out. One walk with its own stack.
fn count_below(tree: &Tree, node: NodeId) -> usize {
    let mut count = 0;
    let mut pending: Vec<NodeId> = tree.children(node).to_vec();
    while let Some(next) = pending.pop() {
        count_node_visit();
        match tree.data(next) {
            NodeData::Element { .. } => {
                count += 1;
                pending.extend_from_slice(tree.children(next));
            }
            NodeData::Text(text) if !text.trim_ascii().is_empty() => count += 1,
            _ => {}
        }
    }
    count
}
