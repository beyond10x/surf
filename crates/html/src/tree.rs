//! The node arena tree construction builds: a document or a document fragment and the nodes
//! under it, each named by a [`NodeId`] that stays the same for the life of the tree. Read
//! only from outside the crate: every public function takes `&self`, and the operations that
//! change a tree are those tree construction uses, crate-private. Written by
//! `story:html-tree-body`. No reader recurses over the depth of the tree.

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
        let mut tree = Tree { nodes: Vec::new() };
        tree.create(data);
        tree
    }

    /// Adds a node with no parent to the arena.
    fn create(&mut self, data: NodeData) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            data,
            parent: None,
            children: Vec::new(),
        });
        id
    }

    /// Creates an element with no parent (the DOM's "create an element", with no custom element
    /// definition looked up). An HTML `template` gets its template contents, a document fragment
    /// of this tree.
    pub(crate) fn create_element(
        &mut self,
        namespace: Namespace,
        local_name: &str,
        attributes: Vec<Attribute>,
    ) -> NodeId {
        let template_contents = (namespace == Namespace::Html && local_name == "template")
            .then(|| self.create(NodeData::DocumentFragment));
        self.create(NodeData::Element {
            namespace,
            local_name: local_name.to_string(),
            attributes,
            template_contents,
        })
    }

    /// Inserts `node`, which has no parent, into `parent` before `reference`, or as its last
    /// child when `reference` is `None` (the DOM's "insert").
    pub(crate) fn insert(&mut self, parent: NodeId, node: NodeId, reference: Option<NodeId>) {
        debug_assert!(self.nodes[node.0].parent.is_none());
        let children = &mut self.nodes[parent.0].children;
        let at = reference
            .and_then(|r| children.iter().position(|&c| c == r))
            .unwrap_or(children.len());
        children.insert(at, node);
        self.nodes[node.0].parent = Some(parent);
    }

    /// Creates a node of `data` and inserts it into `parent` before `reference`.
    pub(crate) fn insert_new(
        &mut self,
        parent: NodeId,
        data: NodeData,
        reference: Option<NodeId>,
    ) -> NodeId {
        let node = self.create(data);
        self.insert(parent, node, reference);
        node
    }

    /// Inserts `text` into `parent` before `reference`: appended to the data of the node before
    /// that place when it is a text node, otherwise as a new text node ("flush pending text",
    /// 13.2.6.1).
    pub(crate) fn insert_text(&mut self, parent: NodeId, text: &str, reference: Option<NodeId>) {
        let children = &self.nodes[parent.0].children;
        let at = reference
            .and_then(|r| children.iter().position(|&c| c == r))
            .unwrap_or(children.len());
        let previous = at.checked_sub(1).map(|i| children[i]);
        if let Some(previous) = previous
            && let NodeData::Text(data) = &mut self.nodes[previous.0].data
        {
            data.push_str(text);
            return;
        }
        self.insert_new(parent, NodeData::Text(text.to_string()), reference);
    }

    /// Adds to `element` each of `attributes` whose name it has no attribute of, in order (the
    /// `html` and `body` start tag rules of "in body").
    pub(crate) fn add_missing_attributes(&mut self, element: NodeId, attributes: &[Attribute]) {
        if let NodeData::Element {
            attributes: existing,
            ..
        } = &mut self.nodes[element.0].data
        {
            for attribute in attributes {
                if !existing
                    .iter()
                    .any(|a| a.local_name == attribute.local_name)
                {
                    existing.push(attribute.clone());
                }
            }
        }
    }

    /// Removes `node` from its parent, if it has one. The node stays in the arena.
    pub(crate) fn remove(&mut self, node: NodeId) {
        if let Some(parent) = self.nodes[node.0].parent.take() {
            self.nodes[parent.0].children.retain(|&c| c != node);
        }
    }

    /// Whether `node` has a child that is an element.
    pub(crate) fn has_element_child(&self, node: NodeId) -> bool {
        self.children(node)
            .iter()
            .any(|&c| matches!(self.data(c), NodeData::Element { .. }))
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
