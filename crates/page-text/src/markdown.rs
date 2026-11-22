//! A page's content as Markdown (`docs/design/page-as-text.md` sections 3, 4 and 6): CommonMark
//! 0.31.2, computed from the tree without CSS, every piece of page text escaped so that it can
//! write neither a Markdown construct nor a line of its own. The rules are in the crate
//! documentation.

use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use surf_answer::{escaped_at_run_start, escaped_in_text};
use surf_html::{Attribute, Namespace, NodeData, NodeId, Tree};
use surf_page::ActionableElement;

use crate::{collapse, count_node_visit};

#[path = "tables.rs"]
mod tables;

/// The bytes of a Markdown line without the `> ` of the answer, its indentation and markers
/// included.
pub const LINE_BYTES_MAX: usize = 1_024;

/// The characters of page text a heading, a link's text or an image's `alt` keeps before it is
/// cut and ends in `[cut]`.
pub const TEXT_CHARS_MAX: usize = 200;

/// The characters of a control's label kept before it is cut and ends in `[cut]`.
pub const LABEL_CHARS_MAX: usize = 60;

/// The bytes of the prefix of a line: the markers and the indentation of the lists and quotes it
/// is in. A list item or a quote that would pass it is written at the indentation of the one it
/// is in, without a marker of its own (`Markdown::deep` counts those levels); so the text of a
/// line keeps at least `LINE_BYTES_MAX - PREFIX_BYTES_MAX` bytes.
pub const PREFIX_BYTES_MAX: usize = 128;

/// The bytes of an atom: a link with its handle, an image, an `iframe`, a code span, a control.
/// An atom is never broken across lines; a longer one is cut before `[cut]` so that it fits one
/// line behind the longest prefix, a heading's `###### ` and emphasis markers.
pub const ATOM_BYTES_MAX: usize = LINE_BYTES_MAX - PREFIX_BYTES_MAX - 32;

/// The bytes of a fence. A `pre` whose longest run of backticks needs a longer one is an
/// indented code block, which has no fence for a run to close.
pub const FENCE_BYTES_MAX: usize = 16;

/// The largest number of an ordered list item: CommonMark allows at most 9 digits.
const LIST_NUMBER_MAX: u64 = 999_999_999;

/// What a cut adds.
const CUT: &str = "[cut]";

/// The elements written as blocks: the `display: block` and `display: list-item` rules of the
/// HTML Standard's rendering section, read on 2026-11-22 from
/// `https://html.spec.whatwg.org/multipage/rendering.html`: 15.3.2 "The page" (`html`, `body`),
/// 15.3.3 "Flow content", 15.3.6 "Sections and headings" (`:heading` is `h1` to `h6`), 15.3.7
/// "Lists", 15.3.12 "The fieldset and legend elements", 15.5.5 "The details and summary
/// elements" and 15.5.16 "The select element" (`option`, `optgroup`); and `table` (15.3.8,
/// `display: table`, a block-level box). Every other element is inline.
pub const BLOCK_ELEMENTS: [&str; 46] = [
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "center",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "html",
    "legend",
    "li",
    "listing",
    "main",
    "menu",
    "nav",
    "ol",
    "optgroup",
    "option",
    "p",
    "plaintext",
    "pre",
    "search",
    "section",
    "summary",
    "table",
    "ul",
    "xmp",
];

pub use tables::COLUMNS_MAX;

/// The landmarks, written with the line `[<name> <n> lines]` before their content.
pub const LANDMARKS: [&str; 5] = ["nav", "header", "footer", "main", "aside"];

/// The lines of `tree` as Markdown, without the `> ` prefix of the answer, an empty line as the
/// empty string, one block at a time. A first walk counts the lines of each landmark, a second
/// writes them; each walk has its own stack, so the time is linear in the number of nodes.
pub fn markdown(tree: &Tree) -> Markdown<'_> {
    let facts = Rc::new(Facts {
        handles: surf_page::handles(tree).into_iter().collect(),
        rows: surf_page::actionable_elements(tree)
            .into_iter()
            .map(|row| (row.node, row))
            .collect(),
        hidden: surf_page::hidden_roots(tree),
    });
    let mut counting = Walker::new(tree, facts, None);
    while counting.step() {}
    let Walker {
        facts,
        counts,
        deep,
        ..
    } = counting;
    Markdown {
        walker: Walker::new(tree, facts, Some(counts)),
        deep,
    }
}

/// The lines of a page's Markdown, one at a time ([`markdown`]).
pub struct Markdown<'a> {
    walker: Walker<'a>,
    deep: usize,
}

impl Markdown<'_> {
    /// The most levels of lists and quotes written at the indentation of the level above them,
    /// because their prefix would pass [`PREFIX_BYTES_MAX`]; 0 when none was. Known before the
    /// first line: the answer says it in the line `note text: deep <n>`.
    pub fn deep(&self) -> usize {
        self.deep
    }
}

impl Iterator for Markdown<'_> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.walker.next()
    }
}

/// What the walk reads from `surf-page`, once per document.
struct Facts {
    handles: HashMap<NodeId, String>,
    rows: HashMap<NodeId, ActionableElement>,
    /// The nodes hidden while their parent is not: the walk enters no hidden subtree.
    hidden: HashSet<NodeId>,
}

#[derive(Clone, Copy)]
enum Frame {
    Enter(NodeId),
    Exit(NodeId, Close),
}

