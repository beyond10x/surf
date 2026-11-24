//! The selective extraction of the HTML Standard (`story:webidl-bindings`, decision 7): the
//! single page holds 213 blocks of Web IDL and constructs the reader refuses (`Promise<T>`,
//! overloads), so only what the selection names is read.
//!
//! The page is not parsed whole. Each `pre` element whose start tag, or the start tag of the
//! `code` element right inside it, has the class `idl` is one block, unless the `pre` has the
//! class `extract` (an example of the standard, 3 blocks); the bytes from its `<pre`
//! to its `</pre>` are parsed with `surf-html` alone, and its text content is the block's
//! text. A block is taken when it defines one of the interfaces asked for (`interface <name>`
//! or `partial interface <name>`). From a block taken, the text read keeps the header of each
//! such definition (`interface Window : EventTarget {`), each member the selection names for
//! it, and the closing `};`; every other part of the block is blanked to spaces, its line
//! feeds kept, and listed in [`HtmlIdl::skipped`] with its line: the extended attributes of a
//! definition, each member not named, each other definition. So a construct the reader
//! refuses is refused only where it is selected, and every line of an error or of the model is
//! a line of [`HtmlIdl::text`]: the texts of the blocks taken, in the order of the page, each
//! ending in a line feed.

use std::collections::{BTreeMap, BTreeSet};

use surf_html::{NodeData, ParseOptions};

use crate::ReadError;

/// The definitions of the HTML Standard the bindings read (S6 of the epic, option a):
/// `Window`, and the partials of `Document`.
pub const HTML_DEFINITIONS: &[&str] = &["Window", "Document"];

/// A part of a block taken that the text read leaves out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skipped {
    /// Its first line in [`HtmlIdl::text`], from 1.
    pub line: usize,
    /// What it is: `extended attributes of \`interface Window\``, `member \`name\` of
    /// \`Window\``, `an unnamed member of \`Window\``, or `definition \`<its first words>\``.
    pub what: String,
}

/// What the extraction read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlIdl {
    /// The blocks of Web IDL of the page.
    pub blocks: usize,
    /// The blocks taken.
    pub taken: usize,
    /// The text the reader reads: the blocks taken, each ending in a line feed, with every
    /// part not selected blanked.
    pub text: String,
    /// The parts left out, in the order of the text.
    pub skipped: Vec<Skipped>,
}

/// Reads the blocks of Web IDL of the page `bytes` that define an interface of `wanted`, keeping
/// of each such definition the members `wanted` names for it. A block that does not parse to
/// one `pre` element is refused with line 0.
pub fn extract_selected(
    bytes: &[u8],
    file: &str,
    wanted: &BTreeMap<String, BTreeSet<String>>,
) -> Result<HtmlIdl, ReadError> {
    let mut blocks = 0;
    let mut taken = 0;
    let mut text = String::new();
    let mut skipped = Vec::new();
    for (start, end) in idl_blocks(bytes) {
        blocks += 1;
        let block = block_text(&bytes[start..end], file, start)?;
        let definitions = split_definitions(&block);
        if !definitions
            .iter()
            .any(|definition| wanted_name(&block, definition, wanted).is_some())
        {
            continue;
        }
        taken += 1;
        let first_line = text.matches('\n').count() + 1;
        let kept = keep_selected(&block, &definitions, wanted, first_line, &mut skipped);
        text.push_str(&kept);
        if !text.ends_with('\n') {
            text.push('\n');
        }
    }
    Ok(HtmlIdl {
        blocks,
        taken,
        text,
        skipped,
    })
}

/// The cut of a page: its blocks of Web IDL ([`extract_selected`]'s rule), each byte for byte
/// from its `<pre` to its `</pre>`, in the page's order, each followed by one line feed. The
/// tree holds the HTML Standard in this form (`data/whatwg/html-idl.html`); read with
/// [`extract_selected`], a cut gives what its page gives.
pub fn cut_idl_blocks(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for (start, end) in idl_blocks(bytes) {
        out.extend_from_slice(&bytes[start..end]);
        out.push(b'\n');
    }
    out
}

