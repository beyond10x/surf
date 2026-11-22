//! The `selectedcontent` element's copy of the selected `option` (4.10.12, "update a select's
//! descendant selectedcontent elements"), which the tree-construction data holds in the tree a
//! parse leaves (story `html-tree-complete`, `webkit02.dat` cases 45 to 48).
//!
//! The standard runs it from the DOM's insertion steps of `option` and `selectedcontent`
//! elements, through a queued microtask. No script runs here, so no microtask checkpoint comes
//! before the end of the parse, and each queued update reads the tree as the parse left it. A
//! deliberate deviation in form, not in result: the update runs once, after the end-of-file
//! token, for each connected `select` with a descendant `selectedcontent`, in tree order. The
//! selectedness of the options is the one the selectedness setting algorithm leaves after the
//! last `option` was inserted: the last option of the list of options with a `selected`
//! attribute, else the first one that is not disabled when the display size is 1. A fragment
//! and template contents are not connected, so they get no update; neither does a parse that a
//! limit stopped or a `meta` that changes the encoding ended. Each node the pass visits or
//! creates is a step, and each node it creates is admitted against the limits.

use super::TreeBuilder;
use crate::limits::Limit;
use crate::tree::{Attribute, Namespace, NodeData, NodeId, stored_bytes_of};

impl TreeBuilder {
    /// The update of every connected `select`'s descendant `selectedcontent` elements.
    pub(super) fn update_selectedcontents(&mut self) {
        let root = self.document();
        let selects: Vec<NodeId> = self
            .tree
            .descendants(root)
            .into_iter()
            .filter(|&node| {
                self.visit();
                self.is_html(node, &["select"])
            })
            .collect();
        for select in selects {
            if self.steps.get() > self.limits.max_steps {
                self.stop_at(Limit::Steps);
                return;
            }
            if !self.update_select(select) {
                return;
            }
        }
    }

    /// Update a select's descendant selectedcontent elements; `false` when a limit stopped it.
    fn update_select(&mut self, select: NodeId) -> bool {
        if has_attribute(self.tree.data(select), "multiple") {
            return true;
        }
        let targets: Vec<NodeId> = self
            .tree
            .descendants(select)
            .into_iter()
            .filter(|&node| {
                self.visit();
                self.is_html(node, &["selectedcontent"]) && !self.selectedcontent_disabled(node)
            })
            .collect();
        if targets.is_empty() {
            return true;
        }
        let option = self.selected_option(select);
        for target in targets {
            self.tree.remove_children(target, &self.steps);
            if let Some(option) = option
                && !self.clone_children(option, target)
            {
                return false;
            }
        }
        true
    }

    /// Recalculate a selectedcontent element's disabledness: from its parent up, a second
    /// `select`, an `option` or a `selectedcontent` disables it.
    fn selectedcontent_disabled(&self, node: NodeId) -> bool {
        let mut selects = 0;
        let mut ancestor = self.tree.parent(node);
        while let Some(current) = ancestor {
            self.visit();
            if self.is_html(current, &["select"]) {
                selects += 1;
                if selects == 2 {
                    return true;
                }
            } else if self.is_html(current, &["option", "selectedcontent"]) {
                return true;
            }
            ancestor = self.tree.parent(current);
        }
        false
    }

    /// The first option of `select`'s list of options whose selectedness is true, after the
    /// selectedness setting algorithm (the module documentation says which one that is).
    fn selected_option(&self, select: NodeId) -> Option<NodeId> {
        let options = self.list_of_options(select);
        if let Some(&last) = options
            .iter()
            .rev()
            .find(|&&option| has_attribute(self.tree.data(option), "selected"))
        {
            return Some(last);
        }
        if display_size(self.tree.data(select)) != 1 {
            return None;
        }
        options
            .into_iter()
            .find(|&option| !self.option_disabled(option))
    }

    /// Get the list of options: the `option` descendants of `select`, not under an `option`,
    /// `select`, `hr`, `datalist` or nested `optgroup`.
    fn list_of_options(&self, select: NodeId) -> Vec<NodeId> {
        let mut options = Vec::new();
        let mut pending: Vec<(NodeId, bool)> = self
            .tree
            .children(select)
            .iter()
            .rev()
            .map(|&child| (child, false))
            .collect();
        while let Some((node, in_optgroup)) = pending.pop() {
            self.visit();
            if self.is_html(node, &["option"]) {
                options.push(node);
            }
            let is_optgroup = self.is_html(node, &["optgroup"]);
            if self.is_html(node, &["select", "hr", "option", "datalist"])
                || (is_optgroup && in_optgroup)
            {
                continue;
            }
            let inside = in_optgroup || is_optgroup;
            pending.extend(
                self.tree
                    .children(node)
                    .iter()
                    .rev()
                    .map(|&child| (child, inside)),
            );
        }
        options
    }

