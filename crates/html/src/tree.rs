//! The node arena tree construction builds: a document or a document fragment and the nodes
//! under it, each named by a [`NodeId`] that stays the same for the life of the tree. Written
//! by `story:html-tree-body`. No reader recurses over the depth of the tree, and no write does.
//!
//! Since `story:html-parser-limits` the tree keeps what the limits read as it is built: the
//! number of its nodes ([`Tree::node_count`]), the bytes of its strings and attribute records
//! ([`Tree::stored_bytes`]), the limit that stopped its parse ([`Tree::stopped_by`]) and the
//! steps the parse used ([`Tree::steps`]). The operations that scan a node's children add each
//! child they visit to the step count they are given. Each element keeps its categories
//! (special, the scopes it bounds), decided once when it is created.
//!
//! Since `story:html-script-hook` the tree keeps the limits of its parse ([`Tree::limits`]) and
//! has a public write surface, the DOM's operations a script's bindings need: create a node;
//! insert, append, remove, replace and move one; clone, import and adopt one; normalize, split
//! a text node, change character data, set the text content; set, remove and toggle an
//! attribute. Each write checks what the DOM Standard checks before it changes anything and
//! answers the DOM's exception as a [`WriteError`], then is admitted against the limits of the
//! parse: a write that would pass one answers [`WriteError::Limit`] with it and leaves the tree
//! as it was. A write never stops the parse ([`Tree::stopped_by`] stays as it was) and counts
//! no step. A node removed from its parent stays in the arena with its data, and its strings
//! stay counted. What the surface leaves to its caller: the names are not validated (the DOM's
//! `InvalidCharacterError`), an element has no namespace prefix, an attribute's namespace is one
//! of [`Namespace`], and a range or a mutation record is not kept.
//!
//! Depth for a write: the elements among a node's inclusive ancestors, a template counted for
//! the nodes of its contents, the count by which the depth limit bounds the tree in every parse
//! (the crate documentation, "Limits"). An insertion that would give an element a depth past
//! [`Limits::max_depth`] answers [`Limit::Depth`]. Each node's record keeps its depth for the
//! parser's checks, so a write that gives a node a new parent gives every node under it its
//! new depth: its cost grows with the subtree it moves.
//!
//! Offsets and counts of character data are in UTF-16 code units, as the DOM's; an offset that
//! falls between the two units of a surrogate pair answers [`WriteError::IndexSize`], since a
//! Rust string holds no lone surrogate (a deviation: the DOM splits the pair).

use std::cell::Cell;

use crate::limits::{ATTRIBUTE_BYTES, Limit, Limits};

/// Why a write of the public surface of [`Tree`] was not made. The tree is as it was before
/// the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WriteError {
    /// The write would pass this limit of the parse ([`Tree::limits`]): [`Limit::Nodes`],
    /// [`Limit::Depth`], [`Limit::Attributes`] or [`Limit::StoredBytes`].
    Limit(Limit),
    /// The DOM's `HierarchyRequestError`: the node may not go where the write would put it.
    HierarchyRequest,
    /// The DOM's `NotFoundError`: the reference child is not a child of the parent.
    NotFound,
    /// The DOM's `IndexSizeError`: an offset past the length of character data, or between
    /// the two code units of a surrogate pair.
    IndexSize,
    /// The DOM's `InvalidNodeTypeError`, for an operation on a node of the wrong kind (an
    /// attribute of a text node, the data of an element).
    InvalidNodeType,
    /// The DOM's `NamespaceError`, from "validate and extract" of a qualified name.
    Namespace,
    /// The DOM's `NotSupportedError`: a document cloned, imported or adopted. A tree holds one
    /// document, its root.
    NotSupported,
}

/// What a write answers.
pub type WriteResult<T> = Result<T, WriteError>;

/// A namespace an element or an attribute is in (Infra, "Namespaces").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// `http://www.w3.org/1999/xhtml`.
    Html,
    /// `http://www.w3.org/1998/Math/MathML`.
    MathMl,
    /// `http://www.w3.org/2000/svg`.
    Svg,
    /// `http://www.w3.org/1999/xlink`.
    XLink,
    /// `http://www.w3.org/XML/1998/namespace`.
    Xml,
    /// `http://www.w3.org/2000/xmlns/`.
    Xmlns,
}

impl Namespace {
    /// The namespace's URL.
    pub fn url(self) -> &'static str {
        match self {
            Namespace::Html => "http://www.w3.org/1999/xhtml",
            Namespace::MathMl => "http://www.w3.org/1998/Math/MathML",
            Namespace::Svg => "http://www.w3.org/2000/svg",
            Namespace::XLink => "http://www.w3.org/1999/xlink",
            Namespace::Xml => "http://www.w3.org/XML/1998/namespace",
            Namespace::Xmlns => "http://www.w3.org/2000/xmlns/",
        }
    }
}

/// An attribute of an element: its namespace (`None` for most), its prefix, its local name and
/// its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    /// The namespace; `None` for an attribute in no namespace.
    pub namespace: Option<Namespace>,
    /// The namespace prefix (`xlink` in `xlink:href` once adjusted), if any.
    pub prefix: Option<String>,
    /// The local name.
    pub local_name: String,
    /// The value.
    pub value: String,
}

/// The identity of a node in its [`Tree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(usize);

impl NodeId {
    /// The node's index in the arena: 0 for the root, then in the order the nodes were created.
    pub(crate) fn index(self) -> usize {
        self.0
    }
}

