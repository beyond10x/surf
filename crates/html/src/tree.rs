//! The node arena tree construction builds: a document or a document fragment and the nodes
//! under it, each named by a [`NodeId`] that stays the same for the life of the tree. Read
//! only from outside the crate. Its operations are written by `story:html-tree-body` and the
//! tree stories after it.

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
    children: Vec<NodeId>,
}

/// A tree of nodes: a document or a document fragment at its root, every other node under it
/// or under a template's contents.
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
        Tree {
            nodes: vec![Node {
                data,
                children: Vec::new(),
            }],
        }
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
