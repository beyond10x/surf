//! A page's content as Markdown (`docs/design/page-as-text.md` sections 3, 4 and 6): CommonMark
//! 0.31.2, computed from the tree without CSS, every piece of page text escaped so that it can
//! write neither a Markdown construct nor a line of its own. The rules are in the crate
//! documentation.

use std::collections::{HashMap, HashSet, VecDeque};

use surf_answer::{escaped_at_run_start, escaped_in_text};
use surf_html::{Attribute, Namespace, NodeData, NodeId, Tree};
use surf_page::ActionableElement;

use crate::{collapse, count_node_visit};

/// The bytes of a Markdown line without the `> ` of the answer, its indentation and markers
/// included.
pub const LINE_BYTES_MAX: usize = 1_024;

/// The characters of page text a heading, a link's text or an image's `alt` keeps before it is
/// cut and ends in `[cut]`.
pub const TEXT_CHARS_MAX: usize = 200;

/// The characters of a control's label kept before it is cut and ends in `[cut]`.
pub const LABEL_CHARS_MAX: usize = 60;

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

/// The landmarks, written with the line `[<name> <n> lines]` before their content.
pub const LANDMARKS: [&str; 5] = ["nav", "header", "footer", "main", "aside"];

/// The lines of `tree` as Markdown, without the `> ` prefix of the answer, an empty line as the
/// empty string, one block at a time. A first walk counts the lines of each landmark, a second
/// writes them; each walk has its own stack, so the time is linear in the number of nodes.
pub fn markdown(tree: &Tree) -> impl Iterator<Item = String> + '_ {
    let facts = Facts {
        handles: surf_page::handles(tree).into_iter().collect(),
        rows: surf_page::actionable_elements(tree)
            .into_iter()
            .map(|row| (row.node, row))
            .collect(),
        hidden: surf_page::hidden_roots(tree),
    };
    let mut counting = Walker::new(tree, facts, None);
    while counting.step() {}
    let Walker { facts, counts, .. } = counting;
    Walker::new(tree, facts, Some(counts))
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
    Table,
    Term,
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
    facts: Facts,
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
    fn new(tree: &'a Tree, facts: Facts, counts: Option<HashMap<NodeId, usize>>) -> Self {
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
            stack,
            out: writing.then(VecDeque::new),
            counts: counts.unwrap_or_default(),
            emitted: 0,
            pending_separator: None,
            containers: Vec::new(),
            lists: Vec::new(),
            landmarks: Vec::new(),
            flatten: 0,
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
                let mut atom = String::from("[");
                push_cut(&text, TEXT_CHARS_MAX, &mut atom);
                atom.push_str("](<");
                escape_text(target, true, &mut atom);
                atom.push_str(">) [@");
                atom.push_str(handle);
                atom.push(']');
                self.para.push_atom(atom);
            } else {
                let mut atom = format!("[@{handle} {}", row.kind.name());
                for (set, flag) in [
                    (row.disabled, " disabled"),
                    (row.required, " required"),
                    (row.checked, " checked"),
                ] {
                    if set {
                        atom.push_str(flag);
                    }
                }
                if !row.label.is_empty() {
                    atom.push_str(": ");
                    push_cut(&row.label, LABEL_CHARS_MAX, &mut atom);
                }
                atom.push(']');
                self.para.push_space();
                self.para.push_atom(atom);
                self.para.push_space();
            }
            return;
        }
        let close = match local_name {
            "img" => {
                if let Some(alt) = attribute(attributes, "alt").map(collapse)
                    && !alt.is_empty()
                {
                    let mut atom = String::from("![");
                    push_cut(&alt, TEXT_CHARS_MAX, &mut atom);
                    atom.push(']');
                    self.para.push_atom(atom);
                }
                return;
            }
            "iframe" => {
                if let Some(src) = attribute(attributes, "src") {
                    let mut atom = String::from("[iframe](<");
                    escape_text(src, true, &mut atom);
                    atom.push_str(">)");
                    self.para.push_atom(atom);
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
                self.para.push_atom(code_span(&text));
                return;
            }
            "pre" | "listing" | "xmp" => {
                let text = pre_text(tree, &self.facts.hidden, node);
                if self.flatten > 0 {
                    self.para.push_space();
                    self.para.push_atom(code_span(&collapse(&text)));
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
                self.flatten += 1;
                Close::Table
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
                let marker = match self.lists.last_mut() {
                    Some(list) => {
                        list.items += 1;
                        if list.items > 1 {
                            self.pending_separator = None;
                        }
                        if list.ordered {
                            let marker = format!("{}. ", list.next);
                            list.next = list.next.saturating_add(1);
                            marker
                        } else {
                            "- ".to_string()
                        }
                    }
                    None => "- ".to_string(),
                };
                let rest = " ".repeat(marker.len());
                self.containers.push(Container {
                    first: marker,
                    rest,
                    first_pending: true,
                });
                Close::Item
            }
            "blockquote" => {
                self.block_boundary();
                self.containers.push(Container {
                    first: "> ".to_string(),
                    rest: "> ".to_string(),
                    first_pending: true,
                });
                Close::Quote
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
            Close::Table => {
                self.flatten -= 1;
                if self.flatten == 0 {
                    self.block_boundary();
                } else {
                    self.para.push_space();
                }
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

    /// Write the paragraph collected so far.
    fn flush(&mut self) {
        let para = std::mem::take(&mut self.para);
        let heading = para.kind.map(|level| format!("{} ", "#".repeat(level)));
        let width = LINE_BYTES_MAX.saturating_sub(self.prefix_width());
        let lines = para.lines(width, heading.as_deref().unwrap_or(""));
        for line in lines {
            self.emit(&line);
        }
    }

    fn prefix_width(&self) -> usize {
        self.containers.iter().map(|c| c.rest.len()).sum()
    }

    /// A fenced code block of `text`, its lines as they are, escaped as code.
    fn fence(&mut self, text: &str) {
        let text = text.strip_suffix('\n').unwrap_or(text);
        if text.is_empty() {
            return;
        }
        let fence = "`".repeat(longest_backtick_run(text).max(2) + 1);
        let width = LINE_BYTES_MAX.saturating_sub(self.prefix_width());
        self.emit(&fence);
        for raw in text.split('\n') {
            let mut line = String::new();
            escape_code(raw, true, &mut line);
            if line.is_empty() {
                self.emit("");
            }
            for chunk in break_units(&line, width) {
                self.emit(&escape_mark_at_start(chunk));
            }
        }
        self.emit(&fence);
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

/// A code span of `text`: backticks one more than its longest run of them, a space inside each
/// when the text starts or ends with one, the text escaped as code.
fn code_span(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let ticks = "`".repeat(longest_backtick_run(text) + 1);
    let pad = if text.starts_with('`') || text.ends_with('`') {
        " "
    } else {
        ""
    };
    let mut span = format!("{ticks}{pad}");
    escape_code(text, true, &mut span);
    span.push_str(pad);
    span.push_str(&ticks);
    span
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

/// `text` in pieces of at most `width` bytes, each cut between escape units and holding at
/// least one unit.
fn break_units(text: &str, width: usize) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = start + unit_len(&text[start..]);
        while end < text.len() {
            let next = end + unit_len(&text[end..]);
            if next - start > width {
                break;
            }
            end = next;
        }
        pieces.push(&text[start..end]);
        start = end;
    }
    pieces
}

/// A line that starts with a combining mark kept inside its run gets the mark escaped.
fn escape_mark_at_start(line: &str) -> String {
    match line.chars().next() {
        Some(c) if escaped_at_run_start(c) => {
            let mut out = String::new();
            push_hex(c, &mut out);
            out.push_str(&line[c.len_utf8()..]);
            out
        }
        _ => line.to_string(),
    }
}

/// The escapes of the start of a line of text: `-`, `+`, `=` and the `.` or `)` after leading
/// digits get a backslash, and a leading combining mark is written as its bytes. Page text is
/// the only thing that can start a line of a paragraph with one of them.
fn escape_line_start(line: &str) -> String {
    if line.starts_with(['-', '+', '=']) {
        return format!("\\{line}");
    }
    let digits = line.len() - line.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits > 0 && line[digits..].starts_with(['.', ')']) {
        return format!("{}\\{}", &line[..digits], &line[digits..]);
    }
    escape_mark_at_start(line)
}

#[derive(Debug)]
enum Token {
    /// Text as written, escaped; whether its last character is page text.
    Word(String, bool),
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
    /// The characters of page text a heading still keeps.
    budget: Option<usize>,
    cut: bool,
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
        let run_start = !matches!(self.tokens.last(), Some(Token::Word(_, true)));
        let mut escaped = String::new();
        if let Some(budget) = &mut self.budget {
            let count = word.chars().count();
            if count > *budget {
                let end = word
                    .char_indices()
                    .nth(*budget)
                    .map_or(word.len(), |(at, _)| at);
                escape_text(&word[..end], run_start, &mut escaped);
                self.tokens.push(Token::Word(escaped, true));
                self.tokens.push(Token::Word(CUT.to_string(), false));
                self.cut = true;
                return;
            }
            *budget -= count;
        }
        escape_text(word, run_start, &mut escaped);
        self.tokens.push(Token::Word(escaped, true));
    }

    fn push_atom(&mut self, atom: String) {
        if !self.cut && !atom.is_empty() {
            self.tokens.push(Token::Word(atom, false));
        }
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
                        self.tokens.push(Token::Word(CUT.to_string(), false));
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

    /// The words of the paragraph, each with whether a hard break follows it: emphasis markers
    /// without their pair dropped, adjacent pieces joined.
    fn words(self) -> Vec<(String, bool)> {
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
        let mut words: Vec<(String, bool)> = Vec::new();
        let mut word = String::new();
        for (token, kept) in self.tokens.into_iter().zip(keep) {
            if !kept {
                continue;
            }
            match token {
                Token::Word(text, _) => word.push_str(&text),
                Token::Open(marker) | Token::Close(marker) => word.push_str(marker),
                Token::Space => {
                    if !word.is_empty() {
                        words.push((std::mem::take(&mut word), false));
                    }
                }
                Token::Break => {
                    if !word.is_empty() {
                        words.push((std::mem::take(&mut word), false));
                    }
                    if let Some(last) = words.last_mut() {
                        last.1 = true;
                    }
                }
            }
        }
        if !word.is_empty() {
            words.push((word, false));
        }
        if let Some(last) = words.last_mut() {
            last.1 = false;
        }
        words
    }

    /// The lines of the paragraph: wrapped at the last space that keeps a line within `width`
    /// bytes, a longer word broken between escape units, `prefix` before the first line, the
    /// start of each line escaped.
    fn lines(self, width: usize, prefix: &str) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        let mut first = true;
        let finish = |line: &mut String, first: &mut bool, lines: &mut Vec<String>| {
            let text = std::mem::take(line);
            if *first {
                lines.push(format!("{prefix}{text}"));
                *first = false;
            } else {
                lines.push(text);
            }
        };
        for (word, breaks) in self.words() {
            let room = |first: bool| width.saturating_sub(if first { prefix.len() } else { 0 });
            if !line.is_empty() && line.len() + 1 + word.len() <= room(first) {
                line.push(' ');
                line.push_str(&word);
            } else {
                if !line.is_empty() {
                    finish(&mut line, &mut first, &mut lines);
                }
                let word = escape_line_start(&word);
                if word.len() <= room(first) {
                    line = word;
                } else {
                    let pieces = break_units(&word, room(first).max(1));
                    let last = pieces.len() - 1;
                    for (index, piece) in pieces.into_iter().enumerate() {
                        line = if index == 0 {
                            piece.to_string()
                        } else {
                            escape_line_start(piece)
                        };
                        if index < last {
                            finish(&mut line, &mut first, &mut lines);
                        }
                    }
                }
            }
            if breaks {
                if line.len() < room(first) {
                    line.push('\\');
                }
                finish(&mut line, &mut first, &mut lines);
            }
        }
        if !line.is_empty() {
            finish(&mut line, &mut first, &mut lines);
        }
        lines
    }
}