/// What a node is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeData {
    /// The document.
    Document,
    /// A document fragment: the root of what the fragment parsing algorithm returns, and a
    /// `template` element's template contents.
    DocumentFragment,
    /// A doctype, with the DOM's empty string for a missing name or identifier.
    Doctype {
        /// The name.
        name: String,
        /// The public identifier.
        public_id: String,
        /// The system identifier.
        system_id: String,
    },
    /// An element.
    Element {
        /// The namespace.
        namespace: Namespace,
        /// The local name.
        local_name: String,
        /// The attributes, in the order they were added.
        attributes: Vec<Attribute>,
        /// For a `template` element in the HTML namespace, its template contents: a
        /// [`NodeData::DocumentFragment`] node of the same tree. `None` for every other element.
        template_contents: Option<NodeId>,
    },
    /// A text node.
    Text(String),
    /// A comment.
    Comment(String),
    /// A processing instruction.
    ProcessingInstruction {
        /// The target.
        target: String,
        /// The data.
        data: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Node {
    data: NodeData,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    /// For an element, its categories ([`Categories`]), decided when it was created; 0 for
    /// every other node.
    categories: u8,
    depth: Depth,
}

/// The bytes of one node's record in the arena and of its entry in its parent's child list
/// ([`crate::limits::NODE_BYTES`]).
pub(crate) const NODE_RECORD_BYTES: usize =
    std::mem::size_of::<Node>() + std::mem::size_of::<NodeId>();

/// The categories of an element that tree construction asks for at every visit of the stack of
/// open elements, decided once from its namespace and local name when it is created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Categories(pub(crate) u8);

impl Categories {
    /// In the special category (13.2.4.2).
    pub(crate) const SPECIAL: u8 = 1;
    /// One of the element types of "has an element in scope".
    pub(crate) const SCOPE: u8 = 1 << 1;
    /// One of the element types "in list item scope" adds (`ol`, `ul`).
    pub(crate) const LIST_ITEM_SCOPE: u8 = 1 << 2;
    /// The element type "in button scope" adds (`button`).
    pub(crate) const BUTTON_SCOPE: u8 = 1 << 3;
    /// A MathML `annotation-xml` whose start tag had an `encoding` of `text/html` or
    /// `application/xhtml+xml`: an HTML integration point, whatever a write does to the
    /// attribute later (13.2.6).
    pub(crate) const HTML_INTEGRATION_POINT: u8 = 1 << 4;

    /// Whether every bit of `bits` is set.
    pub(crate) fn has(self, bits: u8) -> bool {
        self.0 & bits == bits
    }
}

/// The stored bytes of `attribute`: its record ([`ATTRIBUTE_BYTES`]) and the UTF-8 bytes of its
/// prefix, local name and value.
fn attribute_bytes(attribute: &Attribute) -> usize {
    ATTRIBUTE_BYTES
        + attribute.prefix.as_ref().map_or(0, String::len)
        + attribute.local_name.len()
        + attribute.value.len()
}

/// The stored bytes of `data` ([`Tree::stored_bytes`]): the UTF-8 bytes of its strings and
/// [`ATTRIBUTE_BYTES`] for each attribute.
pub(crate) fn stored_bytes_of(data: &NodeData) -> usize {
    match data {
        NodeData::Document | NodeData::DocumentFragment => 0,
        NodeData::Doctype {
            name,
            public_id,
            system_id,
        } => name.len() + public_id.len() + system_id.len(),
        NodeData::Element {
            local_name,
            attributes,
            ..
        } => local_name.len() + attributes.iter().map(attribute_bytes).sum::<usize>(),
        NodeData::Text(text) => text.len(),
        NodeData::Comment(data) => data.len(),
        NodeData::ProcessingInstruction { target, data } => target.len() + data.len(),
    }
}

/// Adds one visit to `steps`.
fn visit(steps: &Cell<usize>) {
    steps.set(steps.get().saturating_add(1));
}

/// A tree of nodes: a document or a document fragment at its root, every other node under it
/// or under a template's contents.
///
/// Every node created stays in the arena for the life of the tree, so a [`NodeId`] never
/// changes its meaning. A node removed from its parent (the `body` that a `frameset` start tag
/// takes the place of, or a node a write removed) has no parent and is reached from the root no
/// longer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    nodes: Vec<Node>,
    stored_bytes: usize,
    stopped_by: Option<Limit>,
    steps: usize,
    limits: Limits,
}

/// The depth a node's record keeps ([`Tree::depth`]): the elements among its inclusive
/// ancestors, a template counted for its contents. It is kept for every node with a parent or
/// a host equal to its parent's or host's plus one for an element; a node with neither keeps
/// what it had when it left its parent, at least its own count, so every node under it keeps
/// at least its depth. It is not part of what a tree holds: two trees compare equal whatever
/// it is.
#[derive(Clone, Copy, Debug, Default, Eq)]
struct Depth(u32);

impl PartialEq for Depth {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Tree {
    /// A tree holding one [`NodeData::Document`] and nothing else, held to `limits`.
    pub(crate) fn document(limits: Limits) -> Tree {
        Tree::with_root(NodeData::Document, limits)
    }

    /// A tree holding one [`NodeData::DocumentFragment`] and nothing else, held to `limits`.
    pub(crate) fn fragment(limits: Limits) -> Tree {
        Tree::with_root(NodeData::DocumentFragment, limits)
    }

    fn with_root(data: NodeData, limits: Limits) -> Tree {
        let mut tree = Tree {
            nodes: Vec::new(),
            stored_bytes: 0,
            stopped_by: None,
            steps: 0,
            limits,
        };
        tree.create(data, 0);
        tree
    }

    /// Adds a node with no parent to the arena, its depth its own count.
    fn create(&mut self, data: NodeData, categories: u8) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.stored_bytes += stored_bytes_of(&data);
        let depth = Depth(u32::from(matches!(data, NodeData::Element { .. })));
        self.nodes.push(Node {
            data,
            parent: None,
            children: Vec::new(),
            categories,
            depth,
        });
        id
    }

    /// Creates an element with no parent (the DOM's "create an element", with no custom element
    /// definition looked up), with its `categories`. An HTML `template` gets its template
    /// contents, a document fragment of this tree, created first: a `template` is two nodes.
    pub(crate) fn create_element(
        &mut self,
        namespace: Namespace,
        local_name: &str,
        attributes: Vec<Attribute>,
        categories: Categories,
    ) -> NodeId {
        let template_contents = (namespace == Namespace::Html && local_name == "template")
            .then(|| self.create(NodeData::DocumentFragment, 0));
        if let Some(contents) = template_contents {
            // Below its template, which has no parent: as deep as the template.
            self.nodes[contents.0].depth = Depth(1);
        }
        self.create(
            NodeData::Element {
                namespace,
                local_name: local_name.to_string(),
                attributes,
                template_contents,
            },
            categories.0,
        )
    }

    /// The depth of `node` ([`Depth`]): for a node of the tree, the elements on its path from
    /// the root, itself included, a template counted for the nodes of its contents (the count
    /// of [`crate::limits::Limits::max_depth`]); for a node out of the tree, at least the
    /// elements on its path from its own root. Kept by every insertion, so reading it visits
    /// nothing.
    pub(crate) fn depth(&self, node: NodeId) -> usize {
        self.nodes[node.0].depth.0 as usize
    }

    /// The template contents of `node`, if it is a template.
    pub(crate) fn contents(&self, node: NodeId) -> Option<NodeId> {
        match self.nodes[node.0].data {
            NodeData::Element {
                template_contents, ..
            } => template_contents,
            _ => None,
        }
    }

    /// Gives `node` the depth its parent or host gives it, and nothing under it: the change.
    pub(crate) fn settle(&mut self, node: NodeId) -> i64 {
        let above = self
            .parent_or_host(node)
            .map_or(0, |p| self.nodes[p.0].depth.0);
        let new = above.saturating_add(u32::from(self.is_element(node)));
        let old = std::mem::replace(&mut self.nodes[node.0].depth, Depth(new)).0;
        i64::from(new) - i64::from(old)
    }

    /// Adds `change` to the depth of every node under `node`: its children and its template
    /// contents, and theirs. Each node visited is a step, but a template's contents, which go
    /// with their template.
    fn shift_below(&mut self, node: NodeId, change: i64, steps: &Cell<usize>) {
        let mut pending: Vec<NodeId> = self.contents(node).into_iter().collect();
        pending.extend(self.nodes[node.0].children.iter().copied());
        self.shift_all(pending, change, steps);
    }

    /// Adds `change` to the depth of `node` and of every node under it, each a step.
    pub(crate) fn shift(&mut self, node: NodeId, change: i64, steps: &Cell<usize>) {
        self.shift_all(vec![node], change, steps);
    }

    /// Adds `change` to the depth of the nodes of `pending` and of every node under them. Each
    /// node visited is a step, but a template's contents, which go with their template.
    fn shift_all(&mut self, mut pending: Vec<NodeId>, change: i64, steps: &Cell<usize>) {
        if change == 0 {
            return;
        }
        while let Some(at) = pending.pop() {
            if self.nodes[at.0].data != NodeData::DocumentFragment {
                visit(steps);
            }
            let depth = &mut self.nodes[at.0].depth.0;
            *depth = u32::try_from((i64::from(*depth) + change).max(0)).unwrap_or(u32::MAX);
            pending.extend(self.contents(at));
            pending.extend(self.nodes[at.0].children.iter().copied());
        }
    }

    /// Gives `node`, which just got its parent, and every node under it, their depths. A node
    /// with nothing under it visits nothing.
    fn place(&mut self, node: NodeId, steps: &Cell<usize>) {
        let change = self.settle(node);
        self.shift_below(node, change, steps);
    }

