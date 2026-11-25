//! The slice `reading` (`story:scripts-dom-reading`): the members that read the document's
//! tree and never write it, as the DOM Standard (the HTML Standard for `Window` and
//! `Document.title`) says, with the deviations `crates/webidl/selection.txt` names.
//!
//! Steps. The bindings charge one step per call. Every read of the tree is a function of
//! [`super::reads`], and each charges before the work it pays for: a node it walks one step,
//! attributes it passes by name one step per 4, a byte it reads, compares, folds, collapses or
//! copies one step per 16 (the one price of a byte, the glue's for a String it copies in or
//! out), and a piece a value splits into (a class token, a word of a title) one step. Where the
//! count is not known before the walk (a scan that stops at its first match), the step goes
//! before each node it reads. So a step buys a bounded piece of work: one node, four name
//! compares, sixteen bytes, one token; a script over a large tree ends at its step limit and
//! not at the wall clock. The walks keep their place by a stack of one entry a level, so a
//! deep tree does not deepen the call stack, and copy no child list.
//!
//! Strings. A String a member answers with more UTF-16 code units than the agent's
//! `string_length` is the realm's `RangeError`; the bindings check every answer before the
//! engine copies it. The members here do not build what would be thrown away: `data`,
//! `nodeValue` and a text's `textContent` charge the data's bytes, then check it before they
//! copy it, and the members that concatenate (`textContent`, `wholeText`, `title`) stop at the
//! piece that would take the String past the limit, with the steps of what they read charged.
//!
//! Siblings. The tree has no sibling read: a sibling is found by the node's index among its
//! parent's children. [`SurfReading`] keeps two such indices (the two last answered, so a parent
//! read from both ends keeps one for each end) for the last eight parents it answered
//! about ([`Hints`]). A hint is never trusted: the child at the hinted index, the one before it
//! and the one after it are compared with the node, and a miss scans the parent's children at
//! one step per node. So a walk over the children costs one step per sibling read, and a write
//! between two reads costs a scan, never a wrong answer.
//!
//! Reads. Every read of the tree is a function of [`super::reads`], which charges for it; the
//! members here get nodes, text and attribute values from there and read nothing else.

use surf_html::NodeId;

use crate::bindings::{DomCx, DomResult, GetRootNodeOptions, Reading, TheWindow};

use super::DocumentInfo;
use super::reads::{self, DoctypeField};

/// How many parents the sibling hints remember.
const HINTS: usize = 8;

/// Two positions among a parent's children, the two last a sibling read answered about, so a
/// parent read from both ends keeps a hint for each end; the oldest parent is replaced first.
#[derive(Debug, Default)]
pub(super) struct Hints {
    entries: [Option<(NodeId, [usize; 2])>; HINTS],
    next: usize,
}

impl Hints {
    /// The two hinted positions for `parent`, if any.
    pub(super) fn get(&self, parent: NodeId) -> Option<[usize; 2]> {
        self.entries
            .iter()
            .flatten()
            .find(|(hinted, _)| *hinted == parent)
            .map(|&(_, positions)| positions)
    }

    /// Remembers `index` for `parent`: in place of the nearer of its two positions when
    /// `parent` has an entry (the read went on from there), else as both positions of a new
    /// entry in place of the oldest.
    pub(super) fn set(&mut self, parent: NodeId, index: usize) {
        for (hinted, at) in self.entries.iter_mut().flatten() {
            if *hinted == parent {
                let nearer = usize::from(at[1].abs_diff(index) < at[0].abs_diff(index));
                at[nearer] = index;
                return;
            }
        }
        self.entries[self.next] = Some((parent, [index, index]));
        self.next = (self.next + 1) % HINTS;
    }
}

/// surf's members of `reading`.
pub struct SurfReading {
    info: DocumentInfo,
    hints: Hints,
}

impl SurfReading {
    /// The members of a document with `info`.
    pub fn new(info: DocumentInfo) -> SurfReading {
        SurfReading {
            info,
            hints: Hints::default(),
        }
    }
}