    /// Whether `option` is disabled: its own `disabled` attribute, or that of the nearest
    /// `optgroup` above it before a `select`, `hr`, `datalist` or `option`.
    fn option_disabled(&self, option: NodeId) -> bool {
        if has_attribute(self.tree.data(option), "disabled") {
            return true;
        }
        let mut ancestor = self.tree.parent(option);
        while let Some(current) = ancestor {
            self.visit();
            if self.is_html(current, &["select", "hr", "datalist", "option"]) {
                return false;
            }
            if self.is_html(current, &["optgroup"]) {
                return has_attribute(self.tree.data(current), "disabled");
            }
            ancestor = self.tree.parent(current);
        }
        false
    }

    /// Clone an option into a selectedcontent: each child of `option` cloned with its subtree
    /// and appended to `target`, leaving out every `script`, SVG `script`, `frame`, `iframe`,
    /// `embed` and `object` element with its subtree. `false` when a limit refused a node.
    fn clone_children(&mut self, option: NodeId, target: NodeId) -> bool {
        let mut pending: Vec<(NodeId, NodeId)> = self
            .tree
            .children(option)
            .iter()
            .rev()
            .map(|&child| (child, target))
            .collect();
        while let Some((source, parent)) = pending.pop() {
            self.visit();
            let data = self.tree.data(source).clone();
            let copy = match &data {
                NodeData::Element {
                    namespace,
                    local_name,
                    attributes,
                    template_contents,
                } => {
                    let removed = match namespace {
                        Namespace::Html => matches!(
                            local_name.as_str(),
                            "script" | "frame" | "iframe" | "embed" | "object"
                        ),
                        Namespace::Svg => local_name == "script",
                        _ => false,
                    };
                    if removed {
                        continue;
                    }
                    let nodes = 1 + usize::from(template_contents.is_some());
                    if !self.admit(nodes, stored_bytes_of(&data)) {
                        return false;
                    }
                    let attributes: Vec<Attribute> = attributes.clone();
                    let categories = self.tree.categories(source);
                    let element = self
                        .tree
                        .create_element(*namespace, local_name, attributes, categories);
                    if let (
                        Some(from),
                        NodeData::Element {
                            template_contents: Some(to),
                            ..
                        },
                    ) = (template_contents, self.tree.data(element))
                    {
                        let to = *to;
                        pending.extend(self.tree.children(*from).iter().rev().map(|&c| (c, to)));
                    }
                    element
                }
                _ => {
                    if !self.admit(1, stored_bytes_of(&data)) {
                        return false;
                    }
                    let node = self.tree.insert_new(parent, data, None, &self.steps);
                    pending.extend(self.tree.children(source).iter().rev().map(|&c| (c, node)));
                    continue;
                }
            };
            self.tree.insert(parent, copy, None, &self.steps);
            pending.extend(self.tree.children(source).iter().rev().map(|&c| (c, copy)));
        }
        true
    }
}

/// Whether `data` is an element with an attribute of no namespace named `name`.
fn has_attribute(data: &NodeData, name: &str) -> bool {
    matches!(data, NodeData::Element { attributes, .. }
        if attributes.iter().any(|a| a.namespace.is_none() && a.local_name == name))
}

/// The display size of a `select` without `multiple`: its `size` attribute by the rules for
/// parsing non-negative integers, else 1.
fn display_size(data: &NodeData) -> u64 {
    let NodeData::Element { attributes, .. } = data else {
        return 1;
    };
    let Some(size) = attributes
        .iter()
        .find(|a| a.namespace.is_none() && a.local_name == "size")
    else {
        return 1;
    };
    let rest = size
        .value
        .trim_start_matches(['\t', '\n', '\x0C', '\r', ' '])
        .strip_prefix('+')
        .unwrap_or_else(|| {
            size.value
                .trim_start_matches(['\t', '\n', '\x0C', '\r', ' '])
        });
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return 1;
    }
    digits.parse().unwrap_or(u64::MAX)
}
