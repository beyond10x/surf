//! The slice `reading` (`story:scripts-dom-reading`): the members that read the document's
//! tree and never write it, as the DOM Standard (the HTML Standard for `Window` and
//! `Document.title`) says, with the deviations `crates/webidl/selection.txt` names.
//!
//! Steps. The bindings charge one step per call. A member whose work grows with the tree
//! charges one step per node it visits and one per 64 bytes of a String it builds, before the
//! work, so a script over a large tree ends at its step limit and not at the wall clock. The
//! walks keep their own list of nodes, so a deep tree does not deepen the call stack. A member
//! that looks for attributes by name charges one step per 64 attributes it passes and one per 64
//! bytes of each value it reads or compares; one that reads attributes whole (a compare of two
//! lists, a scan by value) charges one step per attribute and per 64 bytes of their names and
//! values. Either is linear in what it charges.
//!
//! Strings. A String a member answers with more UTF-16 code units than the agent's
//! `string_length` is the realm's `RangeError`; the bindings check every answer before the
//! engine copies it. The members here do not build what would be thrown away: `data`,
//! `nodeValue` and a text's `textContent` check the data before they copy it, and the members
//! that concatenate (`textContent`, `wholeText`, `title`) stop at the piece that would take the
//! String past the limit, with the steps of what they visited charged.
//!
//! Siblings. The tree has no sibling read: a sibling is found by the node's index among its
//! parent's children. [`SurfReading`] keeps two such indices (the two last answered, so a parent
//! read from both ends keeps one for each end) for the last eight parents it answered
//! about ([`Hints`]). A hint is never trusted: the child at the hinted index, the one before it
//! and the one after it are compared with the node, and a miss scans the parent's children at
//! one step per node. So a walk over the children costs one step per sibling read, and a write
//! between two reads costs a scan, never a wrong answer.

use std::collections::HashMap;

use surf_html::{Namespace, NodeData, NodeId, QuirksMode, Tree};
use surf_url::Url;

use crate::bindings::{
    DomCx, DomEnd, DomException, DomResult, GetRootNodeOptions, Reading, TheWindow, longer_than,
    utf16_units,
};

use super::DocumentInfo;

/// How many parents the sibling hints remember.
const HINTS: usize = 8;

/// Two positions among a parent's children, the two last a sibling read answered about, so a
/// parent read from both ends keeps a hint for each end; the oldest parent is replaced first.
#[derive(Debug, Default)]
struct Hints {
    entries: [Option<(NodeId, [usize; 2])>; HINTS],
    next: usize,
}

impl Hints {
    /// The two hinted positions for `parent`, if any.
    fn get(&self, parent: NodeId) -> Option<[usize; 2]> {
        self.entries
            .iter()
            .flatten()
            .find(|(hinted, _)| *hinted == parent)
            .map(|&(_, positions)| positions)
    }

    /// Remembers `index` for `parent`: in place of the nearer of its two positions when
    /// `parent` has an entry (the read went on from there), else as both positions of a new
    /// entry in place of the oldest.
    fn set(&mut self, parent: NodeId, index: usize) {
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

    /// The index of `node` among the children of `parent`: a hint's, or a scan's at one step
    /// per node visited.
    fn index_in(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        parent: NodeId,
        node: NodeId,
    ) -> DomResult<usize> {
        let children = cx.tree().children(parent);
        for hint in self.hints.get(parent).into_iter().flatten() {
            for candidate in [Some(hint), hint.checked_add(1), hint.checked_sub(1)]
                .into_iter()
                .flatten()
            {
                if children.get(candidate) == Some(&node) {
                    return Ok(candidate);
                }
            }
        }
        let found = children.iter().position(|&child| child == node);
        let visited = found.map_or(children.len(), |index| index + 1);
        charge(cx, visited)?;
        found.ok_or_else(|| {
            DomEnd::Throw(DomException::new(
                "InvalidStateError",
                "a node is not among its parent's children",
            ))
        })
    }

    /// The sibling of `node` one place before it (`forward` false) or after it, of any kind
    /// (`elements` false) or the nearest element, at one step per node passed.
    fn sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        node: NodeId,
        forward: bool,
        elements: bool,
    ) -> DomResult<Option<NodeId>> {
        let Some(parent) = cx.tree().parent(node) else {
            return Ok(None);
        };
        let mut index = self.index_in(cx, parent, node)?;
        self.hints.set(parent, index);
        let mut passed = 0;
        let answer = loop {
            let next = if forward {
                index.checked_add(1)
            } else {
                index.checked_sub(1)
            };
            let Some(next) = next else {
                break None;
            };
            let Some(&child) = cx.tree().children(parent).get(next) else {
                break None;
            };
            index = next;
            if !elements || is_element(cx.tree(), child) {
                break Some(child);
            }
            passed += 1;
            if passed % 64 == 0 {
                charge(cx, 64)?;
            }
        };
        charge(cx, passed % 64)?;
        if answer.is_some() {
            self.hints.set(parent, index);
        }
        Ok(answer)
    }