    /// The deepest depth under `node` (its children and template contents, and theirs), or
    /// `node`'s own when nothing is under it. Each node visited is a step, but a template's
    /// contents.
    pub(crate) fn deepest_below(&self, node: NodeId, steps: &Cell<usize>) -> usize {
        let mut deepest = self.nodes[node.0].depth.0;
        let mut pending: Vec<NodeId> = self.contents(node).into_iter().collect();
        pending.extend(self.nodes[node.0].children.iter().copied());
        while let Some(at) = pending.pop() {
            if self.nodes[at.0].data != NodeData::DocumentFragment {
                visit(steps);
            }
            deepest = deepest.max(self.nodes[at.0].depth.0);
            pending.extend(self.contents(at));
            pending.extend(self.nodes[at.0].children.iter().copied());
        }
        deepest as usize
    }

    /// The categories `node` was created with: none for a node that is not an element.
    pub(crate) fn categories(&self, node: NodeId) -> Categories {
        Categories(self.nodes[node.0].categories)
    }

    /// The index in `parent`'s children before which a node inserted before `reference` goes:
    /// the end for `None` or a reference that is not a child. Each child the scan visits is a
    /// step. The scan starts at the last child: the one reference the tree builder passes is
    /// the `table` of foster parenting, in front of which every foster parented node goes, so it
    /// stays its parent's last child or close to it while it is open, and the cost of one
    /// insertion does not grow with the siblings in front of it.
    fn insertion_index(
        &self,
        parent: NodeId,
        reference: Option<NodeId>,
        steps: &Cell<usize>,
    ) -> usize {
        let children = &self.nodes[parent.0].children;
        reference
            .and_then(|r| {
                children.iter().rposition(|&c| {
                    visit(steps);
                    c == r
                })
            })
            .unwrap_or(children.len())
    }

    /// Inserts `node`, which has no parent, into `parent` before `reference`, or as its last
    /// child when `reference` is `None` (the DOM's "insert"), and gives it and every node under
    /// it their depths ([`Tree::depth`]; each node under it a step). The caller has checked
    /// that `node` is no host-including inclusive ancestor of `parent`, and the depth limit.
    pub(crate) fn insert(
        &mut self,
        parent: NodeId,
        node: NodeId,
        reference: Option<NodeId>,
        steps: &Cell<usize>,
    ) {
        self.link(parent, node, reference, steps);
        self.place(node, steps);
    }

    /// [`Tree::insert`] without the depths: the adoption agency algorithm moves a chain of
    /// nodes and gives them their depths once, at the end of its iteration.
    pub(crate) fn link(
        &mut self,
        parent: NodeId,
        node: NodeId,
        reference: Option<NodeId>,
        steps: &Cell<usize>,
    ) {
        debug_assert!(self.nodes[node.0].parent.is_none());
        let at = self.insertion_index(parent, reference, steps);
        self.nodes[parent.0].children.insert(at, node);
        self.nodes[node.0].parent = Some(parent);
    }

    /// Creates a node of `data` and inserts it into `parent` before `reference`.
    pub(crate) fn insert_new(
        &mut self,
        parent: NodeId,
        data: NodeData,
        reference: Option<NodeId>,
        steps: &Cell<usize>,
    ) -> NodeId {
        let node = self.create(data, 0);
        self.insert(parent, node, reference, steps);
        node
    }

    /// The text node that text inserted into `parent` before `reference` is appended to ("flush
    /// pending text", 13.2.6.1): the node just before that place, when it is a text node.
    pub(crate) fn text_before(
        &self,
        parent: NodeId,
        reference: Option<NodeId>,
        steps: &Cell<usize>,
    ) -> Option<NodeId> {
        let at = self.insertion_index(parent, reference, steps);
        let previous = self.nodes[parent.0].children[..at].last().copied()?;
        matches!(self.nodes[previous.0].data, NodeData::Text(_)).then_some(previous)
    }

    /// Appends `text` to the data of the text node `node`.
    pub(crate) fn push_text(&mut self, node: NodeId, text: &str) {
        if let NodeData::Text(data) = &mut self.nodes[node.0].data {
            data.push_str(text);
            self.stored_bytes += text.len();
        }
    }

    /// Adds `attributes` to `element`, in order. The caller has left out every name the element
    /// has an attribute of (the `html` and `body` start tag rules of "in body"). A list that was
    /// empty gets exactly the room it needs, so that a list never holds more than twice the
    /// records it has (the heap bound of [`crate::limits::Limits::max_stored_bytes`]).
    pub(crate) fn add_attributes(&mut self, element: NodeId, attributes: Vec<Attribute>) {
        if let NodeData::Element {
            attributes: existing,
            ..
        } = &mut self.nodes[element.0].data
        {
            self.stored_bytes += attributes.iter().map(attribute_bytes).sum::<usize>();
            if existing.is_empty() {
                existing.reserve_exact(attributes.len());
            }
            existing.extend(attributes);
        }
    }

    /// Removes `node` from its parent, if it has one. The node stays in the arena. Each child of
    /// the parent the scan visits is a step.
    pub(crate) fn remove(&mut self, node: NodeId, steps: &Cell<usize>) {
        if let Some(parent) = self.nodes[node.0].parent.take() {
            let children = &mut self.nodes[parent.0].children;
            if let Some(at) = children.iter().rposition(|&c| {
                visit(steps);
                c == node
            }) {
                children.remove(at);
            }
        }
    }

    /// Removes every child of `node` (the DOM's "replace all" with null). The children stay in
    /// the arena. Each child removed is a step.
    pub(crate) fn remove_children(&mut self, node: NodeId, steps: &Cell<usize>) {
        for child in std::mem::take(&mut self.nodes[node.0].children) {
            visit(steps);
            self.nodes[child.0].parent = None;
        }
    }

    /// Moves every child of `from` to the end of `to`'s children, in order, in one pass (the
    /// adoption agency algorithm's "take all of the child nodes of furthestBlock and append them
    /// to newElement"). `to` is not `from` and not under it. Each child moved is a step. The
    /// depths are the caller's, as for [`Tree::link`].
    pub(crate) fn move_children(&mut self, from: NodeId, to: NodeId, steps: &Cell<usize>) {
        let children = std::mem::take(&mut self.nodes[from.0].children);
        for &child in &children {
            visit(steps);
            self.nodes[child.0].parent = Some(to);
        }
        self.nodes[to.0].children.extend(children);
    }

    /// Whether `node` has a child that is an element. Each child visited is a step.
    pub(crate) fn has_element_child(&self, node: NodeId, steps: &Cell<usize>) -> bool {
        self.children(node).iter().any(|&c| {
            visit(steps);
            matches!(self.data(c), NodeData::Element { .. })
        })
    }

    /// Records the limit that stopped the parse; the first one recorded stays.
    pub(crate) fn stop_at(&mut self, limit: Limit) {
        self.stopped_by.get_or_insert(limit);
    }

    /// The limit that stopped the parse of this tree ([`crate::limits::Limits`]), `None` for a
    /// parse that ended without reaching one. The tree holds every node inserted before that
    /// point.
    pub fn stopped_by(&self) -> Option<Limit> {
        self.stopped_by
    }