impl Reading for SurfReading {
    fn node_node_type(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<u16> {
        Ok(reads::node_type(cx, this))
    }

    fn node_node_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        reads::node_name(cx, this)
    }

    fn node_base_uri(&mut self, cx: &mut DomCx<'_, '_>, _this: NodeId) -> DomResult<String> {
        reads::base_url(cx, &self.info.address)
    }

    fn node_is_connected(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<bool> {
        let root = reads::root_of(cx, this)?;
        Ok(reads::is_document(cx, root))
    }

    fn node_owner_document(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        Ok(reads::owner_document(cx, this))
    }

    fn node_get_root_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        _options: GetRootNodeOptions,
    ) -> DomResult<NodeId> {
        // No shadow trees: the shadow-including root is the root, whatever `composed` is.
        reads::root_of(cx, this)
    }

    fn node_parent_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        Ok(reads::parent(cx, this))
    }

    fn node_parent_element(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        Ok(reads::parent_element(cx, this))
    }

    fn node_has_child_nodes(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<bool> {
        Ok(reads::has_children(cx, this))
    }

    fn node_child_nodes(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<Vec<NodeId>> {
        reads::children(cx, this)
    }

    fn node_first_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        Ok(reads::first_child(cx, this))
    }

    fn node_last_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        Ok(reads::last_child(cx, this))
    }

    fn node_previous_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::sibling(cx, &mut self.hints, this, false, false)
    }

    fn node_next_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::sibling(cx, &mut self.hints, this, true, false)
    }

    fn node_node_value(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<String>> {
        reads::copied(cx, this)
    }

    fn node_text_content(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<String>> {
        reads::text_content(cx, this)
    }

    fn node_is_equal_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        other_node: Option<NodeId>,
    ) -> DomResult<bool> {
        let Some(other) = other_node else {
            return Ok(false);
        };
        reads::is_equal(cx, this, other)
    }

    fn node_is_same_node(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        this: NodeId,
        other_node: Option<NodeId>,
    ) -> DomResult<bool> {
        Ok(other_node == Some(this))
    }

    fn node_compare_document_position(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        other: NodeId,
    ) -> DomResult<u16> {
        const DISCONNECTED: u16 = 1;
        const PRECEDING: u16 = 2;
        const FOLLOWING: u16 = 4;
        const CONTAINS: u16 = 8;
        const CONTAINED_BY: u16 = 16;
        const IMPLEMENTATION_SPECIFIC: u16 = 32;
        if this == other {
            return Ok(0);
        }
        let mut mine = reads::ancestors(cx, this)?;
        let mut theirs = reads::ancestors(cx, other)?;
        if mine.last() != theirs.last() {
            // Different trees: an order of the implementation's, the same each time.
            let order = if other < this { PRECEDING } else { FOLLOWING };
            return Ok(DISCONNECTED | IMPLEMENTATION_SPECIFIC | order);
        }
        mine.reverse();
        theirs.reverse();
        let shared = mine.iter().zip(&theirs).take_while(|(a, b)| a == b).count();
        if shared == theirs.len() {
            // `other` is an ancestor of `this`.
            return Ok(CONTAINS | PRECEDING);
        }
        if shared == mine.len() {
            return Ok(CONTAINED_BY | FOLLOWING);
        }
        // The two children of the deepest common ancestor on the two paths: the one first among
        // its children precedes.
        let parent = mine[shared - 1];
        let (my_child, their_child) = (mine[shared], theirs[shared]);
        let first = reads::first_of_two(cx, parent, my_child, their_child)?;
        Ok(if first == Some(their_child) {
            PRECEDING
        } else {
            FOLLOWING
        })
    }

    fn node_contains(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        other: Option<NodeId>,
    ) -> DomResult<bool> {
        let Some(other) = other else {
            return Ok(false);
        };
        // An inclusive descendant.
        reads::contains(cx, this, other)
    }

    fn node_lookup_prefix(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
    ) -> DomResult<Option<String>> {
        match namespace.filter(|namespace| !namespace.is_empty()) {
            Some(namespace) => reads::locate_prefix(cx, this, &namespace),
            None => Ok(None),
        }
    }

    fn node_lookup_namespace_uri(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        prefix: Option<String>,
    ) -> DomResult<Option<String>> {
        let prefix = prefix.filter(|prefix| !prefix.is_empty());
        reads::locate_namespace(cx, this, prefix.as_deref())
    }

    fn node_is_default_namespace(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
    ) -> DomResult<bool> {
        let namespace = namespace.filter(|namespace| !namespace.is_empty());
        Ok(reads::locate_namespace(cx, this, None)? == namespace)
    }

    fn element_namespace_uri(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<String>> {
        Ok(reads::namespace_uri(cx, this))
    }

    fn element_prefix(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<Option<String>> {
        // The parser makes no element with a prefix.
        Ok(None)
    }

    fn element_local_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        reads::local_name(cx, this)
    }

    fn element_tag_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        reads::tag_name(cx, this)
    }

    fn element_shadow_root(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        // No shadow root is ever attached.
        Ok(None)
    }

    fn element_get_elements_by_tag_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<Vec<NodeId>> {
        reads::by_tag_name(cx, this, &qualified_name)
    }

    fn element_get_elements_by_tag_name_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<Vec<NodeId>> {
        reads::by_tag_name_ns(cx, this, namespace, &local_name)
    }

    fn element_get_elements_by_class_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        class_names: String,
    ) -> DomResult<Vec<NodeId>> {
        reads::by_class_name(cx, this, &class_names)
    }

    fn element_children(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<Vec<NodeId>> {
        reads::element_children(cx, this)
    }

    fn element_first_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::edge_element_child(cx, this, false)
    }

    fn element_last_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::edge_element_child(cx, this, true)
    }

    fn element_child_element_count(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<u32> {
        reads::element_count(cx, this)
    }

    fn element_previous_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::sibling(cx, &mut self.hints, this, false, true)
    }

    fn element_next_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::sibling(cx, &mut self.hints, this, true, true)
    }

    fn element_assigned_slot(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        // No shadow trees, so no slot assigns it.
        Ok(None)
    }

    fn document_url(&mut self, cx: &mut DomCx<'_, '_>, _this: NodeId) -> DomResult<String> {
        reads::text(cx, self.info.address.href())
    }

    fn document_document_uri(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<String> {
        reads::text(cx, self.info.address.href())
    }

    fn document_compat_mode(&mut self, cx: &mut DomCx<'_, '_>, _this: NodeId) -> DomResult<String> {
        Ok(if reads::quirks(cx) {
            "BackCompat"
        } else {
            "CSS1Compat"
        }
        .to_string())
    }

    fn document_character_set(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<String> {
        reads::text(cx, self.info.encoding.clone())
    }

    fn document_charset(&mut self, cx: &mut DomCx<'_, '_>, _this: NodeId) -> DomResult<String> {
        reads::text(cx, self.info.encoding.clone())
    }

    fn document_input_encoding(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<String> {
        reads::text(cx, self.info.encoding.clone())
    }

    fn document_content_type(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<String> {
        reads::text(cx, self.info.content_type.clone())
    }

    fn document_doctype(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::doctype(cx, this)
    }

    fn document_document_element(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::document_element(cx, this)
    }

    fn document_get_elements_by_tag_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<Vec<NodeId>> {
        reads::by_tag_name(cx, this, &qualified_name)
    }

    fn document_get_elements_by_tag_name_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<Vec<NodeId>> {
        reads::by_tag_name_ns(cx, this, namespace, &local_name)
    }

    fn document_get_elements_by_class_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        class_names: String,
    ) -> DomResult<Vec<NodeId>> {
        reads::by_class_name(cx, this, &class_names)
    }

    fn document_get_element_by_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        element_id: String,
    ) -> DomResult<Option<NodeId>> {
        reads::by_id(cx, this, &element_id)
    }

    fn document_children(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Vec<NodeId>> {
        reads::element_children(cx, this)
    }

    fn document_first_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::edge_element_child(cx, this, false)
    }

    fn document_last_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::edge_element_child(cx, this, true)
    }

    fn document_child_element_count(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<u32> {
        reads::element_count(cx, this)
    }

    fn document_title(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        reads::title(cx, this)
    }

    fn character_data_data(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        reads::copied(cx, this).map(Option::unwrap_or_default)
    }

    fn character_data_length(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<u32> {
        reads::length(cx, this)
    }

    fn character_data_substring_data(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        offset: u32,
        count: u32,
    ) -> DomResult<Vec<u16>> {
        reads::substring_data(cx, this, offset, count)
    }

    fn character_data_previous_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::sibling(cx, &mut self.hints, this, false, true)
    }

    fn character_data_next_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::sibling(cx, &mut self.hints, this, true, true)
    }

    fn text_whole_text(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        reads::whole_text(cx, &self.hints, this)
    }

    fn text_assigned_slot(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        // No shadow trees, so no slot assigns it.
        Ok(None)
    }

    fn processing_instruction_target(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<String> {
        reads::target(cx, this)
    }

    fn document_type_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        reads::doctype_field(cx, this, DoctypeField::Name)
    }

    fn document_type_public_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<String> {
        reads::doctype_field(cx, this, DoctypeField::PublicId)
    }

    fn document_type_system_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<String> {
        reads::doctype_field(cx, this, DoctypeField::SystemId)
    }

    fn document_fragment_get_element_by_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        element_id: String,
    ) -> DomResult<Option<NodeId>> {
        reads::by_id(cx, this, &element_id)
    }

    fn document_fragment_children(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Vec<NodeId>> {
        reads::element_children(cx, this)
    }

    fn document_fragment_first_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::edge_element_child(cx, this, false)
    }

    fn document_fragment_last_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        reads::edge_element_child(cx, this, true)
    }

    fn document_fragment_child_element_count(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<u32> {
        reads::element_count(cx, this)
    }

    fn node_list_item(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        this: &[NodeId],
        index: u32,
    ) -> DomResult<Option<NodeId>> {
        Ok(usize::try_from(index)
            .ok()
            .and_then(|index| this.get(index))
            .copied())
    }

    fn node_list_length(&mut self, _cx: &mut DomCx<'_, '_>, this: &[NodeId]) -> DomResult<u32> {
        Ok(u32::try_from(this.len()).unwrap_or(u32::MAX))
    }

    fn html_collection_length(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        this: &[NodeId],
    ) -> DomResult<u32> {
        Ok(u32::try_from(this.len()).unwrap_or(u32::MAX))
    }

    fn html_collection_item(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        this: &[NodeId],
        index: u32,
    ) -> DomResult<Option<NodeId>> {
        Ok(usize::try_from(index)
            .ok()
            .and_then(|index| this.get(index))
            .copied())
    }

    fn html_collection_named_item(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: &[NodeId],
        name: String,
    ) -> DomResult<Option<NodeId>> {
        // DOM, `namedItem`: the first element whose ID is the key or, in the HTML namespace,
        // whose `name` attribute is; none for the empty key.
        if name.is_empty() {
            return Ok(None);
        }
        reads::named_item(cx, this, &name)
    }

    fn window_window(&mut self, _cx: &mut DomCx<'_, '_>) -> DomResult<TheWindow> {
        Ok(TheWindow)
    }

    fn window_self(&mut self, _cx: &mut DomCx<'_, '_>) -> DomResult<TheWindow> {
        Ok(TheWindow)
    }

    fn window_document(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<NodeId> {
        Ok(reads::document(cx))
    }

    fn window_event(&mut self, _cx: &mut DomCx<'_, '_>) -> DomResult<()> {
        // No event is ever dispatched, so there is no current event: `undefined`.
        Ok(())
    }
}