/// What the end of an element does.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Close {
    Inline,
    Emphasis(&'static str),
    Block,
    Heading,
    List,
    Item,
    Quote,
    Landmark,
    Term,
    /// A list item or a quote written without a container: its prefix would pass
    /// `PREFIX_BYTES_MAX`.
    Flat,
}

/// A list item or a block quote: the prefix of its first line and of the others.
struct Container {
    first: String,
    rest: String,
    first_pending: bool,
}

struct List {
    ordered: bool,
    next: u64,
    items: usize,
}

struct Landmark {
    node: NodeId,
    name: &'static str,
    /// The header line was written, and the number of lines written when it was.
    headed: Option<usize>,
}

struct Walker<'a> {
    tree: &'a Tree,
    facts: Rc<Facts>,
    /// The numbers of the tables nested in the table whose cells this walk writes.
    numbers: Rc<HashMap<NodeId, usize>>,
    /// The nested tables numbered so far in the document.
    tables_seen: usize,
    stack: Vec<Frame>,
    /// The lines ready, when the walk writes; `None` while it counts.
    out: Option<VecDeque<String>>,
    /// The lines of each landmark, counted by the first walk and read by the second.
    counts: HashMap<NodeId, usize>,
    emitted: usize,
    /// A block ended or began: one empty line before the next line, at this many containers.
    pending_separator: Option<usize>,
    containers: Vec<Container>,
    lists: Vec<List>,
    landmarks: Vec<Landmark>,
    /// Inside a table or a heading: blocks are spaces of one paragraph.
    flatten: usize,
    /// The list items and quotes open now that have no container.
    flat: usize,
    /// The most of them open at once.
    deep: usize,
    para: Para,
    done: bool,
}

impl Iterator for Walker<'_> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        loop {
            if let Some(line) = self.out.as_mut().and_then(VecDeque::pop_front) {
                return Some(line);
            }
            if !self.step() {
                return self.out.as_mut().and_then(VecDeque::pop_front);
            }
        }
    }
}

impl<'a> Walker<'a> {
    fn new(tree: &'a Tree, facts: Rc<Facts>, counts: Option<HashMap<NodeId, usize>>) -> Self {
        let writing = counts.is_some();
        let stack = tree
            .children(tree.root())
            .iter()
            .rev()
            .map(|&c| Frame::Enter(c))
            .collect();
        Walker {
            tree,
            facts,
            numbers: Rc::default(),
            tables_seen: 0,
            stack,
            out: writing.then(VecDeque::new),
            counts: counts.unwrap_or_default(),
            emitted: 0,
            pending_separator: None,
            containers: Vec::new(),
            lists: Vec::new(),
            landmarks: Vec::new(),
            flatten: 0,
            flat: 0,
            deep: 0,
            para: Para::default(),
            done: false,
        }
    }

    /// One step of the walk; `false` once it has ended.
    fn step(&mut self) -> bool {
        if self.done {
            return false;
        }
        match self.stack.pop() {
            Some(Frame::Enter(node)) => self.enter(node),
            Some(Frame::Exit(node, close)) => self.exit(node, close),
            None => {
                self.flush();
                self.done = true;
            }
        }
        true
    }