    /// The document base URL (HTML, "document base URL"): the frozen base URL of the first
    /// `base` element of the HTML namespace with an `href` in tree order, else the fallback
    /// base URL, the document's address. A `href` that does not parse against the address, is
    /// longer than the URL parser reads, or gives a `data` or `javascript` URL gives the address.
    fn base_url(&self, cx: &mut DomCx<'_, '_>) -> DomResult<String> {
        let root = cx.tree().root();
        // The walk passes the attributes' names; the one `href` it reads is charged below.
        let base = find(cx, root, Reads::Named(&[]), |tree, node| {
            html_element(tree, node, "base") && attribute(tree, node, "href").is_some()
        })?;
        let href = base.and_then(|base| attribute(cx.tree(), base, "href").map(str::to_string));
        let Some(href) = href else {
            return Ok(self.info.address.href());
        };
        charge(cx, href.len() / 64)?;
        // HTML, "set the frozen base URL": a href that does not parse, or parses to a URL whose
        // scheme is `data` or `javascript`, gives the fallback base URL; a later `base` is not
        // read instead.
        let url = match Url::parse(&href, Some(&self.info.address)) {
            Ok(url) if !matches!(url.scheme(), "data" | "javascript") => url,
            Ok(_) | Err(_) => self.info.address.clone(),
        };
        text(cx, url.href())
    }
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

/// A String a member answers, charged at one step per 64 bytes.
fn text(cx: &mut DomCx<'_, '_>, text: String) -> DomResult<String> {
    charge(cx, text.len() / 64)?;
    Ok(text)
}

/// The character data of `node` as `data`, `nodeValue` and `textContent` answer it, `None` for
/// a node without: past the agent's `string_length` the `RangeError`, before it is charged or
/// copied; else charged as [`text`], then copied.
fn copied(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Option<String>> {
    let limit = cx.string_length();
    let Some(data) = character_data(cx.tree(), node) else {
        return Ok(None);
    };
    if longer_than(data, limit) {
        return Err(DomEnd::StringLength);
    }
    let steps = data.len() / 64;
    charge(cx, steps)?;
    Ok(character_data(cx.tree(), node).map(str::to_string))
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

    /// The steps of appending `bytes` more: one per 64 bytes of the whole.
    fn steps(&self, bytes: usize) -> usize {
        (self.text.len() + bytes) / 64 - self.text.len() / 64
    }

    fn append(&mut self, piece: &str) {
        self.text.push_str(piece);
    }
}

/// Appends the data of the text `node` to `gathered`: admitted, charged, then copied.
fn gather(cx: &mut DomCx<'_, '_>, gathered: &mut Gathered, node: NodeId) -> DomResult<()> {
    let data = character_data(cx.tree(), node).unwrap_or_default();
    gathered.admit(data)?;
    let steps = gathered.steps(data.len());
    charge(cx, steps)?;
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

/// The value of `node`'s attribute `name` in no namespace, if it is an element with one.
fn attribute<'t>(tree: &'t Tree, node: NodeId, name: &str) -> Option<&'t str> {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes
            .iter()
            .find(|attribute| attribute.namespace.is_none() && attribute.local_name == name)
            .map(|attribute| attribute.value.as_str()),
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => None,
    }
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
    /// Also the attributes of the given names, found by name: one step a node, plus
    /// [`named_steps`].
    Named(&'static [&'static str]),
}

