//! The adoption agency algorithm (13.2.6.4.7, "in body", after the rules of that mode), which
//! repairs misnested formatting elements. Written by `story:html-tree-formatting`; the comments
//! carry the step numbers of the snapshot `docs/design/html-parser.md` names.
//!
//! The parser has no parser sanitizer configuration, its fragment parser abort is never
//! requested and its insertion target redirection map is empty, so the steps that read them
//! (4.12, 4.13, 4.18.8 to 4.18.10, 4.21, 4.27, 4.28, 4.31 and 4.32) change nothing: every
//! sanitizer action is "Keep", and an adjusted insertion location is the location it is given.
//! Foster parenting is not written (story `html-tree-tables`): the appropriate place has no
//! reference child yet, and step 4.14 and step 4.19 are where it will give one.

use super::{Entry, Scope, TagToken, TreeBuilder};
use crate::tree::{NodeData, NodeId};

/// Step 4.1: the outer loop runs at most this many times.
const OUTER_LOOP_LIMIT: usize = 8;

/// Step 4.18.4: a node the inner loop meets after this many iterations leaves the list of
/// active formatting elements, so each outer iteration clones at most this many elements.
const INNER_LOOP_CLONES: usize = 3;

impl TreeBuilder {
    /// The last element of the list of active formatting elements after the last marker (or in
    /// the whole list when there is none) whose tag name is `subject`: its index in the list,
    /// the element and the token it was created for.
    pub(super) fn last_active_named(&self, subject: &str) -> Option<(usize, NodeId, TagToken)> {
        for (index, entry) in self.active.iter().enumerate().rev() {
            match entry {
                Entry::Marker => return None,
                Entry::Element { node, token } if self.html_name(*node) == Some(subject) => {
                    return Some((index, *node, token.clone()));
                }
                Entry::Element { .. } => {}
            }
        }
        None
    }

