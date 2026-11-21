//! The `| ` tree format of the tree-construction data (`html/syntax/parsing/resources/
//! README.md` of web-platform-tests): a reader of the expected trees, a writer, and the
//! conversion of a `surf_html` tree into it. It is the only serialization the epic allows.
//!
//! One node per line: `| `, two spaces per ancestor, then the node. An element is
//! `<tag name string>`, its attributes on the lines after it one level deeper, sorted by their
//! attribute name string compared by UTF-16 code unit, then for a template `content` one level
//! deeper with the template contents below it, then its children. Text, comments and attribute
//! values are not escaped: a newline in them continues the node on the next line, which does
//! not start with `| `.

use std::fmt;

use surf_html::{Namespace, NodeData, NodeId, Tree};

/// A node of the format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DumpNode {
    /// `<!DOCTYPE name>`, or `<!DOCTYPE name "public" "system">` when either identifier is not
    /// empty.
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
        /// The tag name string: the local name, prefixed with `svg ` or `math ` for those
        /// namespaces.
        name: String,
        /// The attributes: attribute name string (prefixed with `xlink `, `xml ` or `xmlns `
        /// for those namespaces) and value. Written sorted, whatever the order here.
        attributes: Vec<(String, String)>,
        /// A template's contents, written under a `content` line; `None` for other elements.
        content: Option<Vec<DumpNode>>,
        /// The children.
        children: Vec<DumpNode>,
    },
    /// `"text"`.
    Text(String),
    /// `<!-- data -->`.
    Comment(String),
    /// `<?target data?>`.
    ProcessingInstruction {
        /// The target.
        target: String,
        /// The data.
        data: String,
    },
}

/// Why a tree could not be read, with the 1-based line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// The line.
    pub line: usize,
    /// What was wrong.
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tree line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for Error {}

/// Writes nodes in the format: lines joined by LF, no final LF, the empty string for no node.
pub fn write(nodes: &[DumpNode]) -> String {
    let mut lines = Vec::new();
    write_nodes(nodes, 0, &mut lines);
    lines.join("\n")
}

fn line(depth: usize, text: &str) -> String {
    format!("| {}{text}", "  ".repeat(depth))
}

fn write_nodes(nodes: &[DumpNode], depth: usize, out: &mut Vec<String>) {
    for node in nodes {
        match node {
            DumpNode::Doctype {
                name,
                public_id,
                system_id,
            } => {
                if public_id.is_empty() && system_id.is_empty() {
                    out.push(line(depth, &format!("<!DOCTYPE {name}>")));
                } else {
                    out.push(line(
                        depth,
                        &format!("<!DOCTYPE {name} \"{public_id}\" \"{system_id}\">"),
                    ));
                }
            }
            DumpNode::Element {
                name,
                attributes,
                content,
                children,
            } => {
                out.push(line(depth, &format!("<{name}>")));
                let mut sorted: Vec<&(String, String)> = attributes.iter().collect();
                sorted.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
                for (attribute, value) in sorted {
                    out.push(line(depth + 1, &format!("{attribute}=\"{value}\"")));
                }
                if let Some(content) = content {
                    out.push(line(depth + 1, "content"));
                    write_nodes(content, depth + 2, out);
                }
                write_nodes(children, depth + 1, out);
            }
            DumpNode::Text(text) => out.push(line(depth, &format!("\"{text}\""))),
            DumpNode::Comment(data) => out.push(line(depth, &format!("<!-- {data} -->"))),
            DumpNode::ProcessingInstruction { target, data } => {
                out.push(line(depth, &format!("<?{target} {data}?>")))
            }
        }
    }
}

/// One node's text: its depth, the text after the indentation (continuation lines joined by
/// LF), and its first line's number.
struct Item {
    depth: usize,
    text: String,
    line: usize,
}

