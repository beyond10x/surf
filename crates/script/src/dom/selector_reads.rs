//! The reads of the tree for the slice `selectors` (`story:scripts-dom-selectors`): every
//! function that reads the document's tree, or charges, for a member of
//! [`super::SurfSelectors`]. Each charges before the work it pays for, through
//! [`super::reads::charge`] and at the one price of a byte, [`STRING_BYTES_A_STEP`]; its doc
//! comment says what it charges. A charge past the limit answers a throw the engine does not
//! deliver: the run is ending.
//!
//! Three rules of `reads.rs` are copied here on purpose, so that this slice changes that file
//! by visibility only: the split of a `class` value into tokens and its fold in quirks mode
//! ([`has_class`], from the closure of `reads::by_class_name`), the rule of a type name, lowered
//! for an element of the HTML namespace and exact for any other ([`test_compound`], from
//! `reads::by_tag_name`), and the step to an element parent ([`next_ancestor`], from
//! `reads::parent_element_in`). A later story may share them. A fourth is the read of the value
//! a scan found ([`value_of`], from `reads::value_at`): `reads.rs` answers no reference into the
//! tree from a function another module calls, which a test of the reading slice holds.

use surf_html::{Namespace, NodeData, NodeId, Tree};

use crate::bindings::{DomCx, DomResult, STRING_BYTES_A_STEP};

use super::reads::{Reads, attribute_count, charge, scan_for};
use super::selectors::{AttributeSelector, Name, Simple};

/// The bytes one step pays for: [`STRING_BYTES_A_STEP`].
const BYTES_A_STEP: usize = STRING_BYTES_A_STEP as usize;

/// The parse of a selector text of `bytes`: `1 + bytes / 16` steps, before the lexer reads it.
pub(super) fn selector_text(cx: &mut DomCx<'_, '_>, bytes: usize) -> DomResult<()> {
    charge(cx, 1 + bytes / BYTES_A_STEP)
}

/// One token of the selector text: one step, before the lexer takes it.
pub(super) fn selector_token(cx: &mut DomCx<'_, '_>) -> DomResult<()> {
    charge(cx, 1)
}

/// The fold of a type name or an attribute name of `bytes` to ASCII lower case, once in the
/// parse: `bytes / 16` steps, before the fold.
pub(super) fn selector_name(cx: &mut DomCx<'_, '_>, bytes: usize) -> DomResult<()> {
    charge(cx, bytes / BYTES_A_STEP)
}

/// The stack of a walk in tree order: frames of a parent and the index of its next child, one
/// for each level of depth that has children still to visit. It charges nothing and holds no
/// context; the caller charges each node before it asks for it. No list of children is copied:
/// the next child is read at its index in the child list, in constant time.
#[derive(Debug)]
pub struct Frames {
    frames: Vec<(NodeId, usize)>,
    most: usize,
}

impl Frames {
    /// A walk of the nodes under `scope`, `scope` left out.
    pub fn new(scope: NodeId) -> Frames {
        Frames {
            frames: vec![(scope, 0)],
            most: 1,
        }
    }

    /// The next node under the scope in tree order, `None` at the end. A node that has
    /// children is pushed as a frame; an exhausted frame is popped, which reads no node.
    pub fn next(&mut self, tree: &Tree) -> Option<NodeId> {
        loop {
            let top = self.frames.last_mut()?;
            let (parent, index) = *top;
            let Some(&child) = tree.children(parent).get(index) else {
                self.frames.pop();
                continue;
            };
            top.1 = index + 1;
            if !tree.children(child).is_empty() {
                self.frames.push((child, 0));
                self.most = self.most.max(self.frames.len());
            }
            return Some(child);
        }
    }

    /// The frames held now.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether no frame is held: the walk is at its end.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// The most frames held at once.
    pub fn most(&self) -> usize {
        self.most
    }
}

