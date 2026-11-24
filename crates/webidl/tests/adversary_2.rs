//! Review of `story:webidl-bindings`: the generator and the cut of the HTML Standard with
//! inputs written here (a selection, an IDL fragment, a page of a few lines).

use std::collections::{BTreeMap, BTreeSet};

use surf_webidl::{cut_idl_blocks, extract_selected};

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