/// Reads a tree written in the format. The empty string is no node.
pub fn read(text: &str) -> Result<Vec<DumpNode>, Error> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut items: Vec<Item> = Vec::new();
    for (i, raw) in text.split('\n').enumerate() {
        if let Some(rest) = raw.strip_prefix("| ") {
            let spaces = rest.len() - rest.trim_start_matches(' ').len();
            if spaces % 2 != 0 {
                return Err(Error {
                    line: i + 1,
                    message: "an odd indentation".to_string(),
                });
            }
            items.push(Item {
                depth: spaces / 2,
                text: rest[spaces..].to_string(),
                line: i + 1,
            });
        } else if let Some(item) = items.last_mut() {
            item.text.push('\n');
            item.text.push_str(raw);
        } else {
            return Err(Error {
                line: i + 1,
                message: "the first line does not start with `| `".to_string(),
            });
        }
    }
    let mut pos = 0;
    let nodes = read_nodes(&items, &mut pos, 0)?;
    if let Some(item) = items.get(pos) {
        return Err(Error {
            line: item.line,
            message: "deeper than its parent allows".to_string(),
        });
    }
    Ok(nodes)
}

/// Whether a line one level under an element, before its children, is an attribute: it ends
/// with `"`, holds `="` after its first character, and is not text (which starts with `"`).
/// Every other node ends with `>` or is `content`. An attribute may itself start with `<`
/// (`<=""`, in `webkit01.dat`).
fn is_attribute(text: &str) -> bool {
    !text.starts_with('"')
        && text.ends_with('"')
        && text.get(1..).is_some_and(|t| t.contains("=\""))
}

fn read_nodes(items: &[Item], pos: &mut usize, depth: usize) -> Result<Vec<DumpNode>, Error> {
    let mut nodes = Vec::new();
    while let Some(item) = items.get(*pos) {
        if item.depth < depth {
            break;
        }
        if item.depth > depth {
            return Err(Error {
                line: item.line,
                message: "deeper than its parent allows".to_string(),
            });
        }
        *pos += 1;
        let node = read_node(item)?;
        let node = match node {
            DumpNode::Element { name, .. } => {
                let mut attributes = Vec::new();
                while let Some(next) = items.get(*pos)
                    && next.depth == depth + 1
                    && is_attribute(&next.text)
                {
                    attributes.push(read_attribute(next)?);
                    *pos += 1;
                }
                let mut content = None;
                if let Some(next) = items.get(*pos)
                    && next.depth == depth + 1
                    && next.text == "content"
                {
                    *pos += 1;
                    content = Some(read_nodes(items, pos, depth + 2)?);
                }
                let children = read_nodes(items, pos, depth + 1)?;
                DumpNode::Element {
                    name,
                    attributes,
                    content,
                    children,
                }
            }
            other => {
                if let Some(next) = items.get(*pos)
                    && next.depth > depth
                {
                    return Err(Error {
                        line: next.line,
                        message: "a child of a node that is not an element".to_string(),
                    });
                }
                other
            }
        };
        nodes.push(node);
    }
    Ok(nodes)
}

fn error(item: &Item, message: &str) -> Error {
    Error {
        line: item.line,
        message: format!("{message}: `{}`", item.text),
    }
}

fn read_node(item: &Item) -> Result<DumpNode, Error> {
    let text = item.text.as_str();
    if let Some(rest) = text.strip_prefix("<!DOCTYPE ") {
        let rest = rest
            .strip_suffix('>')
            .ok_or_else(|| error(item, "a doctype without `>`"))?;
        return Ok(match rest.split_once(" \"") {
            None => DumpNode::Doctype {
                name: rest.to_string(),
                public_id: String::new(),
                system_id: String::new(),
            },
            Some((name, ids)) => {
                let (public_id, system_id) = ids
                    .strip_suffix('"')
                    .and_then(|ids| ids.split_once("\" \""))
                    .ok_or_else(|| {
                        error(item, "doctype identifiers not `\"public\" \"system\"`")
                    })?;
                DumpNode::Doctype {
                    name: name.to_string(),
                    public_id: public_id.to_string(),
                    system_id: system_id.to_string(),
                }
            }
        });
    }
    if let Some(rest) = text.strip_prefix("<!-- ") {
        let data = rest
            .strip_suffix(" -->")
            .ok_or_else(|| error(item, "a comment without ` -->`"))?;
        return Ok(DumpNode::Comment(data.to_string()));
    }
    if let Some(rest) = text.strip_prefix("<?") {
        let (target, data) = rest
            .strip_suffix("?>")
            .and_then(|inner| inner.split_once(' '))
            .ok_or_else(|| error(item, "a processing instruction not `<?target data?>`"))?;
        return Ok(DumpNode::ProcessingInstruction {
            target: target.to_string(),
            data: data.to_string(),
        });
    }
    if let Some(rest) = text.strip_prefix('<') {
        let name = rest
            .strip_suffix('>')
            .filter(|name| !name.is_empty())
            .ok_or_else(|| error(item, "an element not `<name>`"))?;
        return Ok(DumpNode::Element {
            name: name.to_string(),
            attributes: Vec::new(),
            content: None,
            children: Vec::new(),
        });
    }
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        return Ok(DumpNode::Text(text[1..text.len() - 1].to_string()));
    }
    Err(error(item, "not a node"))
}