    /// The adoption agency algorithm, run for a token whose tag name is `subject`.
    pub(super) fn adoption_agency(&mut self, subject: &str) {
        // 1. `subject` is the token's tag name.
        // 2. The current node is an HTML element named `subject` that is not in the list.
        let current = self.current_node();
        if self.html_name(current) == Some(subject) && self.active_position(current).is_none() {
            self.open.pop();
            return;
        }
        // 3.
        let mut outer_loop_counter = 0;
        // 4.
        loop {
            // 4.1.
            if outer_loop_counter >= OUTER_LOOP_LIMIT {
                return;
            }
            // 4.2.
            outer_loop_counter += 1;
            // 4.3. The formatting element; with none, "any other end tag".
            let Some((formatting_index, formatting_element, formatting_token)) =
                self.last_active_named(subject)
            else {
                self.any_other_end_tag(subject);
                return;
            };
            // 4.4. Not in the stack of open elements: a parse error, and the entry goes.
            let Some(formatting_stack) = self.open.iter().rposition(|&n| n == formatting_element)
            else {
                self.active.remove(formatting_index);
                return;
            };
            // 4.5. In the stack but not in scope: a parse error.
            if !self.has_in_scope(Scope::Default, |_, n| n == formatting_element) {
                return;
            }
            // 4.6. Not the current node: a parse error, and the algorithm goes on.
            // 4.7. The furthest block: the topmost special element below the formatting element.
            let furthest_block_stack = (formatting_stack + 1..self.open.len())
                .find(|&index| self.is_special(self.open[index]));
            // 4.8. No furthest block: pop down to the formatting element, and the entry goes.
            let Some(furthest_block_stack) = furthest_block_stack else {
                self.open.truncate(formatting_stack);
                self.active.remove(formatting_index);
                return;
            };
            let furthest_block = self.open[furthest_block_stack];
            // 4.9. The common ancestor: the element above the formatting element.
            let common_ancestor = self.open[formatting_stack - 1];
            // 4.10. The bookmark: the index in the list at which the new element goes once the
            // formatting element's entry is removed (step 4.25).
            let mut bookmark = formatting_index;
            // 4.11. `node` is followed by its index in the stack of open elements.
            let mut node_stack = furthest_block_stack;
            let mut last_node = Some(furthest_block);
            // 4.12, 4.13. No pending redirects, no sanitizer.
            // 4.14.
            let (initial_target, initial_reference) = self.appropriate_place(Some(common_ancestor));
            // 4.15.
            if !self.is_child_or_none(initial_reference, initial_target) {
                last_node = None;
            }
            // 4.16.
            let mut inner_loop_counter = 0;
            // 4.17.
            let mut bookmark_moved = false;
            // 4.18. The inner loop.
            loop {
                // 4.18.1.
                inner_loop_counter += 1;
                // 4.18.2. The element above `node`, also when `node` was removed from the stack.
                node_stack -= 1;
                let node = self.open[node_stack];
                // 4.18.3.
                if node == formatting_element {
                    break;
                }
                // 4.18.4. After the third iteration a node leaves the list.
                let mut node_entry = self.active_position(node);
                if inner_loop_counter > INNER_LOOP_CLONES
                    && let Some(at) = node_entry
                {
                    self.active.remove(at);
                    if at < bookmark {
                        bookmark -= 1;
                    }
                    node_entry = None;
                }
                // 4.18.5. A node not in the list leaves the stack.
                let Some(at) = node_entry else {
                    self.open.remove(node_stack);
                    continue;
                };
                // 4.18.6. A new element for `node`'s token takes `node`'s place in the list and
                // in the stack. Finishing the parsing of `node`'s children changes no tree.
                let token = match &self.active[at] {
                    Entry::Element { token, .. } => token.clone(),
                    Entry::Marker => unreachable!("`active_position` finds an element"),
                };
                let new = self.create_element(&token.name, &token.attributes);
                self.active[at] = Entry::Element { node: new, token };
                self.open[node_stack] = new;
                let node = new;
                // 4.18.7. The bookmark goes after the first new element.
                if !bookmark_moved {
                    bookmark = at + 1;
                    bookmark_moved = true;
                }
                // 4.18.8 to 4.18.10. The action is "Keep".
                // 4.18.11. "Keep": append `lastNode` to `node`.
                if let Some(last) = last_node {
                    self.tree.remove(last);
                    self.tree.insert(node, last, None);
                }
                last_node = Some(node);
            }
            // 4.19, 4.20. Where `lastNode` goes: the appropriate place given the common ancestor.
            let (target, reference) = self.appropriate_place(Some(common_ancestor));
            // 4.21. No pending redirects.
            // 4.22. `lastNode` is taken out of its parent.
            if let Some(last) = last_node
                && self.tree.parent(last).is_some()
                && self.is_child_or_none(reference, target)
            {
                self.tree.remove(last);
            }
            // 4.23. And inserted at that place.
            if let Some(last) = last_node
                && self.tree.parent(last).is_none()
                && !self.is_host_including_inclusive_ancestor(last, target)
                && !(*self.tree.data(target) == NodeData::Document
                    && self.tree.has_element_child(target))
                && self.is_child_or_none(reference, target)
            {
                self.tree.insert(target, last, reference);
            }
            // 4.24. A new element for the formatting element's token.
            let new_element =
                self.create_element(&formatting_token.name, &formatting_token.attributes);
            // 4.25. It takes the formatting element's entry, at the bookmark.
            let formatting_at = self
                .active_position(formatting_element)
                .expect("the formatting element is in the list");
            self.active.remove(formatting_at);
            if formatting_at < bookmark {
                bookmark -= 1;
            }
            self.active.insert(
                bookmark,
                Entry::Element {
                    node: new_element,
                    token: formatting_token,
                },
            );
            // 4.26. It goes on the stack just below the furthest block.
            self.remove_from_stack(formatting_element);
            let furthest_block_stack = self
                .open
                .iter()
                .rposition(|&n| n == furthest_block)
                .expect("the furthest block is on the stack");
            self.open.insert(furthest_block_stack + 1, new_element);
            // 4.27, 4.28. The action is "Keep".
            // 4.29. The adjusted insertion location given (furthest block, null).
            let (parent, reference) = (furthest_block, None);
            // 4.30. "Keep": the furthest block's children move to the new element, which
            // becomes the furthest block's last child.
            if self.is_child_or_none(reference, parent)
                && !(*self.tree.data(parent) == NodeData::Document
                    && self.tree.has_element_child(parent))
            {
                self.tree.move_children(furthest_block, new_element);
                self.tree.insert(parent, new_element, reference);
            }
            // 4.31, 4.32. No redirection to update.
        }
    }

    /// Whether `reference` is null or a child of `target` (steps 4.15, 4.22, 4.23 and 4.30).
    fn is_child_or_none(&self, reference: Option<NodeId>, target: NodeId) -> bool {
        reference.is_none_or(|r| self.tree.parent(r) == Some(target))
    }

    /// Whether `ancestor` is a host-including inclusive ancestor of `node` (DOM): `node`'s
    /// parents are followed up, and from a template's contents to the template. The arena keeps
    /// no link from a template's contents to the template, so the template is looked for on the
    /// stack of open elements.
    fn is_host_including_inclusive_ancestor(&self, ancestor: NodeId, node: NodeId) -> bool {
        let mut next = Some(node);
        while let Some(at) = next {
            if at == ancestor {
                return true;
            }
            next = self.tree.parent(at).or_else(|| self.template_host(at));
        }
        false
    }

    /// The template element on the stack of open elements whose template contents is `fragment`.
    fn template_host(&self, fragment: NodeId) -> Option<NodeId> {
        self.open.iter().copied().find(|&n| {
            matches!(
                self.tree.data(n),
                NodeData::Element { template_contents: Some(contents), .. } if *contents == fragment
            )
        })
    }
}
