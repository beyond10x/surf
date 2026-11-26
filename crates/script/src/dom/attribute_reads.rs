//! The reads of the tree for the slice `attributes` (`story:scripts-dom-attributes`): every
//! read of the document's tree that a member of [`super::SurfAttributes`] makes, and the pure
//! helpers those reads need. Each function charges before its pass, from a length known
//! without one (the attribute count, a string's byte length): one step per 4 attributes passed
//! in a scan by name ([`reads::ATTRIBUTES_A_STEP`]) and one step per 16 bytes read, compared or
//! copied ([`STRING_BYTES_A_STEP`]). This file writes nothing.

use surf_html::{Attribute, Namespace, NodeData, NodeId, Tree};

use crate::bindings::{DomCx, DomEnd, DomResult, STRING_BYTES_A_STEP, longer_than};

use super::reads;

/// What a scan looks for: a qualified name (`getAttribute`, `hasAttribute` and the writes by
/// qualified name), or a namespace and a local name (`getAttributeNS`, `hasAttributeNS`, the
/// writes by namespace and the reflected attributes, which are in no namespace).
#[derive(Clone, Copy, Debug)]
pub(super) enum Wanted<'a> {
    /// A qualified name, lowered as it is compared on an HTML element.
    Qualified(&'a str),
    /// A namespace (`None` is null) and a local name.
    Local(Option<Namespace>, &'a str),
}

impl Wanted<'_> {
    /// The bytes of the name a compare reads: the qualified name or the local name.
    fn bytes(self) -> usize {
        match self {
            Wanted::Qualified(name) | Wanted::Local(_, name) => name.len(),
        }
    }
}

/// The attribute a scan found: numbers and no reference, so that nothing of the tree is held
/// across a charge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Found {
    /// Its position in the element's list.
    pub(super) at: usize,
    /// The bytes of its value.
    pub(super) value_bytes: usize,
    /// The tree's count of it, [`Attribute::stored_bytes`].
    pub(super) stored_bytes: usize,
}

/// What [`find`] answers: the attribute found, and the two numbers of its scan that a write
/// reuses for the charge of the tree's own scan, since the list has not changed since.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Scan {
    /// The attribute, `None` for none.
    pub(super) found: Option<Found>,
    /// The attributes of the element (`c`).
    pub(super) count: usize,
    /// The attributes whose name has the wanted name's length (`k`).
    pub(super) same_length: usize,
}

impl Scan {
    /// The steps of one scan for a name of `bytes` bytes over the same list: `c / A + k * n /
    /// S`, the charge a write makes for the tree's own scan.
    pub(super) fn steps(&self, bytes: usize) -> usize {
        pass_steps(self.count).saturating_add(compare_steps(self.same_length, bytes))
    }
}

/// The steps of `bytes` bytes read, compared or copied: one per [`STRING_BYTES_A_STEP`].
pub(super) fn byte_steps(bytes: usize) -> usize {
    bytes / usize::try_from(STRING_BYTES_A_STEP).unwrap_or(usize::MAX)
}

/// The steps of passing `count` attributes in a scan by name: one per
/// [`reads::ATTRIBUTES_A_STEP`].
fn pass_steps(count: usize) -> usize {
    count / reads::ATTRIBUTES_A_STEP
}

/// The steps of `same_length` compares that can each read all `bytes` bytes of a name.
fn compare_steps(same_length: usize, bytes: usize) -> usize {
    byte_steps(same_length.saturating_mul(bytes))
}

/// The attributes of `node`, an empty list for a node that is no element.
fn attributes_of(tree: &Tree, node: NodeId) -> &[Attribute] {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes,
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => &[],
    }
}

/// The value of the attribute at `at` of `element`, read by index with no scan.
fn value_at(tree: &Tree, element: NodeId, at: usize) -> &str {
    attributes_of(tree, element)
        .get(at)
        .map_or("", |attribute| attribute.value.as_str())
}

/// The namespace a namespace argument names: `None` and the empty string are null
/// (`Some(None)`), a URL of the six the tree stores is that namespace, and any other string is
/// `None`, a namespace no attribute has, so that a read answers without a scan.
pub(super) fn namespace_of(namespace: Option<&str>) -> Option<Option<Namespace>> {
    match namespace {
        None | Some("") => Some(None),
        Some(url) => Namespace::from_url(url).map(Some),
    }
}

/// Finds the attribute `wanted` names on `element`. The order: one step per 4 attributes of the
/// element, from the count; one pass over the list that counts the attributes whose name has
/// the wanted name's length (`k`, lengths only); one step per 16 bytes of `k` compares of the
/// whole name; then the tree's position read, the second pass. A node that is no element has no
/// attribute.
pub(super) fn find(cx: &mut DomCx<'_, '_>, element: NodeId, wanted: Wanted<'_>) -> DomResult<Scan> {
    let count = reads::attribute_count(cx.tree(), element);
    reads::charge(cx, pass_steps(count))?;
    let bytes = wanted.bytes();
    let same_length = attributes_of(cx.tree(), element)
        .iter()
        .filter(|attribute| match wanted {
            Wanted::Qualified(_) => attribute.qualified_len() == bytes,
            Wanted::Local(..) => attribute.local_name.len() == bytes,
        })
        .count();
    reads::charge(cx, compare_steps(same_length, bytes))?;
    let at = match wanted {
        Wanted::Qualified(name) => cx.tree().attribute_position(element, name),
        Wanted::Local(namespace, local_name) => cx
            .tree()
            .attribute_position_ns(element, namespace, local_name),
    };
    let found = at.and_then(|at| {
        attributes_of(cx.tree(), element)
            .get(at)
            .map(|attribute| Found {
                at,
                value_bytes: attribute.value.len(),
                stored_bytes: attribute.stored_bytes(),
            })
    });
    Ok(Scan {
        found,
        count,
        same_length,
    })
}

