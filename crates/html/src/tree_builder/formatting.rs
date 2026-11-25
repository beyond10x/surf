//! The adoption agency algorithm (13.2.6.4.7, "in body", after the rules of that mode), which
//! repairs misnested formatting elements. Written by `story:html-tree-formatting`; the comments
//! carry the step numbers of the snapshot `docs/design/html-parser.md` names.
//!
//! The parser has no parser sanitizer configuration, its fragment parser abort is never
//! requested and its insertion target redirection map is empty, so the steps that read them
//! (4.12, 4.13, 4.18.8 to 4.18.10, 4.21, 4.27, 4.28, 4.31 and 4.32) change nothing: every
//! sanitizer action is "Keep", and an adjusted insertion location is the location it is given.
//! Foster parenting (story `html-tree-tables`) reaches the algorithm at steps 4.14 and 4.19: the
//! appropriate place given the common ancestor gives a reference child, the table, when the
//! algorithm runs from the "anything else" rule of in table and the common ancestor is a table
//! element.
//!
//! The limits (story `html-parser-limits`): an outer iteration that creates elements has them
//! admitted by the node and stored-byte limits all at once, after step 4.8 and before step 4.9,
//! when nothing of the iteration has changed the tree, the stack or the list yet. So an
//! iteration runs whole or not at all, and a limit never leaves a clone out of its parent. Every
//! entry the scans visit, and every node of the walk to the root, is a step.

use super::{Entry, Scope, TreeBuilder};
use crate::limits::Limit;
use crate::tree::{NodeData, NodeId};

/// Step 4.1: the outer loop runs at most this many times.
const OUTER_LOOP_LIMIT: usize = 8;

/// Step 4.18.4: a node the inner loop meets after this many iterations leaves the list of
/// active formatting elements, so each outer iteration clones at most this many elements.
const INNER_LOOP_CLONES: usize = 3;

impl TreeBuilder {
    /// The last element of the list of active formatting elements after the last marker (or in
    /// the whole list when there is none) whose tag name is `subject`: its index in the list and
    /// the element. The token stays in the entry: a formatting end tag that the algorithm
    /// ignores at step 4.4 or 4.5 costs its steps and copies nothing.
    pub(super) fn last_active_named(&self, subject: &str) -> Option<(usize, NodeId)> {
        for (index, entry) in self.active.iter().enumerate().rev() {
            self.visit();
            match entry {
                Entry::Marker => return None,
                Entry::Element { node, .. } if self.html_name(*node) == Some(subject) => {
                    return Some((index, *node));
                }
                Entry::Element { .. } => {}
            }
        }
        None
    }

