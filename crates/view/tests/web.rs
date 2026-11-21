//! Cases 9 and 10 of `story:view-page-prototype`: what the page's files under
//! `crates/view/web/` may hold, read as text. The server's content security policy
//! (`docs/view-stream.md`) forbids inline script and style; these cases keep the files inside it
//! and keep every value out of markup.

#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;

fn web() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web")
}

/// Every file under `crates/view/web/`, by name, with its text.
fn files() -> Vec<(String, String)> {
    let mut files = Vec::new();
    for entry in fs::read_dir(web()).expect("read crates/view/web") {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        assert!(path.is_file(), "{} is not a file", path.display());
        let name = entry.file_name().into_string().expect("a UTF-8 file name");
        let text = fs::read_to_string(&path).expect("a UTF-8 file");
        files.push((name, text));
    }
    files.sort();
    assert!(!files.is_empty(), "crates/view/web holds no file");
    files
}

/// One start tag: its lower-case name and its attributes, each a lower-case name and its value
/// (`None` when it has none).
struct Tag {
    name: String,
    attributes: Vec<(String, Option<String>)>,
}

/// The start tags of `html`, skipping comments, end tags and the doctype.
fn tags(html: &str) -> Vec<Tag> {
    let bytes = html.as_bytes();
    let mut tags = Vec::new();
    let mut at = 0;
    while let Some(offset) = html[at..].find('<') {
        let start = at + offset + 1;
        if html[start..].starts_with("!--") {
            let end = html[start..].find("-->").expect("a closed comment");
            at = start + end + 3;
            continue;
        }
        if !bytes.get(start).is_some_and(u8::is_ascii_alphabetic) {
            at = start;
            continue;
        }
        let mut i = start;
        while bytes.get(i).is_some_and(|b| b.is_ascii_alphanumeric()) {
            i += 1;
        }
        let name = html[start..i].to_ascii_lowercase();
        let mut attributes = Vec::new();
        loop {
            while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
                i += 1;
            }
            match bytes.get(i) {
                None => panic!("an unclosed tag <{name}"),
                Some(b'>') => {
                    i += 1;
                    break;
                }
                Some(b'/') => {
                    i += 1;
                    continue;
                }
                Some(_) => {}
            }
            let name_start = i;
            while bytes
                .get(i)
                .is_some_and(|&b| !b.is_ascii_whitespace() && b != b'=' && b != b'>' && b != b'/')
            {
                i += 1;
            }
            let attribute = html[name_start..i].to_ascii_lowercase();
            while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
                i += 1;
            }
            if bytes.get(i) != Some(&b'=') {
                attributes.push((attribute, None));
                continue;
            }
            i += 1;
            while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
                i += 1;
            }
            let value = match bytes.get(i) {
                Some(&quote @ (b'"' | b'\'')) => {
                    let value_start = i + 1;
                    let end = html[value_start..]
                        .find(quote as char)
                        .expect("a closed attribute value");
                    i = value_start + end + 1;
                    html[value_start..value_start + end].to_string()
                }
                _ => {
                    let value_start = i;
                    while bytes
                        .get(i)
                        .is_some_and(|&b| !b.is_ascii_whitespace() && b != b'>')
                    {
                        i += 1;
                    }
                    html[value_start..i].to_string()
                }
            };
            attributes.push((attribute, Some(value)));
        }
        tags.push(Tag { name, attributes });
        at = i;
    }
    tags
}

#[test]
fn case_09_index_has_one_module_script_and_no_inline_style_or_event_attribute() {
    let index = fs::read_to_string(web().join("index.html")).expect("read index.html");
    let tags = tags(&index);
    assert!(
        tags.iter().any(|tag| tag.name == "main"),
        "index.html has no main element"
    );

    let scripts: Vec<&Tag> = tags.iter().filter(|tag| tag.name == "script").collect();
    assert_eq!(
        scripts.len(),
        1,
        "index.html holds exactly one script element"
    );
    let script = scripts[0];
    let value = |name: &str| {
        script
            .attributes
            .iter()
            .find(|(attribute, _)| attribute == name)
            .and_then(|(_, value)| value.clone())
    };
    assert_eq!(
        value("type").as_deref(),
        Some("module"),
        "the script's type"
    );
    assert!(
        value("src").is_some_and(|src| !src.is_empty()),
        "the script has a src"
    );
    assert_eq!(
        index.to_ascii_lowercase().matches("<script").count(),
        1,
        "the text of index.html opens one script element"
    );

    assert!(
        !tags.iter().any(|tag| tag.name == "style"),
        "index.html holds a style element"
    );
    assert!(
        !index.to_ascii_lowercase().contains("<style"),
        "index.html opens a style element"
    );
    for tag in &tags {
        for (attribute, _) in &tag.attributes {
            assert_ne!(attribute, "style", "<{}> has a style attribute", tag.name);
            assert!(
                !attribute.starts_with("on"),
                "<{}> has the attribute {attribute}",
                tag.name
            );
        }
    }
    assert!(
        !index.to_ascii_lowercase().contains("style="),
        "index.html holds style="
    );
}

#[test]
fn case_10_no_page_file_writes_markup_runs_text_or_names_another_origin() {
    const FORBIDDEN: [&str; 11] = [
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
        "document.write",
        "eval(",
        "new Function",
        "setAttribute(\"on",
        "javascript:",
        "http://",
        "https://",
        "setAttribute('on",
    ];
    let files = files();
    for (name, text) in &files {
        for forbidden in FORBIDDEN {
            assert!(!text.contains(forbidden), "{name} contains {forbidden}");
        }
        for start in ["=\"//", "='//", "=//"] {
            let squeezed: String = text.split_whitespace().collect();
            assert!(
                !squeezed.contains(start),
                "{name} has an attribute value starting with //"
            );
        }
        if name.ends_with(".html") {
            for tag in tags(text) {
                for (attribute, value) in tag.attributes {
                    assert!(
                        !value.is_some_and(|value| value.trim_start().starts_with("//")),
                        "{name}: <{}> {attribute} starts with //",
                        tag.name
                    );
                }
            }
        }
    }
}