    /// The number of nodes of the arena: the root, every node under it, template contents and
    /// nodes removed from their parent while parsing.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// The stored bytes of the tree ([`crate::limits::Limits::max_stored_bytes`]): the UTF-8
    /// bytes of every string it holds (element local names, attribute names, prefixes and
    /// values, text, comment data, doctype fields, processing instruction targets and data), and
    /// [`ATTRIBUTE_BYTES`] for every attribute, in every node of the arena.
    pub fn stored_bytes(&self) -> usize {
        self.stored_bytes
    }

    /// Records the steps the parse used.
    pub(crate) fn set_steps(&mut self, steps: usize) {
        self.steps = steps;
    }

    /// The steps the parse of this tree used ([`crate::limits::Limits::max_steps`]): every entry
    /// tree construction visited in its scans, those of the token that took the count past the
    /// limit included. For [`crate::parse_document_bytes`] they are those of its last parse: a
    /// parse again after a `meta` counts from 0. 0 for [`crate::parse_fragment`], which builds no
    /// tree yet. A write of the public surface counts no step.
    pub fn steps(&self) -> usize {
        self.steps
    }

    /// The limits of the parse that built this tree ([`crate::ParseOptions::limits`]), which
    /// every write of the public surface is held to.
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// The parent of `node`: `None` for the root, for a template's contents and for a node
    /// removed from its parent while parsing.
    ///
    /// # Panics
    ///
    /// When `node` is not of this tree.
    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.nodes[node.0].parent
    }

    /// `node` and the nodes under it, in tree order: `node` first, then each child followed by
    /// the nodes under it. A `template` element's contents are not among them. The walk keeps
    /// its own stack, so a deep tree does not deepen the call stack.
    ///
    /// # Panics
    ///
    /// When `node` is not of this tree.
    pub fn descendants(&self, node: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut pending = vec![node];
        while let Some(next) = pending.pop() {
            out.push(next);
            pending.extend(self.children(next).iter().rev());
        }
        out
    }

    /// The root: a [`NodeData::Document`] or a [`NodeData::DocumentFragment`].
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// What `node` is.
    ///
    /// # Panics
    ///
    /// When `node` is not of this tree.
    pub fn data(&self, node: NodeId) -> &NodeData {
        &self.nodes[node.0].data
    }

    /// The children of `node`, in tree order. A `template` element's contents are not its
    /// children; they are the children of its `template_contents`.
    ///
    /// # Panics
    ///
    /// When `node` is not of this tree.
    pub fn children(&self, node: NodeId) -> &[NodeId] {
        &self.nodes[node.0].children
    }
}

/// The length of `data` in UTF-16 code units (the DOM's length of character data).
fn utf16_len(data: &str) -> usize {
    data.chars().map(char::len_utf16).sum()
}

/// The byte index in `data` of the offset `units` in UTF-16 code units: `IndexSize` past the
/// end or between the two units of a surrogate pair.
fn byte_index(data: &str, units: usize) -> WriteResult<usize> {
    let mut at = 0;
    for (index, c) in data.char_indices() {
        if at == units {
            return Ok(index);
        }
        at += c.len_utf16();
        if at > units {
            return Err(WriteError::IndexSize);
        }
    }
    if at == units {
        Ok(data.len())
    } else {
        Err(WriteError::IndexSize)
    }
}

/// The qualified name of `attribute`: its prefix, a colon and its local name, or its local name.
fn qualified_name_is(attribute: &Attribute, name: &str) -> bool {
    match &attribute.prefix {
        None => attribute.local_name == name,
        Some(prefix) => {
            name.len() == prefix.len() + 1 + attribute.local_name.len()
                && name.starts_with(prefix.as_str())
                && name[prefix.len()..].starts_with(':')
                && name.ends_with(attribute.local_name.as_str())
        }
    }
}

/// Gives a list back the room it holds past twice its entries, so that a list a write shrank
/// keeps the heap bound of [`Limits::max_stored_bytes`].
fn shrink<T>(list: &mut Vec<T>) {
    if list.capacity() > (2 * list.len()).max(4) {
        list.shrink_to_fit();
    }
}

/// The public write surface (the module documentation): the DOM's operations, each checked as
/// the DOM checks it and admitted against the limits of the parse before it writes.
///
/// A [`NodeId`] is an id of one tree: an id of another tree is the caller's error, which a
/// write may answer with a panic, a refusal or a change to the wrong node. A removed node stays
/// in the arena and keeps its count: removal gives no room back for nodes. A write may cost
/// time in proportion to the children or the subtree it touches, which no limit counts: the
/// caller charges for it.
///
/// # Panics
///
/// Each when a [`NodeId`] it is given is not of this tree.
impl Tree {
    /// Whether a creation of `nodes` nodes that store `bytes` bytes stays inside the node and
    /// stored-byte limits.
    fn admit(&self, nodes: usize, bytes: usize) -> WriteResult<()> {
        if self.nodes.len().saturating_add(nodes) > self.limits.max_nodes {
            return Err(WriteError::Limit(Limit::Nodes));
        }
        if self.stored_bytes.saturating_add(bytes) > self.limits.max_stored_bytes {
            return Err(WriteError::Limit(Limit::StoredBytes));
        }
        Ok(())
    }

    /// Whether a change that frees `freed` stored bytes and stores `added` stays inside the
    /// stored-byte limit.
    fn admit_change(&self, freed: usize, added: usize) -> WriteResult<()> {
        if added > freed {
            self.admit(0, added - freed)
        } else {
            Ok(())
        }
    }

    fn is_element(&self, node: NodeId) -> bool {
        matches!(self.nodes[node.0].data, NodeData::Element { .. })
    }

    fn is_text(&self, node: NodeId) -> bool {
        matches!(self.nodes[node.0].data, NodeData::Text(_))
    }

    fn is_doctype(&self, node: NodeId) -> bool {
        matches!(self.nodes[node.0].data, NodeData::Doctype { .. })
    }

    /// The template whose contents `fragment` is, if it is one: the arena creates a template's
    /// contents just before the template.
    fn host(&self, fragment: NodeId) -> Option<NodeId> {
        let template = NodeId(fragment.0 + 1);
        match self.nodes.get(template.0).map(|n| &n.data) {
            Some(NodeData::Element {
                template_contents: Some(contents),
                ..
            }) if *contents == fragment => Some(template),
            _ => None,
        }
    }

    /// The parent of `node`, or for a template's contents its template.
    pub(crate) fn parent_or_host(&self, node: NodeId) -> Option<NodeId> {
        self.nodes[node.0].parent.or_else(|| self.host(node))
    }

    /// Whether `ancestor` is a host-including inclusive ancestor of `node` (DOM). The walk
    /// takes at most one step per node of the tree; a longer one is a cycle, which this
    /// answers as true, so that a write refuses it.
    pub(crate) fn is_host_including_inclusive_ancestor(
        &self,
        ancestor: NodeId,
        node: NodeId,
    ) -> bool {
        let mut next = Some(node);
        for _ in 0..=self.nodes.len() {
            match next {
                Some(at) if at == ancestor => return true,
                Some(at) => next = self.parent_or_host(at),
                None => return false,
            }
        }
        true
    }

    /// The root of `node` (DOM "root", which for a node of a template's contents is the
    /// contents), or `None` on a cycle: the walk takes at most one step per node.
    fn root_of(&self, node: NodeId) -> Option<NodeId> {
        let mut at = node;
        for _ in 0..=self.nodes.len() {
            match self.nodes[at.0].parent {
                Some(parent) => at = parent,
                None => return Some(at),
            }
        }
        None
    }