/// The steps of a scan of `node`'s attributes for those of `names` in no namespace: one per 64
/// attributes passed, and one per 64 bytes of each value of those names, which the scan reads or
/// compares. The bytes of the other attributes are not read and not charged.
fn named_steps(tree: &Tree, node: NodeId, names: &[&str]) -> usize {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => {
            let values: usize = attributes
                .iter()
                .filter(|attribute| {
                    attribute.namespace.is_none() && names.contains(&attribute.local_name.as_str())
                })
                .map(|attribute| attribute.value.len() / 64)
                .sum();
            attributes.len() / 64 + values
        }
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => 0,
    }
}

/// The steps of reading `node`'s attributes whole: one per attribute and one per 64 bytes of
/// their names and values, so a compare of whole lists or a scan by value is bounded by its
/// charge.
fn attribute_steps(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => {
            let bytes: usize = attributes
                .iter()
                .map(|attribute| attribute.local_name.len() + attribute.value.len())
                .sum();
            attributes.len() + bytes / 64
        }
        NodeData::Document
        | NodeData::DocumentFragment
        | NodeData::Doctype { .. }
        | NodeData::Text(_)
        | NodeData::Comment(_)
        | NodeData::ProcessingInstruction { .. } => 0,
    }
}

/// `node` and the nodes under it in tree order, each charged before it is visited (one step,
/// and its attributes when the walk `reads` them), until `stop` answers true for one: that
/// node.
fn find(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    reads: Reads,
    mut stop: impl FnMut(&Tree, NodeId) -> bool,
) -> DomResult<Option<NodeId>> {
    let mut pending = vec![node];
    while let Some(next) = pending.pop() {
        let steps = match reads {
            Reads::Names => 1,
            Reads::Named(names) => 1 + named_steps(cx.tree(), next, names),
        };
        charge(cx, steps)?;
        let tree = cx.tree();
        if stop(tree, next) {
            return Ok(Some(next));
        }
        pending.extend(tree.children(next).iter().rev());
    }
    Ok(None)
}

/// The nodes under `node`, `node` left out, for which `keep` answers true, in tree order;
/// charged as [`find`].
fn select(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    reads: Reads,
    mut keep: impl FnMut(&Tree, NodeId) -> bool,
) -> DomResult<Vec<NodeId>> {
    let mut kept = Vec::new();
    find(cx, node, reads, |tree, next| {
        if next != node && keep(tree, next) {
            kept.push(next);
        }
        false
    })?;
    Ok(kept)
}

/// The elements under `node` for `getElementsByTagName(qualified_name)` (DOM, "list of
/// elements with qualified name"): every element for `*`; else, in this HTML document, an
/// element of the HTML namespace whose name is the name in ASCII lower case, and any other
/// whose name is the name.
fn by_tag_name(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    qualified_name: &str,
) -> DomResult<Vec<NodeId>> {
    let lower = qualified_name.to_ascii_lowercase();
    select(cx, node, Reads::Names, |tree, next| match tree.data(next) {
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
    })
}

/// The elements under `node` for `getElementsByTagNameNS(namespace, local_name)` (DOM, "list
/// of elements with namespace namespace and local name localName"); the empty namespace is
/// null, which no element of the tree has.
fn by_tag_name_ns(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    namespace: Option<String>,
    local_name: &str,
) -> DomResult<Vec<NodeId>> {
    let namespace = namespace.filter(|namespace| !namespace.is_empty());
    select(cx, node, Reads::Names, |tree, next| match tree.data(next) {
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
    })
}