    fn enter(&mut self, node: NodeId) {
        count_node_visit();
        if self.facts.hidden.contains(&node) {
            return;
        }
        let tree = self.tree;
        let (namespace, local_name, attributes) = match tree.data(node) {
            NodeData::Text(text) => {
                self.para.push_text(text);
                return;
            }
            NodeData::Element {
                namespace,
                local_name,
                attributes,
                ..
            } => (*namespace, local_name.as_str(), attributes.as_slice()),
            _ => return,
        };
        if namespace != Namespace::Html || matches!(local_name, "svg" | "math") {
            return;
        }
        if let Some(row) = self.facts.rows.get(&node) {
            let handle = self.facts.handles.get(&node).map_or("", String::as_str);
            if let Some(target) = &row.target {
                let text = match plain_text(tree, &self.facts.hidden, node) {
                    text if text.is_empty() => row.label.clone(),
                    text => text,
                };
                let (atom, spent) = link_atom(&text, target, handle);
                self.para.push_atom(atom, spent);
            } else {
                let mut head = format!("[@{handle} {}", row.kind.name());
                for (set, flag) in [
                    (row.disabled, " disabled"),
                    (row.required, " required"),
                    (row.checked, " checked"),
                ] {
                    if set {
                        head.push_str(flag);
                    }
                }
                let mut atom = head.clone();
                if !row.label.is_empty() {
                    let mut label = String::new();
                    push_cut(&row.label, LABEL_CHARS_MAX, &mut label);
                    atom.push_str(": ");
                    atom.push_str(&fit(&label, ATOM_BYTES_MAX - head.len() - 3));
                }
                atom.push(']');
                let spent = row.label.chars().count().min(LABEL_CHARS_MAX);
                self.para.push_space();
                self.para.push_atom(atom, spent);
                self.para.push_space();
            }
            return;
        }
        let close = match local_name {
            "img" => {
                if let Some(alt) = attribute(attributes, "alt").map(collapse)
                    && !alt.is_empty()
                {
                    let mut escaped = String::new();
                    push_cut(&alt, TEXT_CHARS_MAX, &mut escaped);
                    let atom = format!("![{}]", fit(&escaped, ATOM_BYTES_MAX - 3));
                    self.para
                        .push_atom(atom, alt.chars().count().min(TEXT_CHARS_MAX));
                }
                return;
            }
            "iframe" => {
                if let Some(src) = attribute(attributes, "src") {
                    let mut escaped = String::new();
                    escape_text(src, true, &mut escaped);
                    let atom = format!("[iframe](<{}>)", fit(&escaped, ATOM_BYTES_MAX - 12));
                    self.para.push_atom(atom, 1);
                }
                return;
            }
            "br" => {
                if self.flatten > 0 {
                    self.para.push_space();
                } else {
                    self.para.push_break();
                }
                return;
            }
            "code" => {
                let text = collapse(&plain_text(tree, &self.facts.hidden, node));
                self.para.push_atom(code_span(&text), text.chars().count());
                return;
            }
            "pre" | "listing" | "xmp" => {
                let text = pre_text(tree, &self.facts.hidden, node);
                if self.flatten > 0 {
                    let text = collapse(&text);
                    self.para.push_space();
                    self.para.push_atom(code_span(&text), text.chars().count());
                    self.para.push_space();
                } else {
                    self.block_boundary();
                    self.fence(&text);
                    self.block_boundary();
                }
                return;
            }
            "hr" => {
                if self.flatten > 0 {
                    self.para.push_space();
                } else {
                    self.block_boundary();
                    self.emit("---");
                    self.block_boundary();
                }
                return;
            }
            "em" | "i" => {
                self.para.push_open("*");
                Close::Emphasis("*")
            }
            "strong" | "b" => {
                self.para.push_open("**");
                Close::Emphasis("**")
            }
            "table" if self.numbers.contains_key(&node) => {
                let number = self.numbers[&node];
                self.para.push_space();
                self.para.push_atom(format!("[table {number}]"), 1);
                self.para.push_space();
                return;
            }
            _ if self.flatten > 0 => {
                if BLOCK_ELEMENTS.contains(&local_name) {
                    self.para.push_space();
                    Close::Block
                } else {
                    Close::Inline
                }
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.block_boundary();
                let level = usize::from(local_name.as_bytes()[1] - b'0');
                self.para.kind = Some(level);
                self.para.budget = Some(TEXT_CHARS_MAX);
                self.flatten += 1;
                Close::Heading
            }
            "table" => {
                self.block_boundary();
                self.write_tables(node);
                self.block_boundary();
                return;
            }
            "ul" | "ol" | "menu" | "dir" => {
                self.block_boundary();
                let start = if local_name == "ol" {
                    attribute(attributes, "start")
                        .and_then(parse_start)
                        .unwrap_or(1)
                } else {
                    1
                };
                self.lists.push(List {
                    ordered: local_name == "ol",
                    next: start,
                    items: 0,
                });
                Close::List
            }
            "li" => {
                self.block_boundary();
                let (marker, later) = match self.lists.last_mut() {
                    Some(list) => {
                        list.items += 1;
                        let marker = if list.ordered {
                            let marker = format!("{}. ", list.next.min(LIST_NUMBER_MAX));
                            list.next = list.next.saturating_add(1);
                            marker
                        } else {
                            "- ".to_string()
                        };
                        (marker, list.items > 1)
                    }
                    None => ("- ".to_string(), false),
                };
                let close = self.open_container(marker, Close::Item);
                // Items of one list are not parted (a tight list); items without a marker are,
                // so that their texts stay apart.
                if later && close == Close::Item {
                    self.pending_separator = None;
                }
                close
            }
            "blockquote" => {
                self.block_boundary();
                self.open_container("> ".to_string(), Close::Quote)
            }
            "dt" => {
                self.block_boundary();
                self.para.push_open("**");
                Close::Term
            }
            name if LANDMARKS.contains(&name) => {
                self.block_boundary();
                let name = LANDMARKS
                    .iter()
                    .find(|&&l| l == name)
                    .copied()
                    .unwrap_or("nav");
                self.landmarks.push(Landmark {
                    node,
                    name,
                    headed: None,
                });
                Close::Landmark
            }
            name if BLOCK_ELEMENTS.contains(&name) => {
                self.block_boundary();
                Close::Block
            }
            _ => Close::Inline,
        };
        self.stack.push(Frame::Exit(node, close));
        self.stack
            .extend(tree.children(node).iter().rev().map(|&c| Frame::Enter(c)));
    }

    fn exit(&mut self, node: NodeId, close: Close) {
        match close {
            Close::Inline => {}
            Close::Emphasis(marker) => self.para.push_close(marker),
            Close::Block if self.flatten > 0 => self.para.push_space(),
            Close::Block => self.block_boundary(),
            Close::Heading => {
                self.flatten -= 1;
                self.block_boundary();
            }
            Close::List => {
                self.block_boundary();
                self.lists.pop();
            }
            Close::Item | Close::Quote => {
                self.block_boundary();
                self.containers.pop();
                let depth = self.containers.len();
                if let Some(pending) = &mut self.pending_separator {
                    *pending = (*pending).min(depth);
                }
            }
            Close::Term => {
                self.para.push_close("**");
                self.block_boundary();
            }
            Close::Flat => {
                self.block_boundary();
                self.flat -= 1;
            }
            Close::Landmark => {
                self.block_boundary();
                if let Some(landmark) = self.landmarks.pop()
                    && let Some(start) = landmark.headed
                {
                    self.counts.insert(node, self.emitted - start);
                }
            }
        }
    }

