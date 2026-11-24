//! What the reader refuses, each with `<file>:<line>: ...` naming the construct and the line it
//! starts on.

use std::path::Path;

use surf_webidl::{EXTENDED_ATTRIBUTES, extract_index, read};

const FILE: &str = "test.idl";

/// The error `text` fails with, as printed.
fn refused(text: &str) -> String {
    match read(text, FILE) {
        Ok(definitions) => panic!("read: {definitions:?}"),
        Err(error) => error.to_string(),
    }
}

/// `text` fails with exactly `<file>:<line>: refused: <construct>`.
fn assert_refused(text: &str, line: usize, construct: &str) {
    assert_eq!(
        refused(text),
        format!("{FILE}:{line}: refused: {construct}")
    );
}

#[test]
fn promise_type() {
    assert_refused(
        "interface A {\n  undefined f();\n  Promise<undefined> ready();\n};",
        3,
        "`Promise<T>`",
    );
}

#[test]
fn overload() {
    assert_refused(
        "interface A {\n  undefined f(DOMString a);\n\n  undefined f(Node a);\n};",
        4,
        "an overload: a second operation `f` in `A`",
    );
}

#[test]
fn namespace() {
    assert_refused(
        "\n\nnamespace console {\n  undefined log();\n};",
        3,
        "`namespace`",
    );
}

#[test]
fn maplike() {
    assert_refused(
        "interface A {\n  readonly maplike<DOMString, DOMString>;\n};",
        2,
        "`maplike`",
    );
}

#[test]
fn setlike() {
    assert_refused("interface A {\n  setlike<DOMString>;\n};", 2, "`setlike`");
}

#[test]
fn async_iterable() {
    assert_refused(
        "interface A {\n  async_iterable<DOMString>;\n};",
        2,
        "`async iterable`",
    );
    assert_refused(
        "interface A {\n  async iterable<DOMString>;\n};",
        2,
        "`async iterable`",
    );
}

#[test]
fn legacycaller() {
    assert_refused(
        "interface A {\n  legacycaller any item(unsigned long index);\n};",
        2,
        "`legacycaller`",
    );
}

#[test]
fn unknown_extended_attribute() {
    assert!(!EXTENDED_ATTRIBUTES.contains(&"LegacyWindowAlias"));
    assert_refused(
        "[Exposed=Window,\n LegacyWindowAlias=webkitURL]\ninterface URL {};",
        2,
        "the extended attribute `[LegacyWindowAlias]`",
    );
}

#[test]
fn newer_index() {
    let page = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/whatwg/dom.html");
    let bytes = std::fs::read(page).expect("read dom.html");
    let index = extract_index(&bytes, "dom.html").unwrap_or_else(|error| panic!("{error}"));
    let lines: Vec<&str> = index.lines().collect();
    let node = lines
        .iter()
        .position(|line| *line == "interface Node : EventTarget {")
        .expect("interface Node");
    let mut newer: Vec<&str> = lines.clone();
    newer.insert(node + 1, "  Promise<undefined> x();");
    let inserted = node + 2;
    match read(&newer.join("\n"), "dom.html") {
        Ok(_) => panic!("the newer index was read"),
        Err(error) => {
            assert_eq!(
                error.to_string(),
                format!("dom.html:{inserted}: refused: `Promise<T>`")
            );
            assert_eq!(error.line, inserted);
        }
    }
}
