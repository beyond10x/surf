//! What the reader refuses, each with `<file>:<line>: ...` naming the construct and the line it
//! starts on.

use std::path::Path;

use surf_webidl::{EXTENDED_ATTRIBUTES, Model, extract_index, read};

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

/// The error the model of the texts, each read with its file name, fails with, as printed.
fn model_refused(texts: &[(&str, &str)]) -> String {
    let mut definitions = Vec::new();
    for (file, text) in texts {
        definitions.extend(read(text, file).unwrap_or_else(|error| panic!("{error}")));
    }
    match Model::new(definitions) {
        Ok(model) => panic!("a model: {model:?}"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn overload_across_partial() {
    assert_eq!(
        model_refused(&[
            ("a.idl", "interface A {\n  undefined f();\n};"),
            (
                "b.idl",
                "partial interface A {\n  readonly attribute any x;\n  undefined f(DOMString s);\n};"
            ),
        ]),
        "b.idl:3: refused: an overload: a second operation `f` in `A`"
    );
}

#[test]
fn overload_across_mixin() {
    assert_eq!(
        model_refused(&[
            (
                "a.idl",
                "interface A {\n  undefined g();\n};\nA includes M;"
            ),
            ("m.idl", "\ninterface mixin M {\n  undefined g(Node n);\n};"),
        ]),
        "m.idl:3: refused: an overload: a second operation `g` in `A`"
    );
}

/// The refusal of a type's extended attribute `[name]` written before what `what` names.
fn before(name: &str, what: &str) -> String {
    format!("the extended attribute `[{name}]`, which applies to a type, before {what}")
}

#[test]
fn type_extended_attribute_before_an_attribute() {
    assert_refused(
        "interface A {\n  [CEReactions,\n   EnforceRange] attribute unsigned long x;\n};",
        3,
        &before("EnforceRange", "an attribute"),
    );
}

#[test]
fn type_extended_attribute_before_an_operation() {
    assert_refused(
        "interface A {\n  [NewObject, EnforceRange] unsigned long f();\n};",
        2,
        &before("EnforceRange", "an operation"),
    );
}

#[test]
fn type_extended_attribute_before_a_const() {
    assert_refused(
        "interface A {\n  [EnforceRange] const unsigned long X = 1;\n};",
        2,
        &before("EnforceRange", "a constant"),
    );
}

#[test]
fn type_extended_attribute_before_a_definition() {
    assert_refused(
        "\n[LegacyNullToEmptyString]\ntypedef DOMString S;",
        2,
        &before("LegacyNullToEmptyString", "a definition"),
    );
}

#[test]
fn includes_of_what_is_no_mixin() {
    assert_eq!(
        model_refused(&[
            ("a.idl", "interface A {};\n\nA includes B;"),
            ("b.idl", "dictionary B {};"),
        ]),
        "a.idl:3: refused: `A includes B`, where `B` is no interface mixin"
    );
}

#[test]
fn includes_into_what_is_no_interface() {
    assert_eq!(
        model_refused(&[(
            "a.idl",
            "dictionary A {};\ninterface mixin B {};\nA\n  includes B;"
        )]),
        "a.idl:3: refused: `A includes B`, where `A` is no interface"
    );
}

#[test]
fn two_definitions_of_one_name() {
    assert_eq!(
        model_refused(&[
            ("a.idl", "interface A {};\npartial interface A {};"),
            ("b.idl", "\n\ndictionary A {};"),
        ]),
        "b.idl:3: refused: a second definition of `A`, the first at a.idl:1"
    );
    assert_eq!(
        model_refused(&[("a.idl", "enum E { \"a\" };\ntypedef DOMString E;")]),
        "a.idl:2: refused: a second definition of `E`, the first at a.idl:1"
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

#[test]
fn enum_empty_value_between_commas() {
    assert_refused(
        "enum E {\n  \"a\",\n  ,\n  \"b\"\n};",
        3,
        "`,` where a string was expected",
    );
}

#[test]
fn enum_without_values() {
    assert_refused("enum E {\n};", 2, "`}` where a string was expected");
}