/// The byte ranges `<pre ...</pre>` of the blocks of Web IDL: a `pre` start tag with the class
/// `idl`, or one followed at once by a `code` start tag with that class; not a `pre` with the
/// class `extract`.
fn idl_blocks(bytes: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = find(bytes, b"<pre", at) {
        at = found + 4;
        let Some(&after) = bytes.get(at) else {
            break;
        };
        if !(after == b'>' || after.is_ascii_whitespace()) {
            continue;
        }
        let Some(tag_end) = find(bytes, b">", at) else {
            break;
        };
        let mut idl = has_class(&bytes[at..tag_end], "idl");
        if !idl && bytes[tag_end + 1..].starts_with(b"<code") {
            let code_start = tag_end + 1 + 5;
            if let Some(code_end) = find(bytes, b">", code_start) {
                idl = has_class(&bytes[code_start..code_end], "idl");
            }
        }
        if !idl || has_class(&bytes[at..tag_end], "extract") {
            continue;
        }
        let Some(close) = find(bytes, b"</pre>", tag_end) else {
            break;
        };
        out.push((found, close + 6));
        at = close + 6;
    }
    out
}

/// Whether the attributes of a start tag hold `class` with the class `name`, quoted or not.
fn has_class(attributes: &[u8], name: &str) -> bool {
    let text = String::from_utf8_lossy(attributes);
    let mut rest = text.as_ref();
    while let Some(at) = rest.find("class=") {
        let value = &rest[at + 6..];
        let (quote, value) = match value.chars().next() {
            Some(q @ ('\'' | '"')) => (Some(q), &value[1..]),
            _ => (None, value),
        };
        let end = match quote {
            Some(q) => value.find(q).unwrap_or(value.len()),
            None => value
                .find(|c: char| c.is_ascii_whitespace())
                .unwrap_or(value.len()),
        };
        if value[..end]
            .split_ascii_whitespace()
            .any(|class| class == name)
        {
            return true;
        }
        rest = &value[end..];
    }
    false
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| at + from)
}

/// The text content of the one `pre` element of `bytes`, parsed alone as UTF-8, the page's
/// encoding (its `meta charset`, which the block does not hold).
fn block_text(bytes: &[u8], file: &str, offset: usize) -> Result<String, ReadError> {
    let Ok(source) = std::str::from_utf8(bytes) else {
        return Err(ReadError::new(
            file,
            0,
            format!("a block of Web IDL at byte {offset} that is not UTF-8"),
        ));
    };
    let tree = &surf_html::parse_document(source, &ParseOptions::default());
    let pres: Vec<_> = tree
        .descendants(tree.root())
        .into_iter()
        .filter(|&node| {
            matches!(tree.data(node), NodeData::Element { local_name, .. } if local_name == "pre")
        })
        .collect();
    let [pre] = pres.as_slice() else {
        return Err(ReadError::new(
            file,
            0,
            format!("a block of Web IDL at byte {offset} that is not one `pre` element"),
        ));
    };
    let mut text = String::new();
    for node in tree.descendants(*pre) {
        if let NodeData::Text(data) = tree.data(node) {
            text.push_str(data);
        }
    }
    Ok(text)
}

/// A definition of a block: its byte range, and inside it the range of its extended attributes,
/// its header up to `{` and its members.
#[derive(Debug)]
struct Definition {
    start: usize,
    end: usize,
    attributes: Option<(usize, usize)>,
    header: (usize, usize),
    /// The members, each its range up to and with its `;`; empty without a body.
    members: Vec<(usize, usize)>,
    /// The range from the body's `}` to the definition's end.
    close: Option<(usize, usize)>,
}