    /// A block begins or ends: the paragraph before it is written, and the next line is parted
    /// from it by one empty line.
    fn block_boundary(&mut self) {
        self.flush();
        let depth = self.containers.len();
        self.pending_separator = Some(self.pending_separator.map_or(depth, |p| p.min(depth)));
    }

    /// Open a list item (`marker` its `- ` or `<n>. `) or a quote (`> `): a container whose
    /// prefix the lines inside it carry, or none when that prefix would pass
    /// [`PREFIX_BYTES_MAX`]; then its content is written at the indentation it is in.
    fn open_container(&mut self, marker: String, close: Close) -> Close {
        if self.prefix_width() + marker.len() > PREFIX_BYTES_MAX {
            self.flat += 1;
            self.deep = self.deep.max(self.flat);
            return Close::Flat;
        }
        let rest = if close == Close::Quote {
            marker.clone()
        } else {
            " ".repeat(marker.len())
        };
        self.containers.push(Container {
            first: marker,
            rest,
            first_pending: true,
        });
        close
    }

    /// Write the paragraph collected so far.
    fn flush(&mut self) {
        let para = std::mem::take(&mut self.para);
        for line in para.lines(self.text_width()) {
            self.emit(&line);
        }
    }

    fn prefix_width(&self) -> usize {
        self.containers.iter().map(|c| c.rest.len()).sum()
    }

    /// The bytes a line keeps for its text behind the prefix of its containers: at least
    /// `LINE_BYTES_MAX - PREFIX_BYTES_MAX`.
    fn text_width(&self) -> usize {
        LINE_BYTES_MAX - self.prefix_width().min(PREFIX_BYTES_MAX)
    }

    /// A fenced code block of `text`, its lines as they are, escaped as code; a line longer than
    /// the line bound is broken between units. When the fence would pass [`FENCE_BYTES_MAX`]
    /// (a run of 16 backticks or more inside), an indented code block: every line behind four
    /// spaces, no fence.
    fn fence(&mut self, text: &str) {
        let text = text.strip_suffix('\n').unwrap_or(text);
        if text.is_empty() {
            return;
        }
        let ticks = longest_backtick_run(text).max(2) + 1;
        let (fence, indent) = if ticks <= FENCE_BYTES_MAX {
            (Some("`".repeat(ticks)), "")
        } else {
            (None, "    ")
        };
        let width = self.text_width() - indent.len();
        if let Some(fence) = &fence {
            self.emit(fence);
        }
        for raw in text.split('\n') {
            let mut line = String::new();
            escape_code(raw, true, &mut line);
            if line.is_empty() {
                self.emit("");
            }
            for chunk in code_lines(&line, width) {
                self.emit(&format!("{indent}{chunk}"));
            }
        }
        if let Some(fence) = &fence {
            self.emit(fence);
        }
    }

    /// Write one line: the empty line a block boundary asked for, the header lines of the
    /// landmarks this line is the first content of, then the line with its prefixes.
    fn emit(&mut self, content: &str) {
        if let Some(depth) = self.pending_separator.take() {
            let first_pending = self
                .containers
                .iter()
                .position(|c| c.first_pending)
                .unwrap_or(self.containers.len());
            if self.emitted > 0 && depth <= first_pending {
                let prefix: String = self.containers[..depth]
                    .iter()
                    .map(|c| c.rest.as_str())
                    .collect();
                self.push(prefix.trim_end().to_string());
            }
        }
        for index in 0..self.landmarks.len() {
            if self.landmarks[index].headed.is_some() {
                continue;
            }
            let landmark = &self.landmarks[index];
            let count = self.counts.get(&landmark.node).copied().unwrap_or(0);
            let header = format!("[{} {count} lines]", landmark.name);
            let line = self.prefixed(&header);
            self.push(line);
            self.landmarks[index].headed = Some(self.emitted);
        }
        let line = self.prefixed(content);
        self.push(line);
    }

    /// `content` behind the prefixes of the containers; an empty line has no trailing space.
    fn prefixed(&mut self, content: &str) -> String {
        let mut line = String::new();
        for container in &mut self.containers {
            if container.first_pending {
                container.first_pending = false;
                line.push_str(&container.first);
            } else {
                line.push_str(&container.rest);
            }
        }
        if content.is_empty() {
            line.truncate(line.trim_end().len());
        }
        line.push_str(content);
        line
    }

    fn push(&mut self, line: String) {
        self.emitted += 1;
        if let Some(out) = &mut self.out {
            out.push_back(line);
        }
    }
}

/// The value of the attribute in no namespace named `name`.
fn attribute<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|a| a.namespace.is_none() && a.local_name == name)
        .map(|a| a.value.as_str())
}

/// The `start` of an `ol`: the HTML Standard's rules for parsing integers, a value from 0 to
/// 999,999,999 (the numbers a CommonMark ordered list item can have); `None` for another.
fn parse_start(value: &str) -> Option<u64> {
    let digits = value.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let digits = digits.strip_prefix('+').unwrap_or(digits);
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    if end == 0 || end > 9 {
        return None;
    }
    digits[..end].parse().ok()
}

