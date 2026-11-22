//! Tree construction (13.2.6): the insertion modes. Written by `story:html-tree-body` and the
//! tree stories after it.
//!
//! Written: the parse state of 13.2.4 (the insertion mode and the original insertion mode, the
//! stack of open elements with its scopes, the list of active formatting elements with its
//! markers, the head and form element pointers, the frameset-ok flag, the scripting flag),
//! creating and inserting nodes (13.2.6.1) without foster parenting, the generic raw text and
//! RCDATA element parsing algorithms (13.2.6.2), generating implied end tags (13.2.6.3), and the
//! insertion modes initial, before html, before head, in head, in head noscript, after head, in
//! body, text, after body, in frameset, after frameset, after after body and after after
//! frameset (13.2.6.4), in the submodule `modes`, each rule in the standard's order; the Noah's
//! Ark clause of the list of active formatting elements, and the adoption agency algorithm of
//! in body in the submodule `formatting` (story `html-tree-formatting`). The tree construction
//! dispatcher always takes the rules of the current insertion mode: no element in a foreign
//! namespace is created.
//!
//! Not yet written, each by a later story, and named by [`NOT_YET_PARSED`]: the table and select
//! modes and foster parenting (`html-tree-tables`); the in template mode (`html-tree-template`);
//! foreign content and the fragment case (`html-tree-foreign`). Until then a token whose rule
//! one of them writes is processed by in body's "any other start tag" or "any other end tag",
//! so that every input ends.
//!
//! Parse errors are not collected: neither the tokenizer's nor the tree builder's reach a
//! caller. Scripts do not run: at a `script` end tag the text mode pops the element and returns
//! to the original insertion mode.
//!
//! The limits (story `html-parser-limits`, the crate documentation's "Limits"): every element,
//! node, text and attribute merge is admitted against the depth, node, stored-byte and attribute
//! limits before it is made, and every entry a scan visits is counted as a step, checked after
//! each token. A limit reached inside a token stops parsing: the rest of that token's rule
//! makes no change to the tree (each creation is refused once parsing has stopped), the
//! dispatcher takes no further mode, and the parse then pops every open element.

mod formatting;
mod modes;

use std::cell::Cell;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::ParseOptions;
use crate::encoding::{self, Confidence};
use crate::limits::{Limit, Limits};
use crate::tokenizer::{self, Token, TokenAttribute, Tokenizer};
use crate::tree::{Attribute, Categories, Namespace, NodeData, NodeId, Tree};

/// Each part of tree construction that the tree builder does not yet parse as the standard
/// does, named in `[a-z-]+`: tables and `select`, `template`, and foreign content (SVG, MathML
/// and fragments). Each later tree story removes its entry.
pub const NOT_YET_PARSED: &[&str] = &["tables-and-select", "template", "foreign-content"];

/// The mode of a document (DOM, "mode"), which the initial insertion mode decides from the
/// DOCTYPE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuirksMode {
    NoQuirks,
    LimitedQuirks,
    Quirks,
}

/// An insertion mode (13.2.4.1) of the ones written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
}

/// What processing a token by the rules of a mode leaves to do.
enum Flow {
    /// The token is processed.
    Done,
    /// Reprocess the token in the (new) current insertion mode.
    Reprocess,
    /// Process this token in place of the one processed ("change the token's tag name to `img`
    /// and reprocess it").
    Replace(Token),
}

/// A start tag token as an element of the list of active formatting elements keeps it: the
/// element is created again from it when the list is reconstructed. It keeps a key for Noah's
/// Ark with it: its attributes in the order of their names, and a hash of the tag name and of
/// those attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TagToken {
    name: String,
    attributes: Vec<TokenAttribute>,
    /// The indices of `attributes`, in the order of the attributes' names.
    by_name: Vec<usize>,
    /// A hash of the tag name and of the attributes, taken in the order of `by_name`.
    key: u64,
}