    /// The adoption agency algorithm, run for a token whose tag name is `subject`.
    pub(super) fn adoption_agency(&mut self, subject: &str) {
        if self.stopped {
            return;
        }
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
            let Some((formatting_index, formatting_element)) = self.last_active_named(subject)
            else {
                self.any_other_end_tag(subject);
                return;
            };
            // 4.4. Not in the stack of open elements: a parse error, and the entry goes.
            let Some(formatting_stack) = self.stack_position(formatting_element) else {
                self.active.remove(formatting_index);
                return;
            };
            // 4.5. In the stack but not in scope: a parse error.
            if !self.has_in_scope(Scope::Default, |_, n| n == formatting_element) {
                return;
            }
            // 4.6. Not the current node: a parse error, and the algorithm goes on.
            // 4.7. The furthest block: the topmost special element below the formatting element.
            let furthest_block_stack = (formatting_stack + 1..self.open.len()).find(|&index| {
                self.visit();
                self.is_special(self.open[index])
            });
            // 4.8. No furthest block: pop down to the formatting element, and the entry goes.
            let Some(furthest_block_stack) = furthest_block_stack else {
                self.open.truncate(formatting_stack);
                self.active.remove(formatting_index);
                return;
            };
            // The elements steps 4.18.6 and 4.24 create, admitted before the iteration changes
            // anything: with a limit that refuses them, parsing stops here.
            let (nodes, bytes) =
                self.iteration_creates(formatting_index, formatting_stack, furthest_block_stack);
            if !self.admit(nodes, bytes) {
                return;
            }
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
            // 4.15. The place's reference child, the `table` of foster parenting, is a child of
            // its target (`appropriate_place`), so `lastNode` stays the furthest block.
            if !self.is_child_or_none(initial_reference, initial_target) {
                last_node = None;
            }
            debug_assert!(last_node.is_some());
            // Whether step 4.23 will refuse to insert `lastNode` because it would be its own
            // ancestor: the furthest block is above the place. The inner loop moves only the
            // furthest block, with what is under it, so the answer is the same there; the walk
            // to the root is made once, each node a step.
            let block_holds_target =
                self.is_host_including_inclusive_ancestor(furthest_block, initial_target);
            // The depth limit, for the whole iteration, before anything moves. The inner loop
            // removes from the stack only elements between the formatting element and the
            // furthest block, none of them a `table` or a `template` (they are not special), so
            // the place of step 4.19 is the place of step 4.14.
            let inserted =
                !block_holds_target && *self.tree.data(initial_target) != NodeData::Document;
            if !self.adoption_fits(
                furthest_block,
                (initial_target, initial_reference),
                inserted,
                nodes - 1,
            ) {
                return;
            }
            // 4.16.
            let mut inner_loop_counter = 0;
            // 4.17.
            let mut bookmark_moved = false;
            // The nodes step 4.18.5 removes, all of them above the formatting element.
            let mut removed_from_stack = 0;
            // 4.18. The inner loop.
            loop {
                self.visit();
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
                    self.open.remove(node_stack, &self.steps);
                    removed_from_stack += 1;
                    continue;
                };
                // 4.18.6. A new element for `node`'s token takes `node`'s place in the list and
                // in the stack. Finishing the parsing of `node`'s children changes no tree. The
                // entry's token is taken out and put back, not copied.
                let Entry::Element { token, .. } =
                    std::mem::replace(&mut self.active[at], Entry::Marker)
                else {
                    unreachable!("`active_position` finds an element");
                };
                let new = self.create_admitted_element(&token.name, &token.attributes);
                self.active[at] = Entry::Element { node: new, token };
                self.open.replace(node_stack, new);
                let node = new;
                // 4.18.7. The bookmark goes after the first new element.
                if !bookmark_moved {
                    bookmark = at + 1;
                    bookmark_moved = true;
                }
                // 4.18.8 to 4.18.10. The action is "Keep".
                // 4.18.11. "Keep": append `lastNode` to `node`.
                if let Some(last) = last_node {
                    self.tree.remove(last, &self.steps);
                    self.tree.insert(node, last, None, &self.steps);
                }
                last_node = Some(node);
            }
            // 4.19, 4.20. Where `lastNode` goes: the appropriate place given the common ancestor.
            let (target, reference) = self.appropriate_place(Some(common_ancestor));
            debug_assert_eq!((target, reference), (initial_target, initial_reference));
            // 4.21. No pending redirects.
            // 4.22. `lastNode` is taken out of its parent.
            if let Some(last) = last_node
                && self.tree.parent(last).is_some()
                && self.is_child_or_none(reference, target)
            {
                self.tree.remove(last, &self.steps);
            }
            // 4.23. And inserted at that place, unless it is above the place: `lastNode` is the
            // furthest block or a clone above it, so it is exactly when the furthest block is.
            if let Some(last) = last_node
                && self.tree.parent(last).is_none()
                && !block_holds_target
                && !(*self.tree.data(target) == NodeData::Document
                    && self.tree.has_element_child(target, &self.steps))
                && self.is_child_or_none(reference, target)
            {
                self.tree.insert(target, last, reference, &self.steps);
            }
            // 4.24. A new element for the formatting element's token, taken out of its entry,
            // which step 4.25 removes.
            let formatting_at = self
                .active_position(formatting_element)
                .expect("the formatting element is in the list");
            let Entry::Element {
                token: formatting_token,
                ..
            } = self.active.remove(formatting_at)
            else {
                unreachable!("`active_position` finds an element");
            };
            let new_element =
                self.create_admitted_element(&formatting_token.name, &formatting_token.attributes);
            // 4.25. It takes the formatting element's entry, at the bookmark.
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
            // 4.26. It goes on the stack just below the furthest block. No scan finds either: the
            // inner loop removed only nodes above the formatting element, so it is still at
            // `formatting_stack`, and the furthest block is lower by those nodes and by the
            // formatting element. Finishing the parsing of its children changes no tree.
            debug_assert_eq!(self.open[formatting_stack], formatting_element);
            self.open.remove(formatting_stack, &self.steps);
            let furthest_block_stack = furthest_block_stack - removed_from_stack - 1;
            debug_assert_eq!(self.open[furthest_block_stack], furthest_block);
            self.open
                .insert(furthest_block_stack + 1, new_element, &self.steps);
            // 4.27, 4.28. The action is "Keep".
            // 4.29. The adjusted insertion location given (furthest block, null).
            let (parent, reference) = (furthest_block, None);
            // 4.30. "Keep": the furthest block's children move to the new element, which
            // becomes the furthest block's last child.
            if self.is_child_or_none(reference, parent)
                && !(*self.tree.data(parent) == NodeData::Document
                    && self.tree.has_element_child(parent, &self.steps))
            {
                self.tree
                    .move_children(furthest_block, new_element, &self.steps);
                self.tree
                    .insert(parent, new_element, reference, &self.steps);
            }
            // 4.31, 4.32. No redirection to update.
            // The depths of the entries the iteration moved or made: on an ordered stack those
            // from the formatting element's place up (the clones, the furthest block, the new
            // element and the entries under the furthest block), else all.
            self.settle_entries(if self.ordered { formatting_stack } else { 0 });
        }
    }

    /// Whether the moves of an outer iteration keep the tree inside the depth limit, checked
    /// before the inner loop changes anything; when they do not, parsing stops there and the
    /// iteration makes no change. The iteration puts `clones` new elements in a chain, the
    /// furthest block under the last, the new formatting element under the furthest block and
    /// the furthest block's children under that. Step 4.23 puts the chain at `place` when it
    /// is `inserted`; else the chain stays out of the tree, counted from its own top. The new
    /// formatting element is the deepest new place; the furthest block's children, and every
    /// node under them, move down when it is deeper than the furthest block was. Only then are
    /// they walked for the deepest of them, each node a step, and the check made: a move that
    /// makes no node deeper keeps every node inside the limit, and walks nothing.
    fn adoption_fits(
        &mut self,
        furthest_block: NodeId,
        place: (NodeId, Option<NodeId>),
        inserted: bool,
        clones: usize,
    ) -> bool {
        let old = self
            .open
            .depth(furthest_block)
            .expect("the furthest block is an entry with a settled depth");
        let top = if inserted { self.place_depth(place) } else { 0 };
        let new_element = top + clones + 2;
        if new_element <= old {
            return true;
        }
        // The furthest block is no `template`: a `template` start tag puts a marker on the
        // list, so a formatting element above an open `template` is never the one step 4.3
        // finds. So nothing hangs under it but its children.
        debug_assert!(self.tree.contents(furthest_block).is_none());
        let below = self.tree.height_below(furthest_block, &self.steps);
        let fits = new_element + below <= self.limits.max_depth;
        if !fits {
            self.stop_at(Limit::Depth);
        }
        fits
    }

    /// The nodes and the stored bytes of the elements an outer iteration creates, given the
    /// formatting element's index in the list and its and the furthest block's indices in the
    /// stack of open elements: one clone (step 4.18.6) for each of the first
    /// [`INNER_LOOP_CLONES`] elements the inner loop meets above the furthest block, below the
    /// formatting element, that has an entry in the list, and the new formatting element (step
    /// 4.24). The inner loop meets the elements above the furthest block in order, also when it
    /// removes one from the stack (step 4.18.5), and no earlier iteration of it changes which of
    /// them have an entry.
    fn iteration_creates(
        &self,
        formatting_index: usize,
        formatting_stack: usize,
        furthest_block_stack: usize,
    ) -> (usize, usize) {
        let mut nodes = 1;
        let mut bytes = match &self.active[formatting_index] {
            Entry::Element { token, .. } => token.stored_bytes(),
            Entry::Marker => unreachable!("`last_active_named` finds an element"),
        };
        for above in 1..=INNER_LOOP_CLONES {
            let index = furthest_block_stack - above;
            if index <= formatting_stack {
                break;
            }
            if let Some(at) = self.active_position(self.open[index])
                && let Entry::Element { token, .. } = &self.active[at]
            {
                nodes += 1;
                bytes += token.stored_bytes();
            }
        }
        (nodes, bytes)
    }

    /// Whether `reference` is null or a child of `target` (steps 4.15, 4.22, 4.23 and 4.30).
    fn is_child_or_none(&self, reference: Option<NodeId>, target: NodeId) -> bool {
        reference.is_none_or(|r| self.tree.parent(r) == Some(target))
    }

    /// Whether `ancestor` is a host-including inclusive ancestor of `node` (DOM): `node`'s
    /// parents are followed up to its root, and from a root that has a host to that host. Only
    /// a document fragment has a host, and here only a template's contents is one; a Document
    /// or an element without a parent has none, and the walk ends there. The tree finds a
    /// template's host, also a template a hook took off the stack of open elements. The walk
    /// takes at most one step per node; a longer one is a cycle, answered as true, so that the
    /// insertion is not made.
    fn is_host_including_inclusive_ancestor(&self, ancestor: NodeId, node: NodeId) -> bool {
        let mut next = Some(node);
        for _ in 0..=self.tree.node_count() {
            let Some(at) = next else {
                return false;
            };
            self.visit();
            if at == ancestor {
                return true;
            }
            next = self.tree.parent_or_host(at);
        }
        true
    }
}