fn read_attribute(item: &Item) -> Result<(String, String), Error> {
    let text = item.text.as_str();
    // An attribute name holds `=` only as its first character (the tokenizer's
    // unexpected-equals-sign-before-attribute-name), so the separator is the first `="` after it.
    let at = text
        .get(1..)
        .and_then(|rest| rest.find("=\""))
        .map(|i| i + 1)
        .ok_or_else(|| error(item, "an attribute not `name=\"value\"`"))?;
    let value = text[at + 2..]
        .strip_suffix('"')
        .ok_or_else(|| error(item, "an attribute value without its closing `\"`"))?;
    Ok((text[..at].to_string(), value.to_string()))
}

/// The designator a namespace gets in a tag or attribute name string.
fn designator(namespace: Namespace) -> &'static str {
    match namespace {
        Namespace::Html => "",
        Namespace::Svg => "svg ",
        Namespace::MathMl => "math ",
        Namespace::XLink => "xlink ",
        Namespace::Xml => "xml ",
        Namespace::Xmlns => "xmlns ",
    }
}

/// The children of a `surf_html` tree's root in the format's nodes. A tree the format cannot
/// write is an error: a document or a document fragment anywhere but at the root or as a
/// template's contents, or template contents that are not a document fragment.
pub fn from_tree(tree: &Tree) -> Result<Vec<DumpNode>, String> {
    match tree.data(tree.root()) {
        NodeData::Document | NodeData::DocumentFragment => convert_children(tree, tree.root()),
        other => Err(format!(
            "the root is not a document or a fragment: {other:?}"
        )),
    }
}

fn convert_children(tree: &Tree, node: NodeId) -> Result<Vec<DumpNode>, String> {
    tree.children(node)
        .iter()
        .map(|&child| convert(tree, child))
        .collect()
}

fn convert(tree: &Tree, node: NodeId) -> Result<DumpNode, String> {
    Ok(match tree.data(node) {
        NodeData::Document | NodeData::DocumentFragment => {
            return Err("a document or a fragment as a child".to_string());
        }
        NodeData::Doctype {
            name,
            public_id,
            system_id,
        } => DumpNode::Doctype {
            name: name.clone(),
            public_id: public_id.clone(),
            system_id: system_id.clone(),
        },
        NodeData::Element {
            namespace,
            local_name,
            attributes,
            template_contents,
        } => DumpNode::Element {
            name: format!("{}{local_name}", designator(*namespace)),
            attributes: attributes
                .iter()
                .map(|a| {
                    (
                        format!("{}{}", a.namespace.map_or("", designator), a.local_name),
                        a.value.clone(),
                    )
                })
                .collect(),
            content: match template_contents {
                None => None,
                Some(contents) => match tree.data(*contents) {
                    NodeData::DocumentFragment => Some(convert_children(tree, *contents)?),
                    _ => return Err("template contents that are not a fragment".to_string()),
                },
            },
            children: convert_children(tree, node)?,
        },
        NodeData::Text(text) => DumpNode::Text(text.clone()),
        NodeData::Comment(data) => DumpNode::Comment(data.clone()),
        NodeData::ProcessingInstruction { target, data } => DumpNode::ProcessingInstruction {
            target: target.clone(),
            data: data.clone(),
        },
    })
}