impl TagToken {
    fn new(name: &str, attributes: &[TokenAttribute]) -> TagToken {
        let mut by_name: Vec<usize> = (0..attributes.len()).collect();
        by_name.sort_unstable_by(|&a, &b| attributes[a].name.cmp(&attributes[b].name));
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        for &i in &by_name {
            attributes[i].name.hash(&mut hasher);
            attributes[i].value.hash(&mut hasher);
        }
        TagToken {
            name: name.to_string(),
            attributes: attributes.to_vec(),
            by_name,
            key: hasher.finish(),
        }
    }

    /// The Noah's Ark comparison: the same tag name, and attributes that pair up with the same
    /// names and values, in any order. A token holds an attribute name at most once (the
    /// tokenizer drops a duplicate), so the attributes taken in the order of their names pair
    /// up one by one. Two tokens whose keys, names or counts differ are told apart in O(1); the
    /// others in O(k) for k attributes.
    fn is_identical(&self, other: &TagToken) -> bool {
        self.key == other.key
            && self.name == other.name
            && self.attributes.len() == other.attributes.len()
            && self
                .by_name
                .iter()
                .zip(&other.by_name)
                .all(|(&a, &b)| self.attributes[a] == other.attributes[b])
    }

    /// The bytes of strings an element created from this token holds.
    fn stored_bytes(&self) -> usize {
        token_bytes(&self.name, &self.attributes)
    }
}

/// The bytes of strings an element created for a token named `name` with `attributes` holds
/// ([`Tree::stored_bytes`]).
fn token_bytes(name: &str, attributes: &[TokenAttribute]) -> usize {
    name.len()
        + attributes
            .iter()
            .map(|a| a.name.len() + a.value.len())
            .sum::<usize>()
}

/// An entry of the list of active formatting elements (13.2.4.3).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Entry {
    Marker,
    Element { node: NodeId, token: TagToken },
}

/// The encoding of the input and the confidence in it, for the `meta` rules of in head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputEncoding {
    /// The name of the encoding in use ([`encoding::Decoded::encoding`]).
    pub(crate) name: &'static str,
    /// The confidence.
    pub(crate) confidence: Confidence,
}

/// What a parse gives.
pub(crate) struct Parsed {
    /// The document.
    pub(crate) tree: Tree,
    /// The mode the initial insertion mode set.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) quirks_mode: QuirksMode,
    /// The encoding a `meta` changed the encoding to (13.2.3.4, its last step): the parse
    /// stopped at that `meta`, and the caller decodes the same bytes again with this encoding
    /// and parses them with the confidence certain.
    pub(crate) reparse_with: Option<&'static str>,
}

/// Runs the tokenizer and tree construction over decoded text as a document. `encoding` is the
/// encoding the text was decoded from, `None` for text that was given decoded: then no `meta`
/// changes the encoding.
pub(crate) fn parse(
    input: &str,
    options: &ParseOptions,
    encoding: Option<InputEncoding>,
) -> Parsed {
    // The input stream keeps its own copy of the code points: this one goes once it is made.
    let mut builder = {
        let code_points: Vec<u32> = input.chars().map(u32::from).collect();
        TreeBuilder::new(&code_points, options, encoding)
    };
    while !builder.stopped {
        let Some(token) = builder.tokenizer.next_token() else {
            // The tokenizer stopped at a start tag that would keep one attribute too many.
            builder.stop_at(Limit::Attributes);
            break;
        };
        builder.dispatch(token);
        if !builder.stopped && builder.steps.get() > builder.limits.max_steps {
            builder.stop_at(Limit::Steps);
        }
    }
    // "Stop parsing" pops every open element, also after a limit.
    builder.open.clear();
    Parsed {
        tree: builder.tree,
        quirks_mode: builder.quirks_mode,
        reparse_with: builder.reparse_with,
    }
}

