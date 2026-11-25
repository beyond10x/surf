//! The reads of the tree for the slice `reading` (`story:scripts-dom-reading`): every function
//! that reads the document's tree for a member of [`super::SurfReading`]. Each one does work
//! bounded by a constant (a node's kind, a name's length, its parent, its first or last child),
//! or charges before its loop for everything the loop reads; its doc comment says what it
//! charges. The members get nodes, text and attribute values from here only.

use std::collections::HashMap;

use surf_html::{Namespace, NodeData, NodeId, QuirksMode, Tree};
use surf_url::Url;

use crate::bindings::{
    DomCx, DomEnd, DomException, DomResult, STRING_BYTES_A_STEP, longer_than, utf16_units,
};

use super::reading::Hints;

/// The bytes one step pays for: [`STRING_BYTES_A_STEP`], the one price of a byte read,
/// compared, folded, collapsed or copied.
const BYTES_A_STEP: usize = STRING_BYTES_A_STEP as usize;

/// The index of `node` among the children of `parent`: a hint's (at most six compares, no
/// charge), or a scan's at one step per child, charged before the child is read.
pub(super) fn index_in(
    cx: &mut DomCx<'_, '_>,
    hints: &Hints,
    parent: NodeId,
    node: NodeId,
) -> DomResult<usize> {
    let children = cx.tree().children(parent);
    for hint in hints.get(parent).into_iter().flatten() {
        for candidate in [Some(hint), hint.checked_add(1), hint.checked_sub(1)]
            .into_iter()
            .flatten()
        {
            if children.get(candidate) == Some(&node) {
                return Ok(candidate);
            }
        }
    }
    let count = children.len();
    for index in 0..count {
        charge(cx, 1)?;
        if cx.tree().children(parent).get(index) == Some(&node) {
            return Ok(index);
        }
    }
    Err(DomEnd::Throw(DomException::new(
        "InvalidStateError",
        "a node is not among its parent's children",
    )))
}

/// The sibling of `node` one place before it (`forward` false) or after it, of any kind
/// (`elements` false) or the nearest element: [`index_in`]'s charge, then one step per sibling
/// passed, charged before it is read.
pub(super) fn sibling(
    cx: &mut DomCx<'_, '_>,
    hints: &mut Hints,
    node: NodeId,
    forward: bool,
    elements: bool,
) -> DomResult<Option<NodeId>> {
    let Some(parent) = cx.tree().parent(node) else {
        return Ok(None);
    };
    let mut index = index_in(cx, hints, parent, node)?;
    hints.set(parent, index);
    let mut first = true;
    let answer = loop {
        let next = if forward {
            index.checked_add(1)
        } else {
            index.checked_sub(1)
        };
        let Some(next) = next else {
            break None;
        };
        // The sibling one place over is read for the node's own step; each one past it is
        // charged before it is read.
        if !first {
            charge(cx, 1)?;
        }
        first = false;
        let Some(&child) = cx.tree().children(parent).get(next) else {
            break None;
        };
        index = next;
        if !elements || is_element(cx.tree(), child) {
            break Some(child);
        }
    };
    if answer.is_some() {
        hints.set(parent, index);
    }
    Ok(answer)
}

/// The document base URL (HTML, "document base URL"): the frozen base URL of the first
/// `base` element of the HTML namespace with an `href` in tree order, else the fallback
/// base URL, `address`. A `href` that does not parse against the address, is longer than the
/// URL parser reads, or gives a `data` or `javascript` URL gives the address.
pub(super) fn base_url(cx: &mut DomCx<'_, '_>, address: &Url) -> DomResult<String> {
    let root = cx.tree().root();
    // The walk finds `href` by name without reading it; the one it reads is charged before
    // it is copied.
    let base = find(cx, root, Reads::Name("href"), |tree, node, at| {
        at.is_some() && html_element(tree, node, "base")
    })?;
    let Some(base) = base else {
        return Ok(address.href());
    };
    let at = scan_for(cx, base, Reads::Value("href"))?;
    let Some(at) = at else {
        return Ok(address.href());
    };
    let href = value_at(cx.tree(), base, at).to_string();
    // HTML, "set the frozen base URL": a href that does not parse, or parses to a URL whose
    // scheme is `data` or `javascript`, gives the fallback base URL; a later `base` is not
    // read instead.
    let url = match Url::parse(&href, Some(address)) {
        Ok(url) if !matches!(url.scheme(), "data" | "javascript") => url,
        Ok(_) | Err(_) => address.clone(),
    };
    text(cx, url.href())
}

/// Charges `steps` to the running script. Past its limit the run ends; the answer here is then
/// a throw the engine does not deliver.
fn charge(cx: &mut DomCx<'_, '_>, steps: usize) -> DomResult<()> {
    if steps == 0 {
        return Ok(());
    }
    let steps = u64::try_from(steps).unwrap_or(u64::MAX);
    cx.engine().charge_steps(steps).map_err(|_| {
        DomEnd::Throw(DomException::new(
            "InvalidStateError",
            "the steps of the script are spent",
        ))
    })
}

/// A String a member answers that it made of no read of the tree (the document's address, its
/// encoding, a parsed URL): one step per 16 bytes.
pub(super) fn text(cx: &mut DomCx<'_, '_>, text: String) -> DomResult<String> {
    charge(cx, text.len() / BYTES_A_STEP)?;
    Ok(text)
}

/// A copy of the text `pick` reads of `node` (a name, a target, an id of a doctype): one step
/// per 16 bytes, before the copy.
fn copy_of(cx: &mut DomCx<'_, '_>, node: NodeId, pick: fn(&NodeData) -> &str) -> DomResult<String> {
    let bytes = pick(cx.tree().data(node)).len();
    charge(cx, bytes / BYTES_A_STEP)?;
    Ok(pick(cx.tree().data(node)).to_string())
}

/// An element's local name, empty for another node.
fn local_name_of(data: &NodeData) -> &str {
    match data {
        NodeData::Element { local_name, .. } => local_name,
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => "",
    }
}

