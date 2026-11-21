//! The case tags of `docs/design/html-parser.md` item 8, applied in the note's order. Every case
//! gets exactly one.

/// A case's tag.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Tag {
    /// Tokenizer, the data state, no `&`.
    Markup,
    /// Tokenizer, another initial state, no `&`.
    Text,
    /// Tokenizer, an input holding `&`.
    Charref,
    /// Tree construction, a thematic file, none of the rules below.
    Body,
    /// Tree construction, a thematic file, a formatting element's tag.
    Formatting,
    /// Tree construction, a thematic file, a table or `select` tag.
    Tables,
    /// Tree construction, a thematic file, a `template` tag.
    Template,
    /// Tree construction, a thematic file, an `svg` or `math` tag or a fragment case.
    Foreign,
    /// Tree construction, one of the 30 mixed files.
    Mixed,
    /// Encoding.
    Encoding,
    /// Not run, with the reason.
    NotRun(&'static str),
}

impl Tag {
    /// The tag's name, as a `tag` line of `html-inventory` writes it.
    pub fn name(&self) -> &'static str {
        match self {
            Tag::Markup => "markup",
            Tag::Text => "text",
            Tag::Charref => "charref",
            Tag::Body => "body",
            Tag::Formatting => "formatting",
            Tag::Tables => "tables",
            Tag::Template => "template",
            Tag::Foreign => "foreign",
            Tag::Mixed => "mixed",
            Tag::Encoding => "encoding",
            Tag::NotRun(_) => "not-run",
        }
    }

    /// The reason a case is not run; `None` for a case that runs.
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            Tag::NotRun(reason) => Some(reason),
            _ => None,
        }
    }

    /// The key a `file` line counts it under: the name, or `not-run:<reason>`.
    pub fn inventory_key(&self) -> String {
        match self {
            Tag::NotRun(reason) => format!("not-run:{reason}"),
            other => other.name().to_string(),
        }
    }
}

/// The file name of a data path: what follows its last `/`.
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Rule 1: the reason a whole file is not run, or `None`.
pub fn not_run_file(path: &str) -> Option<&'static str> {
    let name = file_name(path);
    if path == "html5lib-tests/tokenizer/xmlViolation.test" {
        Some("xml-infoset")
    } else if path.starts_with("html5lib-tests/encoding/scripted/")
        || (path.starts_with("wpt/html/syntax/parsing/resources/") && name.starts_with("scripted_"))
    {
        Some("needs-script")
    } else if path == "html5lib-tests/encoding/chardet/test_big5.txt" {
        Some("autodetect")
    } else {
        None
    }
}

/// Rule 2: a tokenizer case's tag, from its input (after the `doubleEscaped` unescaping) and
/// whether its initial state is the data state.
pub fn tokenizer(input: &[u32], data_state: bool) -> Tag {
    if input.contains(&u32::from('&')) {
        Tag::Charref
    } else if data_state {
        Tag::Markup
    } else {
        Tag::Text
    }
}

/// Rule 4: whether a tree-construction file is one of the 30 mixed ones: `tests<n>.dat`,
/// `webkit01.dat`, `webkit02.dat`, `tricky01.dat`, `html5test-com.dat`,
/// `pending-spec-changes.dat`.
pub fn is_mixed(path: &str) -> bool {
    let name = file_name(path);
    let numbered = name
        .strip_prefix("tests")
        .and_then(|rest| rest.strip_suffix(".dat"))
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    numbered
        || matches!(
            name,
            "webkit01.dat"
                | "webkit02.dat"
                | "tricky01.dat"
                | "html5test-com.dat"
                | "pending-spec-changes.dat"
        )
}

/// Whether `input` holds a tag of one of `names`: `<x` or `</x` followed by tab, LF, FF, CR,
/// space, `/` or `>`, compared after lowering ASCII letters.
pub fn holds_tag(input: &str, names: &[&str]) -> bool {
    let lower = input.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    for (i, _) in bytes.iter().enumerate().filter(|(_, b)| **b == b'<') {
        let mut start = i + 1;
        if bytes.get(start) == Some(&b'/') {
            start += 1;
        }
        for name in names {
            let end = start + name.len();
            if bytes.get(start..end) == Some(name.as_bytes())
                && matches!(
                    bytes.get(end),
                    Some(b'\t' | b'\n' | 0x0C | b'\r' | b' ' | b'/' | b'>')
                )
            {
                return true;
            }
        }
    }
    false
}

/// Rule 5: a case of a thematic tree-construction file, from its input and whether it has a
/// `#document-fragment` section. The last rule that applies wins.
pub fn thematic(input: &str, fragment: bool) -> Tag {
    let mut tag = Tag::Body;
    if holds_tag(
        input,
        &[
            "a", "b", "big", "code", "em", "font", "i", "nobr", "s", "small", "strike", "strong",
            "tt", "u",
        ],
    ) {
        tag = Tag::Formatting;
    }
    if holds_tag(
        input,
        &[
            "table", "caption", "colgroup", "col", "tbody", "thead", "tfoot", "tr", "td", "th",
            "select",
        ],
    ) {
        tag = Tag::Tables;
    }
    if holds_tag(input, &["template"]) {
        tag = Tag::Template;
    }
    if holds_tag(input, &["svg", "math"]) || fragment {
        tag = Tag::Foreign;
    }
    tag
}