/// The tree builder: the parse state of 13.2.4 and the tree it builds.
struct TreeBuilder {
    tokenizer: Tokenizer,
    tree: Tree,
    mode: Mode,
    original_mode: Mode,
    /// The stack of open elements, the topmost (the `html` element) first.
    open: Vec<NodeId>,
    /// The list of active formatting elements, the least recently added first.
    active: Vec<Entry>,
    head: Option<NodeId>,
    form: Option<NodeId>,
    frameset_ok: bool,
    scripting_disabled: bool,
    quirks_mode: QuirksMode,
    /// The next token is ignored when it is U+000A LINE FEED (`pre`, `listing`, `textarea`).
    ignore_line_feed: bool,
    encoding: Option<InputEncoding>,
    reparse_with: Option<&'static str>,
    /// Parsing stopped: at the end of the input, at a `meta` that changes the encoding, or at a
    /// limit. No node is created after it.
    stopped: bool,
    limits: Limits,
    /// The entries visited by the scans so far (the step limit). A `Cell`, so that the scans
    /// that only read the parse state count too.
    steps: Cell<usize>,
    /// For the `html` and `body` elements that a start tag of in body merged attributes into,
    /// the names of their attributes.
    merged_names: Vec<(NodeId, HashSet<String>)>,
}

const HTML_ELEMENT_SPECIAL: &[&str] = &[
    "address",
    "applet",
    "area",
    "article",
    "aside",
    "base",
    "basefont",
    "bgsound",
    "blockquote",
    "body",
    "br",
    "button",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dir",
    "div",
    "dl",
    "dt",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "iframe",
    "img",
    "input",
    "keygen",
    "li",
    "link",
    "listing",
    "main",
    "marquee",
    "menu",
    "meta",
    "nav",
    "noembed",
    "noframes",
    "noscript",
    "object",
    "ol",
    "p",
    "param",
    "plaintext",
    "pre",
    "script",
    "search",
    "section",
    "select",
    "source",
    "style",
    "summary",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
    "wbr",
    "xmp",
];

/// The element types of "has an element in scope" in the HTML namespace.
const SCOPE_HTML: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "select", "template",
];

/// The element types of "has an element in scope" in the MathML namespace.
const SCOPE_MATHML: &[&str] = &["mi", "mo", "mn", "ms", "mtext", "annotation-xml"];

/// The element types of "has an element in scope" in the SVG namespace.
const SCOPE_SVG: &[&str] = &["foreignObject", "desc", "title"];

/// The elements "generate implied end tags" pops (13.2.6.3).
const IMPLIED_END_TAGS: &[&str] = &[
    "dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc",
];

/// The scopes of 13.2.4.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    /// "In scope".
    Default,
    /// "In list item scope": also `ol` and `ul`.
    ListItem,
    /// "In button scope": also `button`.
    Button,
}

