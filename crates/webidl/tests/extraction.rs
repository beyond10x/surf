//! The selective extraction of the HTML Standard (`story:webidl-bindings`, decision 7), on a
//! small page in the form of the vendored one: only the blocks that define a wanted interface
//! are taken, only the members named are kept, every other part is listed as skipped with its
//! line, and a construct the reader refuses is refused only where it is selected.

use std::collections::{BTreeMap, BTreeSet};

use surf_webidl::{Model, Skipped, cut_idl_blocks, extract_selected, read};

const PAGE: &str = "<!DOCTYPE html><html><head><meta charset=utf-8></head><body>\n\
<pre><code class='idl'>[<c- g>Global</c->=<c- n>Window</c->, <c- g>Exposed</c->=<c- n>Window</c->]\n\
<c- b>interface</c-> <dfn>Window</dfn> : <a>EventTarget</a> {\n\
  [<c- g>LegacyUnforgeable</c->] <c- b>readonly</c-> <c- b>attribute</c-> <a>WindowProxy</a> <a>window</a>;\n\
  <a>Promise</a>&lt;<c- b>undefined</c->&gt; <a>later</a>();\n\
};\n\
<a>Window</a> <c- b>includes</c-> <a>Other</a>;</code></pre>\n\
<pre class=extract><code class='idl'><c- b>partial</c-> <c- b>interface</c-> <a>Window</a> { <c- b>readonly</c-> <c- b>attribute</c-> <c- b>long</c-> <a>fake</a>; };</code></pre>\n\
<p>Text between the blocks.</p>\n\
<pre><code class='idl'><c- b>interface</c-> <dfn>Other</dfn> { };</code></pre>\n\
<pre><code data-var-scope class='idl'><c- b>partial</c-> <c- b>interface</c-> <a>Document</a> {\n\
  [<c- g>CEReactions</c->] <c- b>attribute</c-> <c- b>DOMString</c-> <a>title</a>;\n\
  // a comment\n\
  <c- b>attribute</c-> <c- b>DOMString</c-> <a>dir</a>;\n\
};</code></pre>\n\
</body></html>\n";

fn wanted(window: &[&str], document: &[&str]) -> BTreeMap<String, BTreeSet<String>> {
    BTreeMap::from([
        (
            "Window".to_string(),
            window.iter().map(|name| name.to_string()).collect(),
        ),
        (
            "Document".to_string(),
            document.iter().map(|name| name.to_string()).collect(),
        ),
    ])
}

fn skipped(line: usize, what: &str) -> Skipped {
    Skipped {
        line,
        what: what.to_string(),
    }
}

#[test]
fn only_the_selected_members_are_read() {
    let extracted = extract_selected(
        PAGE.as_bytes(),
        "page.html",
        &wanted(&["window"], &["title"]),
    )
    .expect("extracts");
    assert_eq!(
        (extracted.blocks, extracted.taken),
        (3, 2),
        "the `extract` block is no block"
    );
    assert_eq!(
        extracted.skipped,
        [
            skipped(1, "extended attributes of `interface Window`"),
            skipped(4, "member `later` of `Window`"),
            skipped(6, "definition `Window includes Other`"),
            skipped(10, "member `dir` of `Document`"),
        ]
    );
    let kept: Vec<&str> = extracted
        .text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(
        kept,
        [
            "interface Window : EventTarget {",
            "[LegacyUnforgeable] readonly attribute WindowProxy window;",
            "};",
            "partial interface Document {",
            "[CEReactions] attribute DOMString title;",
            "// a comment",
            "};",
        ]
    );
    let mut definitions = read(
        "interface EventTarget {};\ninterface Document {};\n",
        "dom.idl",
    )
    .expect("reads");
    definitions.extend(read(&extracted.text, "page.html").expect("the selected text reads"));
    let model = Model::new(definitions).expect("model");
    let names: Vec<Option<String>> = model
        .members("Window")
        .iter()
        .map(|member| member.name.clone())
        .collect();
    assert_eq!(names, [Some("window".to_string())]);
    let title = model.members("Document");
    assert_eq!(title.len(), 1);
    assert_eq!(title[0].line, 8, "a line of the extracted text");
}

#[test]
fn a_refused_construct_is_refused_only_where_selected() {
    let extracted = extract_selected(
        PAGE.as_bytes(),
        "page.html",
        &wanted(&["window", "later"], &["title"]),
    )
    .expect("extracts");
    let error = read(&extracted.text, "page.html").expect_err("Promise is refused");
    assert_eq!(error.to_string(), "page.html:4: refused: `Promise<T>`");
}

#[test]
fn a_page_without_a_wanted_interface_gives_nothing() {
    let extracted =
        extract_selected(PAGE.as_bytes(), "page.html", &BTreeMap::new()).expect("extracts");
    assert_eq!((extracted.blocks, extracted.taken), (3, 0));
    assert_eq!(extracted.text, "");
    assert!(extracted.skipped.is_empty());
}

/// The blocks of [`PAGE`], as the cut holds them: each `pre` element byte for byte, each
/// followed by a line feed, the `extract` example left out.
fn page_blocks() -> String {
    let mut blocks = String::new();
    let mut rest = PAGE;
    while let Some(start) = rest.find("<pre") {
        let end = rest[start..].find("</pre>").expect("a closed pre") + start + "</pre>".len();
        let block = &rest[start..end];
        if !block.starts_with("<pre class=extract>") {
            blocks.push_str(block);
            blocks.push('\n');
        }
        rest = &rest[end..];
    }
    blocks
}

#[test]
fn the_cut_of_a_page_is_its_blocks() {
    let cut = cut_idl_blocks(PAGE.as_bytes());
    assert_eq!(String::from_utf8(cut).expect("UTF-8"), page_blocks());
    assert_eq!(page_blocks().matches("<pre").count(), 3);
}

#[test]
fn a_page_and_its_cut_give_the_same_model() {
    let cut = cut_idl_blocks(PAGE.as_bytes());
    let wanted = wanted(&["window"], &["title"]);
    let model = |bytes: &[u8]| {
        let extracted = extract_selected(bytes, "page.html", &wanted).expect("extracts");
        let mut definitions = read(
            "interface EventTarget {};\ninterface Document {};\n",
            "dom.idl",
        )
        .expect("reads");
        definitions.extend(read(&extracted.text, "page.html").expect("reads"));
        (Model::new(definitions).expect("model"), extracted)
    };
    let (from_page, page_read) = model(PAGE.as_bytes());
    let (from_cut, cut_read) = model(&cut);
    assert_eq!(from_page.definitions(), from_cut.definitions());
    assert_eq!(
        page_read, cut_read,
        "the same blocks, text and parts skipped"
    );
}
