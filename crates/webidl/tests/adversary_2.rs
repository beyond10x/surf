//! Review of `story:webidl-bindings`: the generator and the cut of the HTML Standard with
//! inputs written here (a selection, an IDL fragment, a page of a few lines).

use std::collections::{BTreeMap, BTreeSet};

use surf_webidl::{Model, Selection, cut_idl_blocks, extract_selected, generate, read};

fn model(idl: &str) -> Model {
    Model::new(read(idl, "small.idl").expect("read")).expect("model")
}

const SMALL: &str = "interface EventTarget {};\n\
                     interface Node : EventTarget {\n\
                     \x20 readonly attribute unsigned short nodeType;\n\
                     \x20 undefined foo();\n\
                     };\n\
                     interface DOMException { readonly attribute DOMString name; };\n";

/// Red. A note's words go into a `///` line of `bindings.rs` as written (`generate.rs:1043`
/// for `reason:`, `:1607` for `deviation:`). `Selection::read` keeps a carriage return inside a
/// line, so the generated file holds a bare CR in a doc comment, which rustc refuses ("bare CR
/// not allowed in doc-comment"): the selection makes the generator write code that does not
/// compile.
#[test]
fn a_note_with_a_carriage_return_writes_no_bare_cr() {
    let selection = Selection::read(
        "definitions: EventTarget Node DOMException\nglobal: Node\n\
         Node nodeType reading deviation: a\rb\nNode foo throws reason: c\rd\n",
        "s.txt",
    )
    .expect("the reader takes the rows");
    let text = generate(&model(SMALL), &selection, &[]).expect("generates");
    let lines: Vec<&str> = text.lines().filter(|line| line.contains('\r')).collect();
    assert!(lines.is_empty(), "lines with a bare CR: {lines:?}");
}

/// Red. Web IDL's identifier (`/_?[A-Za-z][0-9A-Z_a-z-]*/`) allows `-`, and the reader takes it;
/// an argument's name becomes a Rust parameter name as `snake` writes it, and `identifier`
/// (`generate.rs:317`) makes a keyword raw but knows neither `-` nor `self`, which no raw form
/// allows. The generated trait method then does not compile.
#[test]
fn names_of_the_model_are_written_as_rust_identifiers() {
    let idl = "interface EventTarget {};\n\
               interface Node : EventTarget {\n\
               \x20 undefined foo(DOMString my-arg);\n\
               \x20 undefined bar(DOMString self);\n\
               };\n\
               interface DOMException { readonly attribute DOMString name; };\n";
    let selection = Selection::read(
        "definitions: EventTarget Node DOMException\nglobal: Node\n\
         Node foo mutation\nNode bar mutation\n",
        "s.txt",
    )
    .expect("reads");
    let text = generate(&model(idl), &selection, &[]).expect("generates");
    let bad: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("my-arg: String") || line.contains(" self: String"))
        .collect();
    assert!(
        bad.is_empty(),
        "parameters that are no Rust identifier: {bad:?}"
    );
}

fn wanted() -> BTreeMap<String, BTreeSet<String>> {
    let set = |names: &[&str]| names.iter().map(|name| name.to_string()).collect();
    BTreeMap::from([
        ("Window".to_string(), set(&["a", "b", "d", "f"])),
        ("Document".to_string(), set(&["c", "e", "g"])),
    ])
}

/// Held. A page of hostile blocks and its cut give the same blocks, text and skipped parts:
/// the extraction and the cut share one scan (`idl_blocks`). A `>` in a quoted attribute,
/// `PRE` in capitals, a block in a comment, `idl` beside another class and an unclosed last
/// block; a `pre` inside a `pre` is refused from the page and from its cut alike.
#[test]
fn a_hostile_page_and_its_cut_read_the_same() {
    let page = "<p>x</p>\n\
        <pre class=\"idl\">interface Window { readonly attribute boolean a; };</pre>\n\
        <pre title=\"a>b\" class=idl>interface Window { readonly attribute boolean b; };</pre>\n\
        <PRE class=idl>partial interface Document { readonly attribute boolean c; };</PRE>\n\
        <!-- <pre class=idl>partial interface Window { readonly attribute boolean d; };</pre> -->\n\
        <pre class=\"example idl\">partial interface Document { readonly attribute boolean e; };</pre>\n\
        <pre class=idl>partial interface Document { readonly attribute boolean g; };\n";
    let from_page = extract_selected(page.as_bytes(), "page.html", &wanted()).expect("page");
    let cut = cut_idl_blocks(page.as_bytes());
    let from_cut = extract_selected(&cut, "page.html", &wanted()).expect("cut");
    assert_eq!(from_page, from_cut);
    let nested = "<pre class=idl><pre class=idl>partial interface Window \
                  { readonly attribute boolean f; };</pre></pre>\n";
    let cut = cut_idl_blocks(nested.as_bytes());
    assert!(
        extract_selected(nested.as_bytes(), "page.html", &wanted()).is_err()
            && extract_selected(&cut, "page.html", &wanted()).is_err(),
        "a pre inside a pre"
    );
}

/// Red (a page the pinned snapshot is not: its sha256 is checked by `webidl-html-cut`).
/// `html.rs` says a block is a `pre` element of the page; the scan reads bytes, not HTML: a
/// `pre class=idl` inside a comment is taken, and a `pre` with a `>` in a quoted attribute
/// before its class is missed.
#[test]
fn the_blocks_are_the_pre_elements_of_the_page() {
    let page = "<!-- <pre class=idl>partial interface Window { readonly attribute boolean d; };</pre> -->\n\
        <pre title=\"a>b\" class=idl>interface Window { readonly attribute boolean b; };</pre>\n";
    let idl = extract_selected(page.as_bytes(), "page.html", &wanted()).expect("page");
    assert!(
        idl.blocks == 1 && idl.text.contains("boolean b") && !idl.text.contains("boolean d"),
        "blocks {}, text {:?}",
        idl.blocks,
        idl.text
    );
}