    /// The elements among `node`'s inclusive ancestors, a template counted for its contents,
    /// or `None` on a cycle: the walk takes at most one step per node.
    pub(crate) fn element_depth(&self, node: NodeId) -> Option<usize> {
        let mut depth = 0;
        let mut next = Some(node);
        for _ in 0..=self.nodes.len() {
            match next {
                Some(at) => {
                    depth += usize::from(self.is_element(at));
                    next = self.parent_or_host(at);
                }
                None => return Some(depth),
            }
        }
        None
    }

    /// The most elements on a path down from `node`, `node` included, through template
    /// contents.
    fn element_height(&self, node: NodeId) -> usize {
        let mut height = 0;
        let mut pending = vec![(node, 0)];
        while let Some((at, above)) = pending.pop() {
            let depth = above + usize::from(self.is_element(at));
            height = height.max(depth);
            pending.extend(self.nodes[at.0].children.iter().map(|&c| (c, depth)));
            if let NodeData::Element {
                template_contents: Some(contents),
                ..
            } = self.nodes[at.0].data
            {
                pending.push((contents, depth));
            }
        }
        height
    }

    /// Whether inserting `node` (the children of a fragment) into `parent` stays inside the
    /// depth limit.
    fn admit_depth(&self, parent: NodeId, node: NodeId) -> WriteResult<()> {
        let height = match self.nodes[node.0].data {
            NodeData::DocumentFragment => self.nodes[node.0]
                .children
                .iter()
                .map(|&c| self.element_height(c))
                .max()
                .unwrap_or(0),
            _ => self.element_height(node),
        };
        if height == 0 {
            return Ok(());
        }
        let Some(depth) = self.element_depth(parent) else {
            return Err(WriteError::HierarchyRequest);
        };
        if depth + height > self.limits.max_depth {
            return Err(WriteError::Limit(Limit::Depth));
        }
        Ok(())
    }

    /// The index of `node` in its parent's children.
    fn index_in_parent(&self, node: NodeId) -> Option<usize> {
        let parent = self.nodes[node.0].parent?;
        self.nodes[parent.0]
            .children
            .iter()
            .position(|&c| c == node)
    }

    fn next_sibling(&self, node: NodeId) -> Option<NodeId> {
        let parent = self.nodes[node.0].parent?;
        let at = self.index_in_parent(node)?;
        self.nodes[parent.0].children.get(at + 1).copied()
    }

    /// Removes `node` from its parent, if it has one, and gives the parent's list back its
    /// spare room.
    fn detach(&mut self, node: NodeId) {
        if let Some(parent) = self.nodes[node.0].parent {
            self.remove(node, &Cell::new(0));
            shrink(&mut self.nodes[parent.0].children);
        }
    }

    /// Step 6 of "ensure pre-insertion validity" and of "replace" (DOM) for a document
    /// `parent`: one element and one doctype, the doctype first. `replaced` is the child a
    /// replacement takes the place of.
    fn document_allows(
        &self,
        parent: NodeId,
        node: NodeId,
        child: Option<NodeId>,
        replaced: Option<NodeId>,
    ) -> WriteResult<()> {
        let children = &self.nodes[parent.0].children;
        let other = |n: &NodeId| Some(*n) != replaced;
        let has_element = children
            .iter()
            .filter(|n| other(n))
            .any(|&n| self.is_element(n));
        let has_doctype = children
            .iter()
            .filter(|n| other(n))
            .any(|&n| self.is_doctype(n));
        let at = child.and_then(|c| children.iter().position(|&n| n == c));
        let following = |test: &dyn Fn(NodeId) -> bool| {
            at.is_some_and(|at| children[at + 1..].iter().any(|&n| test(n)))
        };
        let preceding = |test: &dyn Fn(NodeId) -> bool| match at {
            Some(at) => children[..at].iter().any(|&n| test(n)),
            None => false,
        };
        let doctype_follows = following(&|n| self.is_doctype(n));
        let child_is_doctype = replaced.is_none() && child.is_some_and(|c| self.is_doctype(c));
        let refused = match &self.nodes[node.0].data {
            NodeData::DocumentFragment => {
                let fragment = &self.nodes[node.0].children;
                let elements = fragment.iter().filter(|&&n| self.is_element(n)).count();
                elements > 1
                    || fragment.iter().any(|&n| self.is_text(n))
                    || (elements == 1 && (has_element || child_is_doctype || doctype_follows))
            }
            NodeData::Element { .. } => has_element || child_is_doctype || doctype_follows,
            NodeData::Doctype { .. } => {
                has_doctype
                    || preceding(&|n| self.is_element(n))
                    || (replaced.is_none() && child.is_none() && has_element)
            }
            _ => false,
        };
        if refused {
            return Err(WriteError::HierarchyRequest);
        }
        Ok(())
    }

    /// "Ensure pre-insertion validity" (DOM) of `node` into `parent` before `child`, or of a
    /// replacement of `replaced` (then `child` is `replaced`).
    fn insertion_allowed(
        &self,
        node: NodeId,
        parent: NodeId,
        child: Option<NodeId>,
        replaced: Option<NodeId>,
    ) -> WriteResult<()> {
        let parent_data = &self.nodes[parent.0].data;
        if !matches!(
            parent_data,
            NodeData::Document | NodeData::DocumentFragment | NodeData::Element { .. }
        ) || self.is_host_including_inclusive_ancestor(node, parent)
        {
            return Err(WriteError::HierarchyRequest);
        }
        if child.is_some_and(|c| self.nodes[c.0].parent != Some(parent)) {
            return Err(WriteError::NotFound);
        }
        let is_document = *parent_data == NodeData::Document;
        match self.nodes[node.0].data {
            NodeData::Document => return Err(WriteError::HierarchyRequest),
            NodeData::Text(_) if is_document => return Err(WriteError::HierarchyRequest),
            NodeData::Doctype { .. } if !is_document => {
                return Err(WriteError::HierarchyRequest);
            }
            _ => {}
        }
        if is_document {
            self.document_allows(parent, node, child, replaced)?;
        }
        Ok(())
    }

    /// "Insert" (DOM) of `node` into `parent` before `child`, after the checks: a fragment's
    /// children in its place, in order; a node with a parent leaves it first. Each node
    /// inserted, and every node under it, gets its depth.
    fn insert_checked(&mut self, node: NodeId, parent: NodeId, child: Option<NodeId>) {
        let nodes = match self.nodes[node.0].data {
            NodeData::DocumentFragment => {
                let children = std::mem::take(&mut self.nodes[node.0].children);
                for &c in &children {
                    self.nodes[c.0].parent = None;
                }
                children
            }
            _ => {
                self.detach(node);
                vec![node]
            }
        };
        let at = match child {
            Some(c) => self
                .index_in_parent(c)
                .unwrap_or(self.nodes[parent.0].children.len()),
            None => self.nodes[parent.0].children.len(),
        };
        for &n in &nodes {
            self.nodes[n.0].parent = Some(parent);
        }
        self.nodes[parent.0]
            .children
            .splice(at..at, nodes.iter().copied());
        for n in nodes {
            self.place(n, &Cell::new(0));
        }
    }

    /// Creates an element in `namespace` named `local_name`, with no attribute and no parent
    /// (the DOM's "create an element", with no custom element definition looked up). An HTML
    /// `template` gets its contents: two nodes. The name is taken as given.
    pub fn create_element_ns(
        &mut self,
        namespace: Namespace,
        local_name: &str,
    ) -> WriteResult<NodeId> {
        let nodes = if namespace == Namespace::Html && local_name == "template" {
            2
        } else {
            1
        };
        self.admit(nodes, local_name.len())?;
        let categories = crate::tree_builder::categories_of(namespace, local_name);
        Ok(self.create_element(namespace, local_name, Vec::new(), categories))
    }

