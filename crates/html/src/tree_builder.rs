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

mod formatting;
mod modes;

use crate::ParseOptions;
use crate::encoding::{self, Confidence};
use crate::tokenizer::{self, Token, TokenAttribute, Tokenizer};
use crate::tree::{Attribute, Namespace, NodeData, NodeId, Tree};

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
/// element is created again from it when the list is reconstructed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TagToken {
    name: String,
    attributes: Vec<TokenAttribute>,
}

impl TagToken {
    /// The Noah's Ark comparison: the same tag name, and attributes that pair up with the same
    /// names and values, in any order. A token holds an attribute name at most once (the
    /// tokenizer drops a duplicate), so equal counts and a match for each attribute pair them.
    fn is_identical(&self, other: &TagToken) -> bool {
        self.name == other.name
            && self.attributes.len() == other.attributes.len()
            && self.attributes.iter().all(|a| {
                other
                    .attributes
                    .iter()
                    .any(|b| b.name == a.name && b.value == a.value)
            })
    }
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
    let code_points: Vec<u32> = input.chars().map(u32::from).collect();
    let mut builder = TreeBuilder::new(&code_points, options, encoding);
    while !builder.stopped {
        let token = builder.tokenizer.next_token();
        builder.dispatch(token);
    }
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
    stopped: bool,
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
            tokenizer: Tokenizer::new(input),
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
        }
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
    /// not part of the tree).
    fn create_element(&mut self, name: &str, attributes: &[TokenAttribute]) -> NodeId {
        self.tree
            .create_element(Namespace::Html, name, self::attributes(attributes))
    }

    /// Insert an element at the adjusted insertion location, given an insertion location.
    fn insert_element_at(&mut self, element: NodeId, location: (NodeId, Option<NodeId>)) {
        let (parent, reference) = location;
        if self.tree.parent(element).is_some()
            || (parent == self.document() && self.tree.has_element_child(parent))
        {
            return;
        }
        self.tree.insert(parent, element, reference);
    }

    /// Insert an HTML element for a token: created, inserted at the appropriate place, pushed
    /// onto the stack of open elements.
    fn insert_html_element(&mut self, name: &str, attributes: &[TokenAttribute]) -> NodeId {
        let element = self.create_element(name, attributes);
        let location = self.appropriate_place(None);
        self.insert_element_at(element, location);
        self.open.push(element);
        element
    }

    /// Insert a character: `c` appended at the appropriate place, merged into a text node just
    /// before it. A character for the document is dropped.
    fn insert_character(&mut self, c: u32) {
        let (parent, reference) = self.appropriate_place(None);
        if parent == self.document() {
            return;
        }
        let c = char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER);
        self.tree
            .insert_text(parent, c.encode_utf8(&mut [0; 4]), reference);
    }

    /// Insert a comment, at the appropriate place or at the location given.
    fn insert_comment(&mut self, data: &str, location: Option<NodeId>) {
        let (parent, reference) = self.appropriate_place(location);
        self.tree
            .insert_new(parent, NodeData::Comment(data.to_string()), reference);
    }

    /// Insert a processing instruction, at the appropriate place or at the location given.
    fn insert_processing_instruction(
        &mut self,
        target: &str,
        data: &str,
        location: Option<NodeId>,
    ) {
        let (parent, reference) = self.appropriate_place(location);
        self.tree.insert_new(
            parent,
            NodeData::ProcessingInstruction {
                target: target.to_string(),
                data: data.to_string(),
            },
            reference,
        );
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

    /// Whether `node` is in the special category.
    fn is_special(&self, node: NodeId) -> bool {
        match self.tree.data(node) {
            NodeData::Element {
                namespace,
                local_name,
                ..
            } => {
                let name = local_name.as_str();
                match namespace {
                    Namespace::Html => HTML_ELEMENT_SPECIAL.contains(&name),
                    Namespace::MathMl => SCOPE_MATHML.contains(&name),
                    Namespace::Svg => SCOPE_SVG.contains(&name),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Whether `node` is one of the element types of `scope`.
    fn bounds_scope(&self, node: NodeId, scope: Scope) -> bool {
        let NodeData::Element {
            namespace,
            local_name,
            ..
        } = self.tree.data(node)
        else {
            return false;
        };
        let name = local_name.as_str();
        match namespace {
            Namespace::Html => {
                SCOPE_HTML.contains(&name)
                    || match scope {
                        Scope::Default => false,
                        Scope::ListItem => name == "ol" || name == "ul",
                        Scope::Button => name == "button",
                    }
            }
            Namespace::MathMl => SCOPE_MATHML.contains(&name),
            Namespace::Svg => SCOPE_SVG.contains(&name),
            _ => false,
        }
    }

    /// Whether the stack of open elements has an element that `is_target` holds for in `scope`.
    fn has_in_scope(&self, scope: Scope, is_target: impl Fn(&Self, NodeId) -> bool) -> bool {
        for &node in self.open.iter().rev() {
            if is_target(self, node) {
                return true;
            }
            if self.bounds_scope(node, scope) {
                return false;
            }
        }
        false
    }

    /// Whether the stack of open elements has an HTML element named `name` in `scope`.
    fn has_named_in_scope(&self, name: &str, scope: Scope) -> bool {
        self.has_in_scope(scope, |b, node| b.html_name(node) == Some(name))
    }

    /// Pops the current node off the stack until an HTML element named one of `names` has been
    /// popped.
    fn pop_until(&mut self, names: &[&str]) {
        while let Some(node) = self.open.pop() {
            if self.is_html(node, names) {
                return;
            }
        }
    }

    /// Removes `element` from the stack of open elements.
    fn remove_from_stack(&mut self, element: NodeId) {
        if let Some(at) = self.open.iter().rposition(|&n| n == element) {
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
        let token = TagToken {
            name: name.to_string(),
            attributes: attributes.to_vec(),
        };
        // 1. Three elements after the last marker with the same tag name, namespace (every
        // element of the list is in the HTML namespace) and attributes as the token's: the
        // earliest of them is removed.
        let mut identical = 0;
        let mut earliest = None;
        for (index, entry) in self.active.iter().enumerate().rev() {
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
    fn reconstruct_active_formatting_elements(&mut self) {
        let is_open_or_marker = |b: &Self, entry: &Entry| match entry {
            Entry::Marker => true,
            Entry::Element { node, .. } => b.open.iter().rev().any(|n| n == node),
        };
        match self.active.last() {
            None => return,
            Some(last) if is_open_or_marker(self, last) => return,
            Some(_) => {}
        }
        // Rewind: back to the entry after the last marker or open element, or the first entry.
        let mut index = self.active.len() - 1;
        while index > 0 && !is_open_or_marker(self, &self.active[index - 1]) {
            index -= 1;
        }
        // Advance and create, to the end of the list.
        for i in index..self.active.len() {
            let Entry::Element { token, .. } = self.active[i].clone() else {
                continue;
            };
            let node = self.insert_html_element(&token.name, &token.attributes);
            self.active[i] = Entry::Element { node, token };
        }
    }

    /// Clear the list of active formatting elements up to the last marker.
    fn clear_active_formatting_elements_to_last_marker(&mut self) {
        while let Some(entry) = self.active.pop() {
            if entry == Entry::Marker {
                return;
            }
        }
    }

    /// The index in the list of active formatting elements of `node`, if it is there. The list
    /// holds a node at most once; the scan starts at the end of the list, where the standard's
    /// rules look ("the last element in the list").
    fn active_position(&self, node: NodeId) -> Option<usize> {
        self.active
            .iter()
            .rposition(|e| matches!(e, Entry::Element { node: n, .. } if *n == node))
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
        if self.encoding.map(|e| e.confidence) != Some(Confidence::Tentative) {
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
