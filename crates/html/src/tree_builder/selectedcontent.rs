//! The `selectedcontent` element's copy of the selected `option` (4.10.12, "update a select's
//! descendant selectedcontent elements"), which the tree-construction data holds in the tree a
//! parse leaves (story `html-tree-complete`, `webkit02.dat` cases 45 to 48).
//!
//! The standard runs it from the DOM's insertion steps of `option` and `selectedcontent`
//! elements, through a queued microtask. No script runs here, so no microtask checkpoint comes
//! before the end of the parse, and each queued update reads the tree as the parse left it. A
//! deliberate deviation in form, not in result: the update runs once, after the end-of-file
//! token, for each connected `select` with a descendant `selectedcontent`, in tree order (the
//! design note's `open` line `selectedcontent-update-timing`). The selectedness of the options
//! is the one the selectedness setting algorithm leaves after the last `option` was inserted:
//! the last option of the list of options with a `selected` attribute, else the first one that
//! is not disabled when the display size is 1. A fragment and template contents are not
//! connected, so they get no update; neither does a parse that a limit stopped or a `meta` that
//! changes the encoding ended.
//!
//! The cost: one walk of the document, top down, decides for every `selectedcontent` whether it
//! is disabled and which `select` it belongs to, so no element walks up to the root; one walk
//! of each such `select`'s list of options decides each option's disabledness the same way;
//! then each copy costs the nodes it visits and makes. Each node a walk visits, each node the
//! copy visits or creates and each child a `selectedcontent` loses is a step; the step limit is
//! checked after each of them, and each node the copy creates is admitted against the node and
//! stored-byte limits. A limit reached stops the pass where it is, with that limit named.

use super::TreeBuilder;
use crate::limits::Limit;
use crate::tree::{Attribute, Namespace, NodeData, NodeId, stored_bytes_of};

/// What the top-down walk knows of an element's ancestors: the index in the walk's list of the
/// nearest `select` above it, and whether a `selectedcontent` below it is disabled.
#[derive(Clone, Copy)]
struct Above {
    select: Option<usize>,
    disabled: bool,
}

impl TreeBuilder {
    /// Whether the steps have passed the step limit; then parsing stops at it.
    fn past_step_limit(&mut self) -> bool {
        if self.steps.get() > self.limits.max_steps {
            self.stop_at(Limit::Steps);
            return true;
        }
        false
    }

    /// The update of every connected `select`'s descendant `selectedcontent` elements.
    pub(super) fn update_selectedcontents(&mut self) {
        let Some(selects) = self.selects_and_their_selectedcontents() else {
            return;
        };
        for (select, targets) in selects {
            if !targets.is_empty() && !self.update_select(select, &targets) {
                return;
            }
        }
        self.past_step_limit();
    }

    /// One walk of the document in tree order: each `select` without `multiple`, with the
    /// `selectedcontent` elements below it that are not disabled ("recalculate a selectedcontent
    /// element's disabledness": a second `select`, an `option` or a `selectedcontent` among the
    /// ancestors disables one; the nearest `select` is the one it belongs to). Template contents
    /// are not connected and are not walked. `None` when the step limit stopped it.
    fn selects_and_their_selectedcontents(&mut self) -> Option<Vec<(NodeId, Vec<NodeId>)>> {
        let mut selects: Vec<(NodeId, Vec<NodeId>)> = Vec::new();
        let start = Above {
            select: None,
            disabled: false,
        };
        let mut pending: Vec<(NodeId, Above)> = vec![(self.document(), start)];
        while let Some((node, above)) = pending.pop() {
            self.visit();
            if self.past_step_limit() {
                return None;
            }
            let mut below = above;
            if self.is_html(node, &["selectedcontent"]) {
                if let (false, Some(index)) = (above.disabled, above.select) {
                    selects[index].1.push(node);
                }
                below.disabled = true;
            } else if self.is_html(node, &["option"]) {
                below.disabled = true;
            } else if self.is_html(node, &["select"]) {
                if above.select.is_some() || has_attribute(self.tree.data(node), "multiple") {
                    // A second `select` disables what is below it; a `select` with `multiple`
                    // updates nothing ("if select has the multiple attribute, then return").
                    below.disabled = true;
                } else {
                    selects.push((node, Vec::new()));
                    below.select = Some(selects.len() - 1);
                }
            }
            pending.extend(
                self.tree
                    .children(node)
                    .iter()
                    .rev()
                    .map(|&child| (child, below)),
            );
        }
        Some(selects)
    }

