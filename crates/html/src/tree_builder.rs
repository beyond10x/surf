//! Tree construction (13.2.6): the insertion modes. Written by `story:html-tree-body` and the
//! tree stories after it.
//!
//! Written: the parse state of 13.2.4 (the insertion mode and the original insertion mode, the
//! stack of open elements with its scopes, the list of active formatting elements with its
//! markers, the head and form element pointers, the frameset-ok flag, the scripting flag),
//! creating and inserting nodes (13.2.6.1) with foster parenting, the generic raw text and
//! RCDATA element parsing algorithms (13.2.6.2), generating implied end tags (13.2.6.3), and the
//! insertion modes initial, before html, before head, in head, in head noscript, after head, in
//! body, text, after body, in frameset, after frameset, after after body and after after
//! frameset (13.2.6.4), in the submodule `modes`, each rule in the standard's order; the Noah's
//! Ark clause of the list of active formatting elements, and the adoption agency algorithm of
//! in body in the submodule `formatting` (story `html-tree-formatting`); the table insertion
//! modes (in table, in table text, in caption, in column group, in table body, in row, in cell)
//! in the submodule `tables`, and resetting the insertion mode appropriately (story
//! `html-tree-tables`); the `template` rules of in head, the in template mode and the stack of
//! template insertion modes in the submodule `template` (story `html-tree-template`). The
//! `select` rules are those of in body in the snapshot the design note names, which has no
//! select insertion modes. The tree construction dispatcher, the rules for foreign content and
//! the MathML and SVG adjustments are in the submodule `foreign`, and the fragment parsing
//! algorithm (13.4) is [`parse_fragment`] (story `html-tree-foreign`): its context element and
//! `html` root are nodes of the arena outside the tree, and the insertion target redirection
//! map's one entry sends the `html` root's insertions to the fragment. The submodule
//! `selectedcontent` copies each `select`'s selected `option` into its `selectedcontent`
//! elements at the end of a document's parse (story `html-tree-complete`).
//!
//! Parse errors are not collected: neither the tokenizer's nor the tree builder's reach a
//! caller. No script runs here: at a `script` end tag the text mode pops the element and returns
//! to the original insertion mode. A document parsed with scripting enabled
//! ([`crate::ParseOptions::scripting_disabled`] false) and a [`ScriptHook`]
//! ([`crate::parse_document_with_hook`]) calls the hook once for each script element the parser
//! inserts, HTML or SVG, when it leaves the stack of open elements or the parse ends, whichever
//! comes first, after the token that ended it (story `html-script-hook`,
//! `docs/design/scripts.md` Q1 and Q2): at an HTML `script` end tag in the text mode, after
//! steps 2 to 4 of 13.2.6.4.8, `run` for an element that the steps of "prepare the script
//! element" Q2 takes would run, else `skip` with the reason of Q1 ([`NotRun`]); for an HTML
//! `script` open when the parser stopped, at the end of the input or at a limit, `skip` with
//! [`NotRun::Eof`]; for an SVG `script`, however it is closed or open at the end, `skip` with
//! [`NotRun::Svg`]. A MathML `script` is no script element and gets no call. The stack of open
//! elements watches each script element it is given and notes the ones that leave it.
//! Parsing then goes on with the stack of open elements and the list of active formatting
//! elements as they were, over the tree the hook left. Without a hook, or with scripting
//! disabled, no call is made and the parse is the parse without one.
//!
//! The limits (story `html-parser-limits`, the crate documentation's "Limits"): every element,
//! node, text and attribute merge is admitted against the depth, node, stored-byte and attribute
//! limits before it is made, and every entry a scan visits is counted as a step, checked after
//! each token. A limit reached inside a token stops parsing: the rest of that token's rule
//! makes no change to the tree (each creation is refused once parsing has stopped), the
//! dispatcher takes no further mode, and the parse then pops every open element.

mod foreign;
mod formatting;
mod modes;
mod selectedcontent;
mod tables;
mod template;

use std::cell::Cell;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::ParseOptions;
use crate::encoding::{self, Confidence};
use crate::limits::{ATTRIBUTE_BYTES, Limit, Limits};
use crate::tokenizer::{self, Token, TokenAttribute, Tokenizer};
use crate::tree::{Attribute, Categories, Namespace, NodeData, NodeId, Tree};

/// Each part of tree construction that the tree builder does not yet parse as the standard
/// does, named in `[a-z-]+`; the tree story that writes it removes its entry. Empty since
/// story `html-tree-foreign` wrote foreign content and the fragment case.
pub const NOT_YET_PARSED: &[&str] = &[];

/// What the tree builder calls at a `script` end tag of a parser-inserted HTML script element
/// while parser scripting is enabled ([`crate::ParseOptions::scripting_disabled`] false), after
/// steps 2 to 4 of 13.2.6.4.8 and the steps of "prepare the script element" the design note's
/// Q2 takes (`docs/design/scripts.md`). `skip` is called as well for an HTML `script` element
/// open when the parser stopped ([`NotRun::Eof`]) and for an SVG `script` element, however it
/// is closed ([`NotRun::Svg`]). The hook is called once for each script element the parser
/// inserts, and never for an element a script inserted.
pub trait ScriptHook {
    /// Run the element's script to its end. The call may write the tree through its public
    /// write surface; it returns when the script ended, however it ended.
    fn run(&mut self, tree: &mut Tree, script: NodeId);
    /// The element is not run, for the reason of Q1's line `reasons:`.
    fn skip(&mut self, tree: &Tree, script: NodeId, reason: NotRun);
}

/// Why a script element is not run: one word of Q1's line `reasons:` each, but `inserted`,
/// which no end tag reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotRun {
    /// No `src` and no source text (step 8 of "prepare the script element").
    Empty,
    /// A `src` attribute: external scripts are not fetched (step 35).
    Src,
    /// A type that is no JavaScript MIME type essence and none of the three below (step 15).
    Type,
    /// `type=module` (step 12).
    Module,
    /// `type=importmap` (step 13).
    Importmap,
    /// `type=speculationrules` (step 14).
    Speculationrules,
    /// The element is not connected, as in a template's contents (step 9).
    Disconnected,
    /// `nomodule` on a classic script (step 21).
    Nomodule,
    /// `event` and `for` that are not `onload` for `window` (step 24).
    Event,
    /// An SVG `script`, however it is closed, which the first version does not run.
    Svg,
    /// The element was open when the parser stopped: the end of the input or a limit.
    Eof,
}