fn is_whitespace(c: u32) -> bool {
    matches!(c, 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

/// The categories of an element in `namespace` named `name`, decided once when it is created:
/// special, and the element types of the scopes of 13.2.4.2.
fn categories_of(namespace: Namespace, name: &str) -> Categories {
    let mut bits = 0;
    match namespace {
        Namespace::Html => {
            if HTML_ELEMENT_SPECIAL.contains(&name) {
                bits |= Categories::SPECIAL;
            }
            if SCOPE_HTML.contains(&name) {
                bits |= Categories::SCOPE;
            }
            if name == "ol" || name == "ul" {
                bits |= Categories::LIST_ITEM_SCOPE;
            }
            if name == "button" {
                bits |= Categories::BUTTON_SCOPE;
            }
        }
        Namespace::MathMl if SCOPE_MATHML.contains(&name) => {
            bits |= Categories::SPECIAL | Categories::SCOPE;
        }
        Namespace::Svg if SCOPE_SVG.contains(&name) => {
            bits |= Categories::SPECIAL | Categories::SCOPE;
        }
        _ => {}
    }
    Categories(bits)
}

fn attributes(token: &[TokenAttribute]) -> Vec<Attribute> {
    token
        .iter()
        .map(|a| Attribute {
            namespace: None,
            prefix: None,
            local_name: a.name.clone(),
            value: a.value.clone(),
        })
        .collect()
}

impl TreeBuilder {
    fn new(input: &[u32], options: &ParseOptions, encoding: Option<InputEncoding>) -> TreeBuilder {
        TreeBuilder {
            tokenizer: Tokenizer::new(input, options.limits.max_attributes),
            tree: Tree::document(),
            mode: Mode::Initial,
            original_mode: Mode::Initial,
            open: Vec::new(),
            active: Vec::new(),
            head: None,
            form: None,
            frameset_ok: true,
            scripting_disabled: options.scripting_disabled,
            quirks_mode: QuirksMode::NoQuirks,
            ignore_line_feed: false,
            encoding,
            reparse_with: None,
            stopped: false,
            limits: options.limits,
            steps: Cell::new(0),
            merged_names: Vec::new(),
        }
    }

    /// One entry visited by a scan: a step.
    fn visit(&self) {
        self.add_steps(1);
    }

    /// `n` entries visited by a scan, counted at its end (the hot scans, so that a visit costs no
    /// more than the comparison it makes).
    fn add_steps(&self, n: usize) {
        self.steps.set(self.steps.get().saturating_add(n));
    }

    /// The tree construction dispatcher (13.2.6): every element is in the HTML namespace, so
    /// each token goes to the rules of the current insertion mode.
    fn dispatch(&mut self, token: Token) {
        if std::mem::take(&mut self.ignore_line_feed) && token == Token::Character(0x0A) {
            return;
        }
        let mut token = token;
        loop {
            let flow = match self.mode {
                Mode::Initial => self.initial(&token),
                Mode::BeforeHtml => self.before_html(&token),
                Mode::BeforeHead => self.before_head(&token),
                Mode::InHead => self.in_head(&token),
                Mode::InHeadNoscript => self.in_head_noscript(&token),
                Mode::AfterHead => self.after_head(&token),
                Mode::InBody => self.in_body(&token),
                Mode::Text => self.text(&token),
                Mode::AfterBody => self.after_body(&token),
                Mode::InFrameset => self.in_frameset(&token),
                Mode::AfterFrameset => self.after_frameset(&token),
                Mode::AfterAfterBody => self.after_after_body(&token),
                Mode::AfterAfterFrameset => self.after_after_frameset(&token),
            };
            match flow {
                Flow::Done => return,
                Flow::Reprocess => {}
                Flow::Replace(replacement) => token = replacement,
            }
            if self.stopped {
                return;
            }
        }
    }

    /// Stop parsing (13.2.7, "the end"), as far as it changes the tree: pop every node off the
    /// stack of open elements.
    fn stop_parsing(&mut self) {
        self.open.clear();
        self.stopped = true;
    }

    /// Stops parsing at `limit` (the crate documentation, "Limits"). The open elements stay
    /// until the token's rule has returned, so that what is left of it runs on the state it
    /// expects; it creates nothing more, and [`parse`] then pops them.
    fn stop_at(&mut self, limit: Limit) {
        self.tree.stop_at(limit);
        self.stopped = true;
    }

    /// Whether a creation of `nodes` nodes that hold `bytes` bytes of strings stays inside the
    /// node and stored-byte limits. When it does not, parsing stops at the limit it would pass;
    /// after parsing stopped, nothing is admitted.
    fn admit(&mut self, nodes: usize, bytes: usize) -> bool {
        if self.stopped {
            return false;
        }
        if self.tree.node_count().saturating_add(nodes) > self.limits.max_nodes {
            self.stop_at(Limit::Nodes);
            return false;
        }
        if self.tree.stored_bytes().saturating_add(bytes) > self.limits.max_stored_bytes {
            self.stop_at(Limit::StoredBytes);
            return false;
        }
        true
    }

    /// Whether the stack of open elements has room for one element more (the depth limit).
    /// When it has not, parsing stops.
    fn room_on_stack(&mut self) -> bool {
        if self.stopped {
            return false;
        }
        if self.open.len() >= self.limits.max_depth {
            self.stop_at(Limit::Depth);
            return false;
        }
        true
    }

    // 13.2.6.1 Creating and inserting nodes.

    fn document(&self) -> NodeId {
        self.tree.root()
    }

    /// The appropriate place for inserting a node, with an override target: the target and the
    /// reference child. Foster parenting is never enabled here (only the table modes enable
    /// it). A `template` target gives its template contents.
    fn appropriate_place(&self, override_target: Option<NodeId>) -> (NodeId, Option<NodeId>) {
        let target = override_target.unwrap_or_else(|| self.current_node());
        if let NodeData::Element {
            template_contents: Some(contents),
            ..
        } = self.tree.data(target)
        {
            return (*contents, None);
        }
        (target, None)
    }

    /// Create an element for a token in the HTML namespace: no custom element definition is
    /// looked up and no form element is associated (a form-associated element's form owner is
    /// not part of the tree). `None` when the node or stored-byte limit refuses it (a `template`
    /// is two nodes, with its contents), or parsing stopped.
    fn create_element(&mut self, name: &str, attributes: &[TokenAttribute]) -> Option<NodeId> {
        let nodes = if name == "template" { 2 } else { 1 };
        if !self.admit(nodes, token_bytes(name, attributes)) {
            return None;
        }
        Some(self.create_admitted_element(name, attributes))
    }

    /// Create an element for a token in the HTML namespace that the limits admitted already
    /// (the adoption agency algorithm admits a whole iteration at once).
    fn create_admitted_element(&mut self, name: &str, attributes: &[TokenAttribute]) -> NodeId {
        self.tree.create_element(
            Namespace::Html,
            name,
            self::attributes(attributes),
            categories_of(Namespace::Html, name),
        )
    }

    /// Insert an element at the adjusted insertion location, given an insertion location.
    fn insert_element_at(&mut self, element: NodeId, location: (NodeId, Option<NodeId>)) {
        let (parent, reference) = location;
        if self.tree.parent(element).is_some()
            || (parent == self.document() && self.tree.has_element_child(parent, &self.steps))
        {
            return;
        }
        self.tree.insert(parent, element, reference, &self.steps);
    }

    /// Insert an HTML element for a token: created, inserted at the appropriate place, pushed
    /// onto the stack of open elements. `None` when a limit refuses it (depth first, then nodes
    /// and stored bytes), or parsing stopped.
    fn insert_html_element(&mut self, name: &str, attributes: &[TokenAttribute]) -> Option<NodeId> {
        if !self.room_on_stack() {
            return None;
        }
        let element = self.create_element(name, attributes)?;
        let location = self.appropriate_place(None);
        self.insert_element_at(element, location);
        self.open.push(element);
        Some(element)
    }

    /// Insert a character: `c` appended at the appropriate place, merged into a text node just
    /// before it. A character for the document is dropped. A character the node or stored-byte
    /// limit refuses is not inserted, and parsing stops.
    fn insert_character(&mut self, c: u32) {
        if self.stopped {
            return;
        }
        let (parent, reference) = self.appropriate_place(None);
        if parent == self.document() {
            return;
        }
        let c = char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER);
        let mut buffer = [0; 4];
        let text: &str = c.encode_utf8(&mut buffer);
        match self.tree.text_before(parent, reference, &self.steps) {
            Some(node) => {
                if self.admit(0, text.len()) {
                    self.tree.push_text(node, text);
                }
            }
            None => {
                if self.admit(1, text.len()) {
                    self.tree.insert_new(
                        parent,
                        NodeData::Text(text.to_string()),
                        reference,
                        &self.steps,
                    );
                }
            }
        }
    }

    /// Insert a comment, at the appropriate place or at the location given.
    fn insert_comment(&mut self, data: &str, location: Option<NodeId>) {
        if !self.admit(1, data.len()) {
            return;
        }
        let (parent, reference) = self.appropriate_place(location);
        self.tree.insert_new(
            parent,
            NodeData::Comment(data.to_string()),
            reference,
            &self.steps,
        );
    }

    /// Insert a processing instruction, at the appropriate place or at the location given.
    fn insert_processing_instruction(
        &mut self,
        target: &str,
        data: &str,
        location: Option<NodeId>,
    ) {
        if !self.admit(1, target.len() + data.len()) {
            return;
        }
        let (parent, reference) = self.appropriate_place(location);
        self.tree.insert_new(
            parent,
            NodeData::ProcessingInstruction {
                target: target.to_string(),
                data: data.to_string(),
            },
            reference,
            &self.steps,
        );
    }

    /// The `html` and `body` start tag rules of in body: each attribute of the token whose name
    /// `element` has no attribute of is added to it, in order. A merge that would give the
    /// element more attributes than the attribute limit, or pass the stored-byte limit, is not
    /// made, and parsing stops. The names of the element's attributes are kept in a set, so the
    /// merge costs O(1) for each attribute of the token.
    fn merge_attributes(&mut self, element: NodeId, attributes: &[TokenAttribute]) {
        if self.stopped {
            return;
        }
        let at = match self.merged_names.iter().position(|(n, _)| *n == element) {
            Some(at) => at,
            None => {
                let names = match self.tree.data(element) {
                    NodeData::Element { attributes, .. } => {
                        attributes.iter().map(|a| a.local_name.clone()).collect()
                    }
                    _ => HashSet::new(),
                };
                self.merged_names.push((element, names));
                self.merged_names.len() - 1
            }
        };
        let names = &self.merged_names[at].1;
        let new: Vec<TokenAttribute> = attributes
            .iter()
            .filter(|a| !names.contains(&a.name))
            .cloned()
            .collect();
        if new.is_empty() {
            return;
        }
        if names.len() + new.len() > self.limits.max_attributes {
            self.stop_at(Limit::Attributes);
            return;
        }
        if !self.admit(0, token_bytes("", &new)) {
            return;
        }
        self.merged_names[at]
            .1
            .extend(new.iter().map(|a| a.name.clone()));
        self.tree.add_attributes(element, self::attributes(&new));
    }

    // 13.2.6.2 Parsing elements that contain only text.

    /// The generic raw text element parsing algorithm (`rawtext`) and the generic RCDATA
    /// element parsing algorithm.
    fn parse_text_element(&mut self, name: &str, attributes: &[TokenAttribute], rawtext: bool) {
        self.insert_html_element(name, attributes);
        self.tokenizer.switch_to(if rawtext {
            tokenizer::State::Rawtext
        } else {
            tokenizer::State::Rcdata
        });
        self.original_mode = self.mode;
        self.mode = Mode::Text;
    }

    // 13.2.6.3 Closing elements that have implied end tags.

    /// Generate implied end tags, except for HTML elements named `except`.
    fn generate_implied_end_tags(&mut self, except: Option<&str>) {
        while let Some(&node) = self.open.last() {
            self.visit();
            match self.html_name(node) {
                Some(name) if IMPLIED_END_TAGS.contains(&name) && Some(name) != except => {
                    self.open.pop();
                }
                _ => return,
            }
        }
    }

    // 13.2.4.2 The stack of open elements.

    fn current_node(&self) -> NodeId {
        *self
            .open
            .last()
            .expect("the stack of open elements is not empty")
    }

    /// The local name of `node` when it is an element in the HTML namespace.
    fn html_name(&self, node: NodeId) -> Option<&str> {
        match self.tree.data(node) {
            NodeData::Element {
                namespace: Namespace::Html,
                local_name,
                ..
            } => Some(local_name),
            _ => None,
        }
    }

    /// Whether `node` is an HTML element named one of `names`.
    fn is_html(&self, node: NodeId, names: &[&str]) -> bool {
        self.html_name(node).is_some_and(|n| names.contains(&n))
    }

    /// Whether the current node is an HTML element named one of `names`.
    fn current_is(&self, names: &[&str]) -> bool {
        self.open.last().is_some_and(|&n| self.is_html(n, names))
    }

    /// Whether `node` is in the special category, as decided when it was created.
    fn is_special(&self, node: NodeId) -> bool {
        self.tree.categories(node).has(Categories::SPECIAL)
    }

    /// Whether `node` is one of the element types of `scope`, as decided when it was created.
    fn bounds_scope(&self, node: NodeId, scope: Scope) -> bool {
        let categories = self.tree.categories(node);
        categories.has(Categories::SCOPE)
            || match scope {
                Scope::Default => false,
                Scope::ListItem => categories.has(Categories::LIST_ITEM_SCOPE),
                Scope::Button => categories.has(Categories::BUTTON_SCOPE),
            }
    }

    /// Whether the stack of open elements has an element that `is_target` holds for in `scope`.
    fn has_in_scope(&self, scope: Scope, is_target: impl Fn(&Self, NodeId) -> bool) -> bool {
        let mut visited = 0;
        let mut found = false;
        for &node in self.open.iter().rev() {
            visited += 1;
            if is_target(self, node) {
                found = true;
                break;
            }
            if self.bounds_scope(node, scope) {
                break;
            }
        }
        self.add_steps(visited);
        found
    }

    /// Whether the stack of open elements has an HTML element named `name` in `scope`.
    fn has_named_in_scope(&self, name: &str, scope: Scope) -> bool {
        self.has_in_scope(scope, |b, node| b.html_name(node) == Some(name))
    }

    /// Pops the current node off the stack until an HTML element named one of `names` has been
    /// popped.
    fn pop_until(&mut self, names: &[&str]) {
        while let Some(node) = self.open.pop() {
            self.visit();
            if self.is_html(node, names) {
                return;
            }
        }
    }

    /// The index of `element` in the stack of open elements, scanning from the current node.
    fn stack_position(&self, element: NodeId) -> Option<usize> {
        let at = self.open.iter().rposition(|&n| n == element);
        self.add_steps(at.map_or(self.open.len(), |at| self.open.len() - at));
        at
    }

    /// Removes `element` from the stack of open elements, scanning from the current node.
    fn remove_from_stack(&mut self, element: NodeId) {
        if let Some(at) = self.stack_position(element) {
            self.open.remove(at);
        }
    }

    // 13.2.4.3 The list of active formatting elements.

    /// Push onto the list of active formatting elements, with the Noah's Ark clause. The scan
    /// runs from the end of the list to the last marker.
    fn push_active_formatting_element(
        &mut self,
        node: NodeId,
        name: &str,
        attributes: &[TokenAttribute],
    ) {
        let token = TagToken::new(name, attributes);
        // 1. Three elements after the last marker with the same tag name, namespace (every
        // element of the list is in the HTML namespace) and attributes as the token's: the
        // earliest of them is removed.
        let mut identical = 0;
        let mut earliest = None;
        for (index, entry) in self.active.iter().enumerate().rev() {
            self.visit();
            match entry {
                Entry::Marker => break,
                Entry::Element { token: other, .. } if other.is_identical(&token) => {
                    identical += 1;
                    earliest = Some(index);
                }
                Entry::Element { .. } => {}
            }
        }
        if identical >= 3
            && let Some(index) = earliest
        {
            self.active.remove(index);
        }
        // 2.
        self.active.push(Entry::Element { node, token });
    }

    /// Reconstruct the active formatting elements, if any. "Is in the stack of open elements"
    /// is asked from the current node down: an entry that is open is almost always near it.
    /// Each element is admitted by the limits before it is created; one they refuse ends the
    /// reconstruction, with the elements created before it in the tree.
    fn reconstruct_active_formatting_elements(&mut self) {
        let is_open_or_marker = |b: &Self, entry: &Entry| match entry {
            Entry::Marker => true,
            Entry::Element { node, .. } => b.stack_position(*node).is_some(),
        };
        match self.active.last() {
            None => return,
            Some(last) if is_open_or_marker(self, last) => return,
            Some(_) => {}
        }
        // Rewind: back to the entry after the last marker or open element, or the first entry.
        let mut index = self.active.len() - 1;
        while index > 0 && !is_open_or_marker(self, &self.active[index - 1]) {
            self.visit();
            index -= 1;
        }
        // Advance and create, to the end of the list.
        for i in index..self.active.len() {
            self.visit();
            let Entry::Element { token, .. } = self.active[i].clone() else {
                continue;
            };
            let Some(node) = self.insert_html_element(&token.name, &token.attributes) else {
                return;
            };
            self.active[i] = Entry::Element { node, token };
        }
    }

    /// Clear the list of active formatting elements up to the last marker.
    fn clear_active_formatting_elements_to_last_marker(&mut self) {
        while let Some(entry) = self.active.pop() {
            self.visit();
            if entry == Entry::Marker {
                return;
            }
        }
    }

    /// The index in the list of active formatting elements of `node`, if it is there. The list
    /// holds a node at most once; the scan starts at the end of the list, where the standard's
    /// rules look ("the last element in the list").
    fn active_position(&self, node: NodeId) -> Option<usize> {
        self.active.iter().rposition(|e| {
            self.visit();
            matches!(e, Entry::Element { node: n, .. } if *n == node)
        })
    }

    /// The index of `node` in the list of active formatting elements after the last marker (or
    /// in the whole list when there is none), if it is there: the scan ends at the marker.
    fn active_position_after_last_marker(&self, node: NodeId) -> Option<usize> {
        for (index, entry) in self.active.iter().enumerate().rev() {
            self.visit();
            match entry {
                Entry::Marker => return None,
                Entry::Element { node: n, .. } if *n == node => return Some(index),
                Entry::Element { .. } => {}
            }
        }
        None
    }

    // 13.2.3.4 Changing the encoding while parsing.

    /// Change the encoding to `new`, for a `meta` of in head while the confidence is
    /// tentative. Step 5, changing the converter on the fly, is optional and not taken; step 6,
    /// restarting the navigation, stops this parse and leaves the caller to parse the same bytes
    /// again with `new` and the confidence certain.
    fn change_the_encoding(&mut self, new: &'static str) {
        let Some(encoding) = self.encoding.as_mut() else {
            return;
        };
        // 1. The encoding in use is UTF-16BE/LE.
        if matches!(encoding.name, "UTF-16BE" | "UTF-16LE") {
            encoding.confidence = Confidence::Certain;
            return;
        }
        // 2. A new UTF-16BE/LE is UTF-8. 3. A new x-user-defined is windows-1252.
        let new = match new {
            "UTF-16BE" | "UTF-16LE" => "UTF-8",
            "x-user-defined" => "windows-1252",
            other => other,
        };
        // 4. The same encoding.
        if new == encoding.name {
            encoding.confidence = Confidence::Certain;
            return;
        }
        // 6. Parse again from the same bytes.
        self.reparse_with = Some(new);
        self.stop_parsing();
    }

    /// The `meta` rule of in head after the element is inserted: a `charset` attribute, or an
    /// `http-equiv` of `Content-Type` with a `content`, while the confidence is tentative.
    fn meta_changes_the_encoding(&mut self, attributes: &[TokenAttribute]) {
        // A limit reached before it means no parse again.
        if self.stopped || self.encoding.map(|e| e.confidence) != Some(Confidence::Tentative) {
            return;
        }
        let value = |name: &str| {
            attributes
                .iter()
                .find(|a| a.name == name)
                .map(|a| a.value.as_str())
        };
        if let Some(new) = value("charset").and_then(encoding::get_an_encoding) {
            self.change_the_encoding(new);
        } else if value("http-equiv").is_some_and(|v| v.eq_ignore_ascii_case("Content-Type"))
            && let Some(new) = value("content")
                .and_then(|content| encoding::extract_from_meta_content(content.as_bytes()))
        {
            self.change_the_encoding(new);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quirks_mode(input: &str) -> QuirksMode {
        parse(input, &ParseOptions::default(), None).quirks_mode
    }

    #[test]
    fn the_doctype_decides_the_quirks_mode() {
        assert_eq!(quirks_mode("<p>"), QuirksMode::Quirks);
        assert_eq!(
            quirks_mode("<!DOCTYPE html PUBLIC \"-//W3O//DTD W3 HTML Strict 3.0//EN//\"><p>"),
            QuirksMode::Quirks
        );
        assert_eq!(
            quirks_mode(
                "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \
                 \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\"><p>"
            ),
            QuirksMode::LimitedQuirks
        );
        assert_eq!(quirks_mode("<!DOCTYPE html><p>"), QuirksMode::NoQuirks);
    }
}