/// The value of the attribute at `at` of `element` as a String a member answers: one step per
/// 16 bytes of it; then held to the agent's `string_length` (`L`, in UTF-16 units): a value of
/// at most `L` bytes needs no count, one of more than `3 * L` bytes has more than `L` units and
/// is the `RangeError` at once, and one between is counted by [`longer_than`], a pass over its
/// bytes charged one step per 16 bytes again before it runs; then the copy. It is alike on
/// purpose to `reads::copied`, which copies a node's character data and cannot name an
/// attribute's value at a position; each has its own case.
fn copy_value(cx: &mut DomCx<'_, '_>, element: NodeId, at: usize) -> DomResult<String> {
    let limit = cx.string_length();
    let bytes = value_at(cx.tree(), element, at).len();
    reads::charge(cx, byte_steps(bytes))?;
    let wide = u64::try_from(bytes).unwrap_or(u64::MAX);
    if wide > limit {
        if wide > limit.saturating_mul(3) {
            return Err(DomEnd::StringLength);
        }
        reads::charge(cx, byte_steps(bytes))?;
        if longer_than(value_at(cx.tree(), element, at), limit) {
            return Err(DomEnd::StringLength);
        }
    }
    Ok(value_at(cx.tree(), element, at).to_string())
}

/// The value of the attribute of `element` in no namespace whose local name is `local_name`
/// (`id`, `class`, `slot`), the empty string for none: [`find`], then [`copy_value`].
pub(super) fn reflected(
    cx: &mut DomCx<'_, '_>,
    element: NodeId,
    local_name: &str,
) -> DomResult<String> {
    let scan = find(cx, element, Wanted::Local(None, local_name))?;
    match scan.found {
        Some(found) => copy_value(cx, element, found.at),
        None => Ok(String::new()),
    }
}

/// Whether `element` has any attribute: a count of a length, no charge.
pub(super) fn any(cx: &DomCx<'_, '_>, element: NodeId) -> bool {
    reads::attribute_count(cx.tree(), element) > 0
}

/// The qualified names of `element`'s attributes, in order: one step per attribute, from the
/// count; one pass that adds up the names' lengths; one step per 16 bytes of them; then the
/// tree's list.
pub(super) fn names(cx: &mut DomCx<'_, '_>, element: NodeId) -> DomResult<Vec<String>> {
    let count = reads::attribute_count(cx.tree(), element);
    reads::charge(cx, count)?;
    let bytes = attributes_of(cx.tree(), element)
        .iter()
        .fold(0_usize, |sum, attribute| {
            sum.saturating_add(attribute.qualified_len())
        });
    reads::charge(cx, byte_steps(bytes))?;
    Ok(cx.tree().attribute_names(element))
}

/// The value of the attribute of `element` whose qualified name is `name`, `None` for none.
pub(super) fn get(
    cx: &mut DomCx<'_, '_>,
    element: NodeId,
    name: &str,
) -> DomResult<Option<String>> {
    let scan = find(cx, element, Wanted::Qualified(name))?;
    match scan.found {
        Some(found) => copy_value(cx, element, found.at).map(Some),
        None => Ok(None),
    }
}

/// The value of the attribute of `element` in `namespace` (the empty string is null) whose
/// local name is `local_name`, `None` for none; `None` with no scan for a namespace outside
/// the six.
pub(super) fn get_ns(
    cx: &mut DomCx<'_, '_>,
    element: NodeId,
    namespace: Option<&str>,
    local_name: &str,
) -> DomResult<Option<String>> {
    let Some(namespace) = namespace_of(namespace) else {
        return Ok(None);
    };
    let scan = find(cx, element, Wanted::Local(namespace, local_name))?;
    match scan.found {
        Some(found) => copy_value(cx, element, found.at).map(Some),
        None => Ok(None),
    }
}

/// Whether `element` has an attribute whose qualified name is `name`.
pub(super) fn has(cx: &mut DomCx<'_, '_>, element: NodeId, name: &str) -> DomResult<bool> {
    let scan = find(cx, element, Wanted::Qualified(name))?;
    Ok(scan.found.is_some())
}

/// Whether `element` has an attribute in `namespace` (the empty string is null) whose local
/// name is `local_name`; `false` with no scan for a namespace outside the six.
pub(super) fn has_ns(
    cx: &mut DomCx<'_, '_>,
    element: NodeId,
    namespace: Option<&str>,
    local_name: &str,
) -> DomResult<bool> {
    let Some(namespace) = namespace_of(namespace) else {
        return Ok(false);
    };
    let scan = find(cx, element, Wanted::Local(namespace, local_name))?;
    Ok(scan.found.is_some())
}