impl NotRun {
    /// The reason's word of Q1's line `reasons:`.
    pub fn word(self) -> &'static str {
        match self {
            NotRun::Empty => "empty",
            NotRun::Src => "src",
            NotRun::Type => "type",
            NotRun::Module => "module",
            NotRun::Importmap => "importmap",
            NotRun::Speculationrules => "speculationrules",
            NotRun::Disconnected => "disconnected",
            NotRun::Nomodule => "nomodule",
            NotRun::Event => "event",
            NotRun::Svg => "svg",
            NotRun::Eof => "eof",
        }
    }
}

/// The JavaScript MIME type essences (MIME Sniffing 4.6), in lower case.
const JAVASCRIPT_ESSENCES: &[&str] = &[
    "application/ecmascript",
    "application/javascript",
    "application/x-ecmascript",
    "application/x-javascript",
    "text/ecmascript",
    "text/javascript",
    "text/javascript1.0",
    "text/javascript1.1",
    "text/javascript1.2",
    "text/javascript1.3",
    "text/javascript1.4",
    "text/javascript1.5",
    "text/jscript",
    "text/livescript",
    "text/x-ecmascript",
    "text/x-javascript",
];

/// ASCII whitespace (Infra).
const ASCII_WHITESPACE: [char; 5] = ['\t', '\n', '\x0C', '\r', ' '];

/// The steps of "prepare the script element" (4.12.1.1) that Q2 takes, for the HTML script
/// element `script` as the parser made it: `None` when it runs, else the reason it does not.
/// Steps 1 to 7 hold for a parser-inserted element at its end tag in a parse with scripting
/// enabled (one document, not started before); step 20 is the caller's.
fn prepare(tree: &Tree, script: NodeId) -> Option<NotRun> {
    let NodeData::Element { attributes, .. } = tree.data(script) else {
        return Some(NotRun::Type);
    };
    let attribute = |name: &str| {
        attributes
            .iter()
            .find(|a| a.namespace.is_none() && a.local_name == name)
            .map(|a| a.value.as_str())
    };
    // 7, 8. The child text content.
    let empty = tree.children(script).iter().all(|&c| match tree.data(c) {
        NodeData::Text(text) => text.is_empty(),
        _ => true,
    });
    if attribute("src").is_none() && empty {
        return Some(NotRun::Empty);
    }
    // 9. Connected: the root is the document.
    let mut root = script;
    while let Some(parent) = tree.parent(root) {
        root = parent;
    }
    if *tree.data(root) != NodeData::Document {
        return Some(NotRun::Disconnected);
    }
    // 10. The script block's type string.
    let type_string = match (attribute("type"), attribute("language")) {
        (Some(""), _) | (None, Some("") | None) => "text/javascript".to_string(),
        (Some(value), _) => value.trim_matches(ASCII_WHITESPACE).to_string(),
        (None, Some(language)) => format!("text/{language}"),
    };
    // 11 to 15.
    let is = |name: &str| type_string.eq_ignore_ascii_case(name);
    if !JAVASCRIPT_ESSENCES.iter().any(|essence| is(essence)) {
        return Some(if is("module") {
            NotRun::Module
        } else if is("importmap") {
            NotRun::Importmap
        } else if is("speculationrules") {
            NotRun::Speculationrules
        } else {
            NotRun::Type
        });
    }
    // 21. A classic script with `nomodule`.
    if attribute("nomodule").is_some() {
        return Some(NotRun::Nomodule);
    }
    // 24. `event` and `for`.
    if let (Some(event), Some(for_)) = (attribute("event"), attribute("for")) {
        let event = event.trim_matches(ASCII_WHITESPACE);
        let for_ = for_.trim_matches(ASCII_WHITESPACE);
        if !for_.eq_ignore_ascii_case("window")
            || !(event.eq_ignore_ascii_case("onload") || event.eq_ignore_ascii_case("onload()"))
        {
            return Some(NotRun::Event);
        }
    }
    // 35. An external script.
    if attribute("src").is_some() {
        return Some(NotRun::Src);
    }
    None
}

/// The mode of a document (DOM, "mode"), which the initial insertion mode decides from the
/// DOCTYPE and writes into the tree ([`Tree::quirks_mode`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuirksMode {
    /// "no-quirks": a document's mode until the parser decides it, and a fragment's.
    NoQuirks,
    /// "limited-quirks".
    LimitedQuirks,
    /// "quirks".
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
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InTemplate,
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

/// The stored bytes of an element created for a token named `name` with `attributes`
/// ([`Tree::stored_bytes`]): its strings and [`ATTRIBUTE_BYTES`] for each attribute.
fn token_bytes(name: &str, attributes: &[TokenAttribute]) -> usize {
    name.len()
        + attributes
            .iter()
            .map(|a| ATTRIBUTE_BYTES + a.name.len() + a.value.len())
            .sum::<usize>()
}

/// The stack of open elements (13.2.4.2), the topmost (the `html` element) first, read as a
/// slice. It keeps, for each node, whether the node is on it and the depth of its entry, so
/// that "is in the stack of open elements" is answered in O(1) and visits no entry
/// ([`OpenElements::holds`]), and the depth of an entry is read without a walk
/// ([`OpenElements::depth`]). It also keeps the positions of the entries that bound "has an
/// element in scope", so that "has a `select` element in scope" visits one entry, also at
/// every `input` start tag of a deep tree.
#[derive(Debug, Default)]
struct OpenElements {
    stack: Vec<NodeId>,
    /// For each node of the arena by its index: 0 when it is not on the stack; [`UNSETTLED`]
    /// when it is and its depth is being read again ([`TreeBuilder::settle_entries`]); else
    /// its depth in the tree plus 1. The depth of an entry is the elements on its path from
    /// the top of its tree, itself included, a template counted for the nodes of its contents
    /// (the count of [`Limits::max_depth`]). A node is on the stack at most once.
    on_stack: Vec<u32>,
    /// For each node of the arena by its index, whether it bounds "has an element in scope"
    /// ([`OpenElements::mark_boundary`]).
    bounds: Vec<bool>,
    /// The positions in `stack` of the entries that bound "has an element in scope", ascending.
    boundaries: Vec<usize>,
    /// Some script element is watched ([`OpenElements::watch`]): a parse with a hook and
    /// scripting enabled. Without one, an entry that leaves costs this test only.
    watching: bool,
    /// For each node of the arena by its index, whether it is a watched script element that
    /// has not left the stack yet.
    watched: Vec<bool>,
    /// The watched script elements that left the stack, in the order they left; the hook is
    /// called for each after the token.
    ended: Vec<NodeId>,
}