/// The text of `node`'s subtree as one run: its text nodes, an `img` by its `alt`, a space for a
/// `br` and around a block; hidden nodes, `svg` and `math` left out. ASCII whitespace stripped
/// and collapsed. One walk with its own stack.
fn plain_text(tree: &Tree, hidden: &HashSet<NodeId>, node: NodeId) -> String {
    let mut text = String::new();
    // `None` is the space after a block.
    let mut pending: Vec<Option<NodeId>> =
        tree.children(node).iter().rev().map(|&c| Some(c)).collect();
    while let Some(next) = pending.pop() {
        let Some(next) = next else {
            text.push(' ');
            continue;
        };
        count_node_visit();
        if hidden.contains(&next) {
            continue;
        }
        match tree.data(next) {
            NodeData::Text(data) => text.push_str(data),
            NodeData::Element {
                namespace: Namespace::Html,
                local_name,
                attributes,
                ..
            } if !matches!(local_name.as_str(), "svg" | "math") => {
                let name = local_name.as_str();
                if name == "img" {
                    text.push(' ');
                    text.push_str(attribute(attributes, "alt").unwrap_or(""));
                    text.push(' ');
                    continue;
                }
                let block = BLOCK_ELEMENTS.contains(&name);
                if name == "br" || block {
                    text.push(' ');
                }
                if block {
                    pending.push(None);
                }
                pending.extend(tree.children(next).iter().rev().map(|&c| Some(c)));
            }
            _ => {}
        }
    }
    collapse(&text)
}

/// The text of a `pre`, `listing` or `xmp` as it is: its text nodes, a line feed for a `br`,
/// hidden nodes left out. One walk with its own stack.
fn pre_text(tree: &Tree, hidden: &HashSet<NodeId>, node: NodeId) -> String {
    let mut text = String::new();
    let mut pending: Vec<NodeId> = tree.children(node).iter().rev().copied().collect();
    while let Some(next) = pending.pop() {
        count_node_visit();
        if hidden.contains(&next) {
            continue;
        }
        match tree.data(next) {
            NodeData::Text(data) => text.push_str(data),
            NodeData::Element {
                namespace: Namespace::Html,
                local_name,
                ..
            } if !matches!(local_name.as_str(), "svg" | "math") => {
                if local_name == "br" {
                    text.push('\n');
                }
                pending.extend(tree.children(next).iter().rev());
            }
            _ => {}
        }
    }
    text
}

fn longest_backtick_run(text: &str) -> usize {
    text.split(|c| c != '`').map(str::len).max().unwrap_or(0)
}
/// Write `c` as `\x` and two lower-case hexadecimal digits per UTF-8 byte.
fn push_hex(c: char, out: &mut String) {
    let mut bytes = [0u8; 4];
    for byte in c.encode_utf8(&mut bytes).bytes() {
        out.push_str(&format!("\\x{byte:02x}"));
    }
}

/// The characters of page text that get a backslash in text, link text and a link destination.
const TEXT_ESCAPED: &[char] = &['\\', '`', '*', '_', '[', ']', '<', '>', '&', '|', '~', '#'];

/// The characters of page text that get a backslash in code.
const CODE_ESCAPED: &[char] = &['\\', '[', ']', '<', '|'];

fn escape_with(text: &str, run_start: bool, set: &[char], out: &mut String) {
    for (index, c) in text.chars().enumerate() {
        if escaped_in_text(c) || (index == 0 && run_start && escaped_at_run_start(c)) {
            push_hex(c, out);
        } else {
            if set.contains(&c) {
                out.push('\\');
            }
            out.push(c);
        }
    }
}

/// Escape page text for text, link text or a link destination; `run_start` when the text
/// follows a character surf wrote.
fn escape_text(text: &str, run_start: bool, out: &mut String) {
    escape_with(text, run_start, TEXT_ESCAPED, out);
}

fn escape_code(text: &str, run_start: bool, out: &mut String) {
    escape_with(text, run_start, CODE_ESCAPED, out);
}

/// `text` cut at `limit` characters and escaped, then `[cut]` when it was cut.
fn push_cut(text: &str, limit: usize, out: &mut String) {
    match text.char_indices().nth(limit) {
        Some((end, _)) => {
            escape_text(&text[..end], true, out);
            out.push_str(CUT);
        }
        None => escape_text(text, true, out),
    }
}

/// The length in bytes of the escape unit `rest` starts with: `\x` and two hexadecimal digits
/// with the escapes that follow for the rest of that UTF-8 character, a `\` with the character
/// after it, or one character.
fn unit_len(rest: &str) -> usize {
    let bytes = rest.as_bytes();
    if bytes.first() != Some(&b'\\') {
        return rest.chars().next().map_or(0, char::len_utf8);
    }
    let Some(lead) = hex_escape(bytes) else {
        return 1 + rest[1..].chars().next().map_or(0, char::len_utf8);
    };
    let width = match lead {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xFF => 4,
        _ => 1,
    };
    let mut length = 4;
    for _ in 1..width {
        match bytes.get(length..).and_then(hex_escape) {
            Some(0x80..=0xBF) => length += 4,
            _ => break,
        }
    }
    length
}

fn hex_escape(bytes: &[u8]) -> Option<u8> {
    match bytes {
        [b'\\', b'x', high, low, ..] => {
            let digit = |b: u8| (b as char).to_digit(16);
            Some((digit(*high)? * 16 + digit(*low)?) as u8)
        }
        _ => None,
    }
}