/// The elements under `node` for `getElementsByClassName(class_names)` (DOM, "list of elements
/// with class names"): those with every class the names give, compared ASCII case-insensitively
/// in quirks mode; none for names that give no class.
fn by_class_name(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    class_names: &str,
) -> DomResult<Vec<NodeId>> {
    // The wanted classes: an ordered set (duplicates dropped), built once, one step per 64
    // bytes of the argument; in quirks mode each in ASCII lower case.
    charge(cx, class_names.len() / 64)?;
    let quirks = cx.tree().quirks_mode() == QuirksMode::Quirks;
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
    // An element matches when its classes hold every wanted one: one set lookup per class, so
    // the match is linear in the bytes of its `class`, which the walk charges before it.
    let mut matched_by = vec![0usize; wanted.len()];
    let mut element = 0usize;
    let mut folded = String::new();
    select(cx, node, Reads::Named(&["class"]), |tree, next| {
        let Some(classes) = attribute(tree, next, "class") else {
            return false;
        };
        element += 1;
        let mut matched = 0;
        for class in classes.split_ascii_whitespace() {
            let key = if quirks {
                folded.clear();
                folded.extend(class.chars().map(|c| c.to_ascii_lowercase()));
                folded.as_str()
            } else {
                class
            };
            if let Some(&at) = wanted.get(key)
                && matched_by[at] != element
            {
                matched_by[at] = element;
                matched += 1;
            }
        }
        matched == wanted.len()
    })
}

/// The first element under `node`, `node` left out, whose ID is `id`; none for the empty ID.
fn by_id(cx: &mut DomCx<'_, '_>, node: NodeId, id: &str) -> DomResult<Option<NodeId>> {
    if id.is_empty() {
        return Ok(None);
    }
    find(cx, node, Reads::Named(&["id"]), |tree, next| {
        next != node && attribute(tree, next, "id") == Some(id)
    })
}