impl std::ops::Deref for OpenElements {
    type Target = [NodeId];

    fn deref(&self) -> &[NodeId] {
        &self.stack
    }
}

impl OpenElements {
    /// Records that `node`, an element just created, bounds "has an element in scope".
    fn mark_boundary(&mut self, node: NodeId) {
        let index = node.index();
        if self.bounds.len() <= index {
            self.bounds.resize(index + 1, false);
        }
        self.bounds[index] = true;
    }

    fn is_boundary(&self, node: NodeId) -> bool {
        self.bounds.get(node.index()).copied().unwrap_or(false)
    }

    fn entered(&mut self, node: NodeId, depth: u32) {
        let index = node.index();
        if self.on_stack.len() <= index {
            self.on_stack.resize(index + 1, 0);
        }
        debug_assert_eq!(
            self.on_stack[index], 0,
            "a node is on the stack at most once"
        );
        self.on_stack[index] = depth;
    }

    fn left(&mut self, node: NodeId) {
        let index = node.index();
        self.on_stack[index] = 0;
        if self.watching && self.watched.get(index).copied().unwrap_or(false) {
            self.watched[index] = false;
            self.ended.push(node);
        }
    }

    /// Watches `node`, a script element just pushed: when it leaves the stack it goes to
    /// `ended`, once.
    fn watch(&mut self, node: NodeId) {
        let index = node.index();
        if self.watched.len() <= index {
            self.watched.resize(index + 1, false);
        }
        self.watched[index] = true;
        self.watching = true;
    }

    /// Whether `node` is in the stack of open elements: no entry is visited.
    fn holds(&self, node: NodeId) -> bool {
        self.on_stack.get(node.index()).is_some_and(|&n| n > 0)
    }

    /// The depth of `node`'s entry, when `node` is on the stack and its depth is settled: no
    /// entry is visited.
    fn depth(&self, node: NodeId) -> Option<usize> {
        match self.on_stack.get(node.index()) {
            Some(&n) if n > 0 && n != UNSETTLED => Some(n as usize - 1),
            _ => None,
        }
    }

    /// Whether `node` is on the stack with its depth being read again.
    fn unsettled(&self, node: NodeId) -> bool {
        self.on_stack.get(node.index()) == Some(&UNSETTLED)
    }

    /// Sets the depth of `node`'s entry; `None` marks it [`UNSETTLED`].
    fn set_depth(&mut self, node: NodeId, depth: Option<usize>) {
        self.on_stack[node.index()] = entry_value(depth);
    }

    /// The lowest entry that bounds "has an element in scope", if any: no entry is scanned.
    fn last_boundary(&self) -> Option<NodeId> {
        self.boundaries.last().map(|&at| self.stack[at])
    }

    /// Pushes `node` with the depth of its entry; `None` leaves it [`UNSETTLED`] for the
    /// caller to settle. A boundary is added at the end of the boundaries: the only place
    /// one is added.
    fn push(&mut self, node: NodeId, depth: Option<usize>) {
        self.entered(node, entry_value(depth));
        if self.is_boundary(node) {
            self.boundaries.push(self.stack.len());
        }
        self.stack.push(node);
    }

    fn pop(&mut self) -> Option<NodeId> {
        let node = self.stack.pop()?;
        self.left(node);
        if self.boundaries.last() == Some(&self.stack.len()) {
            self.boundaries.pop();
        }
        Some(node)
    }

    /// Pops the entries above the first `len`. Not a step: the caller counted the entries it
    /// pops (the scan that found `len`, or the `frameset` rule), except "stop parsing"
    /// ([`TreeBuilder::stop_parsing`]).
    fn truncate(&mut self, len: usize) {
        while self.stack.len() > len {
            self.pop();
        }
    }

    fn clear(&mut self) {
        self.truncate(0);
    }

    /// Removes the entry at `at`. A step for each entry above it, which moves down one place,
    /// and for each boundary, visited once to move it. The entry removed is never a boundary:
    /// the rules that remove an entry from inside the stack remove the `head`, a `form`, an
    /// `a`, the formatting element of the adoption agency algorithm and the elements between
    /// it and the furthest block, none of them of the element types of the default scope.
    fn remove(&mut self, at: usize, steps: &Cell<usize>) -> NodeId {
        let node = self.stack.remove(at);
        debug_assert!(
            !self.is_boundary(node),
            "no rule removes a boundary from inside"
        );
        count_steps(steps, self.stack.len() - at + self.boundaries.len());
        self.left(node);
        for b in &mut self.boundaries {
            if *b > at {
                *b -= 1;
            }
        }
        node
    }

    /// Inserts `node` at `at`, its depth [`UNSETTLED`] for the caller to settle. A step for
    /// each entry from `at` up, which moves up one place. `node` is the adoption agency's new
    /// formatting element, never a boundary, and no boundary is at or above `at`: the algorithm
    /// runs on only when its formatting element is in scope (step 4.5), so no boundary is above
    /// that element, and `at` is above it. So no boundary moves and none is visited.
    fn insert(&mut self, at: usize, node: NodeId, steps: &Cell<usize>) {
        debug_assert!(
            !self.is_boundary(node),
            "a formatting element is no boundary"
        );
        debug_assert!(
            self.boundaries.last().is_none_or(|&b| b < at),
            "no boundary above a formatting element in scope"
        );
        count_steps(steps, self.stack.len() - at);
        self.entered(node, UNSETTLED);
        self.stack.insert(at, node);
    }