/// The elements under `scope`, `scope` left out, in tree order, each given to `visit` until it
/// answers true. One step before each node is taken from [`Frames`], so a node is charged
/// before it is read, and the step that finds the end is paid too; `visit` charges its own work.
pub(super) fn descendant_elements(
    cx: &mut DomCx<'_, '_>,
    scope: NodeId,
    mut visit: impl FnMut(&mut DomCx<'_, '_>, NodeId) -> DomResult<bool>,
) -> DomResult<()> {
    let mut frames = Frames::new(scope);
    loop {
        charge(cx, 1)?;
        let Some(node) = frames.next(cx.tree()) else {
            return Ok(());
        };
        if is_element(cx.tree(), node) && visit(cx, node)? {
            return Ok(());
        }
    }
}

/// The value of the attribute at `at` of `node`'s list, as `reads::scan_for` found it; read for
/// the bytes the scan charged. A copy of `reads::value_at`.
fn value_of(tree: &Tree, node: NodeId, at: usize) -> &str {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes
            .get(at)
            .map_or("", |attribute| attribute.value.as_str()),
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => "",
    }
}

fn is_element(tree: &Tree, node: NodeId) -> bool {
    matches!(tree.data(node), NodeData::Element { .. })
}

/// The parent of `node` when it is an element, else none: one step, before the read, so the
/// read that finds no element parent is charged too. A copy of `reads::parent_element_in`.
pub(super) fn next_ancestor(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Option<NodeId>> {
    charge(cx, 1)?;
    let tree = cx.tree();
    Ok(tree.parent(node).filter(|&parent| is_element(tree, parent)))
}

/// The memo of one complex selector of `compounds` compounds at a chain of `chain` elements:
/// two zeroed bytes for each pair, `bytes / 16` steps, before it is made.
pub(super) fn memo(cx: &mut DomCx<'_, '_>, compounds: usize, chain: usize) -> DomResult<Vec<u8>> {
    let bytes = compounds.saturating_mul(chain).saturating_mul(2);
    charge(cx, bytes / BYTES_A_STEP)?;
    Ok(vec![0; bytes])
}

/// The gate of a complex selector at the element `node`: one step, then [`test_compound`] of
/// its rightmost compound `compound` at the element itself.
pub(super) fn gate(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    compound: &[Simple],
    quirks: bool,
) -> DomResult<bool> {
    charge(cx, 1)?;
    test_compound(cx, node, compound, quirks)
}

/// One cell of the matcher's fill: one step; when `left` holds, [`test_compound`] of
/// `compound` at `node` and its answer, else no.
pub(super) fn cell(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    compound: &[Simple],
    quirks: bool,
    left: bool,
) -> DomResult<bool> {
    charge(cx, 1)?;
    if !left {
        return Ok(false);
    }
    test_compound(cx, node, compound, quirks)
}

/// Whether the element `node` matches the compound `compound`, its simple selectors in order,
/// stopping at the first that fails: one step before each simple selector after the first (the
/// first one's step is the caller's, [`gate`]'s or [`cell`]'s); for a type name one step per 16
/// bytes of the form of the name the element's namespace calls for, before the compare; the
/// id, class and attribute tests charge as their functions say. The type rule is a copy of the
/// match of `reads::by_tag_name`: the name lowered for an element of the HTML namespace, as
/// given for any other.
pub(super) fn test_compound(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    compound: &[Simple],
    quirks: bool,
) -> DomResult<bool> {
    for (at, simple) in compound.iter().enumerate() {
        if at > 0 {
            charge(cx, 1)?;
        }
        let holds = match simple {
            Simple::Universal => true,
            Simple::Type(name) => {
                let html = is_html(cx.tree(), node);
                let wanted = form(name, html);
                charge(cx, wanted.len() / BYTES_A_STEP)?;
                match cx.tree().data(node) {
                    NodeData::Element { local_name, .. } => local_name == wanted,
                    NodeData::Document
                    | NodeData::DocumentFragment
                    | NodeData::Doctype { .. }
                    | NodeData::Text(_)
                    | NodeData::Comment(_)
                    | NodeData::ProcessingInstruction { .. } => false,
                }
            }
            Simple::Id(id) => has_id(cx, node, id, quirks)?,
            Simple::Class(class) => has_class(cx, node, class, quirks)?,
            Simple::Attribute(attribute) => match &attribute.value {
                Some(_) => attribute_equals(cx, node, attribute)?,
                None => has_attribute(cx, node, &attribute.name)?,
            },
        };
        if !holds {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether `node` is an element of the HTML namespace.
fn is_html(tree: &Tree, node: NodeId) -> bool {
    matches!(
        tree.data(node),
        NodeData::Element {
            namespace: Namespace::Html,
            ..
        }
    )
}

/// The form of a name of the selector that an element compares: its ASCII lower case for an
/// element of the HTML namespace, the name as given for any other.
fn form(name: &Name, html: bool) -> &str {
    if html { &name.lower } else { &name.given }
}

/// Whether `node`'s ID is `id`, ASCII case-insensitively in quirks mode: the scan of `id` as
/// `reads::scan_for` charges it, one step per 4 attributes of the list before the pass and one
/// per 16 bytes of the value before it is compared (the fold of quirks mode paid by them).
pub(super) fn has_id(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    id: &str,
    quirks: bool,
) -> DomResult<bool> {
    let Some(at) = scan_for(cx, node, Reads::Value("id"))? else {
        return Ok(false);
    };
    let value = value_of(cx.tree(), node, at);
    Ok(if quirks {
        value.eq_ignore_ascii_case(id)
    } else {
        value == id
    })
}

/// Whether `class` is a token of `node`'s `class` attribute split at ASCII whitespace, ASCII
/// case-insensitively in quirks mode: the scan as `reads::scan_for` charges it, one step per 4
/// attributes, one per 16 bytes of the value and one per class token, before the first token is
/// looked up. A copy of the token rule of the closure of `reads::by_class_name`.
pub(super) fn has_class(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    class: &str,
    quirks: bool,
) -> DomResult<bool> {
    let Some(at) = scan_for(cx, node, Reads::Classes)? else {
        return Ok(false);
    };
    let value = value_of(cx.tree(), node, at);
    Ok(value.split_ascii_whitespace().any(|token| {
        if quirks {
            token.eq_ignore_ascii_case(class)
        } else {
            token == class
        }
    }))
}

/// The steps of comparing the selector's attribute name with each attribute of `node`'s list:
/// one per 16 bytes of the name for each attribute.
fn name_steps(cx: &DomCx<'_, '_>, node: NodeId, name: &str) -> usize {
    attribute_count(cx.tree(), node).saturating_mul(name.len() / BYTES_A_STEP)
}

/// Whether `node` has the attribute `name` in no namespace: `attribute_count x (name bytes /
/// 16)` for the name, then the scan as `reads::scan_for` charges it, one step per 4 attributes.
pub(super) fn has_attribute(cx: &mut DomCx<'_, '_>, node: NodeId, name: &Name) -> DomResult<bool> {
    let wanted = form(name, is_html(cx.tree(), node));
    charge(cx, name_steps(cx, node, wanted))?;
    Ok(scan_for(cx, node, Reads::Name(wanted))?.is_some())
}

/// Whether `node` has the attribute of `attribute` in no namespace with its value: `attribute_count
/// x (name bytes / 16)` for the name, then the scan as `reads::scan_for` charges it, one step per
/// 4 attributes and one per 16 bytes of the value found, before the compare. The compare is
/// ASCII case-insensitive on an element of the HTML namespace when the parse found the name in
/// the HTML Standard's list, exact otherwise.
pub(super) fn attribute_equals(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    attribute: &AttributeSelector,
) -> DomResult<bool> {
    let html = is_html(cx.tree(), node);
    let wanted = form(&attribute.name, html);
    charge(cx, name_steps(cx, node, wanted))?;
    let Some(at) = scan_for(cx, node, Reads::Value(wanted))? else {
        return Ok(false);
    };
    let value = value_of(cx.tree(), node, at);
    let expected = attribute.value.as_deref().unwrap_or_default();
    Ok(if html && attribute.folded {
        value.eq_ignore_ascii_case(expected)
    } else {
        value == expected
    })
}