    /// Creates a text node of `data` with no parent (`createTextNode`).
    pub fn create_text_node(&mut self, data: &str) -> WriteResult<NodeId> {
        self.admit(1, data.len())?;
        Ok(self.create(NodeData::Text(data.to_string()), 0))
    }

    /// Creates a comment of `data` with no parent (`createComment`).
    pub fn create_comment(&mut self, data: &str) -> WriteResult<NodeId> {
        self.admit(1, data.len())?;
        Ok(self.create(NodeData::Comment(data.to_string()), 0))
    }

    /// Creates a processing instruction with no parent (`createProcessingInstruction`).
    pub fn create_processing_instruction(
        &mut self,
        target: &str,
        data: &str,
    ) -> WriteResult<NodeId> {
        self.admit(1, target.len() + data.len())?;
        Ok(self.create(
            NodeData::ProcessingInstruction {
                target: target.to_string(),
                data: data.to_string(),
            },
            0,
        ))
    }

    /// Creates an empty document fragment (`createDocumentFragment`).
    pub fn create_document_fragment(&mut self) -> WriteResult<NodeId> {
        self.admit(1, 0)?;
        Ok(self.create(NodeData::DocumentFragment, 0))
    }

    /// Inserts `node` into `parent` before `child`, or as its last child for `None`
    /// (`insertBefore`, the DOM's "pre-insert"): a node with a parent leaves it first, and a
    /// document fragment's children go in its place, in order, leaving it empty.
    pub fn insert_before(
        &mut self,
        parent: NodeId,
        node: NodeId,
        child: Option<NodeId>,
    ) -> WriteResult<()> {
        self.insertion_allowed(node, parent, child, None)?;
        let child = if child == Some(node) {
            self.next_sibling(node)
        } else {
            child
        };
        self.admit_depth(parent, node)?;
        self.insert_checked(node, parent, child);
        Ok(())
    }

    /// Appends `node` to `parent`'s children (`appendChild`): [`Tree::insert_before`] with no
    /// child.
    pub fn append_child(&mut self, parent: NodeId, node: NodeId) -> WriteResult<()> {
        self.insert_before(parent, node, None)
    }

    /// Removes `node` from its parent (`remove`); a node without a parent stays as it is. The
    /// node stays in the arena with everything under it.
    pub fn remove_node(&mut self, node: NodeId) -> WriteResult<()> {
        self.detach(node);
        Ok(())
    }

    /// Puts `node` in the place of `child`, a child of `parent` (`replaceChild`, the DOM's
    /// "replace"); `child` leaves the tree and stays in the arena.
    pub fn replace_child(
        &mut self,
        parent: NodeId,
        node: NodeId,
        child: NodeId,
    ) -> WriteResult<()> {
        self.insertion_allowed(node, parent, Some(child), Some(child))?;
        self.admit_depth(parent, node)?;
        let mut reference = self.next_sibling(child);
        if reference == Some(node) {
            reference = self.next_sibling(node);
        }
        if child != node {
            self.detach(child);
        }
        self.insert_checked(node, parent, reference);
        Ok(())
    }

    /// Moves `node` into `parent` before `child`, or to its end for `None` (`moveBefore`, the
    /// DOM's "move"): the node keeps its identity and everything under it. Only an element or
    /// character data moves, and only within its root.
    pub fn move_before(
        &mut self,
        parent: NodeId,
        node: NodeId,
        child: Option<NodeId>,
    ) -> WriteResult<()> {
        let parent_data = &self.nodes[parent.0].data;
        if !matches!(
            parent_data,
            NodeData::Document | NodeData::DocumentFragment | NodeData::Element { .. }
        ) || self.root_of(parent).is_none()
            || self.root_of(parent) != self.root_of(node)
            || self.is_host_including_inclusive_ancestor(node, parent)
        {
            return Err(WriteError::HierarchyRequest);
        }
        if child.is_some_and(|c| self.nodes[c.0].parent != Some(parent)) {
            return Err(WriteError::NotFound);
        }
        let is_document = *parent_data == NodeData::Document;
        let refused = match self.nodes[node.0].data {
            NodeData::Element { .. } => {
                is_document && self.document_allows(parent, node, child, None).is_err()
            }
            NodeData::Text(_) => is_document,
            NodeData::Comment(_) | NodeData::ProcessingInstruction { .. } => false,
            _ => true,
        };
        if refused {
            return Err(WriteError::HierarchyRequest);
        }
        let child = if child == Some(node) {
            self.next_sibling(node)
        } else {
            child
        };
        self.admit_depth(parent, node)?;
        self.insert_checked(node, parent, child);
        Ok(())
    }

    /// The nodes and stored bytes a clone of `node` makes.
    fn clone_cost(&self, node: NodeId, deep: bool) -> (usize, usize) {
        let (mut nodes, mut bytes) = (0, 0);
        let mut pending = vec![node];
        while let Some(at) = pending.pop() {
            let data = &self.nodes[at.0].data;
            nodes += 1;
            bytes += stored_bytes_of(data);
            if let NodeData::Element {
                template_contents: Some(contents),
                ..
            } = data
            {
                nodes += 1;
                if deep {
                    pending.extend(&self.nodes[contents.0].children);
                }
            }
            if deep {
                pending.extend(&self.nodes[at.0].children);
            }
        }
        (nodes, bytes)
    }

    /// Clones `node`, with everything under it when `deep` (`cloneNode`, the DOM's "clone a
    /// node"): the copy has no parent; an element keeps its attributes, and a template's copy
    /// gets its own contents, with copies of the contents' children when `deep`.
    pub fn clone_node(&mut self, node: NodeId, deep: bool) -> WriteResult<NodeId> {
        if self.nodes[node.0].data == NodeData::Document {
            return Err(WriteError::NotSupported);
        }
        let (nodes, bytes) = self.clone_cost(node, deep);
        self.admit(nodes, bytes)?;
        let mut copy_of_node = None;
        // Each source with the copy its copy is appended to, in tree order.
        let mut pending: Vec<(NodeId, Option<NodeId>)> = vec![(node, None)];
        while let Some((source, parent)) = pending.pop() {
            let data = self.nodes[source.0].data.clone();
            let copy = match data {
                NodeData::Element {
                    namespace,
                    local_name,
                    attributes,
                    template_contents,
                } => {
                    let categories = self.categories(source);
                    let copy = self.create_element(namespace, &local_name, attributes, categories);
                    if deep
                        && let (
                            Some(from),
                            NodeData::Element {
                                template_contents: Some(to),
                                ..
                            },
                        ) = (template_contents, &self.nodes[copy.0].data)
                    {
                        let to = *to;
                        pending.extend(
                            self.nodes[from.0]
                                .children
                                .iter()
                                .rev()
                                .map(|&c| (c, Some(to))),
                        );
                    }
                    copy
                }
                other => self.create(other, 0),
            };
            if let Some(parent) = parent {
                self.nodes[copy.0].parent = Some(parent);
                self.nodes[parent.0].children.push(copy);
                self.place(copy, &Cell::new(0));
            }
            copy_of_node.get_or_insert(copy);
            if deep {
                pending.extend(
                    self.nodes[source.0]
                        .children
                        .iter()
                        .rev()
                        .map(|&c| (c, Some(copy))),
                );
            }
        }
        Ok(copy_of_node.expect("the first node is copied"))
    }