    /// Puts `node` in the place of the entry at `at`, its depth [`UNSETTLED`] for the caller to
    /// settle. Neither is a boundary (the adoption agency's inner loop: a node below the
    /// furthest block, so not special, and a new element for a formatting element), so the
    /// boundaries do not change and no entry is visited.
    fn replace(&mut self, at: usize, node: NodeId) {
        debug_assert!(!self.is_boundary(node) && !self.is_boundary(self.stack[at]));
        self.entered(node, UNSETTLED);
        let old = std::mem::replace(&mut self.stack[at], node);
        self.left(old);
    }
}

/// The value of [`OpenElements`]'s `on_stack` for an entry whose depth is being read again.
const UNSETTLED: u32 = u32::MAX;

/// Adds `n` visits to `steps` ([`TreeBuilder::add_steps`] for the stack's own methods).
fn count_steps(steps: &Cell<usize>, n: usize) {
    steps.set(steps.get().saturating_add(n));
}

/// The value of `on_stack` for an entry of `depth`, [`UNSETTLED`] for `None`. A depth past
/// `u32::MAX - 2` is kept as that.
fn entry_value(depth: Option<usize>) -> u32 {
    depth.map_or(UNSETTLED, |d| {
        u32::try_from(d + 1).map_or(UNSETTLED - 1, |v| v.min(UNSETTLED - 1))
    })
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
    parse_with_hook(input, options, encoding, None)
}

/// [`parse`] with the hook that the end of each parser-inserted script element is reported to
/// while scripting is enabled.
pub(crate) fn parse_with_hook(
    input: &str,
    options: &ParseOptions,
    encoding: Option<InputEncoding>,
    hook: Option<&mut dyn ScriptHook>,
) -> Parsed {
    // The input stream keeps its own copy of the code points: this one goes once it is made.
    let mut builder = {
        let code_points: Vec<u32> = input.chars().map(u32::from).collect();
        TreeBuilder::new(&code_points, options, encoding)
    };
    builder.report_scripts = hook.is_some() && !options.scripting_disabled;
    run(builder, hook)
}

/// Runs the fragment parsing algorithm (13.4) over decoded text for a context element with no
/// attribute and no ancestor, in a document in no-quirks mode, with no sanitizer and no
/// declarative shadow roots: the context element and the `html` root element are nodes of the
/// arena outside the tree, and the root of the tree is the fragment the `html` root's
/// insertions are redirected to. A limit that refuses either of the two elements stops the
/// parse before its first token.
pub(crate) fn parse_fragment(
    input: &str,
    context: &crate::Context,
    options: &ParseOptions,
) -> Tree {
    let mut builder = {
        let code_points: Vec<u32> = input.chars().map(u32::from).collect();
        TreeBuilder::new(&code_points, options, None)
    };
    builder.tree = Tree::fragment(options.limits);
    let name = context.local_name.as_str();
    let element = match context.namespace {
        Namespace::Html => builder.create_element(name, &[]),
        namespace => builder.create_foreign_element(namespace, name, &[]),
    };
    let html = builder.create_element("html", &[]);
    let (Some(element), Some(html)) = (element, html) else {
        return run(builder, None).tree;
    };
    builder.context = Some(element);
    builder.fragment_html = Some(html);
    if context.namespace == Namespace::Html {
        let state = match name {
            "title" | "textarea" => Some(tokenizer::State::Rcdata),
            "style" | "xmp" | "iframe" | "noembed" | "noframes" => Some(tokenizer::State::Rawtext),
            "script" => Some(tokenizer::State::ScriptData),
            "noscript" if !builder.scripting_disabled => Some(tokenizer::State::Rawtext),
            "plaintext" => Some(tokenizer::State::Plaintext),
            _ => None,
        };
        if let Some(state) = state {
            builder.tokenizer.switch_to(state);
        }
    }
    // Out of the tree: 1 deep in its own. What goes into it goes into the fragment, 0 deep.
    builder.open.push(html, Some(1));
    if builder.is_html(element, &["template"]) {
        builder.template_modes.push(Mode::InTemplate);
    }
    builder.reset_insertion_mode();
    // The nearest `form` going up from the context element, which has no ancestor here.
    if builder.is_html(element, &["form"]) {
        builder.form = Some(element);
    }
    // The fragment parsing algorithm marks every script element it parses as already started
    // (13.4): no hook.
    run(builder, None).tree
}

