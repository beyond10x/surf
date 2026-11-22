//! The node arena tree construction builds: a document or a document fragment and the nodes
//! under it, each named by a [`NodeId`] that stays the same for the life of the tree. Read
//! only from outside the crate: every public function takes `&self`, and the operations that
//! change a tree are those tree construction uses, crate-private. Written by
//! `story:html-tree-body`. No reader recurses over the depth of the tree.
//!
//! Since `story:html-parser-limits` the tree keeps what the limits read as it is built: the
//! number of its nodes ([`Tree::node_count`]), the bytes of its strings and attribute records
//! ([`Tree::stored_bytes`]), the limit that stopped its parse ([`Tree::stopped_by`]) and the
//! steps the parse used ([`Tree::steps`]). The operations that scan a node's children add each
//! child they visit to the step count they are given. Each element keeps its categories
//! (special, the scopes it bounds), decided once when it is created.

use std::cell::Cell;

use crate::limits::{ATTRIBUTE_BYTES, Limit};

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
/// Every node created while parsing stays in the arena for the life of the tree, so a
/// [`NodeId`] never changes its meaning. A node removed from its parent while parsing (the
/// `body` that a `frameset` start tag takes the place of) has no parent and is reached from the
/// root no longer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    nodes: Vec<Node>,
    stored_bytes: usize,
    stopped_by: Option<Limit>,
    steps: usize,
}

impl Tree {
    /// A tree holding one [`NodeData::Document`] and nothing else.
    pub(crate) fn document() -> Tree {
        Tree::with_root(NodeData::Document)
    }

    /// A tree holding one [`NodeData::DocumentFragment`] and nothing else.
    pub(crate) fn fragment() -> Tree {
        Tree::with_root(NodeData::DocumentFragment)
    }

    fn with_root(data: NodeData) -> Tree {
        let mut tree = Tree {
            nodes: Vec::new(),
            stored_bytes: 0,
            stopped_by: None,
            steps: 0,
        };
        tree.create(data, 0);
        tree
    }

    /// Adds a node with no parent to the arena.
    fn create(&mut self, data: NodeData, categories: u8) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.stored_bytes += stored_bytes_of(&data);
        self.nodes.push(Node {
            data,
            parent: None,
            children: Vec::new(),
            categories,
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
    /// child when `reference` is `None` (the DOM's "insert").
    pub(crate) fn insert(
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
    /// to newElement"). `to` is not `from` and not under it. Each child moved is a step.
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
    /// tree yet.
    pub fn steps(&self) -> usize {
        self.steps
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

#[cfg(test)]
mod tests {
    use crate::limits::{ATTRIBUTE_BYTES, NODE_BYTES};

    /// The numbers `limits.rs` states for the records, on the 64-bit targets surf is built for.
    #[test]
    fn the_record_sizes_are_the_numbers_the_limits_state() {
        assert_eq!([ATTRIBUTE_BYTES, NODE_BYTES], [80, 136]);
    }
}