    /// Imports `node` (`importNode`): in the one document of a tree, a clone.
    pub fn import_node(&mut self, node: NodeId, deep: bool) -> WriteResult<NodeId> {
        self.clone_node(node, deep)
    }

    /// Adopts `node` (`adoptNode`): in the one document of a tree, it leaves its parent. A
    /// template's contents stay where they are.
    pub fn adopt_node(&mut self, node: NodeId) -> WriteResult<()> {
        match self.nodes[node.0].data {
            NodeData::Document => Err(WriteError::NotSupported),
            NodeData::DocumentFragment if self.host(node).is_some() => Ok(()),
            _ => {
                self.detach(node);
                Ok(())
            }
        }
    }

    /// The runs of adjacent text nodes among `parent`'s children.
    fn text_runs(&self, parent: NodeId) -> Vec<Vec<NodeId>> {
        let mut runs: Vec<Vec<NodeId>> = Vec::new();
        let mut run = Vec::new();
        for &child in &self.nodes[parent.0].children {
            if self.is_text(child) {
                run.push(child);
            } else if !run.is_empty() {
                runs.push(std::mem::take(&mut run));
            }
        }
        if !run.is_empty() {
            runs.push(run);
        }
        runs
    }

    fn text_len(&self, node: NodeId) -> usize {
        match &self.nodes[node.0].data {
            NodeData::Text(data) => data.len(),
            _ => 0,
        }
    }

    /// Normalizes the subtree of `node` (`normalize`): each text node with no data leaves its
    /// parent, and each run of adjacent text nodes becomes its first one with data, which takes
    /// the data of the others; they leave the tree with their data, so the merge stores those
    /// bytes again. Template contents are not under the template.
    pub fn normalize(&mut self, node: NodeId) -> WriteResult<()> {
        let parents: Vec<NodeId> = self
            .descendants(node)
            .into_iter()
            .filter(|&n| !self.nodes[n.0].children.is_empty())
            .collect();
        let mut plan = Vec::new();
        let mut added = 0;
        for &parent in &parents {
            for run in self.text_runs(parent) {
                let first = run.iter().position(|&t| self.text_len(t) > 0);
                if let Some(first) = first {
                    added += run[first + 1..]
                        .iter()
                        .map(|&t| self.text_len(t))
                        .sum::<usize>();
                }
                plan.push((run, first));
            }
        }
        self.admit_change(0, added)?;
        for (run, first) in plan {
            if let Some(first) = first {
                let mut merged = String::new();
                for &t in &run[first + 1..] {
                    if let NodeData::Text(data) = &self.nodes[t.0].data {
                        merged.push_str(data);
                    }
                }
                if let NodeData::Text(data) = &mut self.nodes[run[first].0].data {
                    data.push_str(&merged);
                }
                self.stored_bytes += merged.len();
            }
            for (i, &t) in run.iter().enumerate() {
                if Some(i) != first {
                    self.detach(t);
                }
            }
        }
        Ok(())
    }

    /// The data of character data `node`: a text node, a comment or a processing instruction.
    fn character_data(&self, node: NodeId) -> WriteResult<&String> {
        match &self.nodes[node.0].data {
            NodeData::Text(data)
            | NodeData::Comment(data)
            | NodeData::ProcessingInstruction { data, .. } => Ok(data),
            _ => Err(WriteError::InvalidNodeType),
        }
    }

    /// Splits the text node `node` at `offset` (`splitText`): the data from the offset goes to
    /// a new text node, which follows `node` when it has a parent.
    pub fn split_text(&mut self, node: NodeId, offset: usize) -> WriteResult<NodeId> {
        let NodeData::Text(data) = &self.nodes[node.0].data else {
            return Err(WriteError::InvalidNodeType);
        };
        let at = byte_index(data, offset)?;
        // The new node holds the bytes the old one gives up.
        self.admit(1, 0)?;
        let tail = data[at..].to_string();
        let head = data[..at].to_string();
        let moved = tail.len();
        let new = self.create(NodeData::Text(tail), 0);
        self.nodes[node.0].data = NodeData::Text(head);
        self.stored_bytes -= moved;
        if let Some(parent) = self.nodes[node.0].parent {
            let next = self.next_sibling(node);
            self.insert_checked(new, parent, next);
        }
        Ok(new)
    }

    /// Replaces `count` code units of the data of `node` from `offset` with `data` (`replaceData`,
    /// the DOM's "replace data"); a count past the end ends at the end.
    pub fn replace_data(
        &mut self,
        node: NodeId,
        offset: usize,
        count: usize,
        data: &str,
    ) -> WriteResult<()> {
        let old = self.character_data(node)?;
        let length = utf16_len(old);
        if offset > length {
            return Err(WriteError::IndexSize);
        }
        let start = byte_index(old, offset)?;
        let end = byte_index(old, offset + count.min(length - offset))?;
        self.admit_change(end - start, data.len())?;
        let mut new = String::with_capacity(old.len() - (end - start) + data.len());
        new.push_str(&old[..start]);
        new.push_str(data);
        new.push_str(&old[end..]);
        self.stored_bytes = self.stored_bytes - (end - start) + data.len();
        match &mut self.nodes[node.0].data {
            NodeData::Text(d)
            | NodeData::Comment(d)
            | NodeData::ProcessingInstruction { data: d, .. } => *d = new,
            _ => unreachable!("checked as character data"),
        }
        Ok(())
    }

    /// Sets the data of `node` (the `data` setter): every code unit replaced.
    pub fn set_data(&mut self, node: NodeId, data: &str) -> WriteResult<()> {
        let length = utf16_len(self.character_data(node)?);
        self.replace_data(node, 0, length, data)
    }

    /// Appends `data` to the data of `node` (`appendData`).
    pub fn append_data(&mut self, node: NodeId, data: &str) -> WriteResult<()> {
        let length = utf16_len(self.character_data(node)?);
        self.replace_data(node, length, 0, data)
    }

    /// Inserts `data` into the data of `node` at `offset` (`insertData`).
    pub fn insert_data(&mut self, node: NodeId, offset: usize, data: &str) -> WriteResult<()> {
        self.replace_data(node, offset, 0, data)
    }

    /// Deletes `count` code units of the data of `node` from `offset` (`deleteData`).
    pub fn delete_data(&mut self, node: NodeId, offset: usize, count: usize) -> WriteResult<()> {
        self.replace_data(node, offset, count, "")
    }

    /// Sets the text content of `node` (the `textContent` setter): an element's or a fragment's
    /// children leave it, and a text node of `value` takes their place unless `value` is empty
    /// (the DOM's "string replace all"); character data gets `value` as its data; a document or
    /// a doctype is left as it is.
    pub fn set_text_content(&mut self, node: NodeId, value: &str) -> WriteResult<()> {
        match self.nodes[node.0].data {
            NodeData::Element { .. } | NodeData::DocumentFragment => {
                let text = if value.is_empty() {
                    None
                } else {
                    Some(self.create_text_node(value)?)
                };
                // The list is taken whole: no room is kept.
                self.remove_children(node, &Cell::new(0));
                if let Some(text) = text {
                    self.insert_checked(text, node, None);
                }
                Ok(())
            }
            NodeData::Text(_) | NodeData::Comment(_) | NodeData::ProcessingInstruction { .. } => {
                self.set_data(node, value)
            }
            NodeData::Document | NodeData::Doctype { .. } => Ok(()),
        }
    }

    /// The namespace and the attributes of the element `node`.
    fn element_attributes(&self, node: NodeId) -> WriteResult<(Namespace, &Vec<Attribute>)> {
        match &self.nodes[node.0].data {
            NodeData::Element {
                namespace,
                attributes,
                ..
            } => Ok((*namespace, attributes)),
            _ => Err(WriteError::InvalidNodeType),
        }
    }