/// The definitions of a block, split at each `;` outside braces, brackets, parentheses,
/// strings and comments; a definition with a body is split into its members the same way.
fn split_definitions(text: &str) -> Vec<Definition> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut body_open: Option<usize> = None;
    let mut body_close: Option<usize> = None;
    let mut member_start = 0;
    let mut members = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'/' if bytes.get(at + 1) == Some(&b'/') => {
                at = text[at..].find('\n').map_or(bytes.len(), |n| at + n);
                continue;
            }
            b'/' if bytes.get(at + 1) == Some(&b'*') => {
                at = text[at + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |n| at + 2 + n + 2);
                continue;
            }
            b'"' => {
                at = text[at + 1..]
                    .find('"')
                    .map_or(bytes.len(), |n| at + 1 + n + 1);
                continue;
            }
            b'{' | b'[' | b'(' => {
                if depth == 0 && bytes[at] == b'{' {
                    body_open = Some(at);
                    member_start = at + 1;
                }
                depth += 1;
            }
            b'}' | b']' | b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && bytes[at] == b'}' {
                    body_close = Some(at);
                }
            }
            b';' if depth == 1 && body_open.is_some() && body_close.is_none() => {
                members.push((member_start, at + 1));
                member_start = at + 1;
            }
            b';' if depth == 0 => {
                let end = at + 1;
                out.push(definition(
                    text, start, end, body_open, body_close, &members,
                ));
                start = end;
                body_open = None;
                body_close = None;
                members.clear();
            }
            _ => {}
        }
        at += 1;
    }
    out
}

fn definition(
    text: &str,
    start: usize,
    end: usize,
    body_open: Option<usize>,
    body_close: Option<usize>,
    members: &[(usize, usize)],
) -> Definition {
    let bytes = text.as_bytes();
    let mut head = skip_space(text, start, end);
    let mut attributes = None;
    if bytes.get(head) == Some(&b'[') {
        let mut depth = 0usize;
        let mut at = head;
        while at < end {
            match bytes[at] {
                b'[' => depth += 1,
                b']' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            at += 1;
        }
        attributes = Some((head, (at + 1).min(end)));
        head = skip_space(text, (at + 1).min(end), end);
    }
    let header_end = body_open.map_or(end, |open| open + 1);
    Definition {
        start,
        end,
        attributes,
        header: (head, header_end),
        members: members.to_vec(),
        close: body_close.map(|close| (close, end)),
    }
}

/// The first byte at or after `at` that is no white space and no comment.
fn skip_space(text: &str, mut at: usize, end: usize) -> usize {
    let bytes = text.as_bytes();
    while at < end {
        if bytes[at].is_ascii_whitespace() {
            at += 1;
        } else if text[at..end].starts_with("//") {
            at = text[at..end].find('\n').map_or(end, |n| at + n);
        } else if text[at..end].starts_with("/*") {
            at = text[at + 2..end].find("*/").map_or(end, |n| at + 2 + n + 2);
        } else {
            break;
        }
    }
    at
}

/// The words of `text` outside comments: identifiers, and each other character that is no
/// white space as a word of its own.
fn words(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let c = bytes[at];
        if c.is_ascii_whitespace() {
            at += 1;
        } else if text[at..].starts_with("//") {
            at = text[at..].find('\n').map_or(bytes.len(), |n| at + n);
        } else if text[at..].starts_with("/*") {
            at = text[at + 2..]
                .find("*/")
                .map_or(bytes.len(), |n| at + 2 + n + 2);
        } else if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' {
            let begin = at;
            while at < bytes.len()
                && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_' || bytes[at] == b'-')
            {
                at += 1;
            }
            out.push(&text[begin..at]);
        } else {
            let len = text[at..].chars().next().map_or(1, char::len_utf8);
            out.push(&text[at..at + len]);
            at += len;
        }
    }
    out
}

/// The name of `definition` when it is `interface <name>` or `partial interface <name>` (not a
/// mixin) and `wanted` asks for it.
fn wanted_name<'t>(
    text: &'t str,
    definition: &Definition,
    wanted: &BTreeMap<String, BTreeSet<String>>,
) -> Option<&'t str> {
    let header = words(&text[definition.header.0..definition.header.1]);
    let name = match header.as_slice() {
        ["interface", name, ..] | ["partial", "interface", name, ..] if *name != "mixin" => *name,
        _ => return None,
    };
    wanted.contains_key(name).then_some(name)
}