/// The tokenizer and tree construction from the first token to the end, with the step limit
/// read after each token (and inside the token's loops and before its writes,
/// [`TreeBuilder::spent`]). A script element the token ended is reported to `hook` after it.
fn run(mut builder: TreeBuilder, mut hook: Option<&mut dyn ScriptHook>) -> Parsed {
    while !builder.stopped {
        builder.tokenizer.set_cdata_allowed(builder.cdata_allowed());
        let Some(token) = builder.tokenizer.next_token() else {
            // The tokenizer stopped at a start tag that would keep one attribute too many.
            builder.stop_at(Limit::Attributes);
            break;
        };
        builder.dispatch(token);
        if let Some(hook) = hook.as_deref_mut()
            && !builder.open.ended.is_empty()
        {
            builder.report_ended(hook);
        }
        // Also the end-of-file token, whose rules scan too (in template pops down to each open
        // `template`), unless another limit stopped the parse.
        if builder.tree.stopped_by().is_none() && builder.steps.get() > builder.limits.max_steps {
            builder.stop_at(Limit::Steps);
        }
    }
    // "Stop parsing" pops every open element, also after a limit (not steps: `stop_parsing`).
    builder.open.clear();
    if let Some(hook) = hook
        && !builder.open.ended.is_empty()
    {
        builder.report_ended(hook);
    }
    if builder.saw_selectedcontent
        && builder.context.is_none()
        && builder.tree.stopped_by().is_none()
        && builder.reparse_with.is_none()
    {
        // The queued selectedcontent updates (the module `selectedcontent`): they create nodes,
        // which a stopped parse refuses, so the flag is lifted for them.
        builder.stopped = false;
        builder.update_selectedcontents();
        builder.stopped = true;
    }
    builder.tree.set_steps(builder.steps.get());
    Parsed {
        tree: builder.tree,
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
    open: OpenElements,
    /// No entry of the stack of open elements is under an entry above it in the tree, as the
    /// last [`Self::settle_entries`] of the whole stack found; the parser's own insertions and
    /// moves keep it so. Then an adoption agency iteration changes the depth of no entry below
    /// its formatting element, and only the entries from there up are read again.
    ordered: bool,
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
    /// Foster parenting is enabled (the "anything else" rule of in table).
    foster_parenting: bool,
    /// The pending table character tokens of in table text while they are all ASCII whitespace
    /// (the module `tables` says why no other character is kept).
    pending_table_text: String,
    /// In table text met a character that is not ASCII whitespace: the pending characters were
    /// processed, and each later one is processed as it comes.
    pending_table_foster: bool,
    /// The fragment parsing algorithm's context element, a node of the arena outside the tree;
    /// `None` for a document.
    context: Option<NodeId>,
    /// The stack of template insertion modes, the current one last.
    template_modes: Vec<Mode>,
    /// The fragment parsing algorithm's `html` root element, a node of the arena outside the
    /// tree whose insertions go to the fragment; `None` for a document.
    fragment_html: Option<NodeId>,
    /// An HTML `selectedcontent` element was created: the end of the parse updates the
    /// `select` elements' copies of their selected option (the module `selectedcontent`).
    saw_selectedcontent: bool,
    /// A hook is set and scripting is enabled: each script element the parser pushes is
    /// watched on the stack of open elements ([`OpenElements::watch`]).
    report_scripts: bool,
    /// The element the last `script` end tag of the text mode popped: if it is a watched HTML
    /// script element, its end tag ended it, and "prepare the script element" decides its call.
    script_end_tag: Option<NodeId>,
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
pub(crate) fn categories_of(namespace: Namespace, name: &str) -> Categories {
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
            tree: Tree::document(options.limits),
            mode: Mode::Initial,
            original_mode: Mode::Initial,
            open: OpenElements::default(),
            ordered: true,
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
            foster_parenting: false,
            pending_table_text: String::new(),
            pending_table_foster: false,
            context: None,
            template_modes: Vec::new(),
            fragment_html: None,
            saw_selectedcontent: false,
            report_scripts: false,
            script_end_tag: None,
        }
    }

    /// Watches `element`, just pushed, when it is a script element and a hook is set with
    /// scripting enabled: an HTML `script` or an SVG `script`, not a MathML one.
    fn watch_script(&mut self, element: NodeId) {
        if self.report_scripts
            && matches!(
                self.tree.data(element),
                NodeData::Element { namespace: Namespace::Html | Namespace::Svg, local_name, .. }
                    if local_name == "script"
            )
        {
            self.open.watch(element);
        }
    }

    /// Calls `hook` once for each watched script element that left the stack of open elements,
    /// in the order they left: an HTML `script` its end tag popped gets `run` or `skip` as
    /// "prepare the script element" decides; another HTML `script`, popped at the end of the
    /// input or by the end of a parse a limit stopped, `skip` with [`NotRun::Eof`]; an SVG
    /// `script`, however it left, `skip` with [`NotRun::Svg`]. After `run` the names the `html`
    /// and `body` merges keep are read again from the elements, which the hook may have
    /// changed, and so are the depths of the entries of the stack of open elements when it
    /// moved or removed an element.
    fn report_ended(&mut self, hook: &mut dyn ScriptHook) {
        let end_tag = self.script_end_tag.take();
        for script in std::mem::take(&mut self.open.ended) {
            let reason = if self.is_html(script, &["script"]) {
                if end_tag == Some(script) {
                    prepare(&self.tree, script)
                } else {
                    Some(NotRun::Eof)
                }
            } else {
                Some(NotRun::Svg)
            };
            match reason {
                Some(reason) => hook.skip(&self.tree, script, reason),
                None => {
                    hook.run(&mut self.tree, script);
                    self.merged_names.clear();
                    if self.tree.take_moved() {
                        self.settle_entries(0);
                    }
                }
            }
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

    /// A read of the step limit: whether the steps, with `pending` more that a scan counts at
    /// its end, have passed it. Every loop over the stack of open elements, its boundaries or
    /// the list of active formatting elements reads it before each entry it visits, and ends
    /// there when they have; so does every write of the parser to the tree, which is then not
    /// made ([`Self::past_step_limit`]). Below the limit it changes nothing.
    fn spent(&self, pending: usize) -> bool {
        self.steps.get().saturating_add(pending) > self.limits.max_steps
    }

    /// Whether the steps have passed the step limit ([`Self::spent`]); then parsing stops at it.
    fn past_step_limit(&mut self) -> bool {
        if self.spent(0) {
            self.stop_at(Limit::Steps);
            return true;
        }
        false
    }

    /// The tree construction dispatcher (13.2.6): each token goes to the rules of the current
    /// insertion mode, or to the rules for foreign content (the module `foreign`).
    fn dispatch(&mut self, token: Token) {
        if std::mem::take(&mut self.ignore_line_feed) && token == Token::Character(0x0A) {
            return;
        }
        let mut token = token;
        loop {
            let flow = if self.in_foreign_content(&token) {
                self.foreign_content(&token)
            } else {
                self.process_by_mode(&token)
            };
            match flow {
                Flow::Done => return,
                Flow::Reprocess => {}
                Flow::Replace(replacement) => token = replacement,
            }
            // Past the step limit no rule runs again for the token: a loop the limit ended
            // may have left the state a rule moves on from (the insertion mode) as it was.
            if self.stopped || self.past_step_limit() {
                return;
            }
        }
    }

    /// Processes `token` by the rules of the current insertion mode.
    fn process_by_mode(&mut self, token: &Token) -> Flow {
        match self.mode {
            Mode::Initial => self.initial(token),
            Mode::BeforeHtml => self.before_html(token),
            Mode::BeforeHead => self.before_head(token),
            Mode::InHead => self.in_head(token),
            Mode::InHeadNoscript => self.in_head_noscript(token),
            Mode::AfterHead => self.after_head(token),
            Mode::InBody => self.in_body(token),
            Mode::Text => self.text(token),
            Mode::InTable => self.in_table(token),
            Mode::InTableText => self.in_table_text(token),
            Mode::InCaption => self.in_caption(token),
            Mode::InColumnGroup => self.in_column_group(token),
            Mode::InTableBody => self.in_table_body(token),
            Mode::InRow => self.in_row(token),
            Mode::InCell => self.in_cell(token),
            Mode::InTemplate => self.in_template(token),
            Mode::AfterBody => self.after_body(token),
            Mode::InFrameset => self.in_frameset(token),
            Mode::AfterFrameset => self.after_frameset(token),
            Mode::AfterAfterBody => self.after_after_body(token),
            Mode::AfterAfterFrameset => self.after_after_frameset(token),
        }
    }

    /// Stop parsing (13.2.7, "the end"), as far as it changes the tree: pop every node off the
    /// stack of open elements. The pops are not steps: the count of a parse up to its last
    /// token stays what the step limit checked. They run once per parse pass, after its last
    /// token, one for each entry left, and every entry is an element the parser made (the node
    /// limit) or the `head` it put back for one token. A `meta` that changes the encoding
    /// ([`Self::change_the_encoding`]) ends the first pass here and the caller parses the bytes
    /// again; that second pass takes no hook, so its stack stays inside the depth limit.
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
    /// after parsing stopped, or past the step limit (a read of it), nothing is admitted.
    fn admit(&mut self, nodes: usize, bytes: usize) -> bool {
        if self.stopped || self.past_step_limit() {
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

    /// The depth of `node` in the tree as it is: the elements on its path from the top of its
    /// tree (the document, or the top of a subtree out of it), itself included, a template
    /// counted for the nodes of its contents. Read from the first entry of the stack of open
    /// elements met on the way up whose depth is settled; each parent or host passed to is a
    /// step. The insertion places of the parser are entries, the parent of an entry, or a
    /// template's contents, so most reads pass to nothing or to one node.
    fn depth_of(&self, node: NodeId) -> usize {
        self.walk_up(node).0
    }

    /// [`Self::depth_of`], and whether the walk passed an [`UNSETTLED`] entry.
    fn walk_up(&self, node: NodeId) -> (usize, bool) {
        let mut depth = 0;
        let mut passed_unsettled = false;
        let mut at = node;
        // A tree has no cycle; the bound only keeps the walk finite.
        for _ in 0..=self.tree.node_count() {
            if let Some(known) = self.open.depth(at) {
                return (depth + known, passed_unsettled);
            }
            passed_unsettled |= at != node && self.open.unsettled(at);
            depth += usize::from(self.tree.is_element(at));
            match self.tree.parent_or_host(at) {
                Some(up) => {
                    self.visit();
                    at = up;
                }
                None => break,
            }
        }
        (depth, passed_unsettled)
    }

    /// The depth of the parent of an insertion place, `(parent, reference)`: a reference child
    /// on the stack (the `table` of foster parenting) gives its parent's depth without a walk.
    fn place_depth(&self, (parent, reference): (NodeId, Option<NodeId>)) -> usize {
        match reference.and_then(|r| self.open.depth(r)) {
            Some(depth) => depth - 1,
            None => self.depth_of(parent),
        }
    }

    /// Reads the depths of the entries of the stack of open elements from `from` up again
    /// from the tree, bottom up, each entry from its parent or host up to the first entry
    /// already read ([`Self::walk_up`]). After the parser's own changes an entry's parent is
    /// most often the entry below it: one step. Each entry read again is a step too, also one
    /// with no parent, which no hop reaches (the step counts both passes over it). From 0 it
    /// also finds whether the stack is [`Self::ordered`]: a walk that passes an entry above its
    /// own is under it.
    fn settle_entries(&mut self, from: usize) {
        for at in from..self.open.len() {
            let node = self.open[at];
            self.open.set_depth(node, None);
        }
        let mut ordered = true;
        for at in from..self.open.len() {
            // Past the step limit the entries above stay unsettled, and parsing stops: no
            // depth is read after it.
            if self.past_step_limit() {
                return;
            }
            self.visit();
            let node = self.open[at];
            let (depth, passed_unsettled) = self.walk_up(node);
            ordered &= !passed_unsettled;
            self.open.set_depth(node, Some(depth));
        }
        if from == 0 {
            self.ordered = ordered;
        } else {
            debug_assert!(
                ordered,
                "an ordered stack has no entry under an entry above it"
            );
        }
    }

    /// Whether an element inserted at `place` stays inside the depth limit: the depth of
    /// `place`'s parent ([`Self::place_depth`]) is below the limit. When it is not, parsing
    /// stops: that parent is an element at the limit. `Some` with that depth when it does;
    /// `None` after parsing stopped, also past the step limit (a read of it). The walk to a
    /// template's host that `place_depth` may take is counted: past the step limit after it,
    /// parsing stops at the step limit, never at the depth limit.
    fn room_at(&mut self, place: (NodeId, Option<NodeId>)) -> Option<usize> {
        if self.stopped || self.past_step_limit() {
            return None;
        }
        let depth = self.place_depth(place);
        if depth >= self.limits.max_depth {
            if !self.past_step_limit() {
                self.stop_at(Limit::Depth);
            }
            return None;
        }
        Some(depth)
    }

    /// The appropriate place for an element and its parent's depth, when it has room under it
    /// ([`Self::room_at`]); `None`, and parsing stopped, when it has not.
    fn place_with_room(&mut self) -> Option<((NodeId, Option<NodeId>), usize)> {
        let place = self.appropriate_place(None);
        let depth = self.room_at(place)?;
        Some((place, depth))
    }

    /// Pushes `element`, just inserted at a place whose parent is `depth` deep, onto the stack
    /// of open elements: its depth is one more. A place refuses an element only when its
    /// parent is the document and has an element (13.2.6.1); then the element is the top of
    /// its own tree, 1 deep, which is the document's 0 and one more.
    fn push_inserted(&mut self, element: NodeId, depth: usize) {
        debug_assert!(self.tree.parent(element).is_some() || depth == 0);
        self.open.push(element, Some(depth + 1));
    }

    // 13.2.6.1 Creating and inserting nodes.

    fn document(&self) -> NodeId {
        self.tree.root()
    }

    /// Whether `node` is the document: the root of a tree parsed as a document. The root of a
    /// fragment is a document fragment, which may hold more than one element and text.
    fn is_document(&self, node: NodeId) -> bool {
        node == self.document() && self.context.is_none()
    }

    /// The adjusted insertion location for a place (13.2.6.1): the fragment's `html` root
    /// element, the only key of the insertion target redirection map, gives the fragment.
    fn redirect(&self, place: (NodeId, Option<NodeId>)) -> (NodeId, Option<NodeId>) {
        match self.fragment_html {
            Some(html) if place.0 == html => (self.document(), None),
            _ => place,
        }
    }

    /// The adjusted insertion location, with an override target: the appropriate place for
    /// inserting a node, and the target and reference child. With foster parenting enabled (the
    /// "anything else" rule of in table) and a `table`, `tbody`, `tfoot`, `thead` or `tr`
    /// target, the place is in front of the last `table` on the stack of open elements, or in
    /// the last `template` when that is lower. A `template` target gives its template contents.
    /// The fragment's `html` root redirects to the fragment ([`Self::redirect`]).
    fn appropriate_place(&self, override_target: Option<NodeId>) -> (NodeId, Option<NodeId>) {
        let mut target = override_target.unwrap_or_else(|| self.current_node());
        if self.foster_parenting
            && self.is_html(target, &["table", "tbody", "tfoot", "thead", "tr"])
        {
            // A scan the step limit ends finds no `table`; each caller reads the limit before
            // it writes at the place.
            let mut visited = 0;
            let mut last = None;
            for index in (0..self.open.len()).rev() {
                if self.spent(visited) {
                    break;
                }
                visited += 1;
                if self.is_html(self.open[index], &["template", "table"]) {
                    last = Some(index);
                    break;
                }
            }
            self.add_steps(visited);
            match last {
                // The fragment case: the `html` element.
                None => target = self.open[0],
                Some(last) => {
                    let node = self.open[last];
                    if self.is_html(node, &["template"]) {
                        target = node;
                    } else if let Some(parent) = self.tree.parent(node) {
                        return self.redirect((parent, Some(node)));
                    } else {
                        target = self.open[last - 1];
                    }
                }
            }
        }
        if let NodeData::Element {
            template_contents: Some(contents),
            ..
        } = self.tree.data(target)
        {
            return (*contents, None);
        }
        self.redirect((target, None))
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

    /// Create an element for a token in a foreign namespace, its attributes adjusted as that
    /// namespace's rules say (`foreign::element_attributes`); `name` is the adjusted name.
    fn create_foreign_element(
        &mut self,
        namespace: Namespace,
        name: &str,
        attributes: &[TokenAttribute],
    ) -> Option<NodeId> {
        let attributes = foreign::element_attributes(namespace, attributes);
        let bytes = name.len()
            + attributes
                .iter()
                .map(|a| {
                    ATTRIBUTE_BYTES
                        + a.prefix.as_ref().map_or(0, String::len)
                        + a.local_name.len()
                        + a.value.len()
                })
                .sum::<usize>();
        if !self.admit(1, bytes) {
            return None;
        }
        let mut categories = categories_of(namespace, name);
        if namespace == Namespace::MathMl
            && name == "annotation-xml"
            && foreign::encoding_is_html(&attributes)
        {
            categories.0 |= Categories::HTML_INTEGRATION_POINT;
        }
        let element = self
            .tree
            .create_element(namespace, name, attributes, categories);
        if categories.has(Categories::SCOPE) {
            self.open.mark_boundary(element);
        }
        Some(element)
    }

    /// Create an element for a token in the HTML namespace that the limits admitted already
    /// (the adoption agency algorithm admits a whole iteration at once).
    fn create_admitted_element(&mut self, name: &str, attributes: &[TokenAttribute]) -> NodeId {
        let categories = categories_of(Namespace::Html, name);
        if name == "selectedcontent" {
            self.saw_selectedcontent = true;
        }
        let element = self.tree.create_element(
            Namespace::Html,
            name,
            self::attributes(attributes),
            categories,
        );
        if categories.has(Categories::SCOPE) {
            self.open.mark_boundary(element);
        }
        element
    }

    /// Whether the stack of open elements has a `select` element in scope. A `select` bounds
    /// that scope itself, so it is in scope exactly when it is the lowest entry that bounds it:
    /// one entry is visited.
    fn has_select_in_scope(&self) -> bool {
        if self.spent(0) {
            return false;
        }
        self.visit();
        self.open
            .last_boundary()
            .is_some_and(|node| self.is_html(node, &["select"]))
    }

    /// Insert an element at the adjusted insertion location, given an insertion location.
    fn insert_element_at(&mut self, element: NodeId, location: (NodeId, Option<NodeId>)) {
        let (parent, reference) = location;
        if self.tree.parent(element).is_some()
            || (self.is_document(parent) && self.tree.has_element_child(parent, &self.steps))
        {
            return;
        }
        self.tree.insert(parent, element, reference, &self.steps);
    }

    /// Insert an HTML element for a token: created, inserted at the appropriate place, pushed
    /// onto the stack of open elements. `None` when a limit refuses it (depth first, then nodes
    /// and stored bytes), or parsing stopped.
    fn insert_html_element(&mut self, name: &str, attributes: &[TokenAttribute]) -> Option<NodeId> {
        self.insert_element(Namespace::Html, name, attributes)
    }

    /// Insert a foreign element for a token in `namespace` (an HTML element for
    /// [`Namespace::Html`]), with `name` already adjusted: the appropriate place is taken and
    /// checked for depth, the element created, inserted there and pushed onto the stack of open
    /// elements.
    fn insert_element(
        &mut self,
        namespace: Namespace,
        name: &str,
        attributes: &[TokenAttribute],
    ) -> Option<NodeId> {
        let (location, depth) = self.place_with_room()?;
        let element = match namespace {
            Namespace::Html => self.create_element(name, attributes)?,
            _ => self.create_foreign_element(namespace, name, attributes)?,
        };
        self.insert_element_at(element, location);
        self.push_inserted(element, depth);
        self.watch_script(element);
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
        // Read after the place: a scan the step limit ended finds no `table`.
        if self.past_step_limit() || self.is_document(parent) {
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
        if self.past_step_limit() {
            return;
        }
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
        if self.past_step_limit() {
            return;
        }
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
    /// `element` has no attribute of is added to it, in order. A token's attribute is in no
    /// namespace, so it matches only an attribute of the element in no namespace of its local
    /// name: an attribute a hook set in a namespace keeps its place and counts. A merge that
    /// would give the element more attributes than the attribute limit, its attributes counted
    /// whatever their names, or pass the stored-byte limit, is not made, and parsing stops. The
    /// names of the element's attributes in no namespace are kept in a set, so the merge costs
    /// O(1) for each attribute of the token. The step limit is read first: the rules that merge
    /// come after a scan for a `template` the limit may have ended, and past it parsing stops
    /// at the step limit, never at the attribute limit.
    fn merge_attributes(&mut self, element: NodeId, attributes: &[TokenAttribute]) {
        if self.stopped || self.past_step_limit() {
            return;
        }
        let held = match self.tree.data(element) {
            NodeData::Element { attributes, .. } => attributes.len(),
            _ => 0,
        };
        let at = match self.merged_names.iter().position(|(n, _)| *n == element) {
            Some(at) => at,
            None => {
                let names = match self.tree.data(element) {
                    NodeData::Element { attributes, .. } => attributes
                        .iter()
                        .filter(|a| a.namespace.is_none())
                        .map(|a| a.local_name.clone())
                        .collect(),
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
        if held + new.len() > self.limits.max_attributes {
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
            if self.spent(0) {
                return;
            }
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
            if self.spent(visited) {
                break;
            }
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
        while !self.spent(0)
            && let Some(node) = self.open.pop()
        {
            self.visit();
            if self.is_html(node, names) {
                return;
            }
        }
    }

    /// Reset the insertion mode appropriately (13.2.4.1): from the current node up, the first
    /// element that names a mode decides it; at the top of the stack the context element of a
    /// fragment stands in for the `html` element. Each entry visited is a step.
    fn reset_insertion_mode(&mut self) {
        let mut index = self.open.len();
        while index > 0 && !self.spent(0) {
            index -= 1;
            self.visit();
            let last = index == 0;
            let node = match (last, self.context) {
                (true, Some(context)) => context,
                _ => self.open[index],
            };
            let mode = match self.html_name(node) {
                Some("td" | "th") if !last => Mode::InCell,
                Some("tr") => Mode::InRow,
                Some("tbody" | "thead" | "tfoot") => Mode::InTableBody,
                Some("caption") => Mode::InCaption,
                Some("colgroup") => Mode::InColumnGroup,
                Some("table") => Mode::InTable,
                Some("template") => self.current_template_mode(),
                Some("head") if !last => Mode::InHead,
                Some("body") => Mode::InBody,
                Some("frameset") => Mode::InFrameset,
                Some("html") if self.head.is_none() => Mode::BeforeHead,
                Some("html") => Mode::AfterHead,
                _ if last => Mode::InBody,
                _ => continue,
            };
            self.mode = mode;
            return;
        }
        self.mode = Mode::InBody;
    }

    /// Whether the parser is parsing template contents: a `template` is on the stack of open
    /// elements, or the fragment's context element is one.
    fn parsing_template_contents(&self) -> bool {
        self.context_is("template") || self.template_on_stack()
    }

    /// Whether the fragment parsing algorithm's context element is an HTML element named
    /// `name`.
    fn context_is(&self, name: &str) -> bool {
        self.context
            .is_some_and(|c| self.html_name(c) == Some(name))
    }

    /// The index of `element` in the stack of open elements, scanning from the current node.
    fn stack_position(&self, element: NodeId) -> Option<usize> {
        let mut visited = 0;
        let mut at = None;
        for index in (0..self.open.len()).rev() {
            if self.spent(visited) {
                break;
            }
            visited += 1;
            if self.open[index] == element {
                at = Some(index);
                break;
            }
        }
        self.add_steps(visited);
        at
    }

    /// Removes `element` from the stack of open elements, scanning from the current node.
    fn remove_from_stack(&mut self, element: NodeId) {
        if let Some(at) = self.stack_position(element) {
            self.open.remove(at, &self.steps);
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
            if self.spent(0) {
                break;
            }
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
    /// is answered by [`OpenElements::holds`] without a scan, so the standard's first checks (the
    /// list is empty, or its last entry is a marker or open) cost O(1) and no step, also for
    /// every character of a text; the rewind counts one step for each entry it steps back over,
    /// the advance one for each entry it creates an element for. Each element is admitted by the
    /// limits before it is created; one they refuse ends the reconstruction, with the elements
    /// created before it in the tree. No start tag token is copied: the entry's own token is
    /// read and kept.
    fn reconstruct_active_formatting_elements(&mut self) {
        let is_open_or_marker = |b: &Self, entry: &Entry| match entry {
            Entry::Marker => true,
            Entry::Element { node, .. } => b.open.holds(*node),
        };
        match self.active.last() {
            None => return,
            Some(last) if is_open_or_marker(self, last) => return,
            Some(_) => {}
        }
        // Rewind: back to the entry after the last marker or open element, or the first entry.
        let mut index = self.active.len() - 1;
        while index > 0 && !is_open_or_marker(self, &self.active[index - 1]) {
            if self.spent(0) {
                return;
            }
            self.visit();
            index -= 1;
        }
        // Advance and create, to the end of the list.
        for i in index..self.active.len() {
            if self.spent(0) {
                return;
            }
            self.visit();
            if matches!(self.active[i], Entry::Marker) {
                continue;
            }
            let Entry::Element { node: old, token } =
                std::mem::replace(&mut self.active[i], Entry::Marker)
            else {
                unreachable!("the entry is an element");
            };
            let Some(node) = self.insert_html_element(&token.name, &token.attributes) else {
                self.active[i] = Entry::Element { node: old, token };
                return;
            };
            self.active[i] = Entry::Element { node, token };
        }
    }

    /// Clear the list of active formatting elements up to the last marker.
    fn clear_active_formatting_elements_to_last_marker(&mut self) {
        while !self.spent(0)
            && let Some(entry) = self.active.pop()
        {
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
        for (index, entry) in self.active.iter().enumerate().rev() {
            if self.spent(0) {
                return None;
            }
            self.visit();
            if matches!(entry, Entry::Element { node: n, .. } if *n == node) {
                return Some(index);
            }
        }
        None
    }

    /// The index of `node` in the list of active formatting elements after the last marker (or
    /// in the whole list when there is none), if it is there: the scan ends at the marker.
    fn active_position_after_last_marker(&self, node: NodeId) -> Option<usize> {
        for (index, entry) in self.active.iter().enumerate().rev() {
            if self.spent(0) {
                return None;
            }
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
        parse(input, &ParseOptions::default(), None)
            .tree
            .quirks_mode()
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