    fn attributes_mut(&mut self, node: NodeId) -> &mut Vec<Attribute> {
        match &mut self.nodes[node.0].data {
            NodeData::Element { attributes, .. } => attributes,
            _ => unreachable!("checked as an element"),
        }
    }

    /// A name an attribute method of an HTML element is given, in ASCII lower case, as the DOM
    /// does in an HTML document.
    fn attribute_name(namespace: Namespace, name: &str) -> String {
        if namespace == Namespace::Html {
            name.to_ascii_lowercase()
        } else {
            name.to_string()
        }
    }

    /// Gives the attribute at `at` of `element` the value `value`, or appends `attribute` when
    /// `at` is `None`, within the attribute and stored-byte limits.
    fn put_attribute(
        &mut self,
        element: NodeId,
        at: Option<usize>,
        attribute: Attribute,
    ) -> WriteResult<()> {
        let (_, attributes) = self.element_attributes(element)?;
        match at {
            Some(at) => {
                let old = attributes[at].value.len();
                self.admit_change(old, attribute.value.len())?;
                self.stored_bytes = self.stored_bytes - old + attribute.value.len();
                self.attributes_mut(element)[at].value = attribute.value;
            }
            None => {
                if attributes.len() >= self.limits.max_attributes {
                    return Err(WriteError::Limit(Limit::Attributes));
                }
                let bytes = attribute_bytes(&attribute);
                self.admit(0, bytes)?;
                self.stored_bytes += bytes;
                self.attributes_mut(element).push(attribute);
            }
        }
        Ok(())
    }

    /// Removes the attribute at `at` of `element`; its bytes are freed.
    fn drop_attribute(&mut self, element: NodeId, at: usize) {
        let attributes = self.attributes_mut(element);
        let removed = attributes.remove(at);
        shrink(attributes);
        self.stored_bytes -= attribute_bytes(&removed);
    }

    /// Sets the attribute of `element` whose qualified name is `name` to `value`, or appends one
    /// in no namespace (`setAttribute`); on an HTML element the name is lowered first.
    pub fn set_attribute(&mut self, element: NodeId, name: &str, value: &str) -> WriteResult<()> {
        let (namespace, attributes) = self.element_attributes(element)?;
        let name = Self::attribute_name(namespace, name);
        let at = attributes.iter().position(|a| qualified_name_is(a, &name));
        self.put_attribute(
            element,
            at,
            Attribute {
                namespace: None,
                prefix: None,
                local_name: name,
                value: value.to_string(),
            },
        )
    }

    /// Sets the attribute of `element` in `namespace` whose local name is that of
    /// `qualified_name`, or appends one with its prefix (`setAttributeNS`, the DOM's "validate
    /// and extract" without the check of the name's characters).
    pub fn set_attribute_ns(
        &mut self,
        element: NodeId,
        namespace: Option<Namespace>,
        qualified_name: &str,
        value: &str,
    ) -> WriteResult<()> {
        let (_, attributes) = self.element_attributes(element)?;
        let (prefix, local_name) = match qualified_name.split_once(':') {
            Some((prefix, local)) => (Some(prefix), local),
            None => (None, qualified_name),
        };
        if (prefix.is_some() && namespace.is_none())
            || (prefix == Some("xml") && namespace != Some(Namespace::Xml))
            || ((qualified_name == "xmlns" || prefix == Some("xmlns"))
                && namespace != Some(Namespace::Xmlns))
            || (namespace == Some(Namespace::Xmlns)
                && qualified_name != "xmlns"
                && prefix != Some("xmlns"))
        {
            return Err(WriteError::Namespace);
        }
        let at = attributes
            .iter()
            .position(|a| a.namespace == namespace && a.local_name == local_name);
        self.put_attribute(
            element,
            at,
            Attribute {
                namespace,
                prefix: prefix.map(str::to_string),
                local_name: local_name.to_string(),
                value: value.to_string(),
            },
        )
    }

    /// Removes the first attribute of `element` whose qualified name is `name`
    /// (`removeAttribute`); on an HTML element the name is lowered first.
    pub fn remove_attribute(&mut self, element: NodeId, name: &str) -> WriteResult<()> {
        let (namespace, attributes) = self.element_attributes(element)?;
        let name = Self::attribute_name(namespace, name);
        if let Some(at) = attributes.iter().position(|a| qualified_name_is(a, &name)) {
            self.drop_attribute(element, at);
        }
        Ok(())
    }

    /// Removes the attribute of `element` in `namespace` named `local_name`
    /// (`removeAttributeNS`).
    pub fn remove_attribute_ns(
        &mut self,
        element: NodeId,
        namespace: Option<Namespace>,
        local_name: &str,
    ) -> WriteResult<()> {
        let (_, attributes) = self.element_attributes(element)?;
        if let Some(at) = attributes
            .iter()
            .position(|a| a.namespace == namespace && a.local_name == local_name)
        {
            self.drop_attribute(element, at);
        }
        Ok(())
    }

    /// Toggles the attribute of `element` whose qualified name is `name` (`toggleAttribute`):
    /// with no `force` it is removed when present and added with an empty value when not;
    /// `Some(true)` only adds and `Some(false)` only removes. Answers whether it is present
    /// after.
    pub fn toggle_attribute(
        &mut self,
        element: NodeId,
        name: &str,
        force: Option<bool>,
    ) -> WriteResult<bool> {
        let (namespace, attributes) = self.element_attributes(element)?;
        let name = Self::attribute_name(namespace, name);
        match attributes.iter().position(|a| qualified_name_is(a, &name)) {
            None if force == Some(false) => Ok(false),
            None => {
                self.put_attribute(
                    element,
                    None,
                    Attribute {
                        namespace: None,
                        prefix: None,
                        local_name: name,
                        value: String::new(),
                    },
                )?;
                Ok(true)
            }
            Some(_) if force == Some(true) => Ok(true),
            Some(at) => {
                self.drop_attribute(element, at);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::limits::{ATTRIBUTE_BYTES, NODE_BYTES};

    /// The numbers `limits.rs` states for the records, on the 64-bit targets surf is built for.
    #[test]
    fn the_record_sizes_are_the_numbers_the_limits_state() {
        assert_eq!([ATTRIBUTE_BYTES, NODE_BYTES], [80, 136]);
    }

    /// A list that removals shrank holds at most twice its entries (or four): the room no
    /// public call shows, which keeps the heap inside the bound of the stored-byte limit.
    #[test]
    fn removals_give_a_list_its_room_back() {
        use super::{Limits, Namespace, NodeData, Tree};
        let mut tree = Tree::document(Limits::default());
        let div = tree.create_element_ns(Namespace::Html, "div").unwrap();
        let mut texts = Vec::new();
        for n in 0..100 {
            let text = tree.create_text_node("t").unwrap();
            tree.append_child(div, text).unwrap();
            tree.set_attribute(div, &format!("a{n}"), "v").unwrap();
            texts.push(text);
        }
        for (n, &text) in texts.iter().enumerate().skip(1) {
            tree.remove_node(text).unwrap();
            tree.remove_attribute(div, &format!("a{n}")).unwrap();
        }
        let NodeData::Element { attributes, .. } = &tree.nodes[div.0].data else {
            unreachable!("a div is an element");
        };
        assert!(tree.nodes[div.0].children.capacity() <= 4);
        assert!(attributes.capacity() <= 4);
    }
}