/// A code span of `text`, one atom: backticks one more than the longest run of them inside, so
/// no run inside can close it, a space inside each when the text starts or ends with one, the
/// text escaped as code. Longer than [`ATOM_BYTES_MAX`]: the text cut between units, then
/// `[cut]` after the span.
fn code_span(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut escaped = String::new();
    escape_code(text, true, &mut escaped);
    let span = |inner: &str| {
        let ticks = "`".repeat(longest_backtick_run(inner) + 1);
        let pad = if inner.starts_with('`') || inner.ends_with('`') {
            " "
        } else {
            ""
        };
        format!("{ticks}{pad}{inner}{pad}{ticks}")
    };
    let whole = span(&escaped);
    if whole.len() <= ATOM_BYTES_MAX {
        return whole;
    }
    let starts = escaped.starts_with('`');
    let (mut end, mut run, mut longest) = (0, 0, 0);
    while end < escaped.len() {
        let next = end + unit_len(&escaped[end..]);
        let tick = &escaped[end..next] == "`";
        let run_next = if tick { run + 1 } else { 0 };
        let longest_next = longest.max(run_next);
        let pads = if starts || tick { 2 } else { 0 };
        if 2 * (longest_next + 1) + pads + next + CUT.len() > ATOM_BYTES_MAX {
            break;
        }
        (end, run, longest) = (next, run_next, longest_next);
    }
    if end == 0 {
        return CUT.to_string();
    }
    format!("{}{CUT}", span(&escaped[..end]))
}

/// `escaped`, escape units of page text that may end in `[cut]`, in at most `max` bytes: cut
/// between units before a `[cut]` when it is longer.
fn fit(escaped: &str, max: usize) -> String {
    if escaped.len() <= max {
        return escaped.to_string();
    }
    let body = escaped.strip_suffix(CUT).unwrap_or(escaped);
    let limit = max.saturating_sub(CUT.len());
    let mut end = 0;
    while end < body.len() {
        let next = end + unit_len(&body[end..]);
        if next > limit {
            break;
        }
        end = next;
    }
    format!("{}{CUT}", &body[..end])
}

/// A link, one atom: `[text](<target>) [@handle]`, its text cut at [`TEXT_CHARS_MAX`]
/// characters; when it is longer than [`ATOM_BYTES_MAX`], its text and its target are cut
/// before `[cut]` (the text keeps at least half of what is left beside the handle), so a cut
/// target ends in `[cut]>`. The handle stays whole. With the characters of page text a
/// renderer shows of it.
fn link_atom(text: &str, target: &str, handle: &str) -> (String, usize) {
    let mut label = String::new();
    push_cut(text, TEXT_CHARS_MAX, &mut label);
    let mut destination = String::new();
    escape_text(target, true, &mut destination);
    let room = ATOM_BYTES_MAX - "[](<>) [@]".len() - handle.len();
    if label.len() + destination.len() > room {
        let label_max = label
            .len()
            .min((room / 2).max(room.saturating_sub(destination.len())));
        label = fit(&label, label_max);
        destination = fit(&destination, room - label.len());
    }
    (
        format!("[{label}](<{destination}>) [@{handle}]"),
        text.chars().count().min(TEXT_CHARS_MAX),
    )
}

/// The lines of one line of code `text`, escaped, broken between units at `width` bytes; a
/// combining mark that starts a line is written as its bytes.
fn code_lines(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let mut line = String::new();
        if let Some(c) = rest.chars().next()
            && escaped_at_run_start(c)
        {
            push_hex(c, &mut line);
            rest = &rest[c.len_utf8()..];
        }
        while !rest.is_empty() {
            let unit = unit_len(rest);
            if !line.is_empty() && line.len() + unit > width {
                break;
            }
            line.push_str(&rest[..unit]);
            rest = &rest[unit..];
        }
        lines.push(line);
    }
    lines
}

/// The escape of the start of a line of text `rest` (escaped page text), when it needs one, and
/// the bytes of `rest` it stands for: `-`, `+` and `=` get a backslash, as the `.` or `)` after
/// leading digits does when they fit the `room` of the line, and a combining mark is written as
/// its bytes. Page text is the only thing that can start a line of a paragraph with one of
/// them; no line of a paragraph starts with a space (a text piece holds no ASCII whitespace,
/// and a space is written only between two words of one line), so the marker is the line's
/// first character as CommonMark counts indentation.
fn line_start(rest: &str, room: usize) -> Option<(String, usize)> {
    let c = rest.chars().next()?;
    if matches!(c, '-' | '+' | '=') {
        return Some((format!("\\{c}"), 1));
    }
    let head = &rest.as_bytes()[..rest.len().min(room)];
    let digits = head.iter().take_while(|b| b.is_ascii_digit()).count();
    if digits > 0 && digits + 2 <= room && matches!(rest.as_bytes().get(digits), Some(b'.' | b')'))
    {
        return Some((
            format!("{}\\{}", &rest[..digits], &rest[digits..=digits]),
            digits + 1,
        ));
    }
    if escaped_at_run_start(c) {
        let mut hex = String::new();
        push_hex(c, &mut hex);
        return Some((hex, c.len_utf8()));
    }
    None
}