/// The name of a member written as `text`, or `None` for an unnamed one (a special operation
/// without an identifier).
fn member_name(text: &str) -> Option<String> {
    let mut all = words(text);
    if all.first() == Some(&"[") {
        let mut depth = 0usize;
        let mut cut = all.len();
        for (index, word) in all.iter().enumerate() {
            match *word {
                "[" => depth += 1,
                "]" => {
                    depth -= 1;
                    if depth == 0 {
                        cut = index + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        all.drain(..cut);
    }
    let first = *all.first()?;
    match first {
        "constructor" => return Some("constructor".to_string()),
        "iterable" | "maplike" | "setlike" | "async" => return Some(first.to_string()),
        "stringifier" if all.get(1) == Some(&";") => return Some("stringifier".to_string()),
        _ => {}
    }
    if all.contains(&"attribute") {
        return all
            .iter()
            .rev()
            .find(|word| is_identifier(word))
            .map(|word| word.to_string());
    }
    if first == "const" {
        let equals = all.iter().position(|word| *word == "=")?;
        return all[..equals].last().map(|word| word.to_string());
    }
    let open = all.iter().position(|word| *word == "(")?;
    let before = *all.get(open.checked_sub(1)?)?;
    let further = open.checked_sub(2).and_then(|at| all.get(at)).copied();
    let special = matches!(
        further,
        None | Some("getter" | "setter" | "deleter" | "static" | "stringifier" | "legacycaller")
    );
    (is_identifier(before) && !special).then(|| before.to_string())
}

fn is_identifier(word: &str) -> bool {
    word.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
}

/// The block's text with every part not selected blanked, the parts listed in `skipped` with
/// their lines counted from `first_line`.
fn keep_selected(
    text: &str,
    definitions: &[Definition],
    wanted: &BTreeMap<String, BTreeSet<String>>,
    first_line: usize,
    skipped: &mut Vec<Skipped>,
) -> String {
    let mut out: Vec<u8> = text.as_bytes().to_vec();
    let line_of = |at: usize| first_line + text[..at].matches('\n').count();
    let mut blank = |from: usize, to: usize, what: String, skipped: &mut Vec<Skipped>| {
        let from = skip_space(text, from, to);
        if from >= to {
            return;
        }
        skipped.push(Skipped {
            line: line_of(from),
            what,
        });
        for byte in &mut out[from..to] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    for definition in definitions {
        let Some(name) = wanted_name(text, definition, wanted) else {
            let first = words(&text[definition.header.0..definition.header.1])
                .into_iter()
                .take_while(|word| *word != "{" && *word != ";")
                .collect::<Vec<_>>()
                .join(" ");
            blank(
                definition.start,
                definition.end,
                format!("definition `{first}`"),
                skipped,
            );
            continue;
        };
        let keep = &wanted[name];
        if let Some((from, to)) = definition.attributes {
            let head = words(&text[definition.header.0..definition.header.1])
                .into_iter()
                .take_while(|word| *word != "{" && *word != ":")
                .collect::<Vec<_>>()
                .join(" ");
            blank(
                from,
                to,
                format!("extended attributes of `{head}`"),
                skipped,
            );
        }
        for &(from, to) in &definition.members {
            match member_name(&text[from..to]) {
                Some(member) if keep.contains(&member) => {}
                Some(member) => blank(from, to, format!("member `{member}` of `{name}`"), skipped),
                None => blank(from, to, format!("an unnamed member of `{name}`"), skipped),
            }
        }
        if let Some((close, _)) = definition.close {
            let body_end = definition
                .members
                .last()
                .map_or(definition.header.1, |m| m.1);
            // What lies between the last member and `}`: comments and white space only.
            blank(
                body_end,
                close,
                format!("the end of `{name}`'s body"),
                skipped,
            );
        }
    }
    String::from_utf8(out).unwrap_or_default()
}