    /// Update a select's descendant selectedcontent elements, `targets` being those that are
    /// not disabled; `false` when a limit stopped it.
    fn update_select(&mut self, select: NodeId, targets: &[NodeId]) -> bool {
        let Some(option) = self.selected_option(select) else {
            return false;
        };
        for &target in targets {
            self.tree.remove_children(target, &self.steps);
            if self.past_step_limit() {
                return false;
            }
            if let Some(option) = option
                && !self.clone_children(option, target)
            {
                return false;
            }
        }
        true
    }

    /// The first option of `select`'s list of options whose selectedness is true, after the
    /// selectedness setting algorithm (the module documentation says which one that is); `None`
    /// when the step limit stopped the walk.
    fn selected_option(&mut self, select: NodeId) -> Option<Option<NodeId>> {
        let options = self.list_of_options(select)?;
        if let Some(&(last, _)) = options
            .iter()
            .rev()
            .find(|&&(option, _)| has_attribute(self.tree.data(option), "selected"))
        {
            return Some(Some(last));
        }
        if display_size(self.tree.data(select)) != 1 {
            return Some(None);
        }
        Some(
            options
                .into_iter()
                .find(|&(_, disabled)| !disabled)
                .map(|(option, _)| option),
        )
    }

    /// Get the list of options: the `option` descendants of `select`, not under an `option`,
    /// `select`, `hr`, `datalist` or nested `optgroup`, each with whether it is disabled (its
    /// own `disabled` attribute, or that of the `optgroup` above it: the walk reaches an option
    /// through at most one `optgroup` and through no `select`, `hr`, `datalist` or `option`).
    /// `None` when the step limit stopped the walk.
    fn list_of_options(&mut self, select: NodeId) -> Option<Vec<(NodeId, bool)>> {
        let mut options = Vec::new();
        // The node, and the `optgroup` above it: `None`, or whether it has `disabled`.
        let mut pending: Vec<(NodeId, Option<bool>)> = self
            .tree
            .children(select)
            .iter()
            .rev()
            .map(|&child| (child, None))
            .collect();
        while let Some((node, optgroup)) = pending.pop() {
            self.visit();
            if self.past_step_limit() {
                return None;
            }
            if self.is_html(node, &["option"]) {
                let disabled =
                    has_attribute(self.tree.data(node), "disabled") || optgroup == Some(true);
                options.push((node, disabled));
            }
            let is_optgroup = self.is_html(node, &["optgroup"]);
            if self.is_html(node, &["select", "hr", "option", "datalist"])
                || (is_optgroup && optgroup.is_some())
            {
                continue;
            }
            let inside = if is_optgroup {
                Some(has_attribute(self.tree.data(node), "disabled"))
            } else {
                optgroup
            };
            pending.extend(
                self.tree
                    .children(node)
                    .iter()
                    .rev()
                    .map(|&child| (child, inside)),
            );
        }
        Some(options)
    }

    /// Clone an option into a selectedcontent: each child of `option` cloned with its subtree
    /// and appended to `target`, leaving out every `script`, SVG `script`, `frame`, `iframe`,
    /// `embed` and `object` element with its subtree. `false` when a limit stopped it: the
    /// step limit, checked after each node visited, the depth limit, checked before each
    /// element is made against the depth of the copy it goes under, or the node or stored-byte
    /// limit, which admits each node before it is made. A copy goes under its parent before
    /// anything goes under it, so the tree holds a copy cut where the limit stopped it.
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
            if self.past_step_limit() {
                return false;
            }
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
                    if self.tree.depth(parent) >= self.limits.max_depth {
                        self.stop_at(Limit::Depth);
                        return false;
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
/// parsing non-negative integers (2.3.4.1, 2.3.4.2), else 1. Those rules read the sign: `-0`
/// is zero, and any other negative value is an error, which gives the default 1. A value past
/// `u64::MAX` is read as `u64::MAX`: it is not 1 either way.
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
    let value = size
        .value
        .trim_start_matches(['\t', '\n', '\x0C', '\r', ' ']);
    let (negative, rest) = match value.as_bytes().first() {
        Some(b'-') => (true, &value[1..]),
        Some(b'+') => (false, &value[1..]),
        _ => (false, value),
    };
    let digits = &rest[..rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len())];
    if digits.is_empty() {
        return 1;
    }
    let zero = digits.bytes().all(|b| b == b'0');
    if negative && !zero {
        return 1;
    }
    digits.parse().unwrap_or(u64::MAX)
}