#[derive(Debug)]
enum Token {
    /// Page text, escaped.
    Text(String),
    /// An atom surf wrote (a link, an image, a code span, a control, `[cut]`): never broken.
    Atom(String),
    Space,
    Open(&'static str),
    Close(&'static str),
    Break,
}

/// The inline content of one block, collected until the block ends.
#[derive(Default)]
struct Para {
    tokens: Vec<Token>,
    /// The level of a heading.
    kind: Option<usize>,
    /// The characters of page text a heading or a cell still keeps.
    budget: Option<usize>,
    cut: bool,
}

/// A piece of a word: escaped page text, which a line may break between two units, or an atom,
/// which it does not break.
#[derive(Clone, Debug)]
enum Piece {
    Text(String),
    Atom(String),
}

impl Piece {
    fn as_str(&self) -> &str {
        match self {
            Piece::Text(text) | Piece::Atom(text) => text,
        }
    }
}

/// The pieces between two spaces, and whether a hard break follows them.
#[derive(Clone, Debug, Default)]
pub(crate) struct Word {
    pieces: Vec<Piece>,
    breaks: bool,
}

impl Word {
    fn len(&self) -> usize {
        self.pieces.iter().map(|p| p.as_str().len()).sum()
    }

    fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    fn push_text(&mut self, text: &str) {
        match self.pieces.last_mut() {
            Some(Piece::Text(last)) => last.push_str(text),
            _ => self.pieces.push(Piece::Text(text.to_string())),
        }
    }

    /// The word as written on one line.
    pub(crate) fn joined(&self) -> String {
        self.pieces.iter().map(Piece::as_str).collect()
    }
}

impl Para {
    fn push_text(&mut self, text: &str) {
        let mut rest = text;
        while !rest.is_empty() {
            let word_end = rest
                .find(|c: char| c.is_ascii_whitespace())
                .unwrap_or(rest.len());
            if word_end == 0 {
                self.push_space();
                rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
                continue;
            }
            self.push_word(&rest[..word_end]);
            rest = &rest[word_end..];
        }
    }

    fn push_word(&mut self, word: &str) {
        if self.cut {
            return;
        }
        let run_start = !matches!(self.tokens.last(), Some(Token::Text(_)));
        let mut escaped = String::new();
        if let Some(budget) = &mut self.budget {
            let count = word.chars().count();
            if count > *budget {
                let end = word
                    .char_indices()
                    .nth(*budget)
                    .map_or(word.len(), |(at, _)| at);
                escape_text(&word[..end], run_start, &mut escaped);
                self.tokens.push(Token::Text(escaped));
                self.tokens.push(Token::Atom(CUT.to_string()));
                self.cut = true;
                return;
            }
            *budget -= count;
        }
        escape_text(word, run_start, &mut escaped);
        self.tokens.push(Token::Text(escaped));
    }

    /// An atom that shows `spent` characters of page text (at least one is counted): within the
    /// budget of a heading or a cell, or `[cut]` instead, so a cut falls between atoms.
    fn push_atom(&mut self, atom: String, spent: usize) {
        if self.cut || atom.is_empty() {
            return;
        }
        if let Some(budget) = &mut self.budget {
            let spent = spent.max(1);
            if spent > *budget {
                self.tokens.push(Token::Atom(CUT.to_string()));
                self.cut = true;
                return;
            }
            *budget -= spent;
        }
        self.tokens.push(Token::Atom(atom));
    }

    fn push_space(&mut self) {
        if self.cut {
            return;
        }
        let opens = self
            .tokens
            .iter()
            .rev()
            .take_while(|t| matches!(t, Token::Open(_)))
            .count();
        let before = self.tokens.len() - opens;
        match before.checked_sub(1).map(|at| &self.tokens[at]) {
            None | Some(Token::Space) | Some(Token::Break) => {}
            Some(_) => {
                if let Some(budget) = &mut self.budget {
                    if *budget == 0 {
                        self.tokens.push(Token::Atom(CUT.to_string()));
                        self.cut = true;
                        return;
                    }
                    *budget -= 1;
                }
                self.tokens.insert(before, Token::Space);
            }
        }
    }

    fn push_open(&mut self, marker: &'static str) {
        if !self.cut {
            self.tokens.push(Token::Open(marker));
        }
    }

    fn push_close(&mut self, marker: &'static str) {
        if self.cut {
            return;
        }
        match self.tokens.last() {
            Some(Token::Open(open)) if *open == marker => {
                self.tokens.pop();
            }
            Some(Token::Space) => {
                self.tokens.pop();
                self.tokens.push(Token::Close(marker));
                self.tokens.push(Token::Space);
            }
            _ => self.tokens.push(Token::Close(marker)),
        }
    }

    fn push_break(&mut self) {
        if self.cut {
            return;
        }
        if matches!(self.tokens.last(), Some(Token::Space)) {
            self.tokens.pop();
        }
        if !self.tokens.is_empty() {
            self.tokens.push(Token::Break);
        }
    }

    /// The words of the paragraph: emphasis markers without their pair dropped, the others
    /// joined to the text beside them, adjacent text joined into one piece.
    pub(crate) fn words(self) -> Vec<Word> {
        let mut keep = vec![true; self.tokens.len()];
        let mut opens: Vec<(usize, &'static str)> = Vec::new();
        for (index, token) in self.tokens.iter().enumerate() {
            match token {
                Token::Open(marker) => opens.push((index, marker)),
                Token::Close(marker) => match opens.last() {
                    Some((_, open)) if open == marker => {
                        opens.pop();
                    }
                    _ => keep[index] = false,
                },
                _ => {}
            }
        }
        for (index, _) in opens {
            keep[index] = false;
        }
        let mut words: Vec<Word> = Vec::new();
        let mut word = Word::default();
        for (token, kept) in self.tokens.into_iter().zip(keep) {
            if !kept {
                continue;
            }
            match token {
                Token::Text(text) => word.push_text(&text),
                Token::Open(marker) | Token::Close(marker) => word.push_text(marker),
                Token::Atom(atom) => word.pieces.push(Piece::Atom(atom)),
                Token::Space => {
                    if !word.is_empty() {
                        words.push(std::mem::take(&mut word));
                    }
                }
                Token::Break => {
                    if !word.is_empty() {
                        words.push(std::mem::take(&mut word));
                    }
                    if let Some(last) = words.last_mut() {
                        last.breaks = true;
                    }
                }
            }
        }
        if !word.is_empty() {
            words.push(word);
        }
        if let Some(last) = words.last_mut() {
            last.breaks = false;
        }
        words
    }

    /// The lines of the paragraph, at most `width` bytes each: wrapped at the last space that
    /// keeps a line within them, else between two pieces or two units of text, never inside an
    /// atom; the start of each line escaped. A heading is one line behind its `#` marker, cut
    /// before `[cut]` when it is longer.
    fn lines(self, width: usize) -> Vec<String> {
        let heading = self.kind.map(|level| format!("{} ", "#".repeat(level)));
        let words = self.words();
        let Some(prefix) = heading else {
            return Layout::run(&words, width, "", false);
        };
        let lines = Layout::run(&words, width, &prefix, false);
        if lines.len() <= 1 {
            return lines;
        }
        Layout::run(&words, width, &prefix, true)
    }
}

/// The lines of one paragraph while they are made.
struct Layout<'p> {
    lines: Vec<String>,
    line: String,
    width: usize,
    /// Before the first line: a heading's marker.
    prefix: &'p str,
    /// One line only, ending in `[cut]` when the words do not fit it.
    single: bool,
    full: bool,
}

impl<'p> Layout<'p> {
    fn run(words: &[Word], width: usize, prefix: &'p str, single: bool) -> Vec<String> {
        let mut layout = Layout {
            lines: Vec::new(),
            line: String::new(),
            width,
            prefix,
            single,
            full: false,
        };
        for word in words {
            if layout.full {
                break;
            }
            if !layout.line.is_empty() && layout.line.len() + 1 + word.len() <= layout.room() {
                layout.line.push(' ');
                for piece in &word.pieces {
                    layout.line.push_str(piece.as_str());
                }
            } else {
                if !layout.line.is_empty() {
                    if single && layout.line.len() + 1 < layout.room() {
                        layout.line.push(' ');
                    } else {
                        layout.new_line();
                    }
                }
                layout.place(word);
            }
            if word.breaks && !single && !layout.full {
                if layout.line.len() < layout.room() {
                    layout.line.push('\\');
                }
                layout.new_line();
            }
        }
        if !layout.line.is_empty() {
            layout.push_line();
        }
        layout.lines
    }

    /// The bytes the current line holds: a heading's marker on the first, and the `[cut]` of a
    /// single line, left out.
    fn room(&self) -> usize {
        let first = if self.lines.is_empty() {
            self.prefix.len()
        } else {
            0
        };
        let cut = if self.single { CUT.len() } else { 0 };
        self.width.saturating_sub(first + cut)
    }

    fn push_line(&mut self) {
        let text = std::mem::take(&mut self.line);
        if self.lines.is_empty() {
            self.lines.push(format!("{}{text}", self.prefix));
        } else {
            self.lines.push(text);
        }
    }

    /// End the current line; on a single line, end it with `[cut]` and take no more.
    fn new_line(&mut self) {
        if self.single {
            if self.line.ends_with(' ') {
                self.line.pop();
            }
            self.line.push_str(CUT);
            self.full = true;
        } else {
            self.push_line();
        }
    }

    /// Place a word that does not fit beside the current line's words: piece by piece, an
    /// atom on the next line when it does not fit this one, text unit by unit.
    fn place(&mut self, word: &Word) {
        for piece in &word.pieces {
            if self.full {
                return;
            }
            match piece {
                Piece::Atom(atom) => {
                    if !self.line.is_empty() && self.line.len() + atom.len() > self.room() {
                        self.new_line();
                        if self.full {
                            return;
                        }
                    }
                    self.line.push_str(atom);
                }
                Piece::Text(text) => self.place_text(text),
            }
        }
    }

    fn place_text(&mut self, text: &str) {
        let mut rest = text;
        while !rest.is_empty() && !self.full {
            let room = self.room();
            if self.line.is_empty()
                && let Some((head, used)) = line_start(rest, room)
            {
                self.line.push_str(&head);
                rest = &rest[used..];
                continue;
            }
            let unit = unit_len(rest);
            if !self.line.is_empty() && self.line.len() + unit > room {
                self.new_line();
                continue;
            }
            self.line.push_str(&rest[..unit]);
            rest = &rest[unit..];
        }
    }
}