/// A processing instruction's target, empty for another node.
fn target_of(data: &NodeData) -> &str {
    match data {
        NodeData::ProcessingInstruction { target, .. } => target,
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Element { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_) => "",
    }
}

/// A doctype's name, empty for another node.
fn doctype_name_of(data: &NodeData) -> &str {
    match data {
        NodeData::Doctype { name, .. } => name,
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Element { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => "",
    }
}

/// A doctype's public ID, empty for another node.
fn public_id_of(data: &NodeData) -> &str {
    match data {
        NodeData::Doctype { public_id, .. } => public_id,
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Element { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => "",
    }
}

/// A doctype's system ID, empty for another node.
fn system_id_of(data: &NodeData) -> &str {
    match data {
        NodeData::Doctype { system_id, .. } => system_id,
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Element { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => "",
    }
}

/// The character data of `node` as `data`, `nodeValue` and `textContent` answer it, `None` for
/// a node without: one step per 16 bytes, before the data is measured against the agent's
/// `string_length` (past it the `RangeError`) and copied.
pub(super) fn copied(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Option<String>> {
    let limit = cx.string_length();
    let Some(bytes) = character_data(cx.tree(), node).map(str::len) else {
        return Ok(None);
    };
    charge(cx, bytes / BYTES_A_STEP)?;
    let data = character_data(cx.tree(), node).unwrap_or_default();
    if longer_than(data, limit) {
        return Err(DomEnd::StringLength);
    }
    Ok(Some(data.to_string()))
}

/// A String a member builds from pieces (`textContent`, `wholeText`, `title`), held to the
/// agent's `string_length` before each piece is appended: a piece that would take it past the
/// limit is not appended, and the member ends with the `RangeError`. Its UTF-16 code units are
/// counted only once its bytes pass the limit: what it holds then once, and each later piece.
struct Gathered {
    text: String,
    limit: u64,
    units: Option<u64>,
}

impl Gathered {
    fn new(limit: u64) -> Gathered {
        Gathered {
            text: String::new(),
            limit,
            units: None,
        }
    }

    /// Admits `piece`: the `RangeError` when the String with it would be longer than the limit.
    /// Admitted, it is to be appended with [`Gathered::append`].
    fn admit(&mut self, piece: &str) -> DomResult<()> {
        let bytes = self.text.len().saturating_add(piece.len());
        if u64::try_from(bytes).unwrap_or(u64::MAX) <= self.limit {
            return Ok(());
        }
        let held = match self.units {
            Some(units) => units,
            None => utf16_units(&self.text),
        };
        let units = held.saturating_add(utf16_units(piece));
        if units > self.limit {
            return Err(DomEnd::StringLength);
        }
        self.units = Some(units);
        Ok(())
    }

    /// The steps of appending `bytes` more: one per 16 bytes of the whole.
    fn steps(&self, bytes: usize) -> usize {
        (self.text.len() + bytes) / BYTES_A_STEP - self.text.len() / BYTES_A_STEP
    }

    fn append(&mut self, piece: &str) {
        self.text.push_str(piece);
    }
}

/// Appends the data of the text `node` to `gathered`: its bytes charged at one step per 16, then
/// admitted and copied.
fn gather(cx: &mut DomCx<'_, '_>, gathered: &mut Gathered, node: NodeId) -> DomResult<()> {
    let bytes = character_data(cx.tree(), node).map_or(0, str::len);
    let steps = gathered.steps(bytes);
    charge(cx, steps)?;
    gathered.admit(character_data(cx.tree(), node).unwrap_or_default())?;
    gathered.append(character_data(cx.tree(), node).unwrap_or_default());
    Ok(())
}

fn is_element(tree: &Tree, node: NodeId) -> bool {
    matches!(tree.data(node), NodeData::Element { .. })
}

/// Whether `node` is an element of the HTML namespace named `name`.
fn html_element(tree: &Tree, node: NodeId, name: &str) -> bool {
    matches!(tree.data(node), NodeData::Element { namespace: Namespace::Html, local_name, .. } if local_name == name)
}

/// Whether `node` is an element of the HTML namespace.
fn html_element_any(tree: &Tree, node: NodeId) -> bool {
    matches!(
        tree.data(node),
        NodeData::Element {
            namespace: Namespace::Html,
            ..
        }
    )
}

/// The data of character data: a text's, a comment's or a processing instruction's.
fn character_data(tree: &Tree, node: NodeId) -> Option<&str> {
    match tree.data(node) {
        NodeData::Text(data) | NodeData::Comment(data) => Some(data),
        NodeData::ProcessingInstruction { data, .. } => Some(data),
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Element { .. } => None,
    }
}

/// What a walk reads of each node it visits, and so what it charges for it beside its one step.
#[derive(Clone, Copy)]
enum Reads {
    /// The node's kind and names only: one step a node.
    Names,
    /// Also the attribute of a name, found by name and its value not read: one step per 4
    /// attributes of the element, from the list's length, before the one pass that finds it.
    Name(&'static str),
    /// As `Name`, and the value of the attribute found is read or compared: one step per 16
    /// bytes of it, before it is read.
    Value(&'static str),
    /// As `Value` for `class`, and one step per class token of the value, before the first
    /// token is looked up; the pass that counts the tokens is paid by the bytes' charge.
    Classes,
}

/// The step of passing attributes in a scan for one by name: one per 4 attributes, so that a
/// step buys a bounded number of name compares.
const ATTRIBUTES_A_STEP: usize = 4;

/// The number of attributes of `node`, 0 for a node that is no element.
fn attribute_count(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes.len(),
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => 0,
    }
}

/// The positions in `node`'s attribute list of the attributes `names` in no namespace, found in
/// one pass over the list.
fn positions<const N: usize>(tree: &Tree, node: NodeId, names: [&str; N]) -> [Option<usize>; N] {
    let mut found = [None; N];
    if let NodeData::Element { attributes, .. } = tree.data(node) {
        for (at, attribute) in attributes.iter().enumerate() {
            if attribute.namespace.is_some() {
                continue;
            }
            for (slot, name) in found.iter_mut().zip(names) {
                if slot.is_none() && attribute.local_name == name {
                    *slot = Some(at);
                }
            }
        }
    }
    found
}

/// The value of the attribute at `at` of `node`'s list, as [`positions`] found it.
fn value_at(tree: &Tree, node: NodeId, at: usize) -> &str {
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

/// Charges the scan of `node`'s attributes for `name`, finds it in one pass, and charges what
/// `reads` reads of its value: the position found.
fn scan_for(cx: &mut DomCx<'_, '_>, node: NodeId, reads: Reads) -> DomResult<Option<usize>> {
    let (name, value, tokens) = match reads {
        Reads::Names => return Ok(None),
        Reads::Name(name) => (name, false, false),
        Reads::Value(name) => (name, true, false),
        Reads::Classes => ("class", true, true),
    };
    charge(cx, attribute_count(cx.tree(), node) / ATTRIBUTES_A_STEP)?;
    let [at] = positions(cx.tree(), node, [name]);
    if let (Some(at), true) = (at, value) {
        charge(cx, value_at(cx.tree(), node, at).len() / BYTES_A_STEP)?;
        if tokens {
            let count = value_at(cx.tree(), node, at)
                .split_ascii_whitespace()
                .count();
            charge(cx, count)?;
        }
    }
    Ok(at)
}

/// The steps of the bytes of `node`'s attributes, one per 16 bytes of their names and values,
/// found by one pass over the list; the caller has charged the pass before it, a step an
/// attribute from the count ([`equal_cost`]).
fn attribute_byte_steps(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => {
            let bytes: usize = attributes
                .iter()
                .map(|attribute| attribute.local_name.len() + attribute.value.len())
                .sum();
            bytes / BYTES_A_STEP
        }
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => 0,
    }
}

/// The bytes of an element's local name, which a walk may compare; 0 for another node.
fn name_bytes(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element { local_name, .. } => local_name.len(),
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => 0,
    }
}

/// `node` and the nodes under it in tree order, each charged before it is visited (one step and
/// one per 16 bytes of an element's name, and its attribute scan as [`scan_for`] charges it
/// when the walk `reads` one), until `stop` answers true for one: that node. `stop` gets the
/// position of the attribute the walk looks for, if the node has it.
fn find(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    reads: Reads,
    mut stop: impl FnMut(&Tree, NodeId, Option<usize>) -> bool,
) -> DomResult<Option<NodeId>> {
    preorder(cx, node, |cx, next| {
        charge(cx, 1 + name_bytes(cx.tree(), next) / BYTES_A_STEP)?;
        let at = scan_for(cx, next, reads)?;
        Ok(stop(cx.tree(), next, at))
    })
}

/// `node` and the nodes under it in tree order, each given to `visit` until it answers true:
/// that node. The walk keeps its place by a stack of (parent, index of the next child), one
/// entry per level of depth, and copies no child list; it charges nothing itself: `visit`
/// charges each node before it reads it.
fn preorder(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    mut visit: impl FnMut(&mut DomCx<'_, '_>, NodeId) -> DomResult<bool>,
) -> DomResult<Option<NodeId>> {
    if visit(cx, node)? {
        return Ok(Some(node));
    }
    let mut stack: Vec<(NodeId, usize)> = vec![(node, 0)];
    while let Some(top) = stack.last_mut() {
        let (parent, next) = *top;
        let Some(&child) = cx.tree().children(parent).get(next) else {
            stack.pop();
            continue;
        };
        top.1 = next + 1;
        if visit(cx, child)? {
            return Ok(Some(child));
        }
        stack.push((child, 0));
    }
    Ok(None)
}

/// The nodes under `node`, `node` left out, for which `keep` answers true, in tree order;
/// charged as [`find`].
fn select(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    reads: Reads,
    mut keep: impl FnMut(&Tree, NodeId, Option<usize>) -> bool,
) -> DomResult<Vec<NodeId>> {
    let mut kept = Vec::new();
    find(cx, node, reads, |tree, next, at| {
        if next != node && keep(tree, next, at) {
            kept.push(next);
        }
        false
    })?;
    Ok(kept)
}

/// The elements under `node` for `getElementsByTagName(qualified_name)` (DOM, "list of
/// elements with qualified name"): every element for `*`; else, in this HTML document, an
/// element of the HTML namespace whose name is the name in ASCII lower case, and any other
/// whose name is the name. A compare of the argument with a name charges nothing more: the
/// argument's copy paid its bytes, the walk charged the name's, and a compare reads no further
/// than the shorter.
pub(super) fn by_tag_name(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    qualified_name: &str,
) -> DomResult<Vec<NodeId>> {
    // The name folded once, one step per 16 bytes of it, before the fold.
    charge(cx, qualified_name.len() / BYTES_A_STEP)?;
    let lower = qualified_name.to_ascii_lowercase();
    select(cx, node, Reads::Names, |tree, next, _| {
        match tree.data(next) {
            NodeData::Element {
                namespace,
                local_name,
                ..
            } => {
                qualified_name == "*"
                    || (*namespace == Namespace::Html && *local_name == lower)
                    || (*namespace != Namespace::Html && local_name == qualified_name)
            }
            NodeData::Document
            | NodeData::DocumentFragment
            | NodeData::Doctype { .. }
            | NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::ProcessingInstruction { .. } => false,
        }
    })
}

/// The elements under `node` for `getElementsByTagNameNS(namespace, local_name)` (DOM, "list
/// of elements with namespace namespace and local name localName"); the empty namespace is
/// null, which no element of the tree has. A compare of an argument with a name charges nothing
/// more: the argument's copy paid its bytes, the walk charged the name's, and a compare reads
/// no further than the shorter.
pub(super) fn by_tag_name_ns(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    namespace: Option<String>,
    local_name: &str,
) -> DomResult<Vec<NodeId>> {
    let namespace = namespace.filter(|namespace| !namespace.is_empty());
    select(cx, node, Reads::Names, |tree, next, _| {
        match tree.data(next) {
            NodeData::Element {
                namespace: own,
                local_name: own_name,
                ..
            } => {
                let namespace_matches = match namespace.as_deref() {
                    Some("*") => true,
                    Some(namespace) => own.url() == namespace,
                    None => false,
                };
                namespace_matches && (local_name == "*" || own_name == local_name)
            }
            NodeData::Document
            | NodeData::DocumentFragment
            | NodeData::Doctype { .. }
            | NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::ProcessingInstruction { .. } => false,
        }
    })
}

/// The elements under `node` for `getElementsByClassName(class_names)` (DOM, "list of elements
/// with class names"): those with every class the names give, compared ASCII case-insensitively
/// in quirks mode; none for names that give no class.
pub(super) fn by_class_name(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    class_names: &str,
) -> DomResult<Vec<NodeId>> {
    // The wanted classes: an ordered set (duplicates dropped), built once, one step per 16
    // bytes of the argument and one per token of it, before it is built; in quirks mode each
    // in ASCII lower case.
    charge(cx, class_names.len() / BYTES_A_STEP)?;
    charge(cx, class_names.split_ascii_whitespace().count())?;
    let quirks = quirks(cx);
    let mut wanted: HashMap<String, usize> = HashMap::new();
    for token in class_names.split_ascii_whitespace() {
        let next = wanted.len();
        let token = if quirks {
            token.to_ascii_lowercase()
        } else {
            token.to_string()
        };
        wanted.entry(token).or_insert(next);
    }
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    // An element matches when its classes hold every wanted one: one set lookup per class (in
    // quirks mode after its fold, a copy of the bytes lowered in place), each paid by the step
    // per token and the step per 16 bytes the walk charges before it.
    let mut matched_by = vec![0usize; wanted.len()];
    let mut element = 0usize;
    let mut folded = String::new();
    select(cx, node, Reads::Classes, |tree, next, at| {
        let Some(classes) = at.map(|at| value_at(tree, next, at)) else {
            return false;
        };
        element += 1;
        let mut matched = 0;
        for class in classes.split_ascii_whitespace() {
            let key = if quirks {
                folded.clear();
                folded.push_str(class);
                folded.make_ascii_lowercase();
                folded.as_str()
            } else {
                class
            };
            if let Some(&index) = wanted.get(key)
                && matched_by[index] != element
            {
                matched_by[index] = element;
                matched += 1;
            }
        }
        matched == wanted.len()
    })
}

/// The first element under `node`, `node` left out, whose ID is `id`; none for the empty ID. A
/// compare of the argument with an ID charges nothing more: the argument's copy paid its bytes,
/// the scan charged the ID's, and a compare reads no further than the shorter.
pub(super) fn by_id(cx: &mut DomCx<'_, '_>, node: NodeId, id: &str) -> DomResult<Option<NodeId>> {
    if id.is_empty() {
        return Ok(None);
    }
    find(cx, node, Reads::Value("id"), |tree, next, at| {
        next != node && at.map(|at| value_at(tree, next, at)) == Some(id)
    })
}

/// The element children of `node`, one step per child.
pub(super) fn element_children(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Vec<NodeId>> {
    charge(cx, cx.tree().children(node).len())?;
    let tree = cx.tree();
    Ok(tree
        .children(node)
        .iter()
        .copied()
        .filter(|&child| is_element(tree, child))
        .collect())
}

/// The first element child of `node` (`last` false) or the last: the child at the edge is read
/// for the call's step, each one past it charged one step before it is read.
pub(super) fn edge_element_child(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    last: bool,
) -> DomResult<Option<NodeId>> {
    let count = cx.tree().children(node).len();
    for read in 0..count {
        if read > 0 {
            charge(cx, 1)?;
        }
        let index = if last { count - 1 - read } else { read };
        let tree = cx.tree();
        let child = tree.children(node)[index];
        if is_element(tree, child) {
            return Ok(Some(child));
        }
    }
    Ok(None)
}

/// The count of `node`'s element children, one step per child.
pub(super) fn element_count(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<u32> {
    let count = element_children(cx, node)?.len();
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

/// The ancestors of `node`, itself first and the root last: one step per node, charged before
/// its parent is read.
pub(super) fn ancestors(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Vec<NodeId>> {
    let mut chain = vec![node];
    let mut at = node;
    loop {
        charge(cx, 1)?;
        let Some(parent) = cx.tree().parent(at) else {
            break;
        };
        chain.push(parent);
        at = parent;
    }
    Ok(chain)
}

/// The root of `node`, one step per ancestor.
pub(super) fn root_of(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<NodeId> {
    let chain = ancestors(cx, node)?;
    Ok(chain.last().copied().unwrap_or(node))
}

/// The parent of `node` when it is an element.
fn parent_element_in(tree: &Tree, node: NodeId) -> Option<NodeId> {
    tree.parent(node).filter(|&parent| is_element(tree, parent))
}

/// The element whose namespace lookups `node` answers with (DOM, "locate a namespace" and
/// "locate a namespace prefix"): itself for an element, the document element for a document,
/// none for a doctype and a fragment, the parent element for the others.
fn lookup_start(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Option<NodeId>> {
    let tree = cx.tree();
    match tree.data(node) {
        NodeData::Element { .. } => Ok(Some(node)),
        NodeData::Document => document_child(cx, node, is_element),
        NodeData::Doctype { .. } | NodeData::DocumentFragment => Ok(None),
        NodeData::Text(_) | NodeData::Comment(_) | NodeData::ProcessingInstruction { .. } => {
            Ok(parent_element_in(tree, node))
        }
    }
}

/// The first child of `document` for which `wanted` answers true (its document element or its
/// doctype), one step per child of the document, charged before the scan.
fn document_child(
    cx: &mut DomCx<'_, '_>,
    document: NodeId,
    wanted: fn(&Tree, NodeId) -> bool,
) -> DomResult<Option<NodeId>> {
    charge(cx, cx.tree().children(document).len())?;
    let tree = cx.tree();
    Ok(tree
        .children(document)
        .iter()
        .copied()
        .find(|&child| wanted(tree, child)))
}

fn is_doctype(tree: &Tree, node: NodeId) -> bool {
    matches!(tree.data(node), NodeData::Doctype { .. })
}

/// DOM, "locate a namespace" for `prefix` (`None` is null), from `node`'s start, one step per
/// element passed and its attributes.
pub(super) fn locate_namespace(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    prefix: Option<&str>,
) -> DomResult<Option<String>> {
    let mut at = lookup_start(cx, node)?;
    // The prefixes `xml` and `xmlns` are the first steps of the case of an element: a node
    // without an element to start from (a doctype, a fragment, a document without a document
    // element, a node without a parent element) answers null for them too.
    if at.is_some() {
        match prefix {
            Some("xml") => return Ok(Some(Namespace::Xml.url().to_string())),
            Some("xmlns") => return Ok(Some(Namespace::Xmlns.url().to_string())),
            Some(_) | None => {}
        }
    }
    // A scan for one attribute by name on each element: one step for the element, and one per 4
    // attributes it passes before the scan, and one per 16 bytes of the one value it reads. The
    // null prefix scans no attribute and is charged none.
    while let Some(element) = at {
        charge(cx, 1)?;
        if let (None, NodeData::Element { namespace, .. }) = (prefix, cx.tree().data(element)) {
            // An element of the tree has no prefix: it matches the null prefix only.
            return Ok(Some(namespace.url().to_string()));
        }
        charge(cx, attribute_count(cx.tree(), element) / ATTRIBUTES_A_STEP)?;
        let tree = cx.tree();
        let NodeData::Element { attributes, .. } = tree.data(element) else {
            break;
        };
        let declared = attributes.iter().position(|attribute| {
            attribute.namespace == Some(Namespace::Xmlns)
                && match prefix {
                    Some(prefix) => {
                        attribute.prefix.as_deref() == Some("xmlns")
                            && attribute.local_name == prefix
                    }
                    None => attribute.prefix.is_none() && attribute.local_name == "xmlns",
                }
        });
        if let Some(declared) = declared {
            charge(
                cx,
                value_at(cx.tree(), element, declared).len() / BYTES_A_STEP,
            )?;
            let value = value_at(cx.tree(), element, declared);
            return Ok(Some(value.to_string()).filter(|value| !value.is_empty()));
        }
        at = parent_element_in(cx.tree(), element);
    }
    Ok(None)
}

/// DOM, "locate a namespace prefix" for `namespace`, from `node`'s start, one step per element
/// passed and its attributes.
pub(super) fn locate_prefix(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    namespace: &str,
) -> DomResult<Option<String>> {
    let mut at = lookup_start(cx, node)?;
    // On each element: one step and one per 4 attributes it passes, before the pass, and one
    // per 16 bytes of each `xmlns` value it compares, before the compare; other values are not
    // read.
    while let Some(element) = at {
        let count = attribute_count(cx.tree(), element);
        charge(cx, 1 + count / ATTRIBUTES_A_STEP)?;
        for index in 0..count {
            let compared = match cx.tree().data(element) {
                NodeData::Element { attributes, .. } => attributes
                    .get(index)
                    .filter(|attribute| attribute.prefix.as_deref() == Some("xmlns"))
                    .map(|attribute| attribute.value.len()),
                NodeData::Document
                | NodeData::DocumentFragment
                | NodeData::Doctype { .. }
                | NodeData::Text(_)
                | NodeData::Comment(_)
                | NodeData::ProcessingInstruction { .. } => None,
            };
            let Some(bytes) = compared else {
                continue;
            };
            charge(cx, bytes / BYTES_A_STEP)?;
            if let NodeData::Element { attributes, .. } = cx.tree().data(element)
                && let Some(attribute) = attributes.get(index)
                && attribute.value == namespace
            {
                return Ok(Some(attribute.local_name.clone()));
            }
        }
        at = parent_element_in(cx.tree(), element);
    }
    Ok(None)
}

/// Whether two nodes are equal, without their children (DOM, "equals", its first steps): the
/// same kind, and the same names, data and attributes.
fn shallow_equal(tree: &Tree, a: NodeId, b: NodeId) -> bool {
    match (tree.data(a), tree.data(b)) {
        (NodeData::Document, NodeData::Document)
        | (NodeData::DocumentFragment, NodeData::DocumentFragment) => true,
        (
            NodeData::Doctype {
                name,
                public_id,
                system_id,
            },
            NodeData::Doctype {
                name: other_name,
                public_id: other_public,
                system_id: other_system,
            },
        ) => name == other_name && public_id == other_public && system_id == other_system,
        (
            NodeData::Element {
                namespace,
                local_name,
                attributes,
                ..
            },
            NodeData::Element {
                namespace: other_namespace,
                local_name: other_name,
                attributes: other_attributes,
                ..
            },
        ) => {
            // The attribute lists compared as maps from namespace and local name to value: one
            // hash and one lookup per attribute, linear in their bytes, which the caller
            // charged for both lists before the compare.
            if namespace != other_namespace
                || local_name != other_name
                || attributes.len() != other_attributes.len()
            {
                return false;
            }
            let mine: HashMap<(Option<&str>, &str), &str> = attributes
                .iter()
                .map(|attribute| {
                    let key = (
                        attribute.namespace.map(Namespace::url),
                        attribute.local_name.as_str(),
                    );
                    (key, attribute.value.as_str())
                })
                .collect();
            other_attributes.iter().all(|attribute| {
                let key = (
                    attribute.namespace.map(Namespace::url),
                    attribute.local_name.as_str(),
                );
                mine.get(&key) == Some(&attribute.value.as_str())
            })
        }
        (NodeData::Text(data), NodeData::Text(other))
        | (NodeData::Comment(data), NodeData::Comment(other)) => data == other,
        (
            NodeData::ProcessingInstruction { target, data },
            NodeData::ProcessingInstruction {
                target: other_target,
                data: other_data,
            },
        ) => target == other_target && data == other_data,
        (
            NodeData::Document
            | NodeData::DocumentFragment
            | NodeData::Doctype { .. }
            | NodeData::Element { .. }
            | NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::ProcessingInstruction { .. },
            _,
        ) => false,
    }
}

/// The steps of `node`'s side of a compare that are known without a pass over a list: one, one
/// per 16 bytes of its names and data, and one per attribute from the count, which pays the
/// pass [`attribute_byte_steps`] makes; a compare charges this for both sides, then both sides'
/// attribute bytes, before it reads either.
fn equal_cost(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element {
            local_name,
            attributes,
            ..
        } => 1 + local_name.len() / BYTES_A_STEP + attributes.len(),
        NodeData::Text(data) | NodeData::Comment(data) => 1 + data.len() / BYTES_A_STEP,
        NodeData::ProcessingInstruction { target, data } => {
            1 + (target.len() + data.len()) / BYTES_A_STEP
        }
        NodeData::Doctype {
            name,
            public_id,
            system_id,
        } => 1 + (name.len() + public_id.len() + system_id.len()) / BYTES_A_STEP,
        NodeData::Document | NodeData::DocumentFragment => 1,
    }
}

/// `nodeType`: the node's kind; no charge.
pub(super) fn node_type(cx: &DomCx<'_, '_>, node: NodeId) -> u16 {
    match cx.tree().data(node) {
        NodeData::Element { .. } => 1,
        NodeData::Text(_) => 3,
        NodeData::ProcessingInstruction { .. } => 7,
        NodeData::Comment(_) => 8,
        NodeData::Document => 9,
        NodeData::Doctype { .. } => 10,
        NodeData::DocumentFragment => 11,
    }
}

/// `nodeName`: as [`tag_name`] for an element; a target or a doctype's name charged as
/// [`copy_of`]; else the kind's fixed name, no charge.
pub(super) fn node_name(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<String> {
    let name = match cx.tree().data(node) {
        NodeData::Element { .. } => return tag_name(cx, node),
        NodeData::ProcessingInstruction { .. } => return copy_of(cx, node, target_of),
        NodeData::Doctype { .. } => return copy_of(cx, node, doctype_name_of),
        NodeData::Text(_) => "#text",
        NodeData::Comment(_) => "#comment",
        NodeData::Document => "#document",
        NodeData::DocumentFragment => "#document-fragment",
    };
    Ok(name.to_string())
}

/// Whether `node` is a document; no charge.
pub(super) fn is_document(cx: &DomCx<'_, '_>, node: NodeId) -> bool {
    matches!(cx.tree().data(node), NodeData::Document)
}

/// `ownerDocument`: the tree's root when it is a document and `node` is not; no charge.
pub(super) fn owner_document(cx: &DomCx<'_, '_>, node: NodeId) -> Option<NodeId> {
    let tree = cx.tree();
    let root = tree.root();
    match (tree.data(node), tree.data(root)) {
        (NodeData::Document, _) => None,
        (_, NodeData::Document) => Some(root),
        (_, _) => None,
    }
}

/// The tree's root, the document; no charge.
pub(super) fn document(cx: &DomCx<'_, '_>) -> NodeId {
    cx.tree().root()
}

/// The parent of `node`; no charge.
pub(super) fn parent(cx: &DomCx<'_, '_>, node: NodeId) -> Option<NodeId> {
    cx.tree().parent(node)
}

/// The parent of `node` when it is an element; no charge.
pub(super) fn parent_element(cx: &DomCx<'_, '_>, node: NodeId) -> Option<NodeId> {
    parent_element_in(cx.tree(), node)
}

/// Whether `node` has a child; no charge.
pub(super) fn has_children(cx: &DomCx<'_, '_>, node: NodeId) -> bool {
    !cx.tree().children(node).is_empty()
}

/// The children of `node`, copied: one step per child, before the copy (the bindings charge
/// the collection they make of them beside it).
pub(super) fn children(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Vec<NodeId>> {
    charge(cx, cx.tree().children(node).len())?;
    Ok(cx.tree().children(node).to_vec())
}

/// The first child of `node`; no charge.
pub(super) fn first_child(cx: &DomCx<'_, '_>, node: NodeId) -> Option<NodeId> {
    cx.tree().children(node).first().copied()
}

/// The last child of `node`; no charge.
pub(super) fn last_child(cx: &DomCx<'_, '_>, node: NodeId) -> Option<NodeId> {
    cx.tree().children(node).last().copied()
}

/// `textContent`: none for a document and a doctype, the data for character data (as
/// [`copied`]), else the data of the texts under `node`, one step per node visited and the
/// texts' bytes as [`gather`] charges them.
pub(super) fn text_content(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Option<String>> {
    match cx.tree().data(node) {
        NodeData::Document | NodeData::Doctype { .. } => Ok(None),
        NodeData::Text(_) | NodeData::Comment(_) | NodeData::ProcessingInstruction { .. } => {
            copied(cx, node)
        }
        NodeData::Element { .. } | NodeData::DocumentFragment => {
            // The descendant text content: the data of the Text nodes under it, in order,
            // one step per node visited; the walk stops at the text that would take the
            // String past `string_length`.
            let mut content = Gathered::new(cx.string_length());
            preorder(cx, node, |cx, next| {
                charge(cx, 1)?;
                if matches!(cx.tree().data(next), NodeData::Text(_)) {
                    gather(cx, &mut content, next)?;
                }
                Ok(false)
            })?;
            Ok(Some(content.text))
        }
    }
}

/// `isEqualNode`: the two trees compared node by node in tree order, each pair charged for both
/// of its nodes before either is read: [`equal_cost`] of each, then the bytes of each one's
/// attributes ([`attribute_byte_steps`]); the walk keeps its place by a stack of (pair, index
/// of the next child), one entry per level, and copies no child list.
pub(super) fn is_equal(cx: &mut DomCx<'_, '_>, this: NodeId, other: NodeId) -> DomResult<bool> {
    let pair_equal = |cx: &mut DomCx<'_, '_>, a: NodeId, b: NodeId| -> DomResult<bool> {
        let known = equal_cost(cx.tree(), a).saturating_add(equal_cost(cx.tree(), b));
        charge(cx, known)?;
        let bytes =
            attribute_byte_steps(cx.tree(), a).saturating_add(attribute_byte_steps(cx.tree(), b));
        charge(cx, bytes)?;
        let tree = cx.tree();
        Ok(shallow_equal(tree, a, b) && tree.children(a).len() == tree.children(b).len())
    };
    if !pair_equal(cx, this, other)? {
        return Ok(false);
    }
    let mut stack: Vec<(NodeId, NodeId, usize)> = vec![(this, other, 0)];
    while let Some(top) = stack.last_mut() {
        let (a, b, next) = *top;
        let tree = cx.tree();
        let (Some(&child_a), Some(&child_b)) =
            (tree.children(a).get(next), tree.children(b).get(next))
        else {
            stack.pop();
            continue;
        };
        top.2 = next + 1;
        if !pair_equal(cx, child_a, child_b)? {
            return Ok(false);
        }
        stack.push((child_a, child_b, 0));
    }
    Ok(true)
}

/// Of `mine` and `theirs`, children of `parent`, the one first among its children: one step
/// per child, charged before it is read.
pub(super) fn first_of_two(
    cx: &mut DomCx<'_, '_>,
    parent: NodeId,
    mine: NodeId,
    theirs: NodeId,
) -> DomResult<Option<NodeId>> {
    // The scan reads on to the second of the two, as it always has, so a child list that does
    // not hold both is read whole.
    let count = cx.tree().children(parent).len();
    let mut first = None;
    for index in 0..count {
        charge(cx, 1)?;
        let child = cx.tree().children(parent)[index];
        if child == mine || child == theirs {
            if first.is_some() {
                break;
            }
            first = Some(child);
        }
    }
    Ok(first)
}

/// Whether `other` is `node` or under it: one step per node visited, as [`find`].
pub(super) fn contains(cx: &mut DomCx<'_, '_>, node: NodeId, other: NodeId) -> DomResult<bool> {
    Ok(find(cx, node, Reads::Names, |_, next, _| next == other)?.is_some())
}

/// `namespaceURI` of an element, none for another node; no charge.
pub(super) fn namespace_uri(cx: &DomCx<'_, '_>, node: NodeId) -> Option<String> {
    match cx.tree().data(node) {
        NodeData::Element { namespace, .. } => Some(namespace.url().to_string()),
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => None,
    }
}

/// `localName` of an element, empty for another node; charged as [`copy_of`].
pub(super) fn local_name(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<String> {
    copy_of(cx, node, local_name_of)
}

/// `tagName`: the HTML-uppercased qualified name, in upper case for an element of the HTML
/// namespace (the document being an HTML document); charged as [`copy_of`], which pays the
/// fold of the copy in place.
pub(super) fn tag_name(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<String> {
    let mut name = copy_of(cx, node, local_name_of)?;
    if html_element_any(cx.tree(), node) {
        name.make_ascii_uppercase();
    }
    Ok(name)
}

/// Whether the document is in quirks mode; no charge.
pub(super) fn quirks(cx: &DomCx<'_, '_>) -> bool {
    cx.tree().quirks_mode() == QuirksMode::Quirks
}

/// The document element of `document`: as [`document_child`].
pub(super) fn document_element(
    cx: &mut DomCx<'_, '_>,
    document: NodeId,
) -> DomResult<Option<NodeId>> {
    document_child(cx, document, is_element)
}

/// The doctype of `document`: as [`document_child`].
pub(super) fn doctype(cx: &mut DomCx<'_, '_>, document: NodeId) -> DomResult<Option<NodeId>> {
    document_child(cx, document, is_doctype)
}

/// `title` (HTML, "document.title"): the child text content of the first `title` element in
/// tree order, with ASCII whitespace stripped and collapsed: the walk charged as [`find`], one
/// step per child of the title, and for each text child one step per 16 bytes and one per word
/// before the collapse reads it.
pub(super) fn title(cx: &mut DomCx<'_, '_>, document: NodeId) -> DomResult<String> {
    let Some(title) = find(cx, document, Reads::Names, |tree, node, _| {
        html_element(tree, node, "title")
    })?
    else {
        return Ok(String::new());
    };
    let children = cx.tree().children(title).len();
    charge(cx, children)?;
    // The collapsed String is gathered word by word, a word running on across two texts,
    // and held to `string_length` as it grows: the limit applies to what `title` answers.
    let mut collapsed = Gathered::new(cx.string_length());
    let mut space = false;
    for index in 0..children {
        let child = cx.tree().children(title)[index];
        let Some(bytes) = text_bytes(cx.tree(), child) else {
            continue;
        };
        // The text's bytes, then its words (a run of other than ASCII whitespace; the pass
        // that counts them is paid by the bytes' charge), before the collapse reads it.
        charge(cx, bytes / BYTES_A_STEP)?;
        let words = text_of(cx.tree(), child).split_ascii_whitespace().count();
        charge(cx, words)?;
        let mut rest = text_of(cx.tree(), child);
        while !rest.is_empty() {
            let word = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
            space |= word.len() < rest.len();
            let end = word
                .find(|c: char| c.is_ascii_whitespace())
                .unwrap_or(word.len());
            if end > 0 {
                let pieces = [" ", &word[..end]];
                let first = usize::from(!space || collapsed.text.is_empty());
                for piece in &pieces[first..] {
                    collapsed.admit(piece)?;
                    collapsed.append(piece);
                }
                space = false;
            }
            rest = &word[end..];
        }
    }
    Ok(collapsed.text)
}

/// The bytes of a text's data, none for another node.
fn text_bytes(tree: &Tree, node: NodeId) -> Option<usize> {
    match tree.data(node) {
        NodeData::Text(data) => Some(data.len()),
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Element { .. }
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => None,
    }
}

/// A text's data, empty for another node.
fn text_of(tree: &Tree, node: NodeId) -> &str {
    match tree.data(node) {
        NodeData::Text(data) => data,
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Element { .. }
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => "",
    }
}

/// `length` of character data, in UTF-16 code units: one step per 16 bytes of the data,
/// before they are counted.
pub(super) fn length(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<u32> {
    let bytes = character_data(cx.tree(), node).map_or(0, str::len);
    charge(cx, bytes / BYTES_A_STEP)?;
    let data = character_data(cx.tree(), node).unwrap_or_default();
    let units: usize = data.chars().map(char::len_utf16).sum();
    Ok(u32::try_from(units).unwrap_or(u32::MAX))
}

/// `substringData(offset, count)` (DOM, "substring data", in UTF-16 code units): one step
/// per 16 of `offset + count`, at most the data's bytes, before the walk.
pub(super) fn substring_data(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    offset: u32,
    count: u32,
) -> DomResult<Vec<u16>> {
    // An offset or an end inside a surrogate pair answers its lone half. The data is walked
    // to the offset a unit at a time, and only the answer is allocated, up to
    // `string_length` units. The data's bytes bound the units the walk can pass (a unit
    // stands for one byte of UTF-8 or more).
    let limit = cx.string_length();
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    let bytes = character_data(cx.tree(), node).map_or(0, str::len);
    charge(cx, offset.saturating_add(count).min(bytes) / BYTES_A_STEP)?;
    let data = character_data(cx.tree(), node).unwrap_or_default();
    let mut units = data.encode_utf16();
    let mut past_the_end = false;
    for _ in 0..offset {
        if units.next().is_none() {
            past_the_end = true;
            break;
        }
    }
    let mut answer = Vec::new();
    let mut over = false;
    if !past_the_end {
        for unit in units.take(count) {
            if u64::try_from(answer.len()).unwrap_or(u64::MAX) >= limit {
                over = true;
                break;
            }
            answer.push(unit);
        }
    }
    if past_the_end {
        return Err(DomEnd::Throw(DomException::new(
            "IndexSizeError",
            "the offset is past the end of the data",
        )));
    }
    if over {
        return Err(DomEnd::StringLength);
    }
    Ok(answer)
}

/// `wholeText` (DOM, "contiguous Text nodes"): the Text siblings next to `node`, without
/// another kind between, itself included, in order: [`index_in`]'s charge, one step per
/// sibling read on each side, each charged before it is read, and the texts' bytes as
/// [`gather`] charges them.
pub(super) fn whole_text(cx: &mut DomCx<'_, '_>, hints: &Hints, node: NodeId) -> DomResult<String> {
    let Some(parent) = cx.tree().parent(node) else {
        return copied(cx, node).map(Option::unwrap_or_default);
    };
    let index = index_in(cx, hints, parent, node)?;
    let is_text = |cx: &DomCx<'_, '_>, at: usize| {
        let child = cx.tree().children(parent)[at];
        matches!(cx.tree().data(child), NodeData::Text(_))
    };
    let mut first = index;
    while first > 0 {
        charge(cx, 1)?;
        if !is_text(cx, first - 1) {
            break;
        }
        first -= 1;
    }
    let count = cx.tree().children(parent).len();
    let mut last = index + 1;
    while last < count {
        charge(cx, 1)?;
        if !is_text(cx, last) {
            break;
        }
        last += 1;
    }
    // The run stops at the text that would take the String past `string_length`.
    let mut whole = Gathered::new(cx.string_length());
    for at in first..last {
        let node = cx.tree().children(parent)[at];
        gather(cx, &mut whole, node)?;
    }
    Ok(whole.text)
}

/// `target` of a processing instruction, empty for another node; charged as [`copy_of`].
pub(super) fn target(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<String> {
    copy_of(cx, node, target_of)
}

/// Which field of a doctype [`doctype_field`] reads.
#[derive(Clone, Copy)]
pub(super) enum DoctypeField {
    Name,
    PublicId,
    SystemId,
}

/// A doctype's `name`, `publicId` or `systemId`, empty for another node; charged as
/// [`copy_of`].
pub(super) fn doctype_field(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    field: DoctypeField,
) -> DomResult<String> {
    let pick = match field {
        DoctypeField::Name => doctype_name_of,
        DoctypeField::PublicId => public_id_of,
        DoctypeField::SystemId => system_id_of,
    };
    copy_of(cx, node, pick)
}

/// `namedItem(name)` (DOM): the first of `nodes` whose ID is `name` or, in the HTML
/// namespace, whose `name` attribute is; one step per element and one per 4 attributes it
/// passes before the pass, one per 16 bytes of each value compared before the compare.
pub(super) fn named_item(
    cx: &mut DomCx<'_, '_>,
    nodes: &[NodeId],
    name: &str,
) -> DomResult<Option<NodeId>> {
    for &node in nodes {
        charge(cx, 1 + attribute_count(cx.tree(), node) / ATTRIBUTES_A_STEP)?;
        let tree = cx.tree();
        let html = matches!(
            tree.data(node),
            NodeData::Element {
                namespace: Namespace::Html,
                ..
            }
        );
        let [id, named] = positions(tree, node, ["id", "name"]);
        let named = named.filter(|_| html);
        for at in [id, named].into_iter().flatten() {
            charge(cx, value_at(cx.tree(), node, at).len() / BYTES_A_STEP)?;
        }
        let tree = cx.tree();
        if [id, named]
            .into_iter()
            .flatten()
            .any(|at| value_at(tree, node, at) == name)
        {
            return Ok(Some(node));
        }
    }
    Ok(None)
}