/// The element children of `node`, one step per child.
fn element_children(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Vec<NodeId>> {
    charge(cx, cx.tree().children(node).len())?;
    let tree = cx.tree();
    Ok(tree
        .children(node)
        .iter()
        .copied()
        .filter(|&child| is_element(tree, child))
        .collect())
}

/// The first element child of `node` (`last` false) or the last, one step per child passed.
fn edge_element_child(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    last: bool,
) -> DomResult<Option<NodeId>> {
    let tree = cx.tree();
    let children = tree.children(node);
    let found = if last {
        children.iter().rposition(|&child| is_element(tree, child))
    } else {
        children.iter().position(|&child| is_element(tree, child))
    };
    let passed = match (found, last) {
        (Some(index), false) => index,
        (Some(index), true) => children.len() - 1 - index,
        (None, _) => children.len(),
    };
    let answer = found.map(|index| children[index]);
    charge(cx, passed)?;
    Ok(answer)
}

/// The count of `node`'s element children, one step per child.
fn element_count(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<u32> {
    let count = element_children(cx, node)?.len();
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

/// The ancestors of `node`, itself first and the root last, one step per node.
fn ancestors(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Vec<NodeId>> {
    let mut chain = vec![node];
    let mut at = node;
    while let Some(parent) = cx.tree().parent(at) {
        chain.push(parent);
        at = parent;
    }
    charge(cx, chain.len())?;
    Ok(chain)
}

/// The root of `node`, one step per ancestor.
fn root_of(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<NodeId> {
    let chain = ancestors(cx, node)?;
    Ok(chain.last().copied().unwrap_or(node))
}

/// The parent of `node` when it is an element.
fn parent_element(tree: &Tree, node: NodeId) -> Option<NodeId> {
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
            Ok(parent_element(tree, node))
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
fn locate_namespace(
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
    while let Some(element) = at {
        let steps = 1 + attribute_steps(cx.tree(), element);
        charge(cx, steps)?;
        let tree = cx.tree();
        let NodeData::Element {
            namespace,
            attributes,
            ..
        } = tree.data(element)
        else {
            break;
        };
        // An element of the tree has no prefix: it matches the null prefix only.
        if prefix.is_none() {
            return Ok(Some(namespace.url().to_string()));
        }
        let declared = attributes.iter().find(|attribute| {
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
            return Ok(Some(declared.value.clone()).filter(|value| !value.is_empty()));
        }
        at = parent_element(tree, element);
    }
    Ok(None)
}

/// DOM, "locate a namespace prefix" for `namespace`, from `node`'s start, one step per element
/// passed and its attributes.
fn locate_prefix(
    cx: &mut DomCx<'_, '_>,
    node: NodeId,
    namespace: &str,
) -> DomResult<Option<String>> {
    let mut at = lookup_start(cx, node)?;
    while let Some(element) = at {
        let steps = 1 + attribute_steps(cx.tree(), element);
        charge(cx, steps)?;
        let tree = cx.tree();
        let NodeData::Element { attributes, .. } = tree.data(element) else {
            break;
        };
        let declared = attributes.iter().find(|attribute| {
            attribute.prefix.as_deref() == Some("xmlns") && attribute.value == namespace
        });
        if let Some(declared) = declared {
            return Ok(Some(declared.local_name.clone()));
        }
        at = parent_element(tree, element);
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
            // hash and one lookup per attribute, linear in their bytes. The lengths first, so
            // the other list is read no further than this one, which the walk charged.
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

/// The steps of comparing `node` with another: one, and one per attribute and per 64 bytes of
/// the names, values and data compared. A compare of two strings reads no further than the
/// shorter, so the bytes of `node`'s side bound it.
fn equal_cost(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element { local_name, .. } => {
            1 + local_name.len() / 64 + attribute_steps(tree, node)
        }
        NodeData::Text(data) | NodeData::Comment(data) => 1 + data.len() / 64,
        NodeData::ProcessingInstruction { target, data } => 1 + (target.len() + data.len()) / 64,
        NodeData::Doctype {
            name,
            public_id,
            system_id,
        } => 1 + (name.len() + public_id.len() + system_id.len()) / 64,
        NodeData::Document | NodeData::DocumentFragment => 1,
    }
}

impl Reading for SurfReading {
    fn node_node_type(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<u16> {
        Ok(match cx.tree().data(this) {
            NodeData::Element { .. } => 1,
            NodeData::Text(_) => 3,
            NodeData::ProcessingInstruction { .. } => 7,
            NodeData::Comment(_) => 8,
            NodeData::Document => 9,
            NodeData::Doctype { .. } => 10,
            NodeData::DocumentFragment => 11,
        })
    }

    fn node_node_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let name = match cx.tree().data(this) {
            NodeData::Element { .. } => return self.element_tag_name(cx, this),
            NodeData::Text(_) => "#text".to_string(),
            NodeData::ProcessingInstruction { target, .. } => target.clone(),
            NodeData::Comment(_) => "#comment".to_string(),
            NodeData::Document => "#document".to_string(),
            NodeData::Doctype { name, .. } => name.clone(),
            NodeData::DocumentFragment => "#document-fragment".to_string(),
        };
        text(cx, name)
    }

    fn node_base_uri(&mut self, cx: &mut DomCx<'_, '_>, _this: NodeId) -> DomResult<String> {
        self.base_url(cx)
    }

    fn node_is_connected(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<bool> {
        let root = root_of(cx, this)?;
        Ok(matches!(cx.tree().data(root), NodeData::Document))
    }

    fn node_owner_document(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let tree = cx.tree();
        let root = tree.root();
        Ok(match (tree.data(this), tree.data(root)) {
            (NodeData::Document, _) => None,
            (_, NodeData::Document) => Some(root),
            (_, _) => None,
        })
    }

    fn node_get_root_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        _options: GetRootNodeOptions,
    ) -> DomResult<NodeId> {
        // No shadow trees: the shadow-including root is the root, whatever `composed` is.
        root_of(cx, this)
    }

    fn node_parent_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        Ok(cx.tree().parent(this))
    }

    fn node_parent_element(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        Ok(parent_element(cx.tree(), this))
    }

    fn node_has_child_nodes(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<bool> {
        Ok(!cx.tree().children(this).is_empty())
    }

    fn node_child_nodes(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<Vec<NodeId>> {
        // The bindings charge a step per node of the collection they make.
        Ok(cx.tree().children(this).to_vec())
    }

    fn node_first_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        Ok(cx.tree().children(this).first().copied())
    }

    fn node_last_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        Ok(cx.tree().children(this).last().copied())
    }

    fn node_previous_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        self.sibling(cx, this, false, false)
    }

    fn node_next_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        self.sibling(cx, this, true, false)
    }

    fn node_node_value(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<String>> {
        copied(cx, this)
    }

    fn node_text_content(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<String>> {
        match cx.tree().data(this) {
            NodeData::Document | NodeData::Doctype { .. } => Ok(None),
            NodeData::Text(_) | NodeData::Comment(_) | NodeData::ProcessingInstruction { .. } => {
                self.node_node_value(cx, this)
            }
            NodeData::Element { .. } | NodeData::DocumentFragment => {
                // The descendant text content: the data of the Text nodes under it, in order,
                // one step per node visited; the walk stops at the text that would take the
                // String past `string_length`.
                let mut content = Gathered::new(cx.string_length());
                let mut pending = vec![this];
                while let Some(next) = pending.pop() {
                    charge(cx, 1)?;
                    let tree = cx.tree();
                    pending.extend(tree.children(next).iter().rev());
                    if matches!(tree.data(next), NodeData::Text(_)) {
                        gather(cx, &mut content, next)?;
                    }
                }
                let content = content.text;
                Ok(Some(content))
            }
        }
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
        let mut pending = vec![(this, other)];
        while let Some((a, b)) = pending.pop() {
            let cost = equal_cost(cx.tree(), a);
            charge(cx, cost)?;
            let tree = cx.tree();
            if !shallow_equal(tree, a, b) || tree.children(a).len() != tree.children(b).len() {
                return Ok(false);
            }
            pending.extend(
                tree.children(a)
                    .iter()
                    .copied()
                    .zip(tree.children(b).iter().copied())
                    .rev(),
            );
        }
        Ok(true)
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
        let mut mine = ancestors(cx, this)?;
        let mut theirs = ancestors(cx, other)?;
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
        let children = cx.tree().children(parent);
        let mut visited = 0;
        let mut first = None;
        for &child in children {
            visited += 1;
            if child == my_child || child == their_child {
                if first.is_some() {
                    break;
                }
                first = Some(child);
            }
        }
        charge(cx, visited)?;
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
        // An inclusive descendant: the nodes under `this` in tree order, one step each.
        Ok(find(cx, this, Reads::Names, |_, node| node == other)?.is_some())
    }

    fn node_lookup_prefix(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
    ) -> DomResult<Option<String>> {
        match namespace.filter(|namespace| !namespace.is_empty()) {
            Some(namespace) => locate_prefix(cx, this, &namespace),
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
        locate_namespace(cx, this, prefix.as_deref())
    }

    fn node_is_default_namespace(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
    ) -> DomResult<bool> {
        let namespace = namespace.filter(|namespace| !namespace.is_empty());
        Ok(locate_namespace(cx, this, None)? == namespace)
    }

    fn element_namespace_uri(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<String>> {
        Ok(match cx.tree().data(this) {
            NodeData::Element { namespace, .. } => Some(namespace.url().to_string()),
            NodeData::Document
            | NodeData::DocumentFragment
            | NodeData::Doctype { .. }
            | NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::ProcessingInstruction { .. } => None,
        })
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
        let name = match cx.tree().data(this) {
            NodeData::Element { local_name, .. } => local_name.clone(),
            NodeData::Document
            | NodeData::DocumentFragment
            | NodeData::Doctype { .. }
            | NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::ProcessingInstruction { .. } => String::new(),
        };
        text(cx, name)
    }

    fn element_tag_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        // The HTML-uppercased qualified name: in upper case for an element of the HTML
        // namespace, the document being an HTML document.
        let name = match cx.tree().data(this) {
            NodeData::Element {
                namespace: Namespace::Html,
                local_name,
                ..
            } => local_name.to_ascii_uppercase(),
            NodeData::Element { local_name, .. } => local_name.clone(),
            NodeData::Document
            | NodeData::DocumentFragment
            | NodeData::Doctype { .. }
            | NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::ProcessingInstruction { .. } => String::new(),
        };
        text(cx, name)
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
        by_tag_name(cx, this, &qualified_name)
    }

    fn element_get_elements_by_tag_name_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<Vec<NodeId>> {
        by_tag_name_ns(cx, this, namespace, &local_name)
    }

    fn element_get_elements_by_class_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        class_names: String,
    ) -> DomResult<Vec<NodeId>> {
        by_class_name(cx, this, &class_names)
    }

    fn element_children(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<Vec<NodeId>> {
        element_children(cx, this)
    }

    fn element_first_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        edge_element_child(cx, this, false)
    }

    fn element_last_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        edge_element_child(cx, this, true)
    }

    fn element_child_element_count(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<u32> {
        element_count(cx, this)
    }

    fn element_previous_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        self.sibling(cx, this, false, true)
    }

    fn element_next_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        self.sibling(cx, this, true, true)
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
        text(cx, self.info.address.href())
    }

    fn document_document_uri(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<String> {
        text(cx, self.info.address.href())
    }

    fn document_compat_mode(&mut self, cx: &mut DomCx<'_, '_>, _this: NodeId) -> DomResult<String> {
        Ok(match cx.tree().quirks_mode() {
            QuirksMode::Quirks => "BackCompat",
            QuirksMode::LimitedQuirks | QuirksMode::NoQuirks => "CSS1Compat",
        }
        .to_string())
    }

    fn document_character_set(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<String> {
        text(cx, self.info.encoding.clone())
    }

    fn document_charset(&mut self, cx: &mut DomCx<'_, '_>, _this: NodeId) -> DomResult<String> {
        text(cx, self.info.encoding.clone())
    }

    fn document_input_encoding(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<String> {
        text(cx, self.info.encoding.clone())
    }

    fn document_content_type(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        _this: NodeId,
    ) -> DomResult<String> {
        text(cx, self.info.content_type.clone())
    }

    fn document_doctype(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        document_child(cx, this, is_doctype)
    }

    fn document_document_element(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        document_child(cx, this, is_element)
    }

    fn document_get_elements_by_tag_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<Vec<NodeId>> {
        by_tag_name(cx, this, &qualified_name)
    }

    fn document_get_elements_by_tag_name_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<Vec<NodeId>> {
        by_tag_name_ns(cx, this, namespace, &local_name)
    }

    fn document_get_elements_by_class_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        class_names: String,
    ) -> DomResult<Vec<NodeId>> {
        by_class_name(cx, this, &class_names)
    }

    fn document_get_element_by_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        element_id: String,
    ) -> DomResult<Option<NodeId>> {
        by_id(cx, this, &element_id)
    }

    fn document_children(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Vec<NodeId>> {
        element_children(cx, this)
    }

    fn document_first_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        edge_element_child(cx, this, false)
    }

    fn document_last_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        edge_element_child(cx, this, true)
    }

    fn document_child_element_count(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<u32> {
        element_count(cx, this)
    }

    fn document_title(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        // HTML, "document.title": the child text content of the first `title` element in tree
        // order, with ASCII whitespace stripped and collapsed; the empty string without one.
        let Some(title) = find(cx, this, Reads::Names, |tree, node| {
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
        let mut over = false;
        let tree = cx.tree();
        'texts: for &child in tree.children(title) {
            let NodeData::Text(data) = tree.data(child) else {
                continue;
            };
            let mut rest: &str = data;
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
                        if collapsed.admit(piece).is_err() {
                            over = true;
                            break 'texts;
                        }
                        collapsed.append(piece);
                    }
                    space = false;
                }
                rest = &word[end..];
            }
        }
        let steps = collapsed.text.len() / 64;
        charge(cx, steps)?;
        if over {
            return Err(DomEnd::StringLength);
        }
        Ok(collapsed.text)
    }

    fn character_data_data(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        copied(cx, this).map(Option::unwrap_or_default)
    }

    fn character_data_length(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<u32> {
        let data = character_data(cx.tree(), this).unwrap_or_default();
        let bytes = data.len();
        let units: usize = data.chars().map(char::len_utf16).sum();
        charge(cx, bytes / 64)?;
        Ok(u32::try_from(units).unwrap_or(u32::MAX))
    }

    fn character_data_substring_data(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        offset: u32,
        count: u32,
    ) -> DomResult<Vec<u16>> {
        // DOM, "substring data", in UTF-16 code units: an offset or an end inside a surrogate
        // pair answers its lone half. The data is walked to the offset a unit at a time, and
        // only the answer is allocated, up to `string_length` units. Charged before the walk:
        // one step per 64 of `offset + count`, at most the data's bytes, which bound the units
        // the walk can pass (a unit stands for one byte of UTF-8 or more).
        let limit = cx.string_length();
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        let count = usize::try_from(count).unwrap_or(usize::MAX);
        let bytes = character_data(cx.tree(), this).map_or(0, str::len);
        charge(cx, offset.saturating_add(count).min(bytes) / 64)?;
        let data = character_data(cx.tree(), this).unwrap_or_default();
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

    fn character_data_previous_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        self.sibling(cx, this, false, true)
    }

    fn character_data_next_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        self.sibling(cx, this, true, true)
    }

    fn text_whole_text(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        // DOM, "contiguous Text nodes": the Text siblings next to it, without another kind
        // between, itself included, in order.
        let Some(parent) = cx.tree().parent(this) else {
            return self.character_data_data(cx, this);
        };
        let index = self.index_in(cx, parent, this)?;
        let tree = cx.tree();
        let children = tree.children(parent);
        let is_text = |node: NodeId| matches!(tree.data(node), NodeData::Text(_));
        let first = children[..index]
            .iter()
            .rposition(|&node| !is_text(node))
            .map_or(0, |at| at + 1);
        let last = children[index..]
            .iter()
            .position(|&node| !is_text(node))
            .map_or(children.len(), |at| index + at);
        let run: Vec<NodeId> = children[first..last].to_vec();
        // One step per text visited; the run stops at the text that would take the String
        // past `string_length`.
        let mut whole = Gathered::new(cx.string_length());
        for node in run {
            charge(cx, 1)?;
            gather(cx, &mut whole, node)?;
        }
        Ok(whole.text)
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
        let target = match cx.tree().data(this) {
            NodeData::ProcessingInstruction { target, .. } => target.clone(),
            NodeData::Document
            | NodeData::DocumentFragment
            | NodeData::Doctype { .. }
            | NodeData::Element { .. }
            | NodeData::Text(_)
            | NodeData::Comment(_) => String::new(),
        };
        text(cx, target)
    }

    fn document_type_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let name = match cx.tree().data(this) {
            NodeData::Doctype { name, .. } => name.clone(),
            NodeData::Document
            | NodeData::DocumentFragment
            | NodeData::Element { .. }
            | NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::ProcessingInstruction { .. } => String::new(),
        };
        text(cx, name)
    }

    fn document_type_public_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<String> {
        let id = match cx.tree().data(this) {
            NodeData::Doctype { public_id, .. } => public_id.clone(),
            NodeData::Document
            | NodeData::DocumentFragment
            | NodeData::Element { .. }
            | NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::ProcessingInstruction { .. } => String::new(),
        };
        text(cx, id)
    }

    fn document_type_system_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<String> {
        let id = match cx.tree().data(this) {
            NodeData::Doctype { system_id, .. } => system_id.clone(),
            NodeData::Document
            | NodeData::DocumentFragment
            | NodeData::Element { .. }
            | NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::ProcessingInstruction { .. } => String::new(),
        };
        text(cx, id)
    }

    fn document_fragment_get_element_by_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        element_id: String,
    ) -> DomResult<Option<NodeId>> {
        by_id(cx, this, &element_id)
    }

    fn document_fragment_children(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Vec<NodeId>> {
        element_children(cx, this)
    }

    fn document_fragment_first_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        edge_element_child(cx, this, false)
    }

    fn document_fragment_last_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        edge_element_child(cx, this, true)
    }

    fn document_fragment_child_element_count(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<u32> {
        element_count(cx, this)
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
        // One step per element visited, and its `id` and `name` found by name, before it is read.
        for &node in this {
            let steps = 1 + named_steps(cx.tree(), node, &["id", "name"]);
            charge(cx, steps)?;
            let tree = cx.tree();
            if attribute(tree, node, "id") == Some(name.as_str())
                || (matches!(
                    tree.data(node),
                    NodeData::Element {
                        namespace: Namespace::Html,
                        ..
                    }
                ) && attribute(tree, node, "name") == Some(name.as_str()))
            {
                return Ok(Some(node));
            }
        }
        Ok(None)
    }

    fn window_window(&mut self, _cx: &mut DomCx<'_, '_>) -> DomResult<TheWindow> {
        Ok(TheWindow)
    }

    fn window_self(&mut self, _cx: &mut DomCx<'_, '_>) -> DomResult<TheWindow> {
        Ok(TheWindow)
    }

    fn window_document(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<NodeId> {
        Ok(cx.tree().root())
    }

    fn window_event(&mut self, _cx: &mut DomCx<'_, '_>) -> DomResult<()> {
        // No event is ever dispatched, so there is no current event: `undefined`.
        Ok(())
    }
}
