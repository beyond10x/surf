//! Cases a reviewer added: each holds a line of the reader that the other modules leave free,
//! with an IDL text or a page of a few lines written here.

use std::collections::BTreeMap;

use surf_webidl::{
    ConstValue, Definition, DefinitionKind, MemberKind, Model, TypeKind, read, read_page,
};

const FILE: &str = "test.idl";

fn all(text: &str) -> Vec<Definition> {
    read(text, FILE).unwrap_or_else(|error| panic!("{error}"))
}

fn model(text: &str) -> Model {
    Model::new(all(text)).unwrap_or_else(|error| panic!("{error}"))
}

/// The kind of the one member of the one interface of `text`.
fn only_member(text: &str) -> MemberKind {
    let definitions = all(text);
    assert_eq!(definitions.len(), 1, "{definitions:?}");
    match &definitions[0].kind {
        DefinitionKind::Interface { members, .. } => {
            assert_eq!(members.len(), 1, "{members:?}");
            members[0].kind.clone()
        }
        other => panic!("not an interface: {other:?}"),
    }
}

fn const_value(text: &str) -> ConstValue {
    match only_member(text) {
        MemberKind::Const { value, .. } => value,
        other => panic!("not a const: {other:?}"),
    }
}

/// (name, line) of each definition of `text`.
fn names_and_lines(definitions: &[Definition]) -> Vec<(&str, usize)> {
    definitions
        .iter()
        .map(|definition| (definition.name.as_str(), definition.line))
        .collect()
}

fn unresolved_names(model: &Model) -> Vec<String> {
    model
        .unresolved()
        .into_iter()
        .map(|unresolved| unresolved.name)
        .collect()
}

#[test]
fn octal_integer_constant() {
    assert_eq!(
        const_value("interface A {\n  const unsigned short X = 010;\n};"),
        ConstValue::Integer(8)
    );
}

#[test]
fn negative_integer_constant() {
    assert_eq!(
        const_value("interface A {\n  const short X = -1;\n};"),
        ConstValue::Integer(-1)
    );
}

#[test]
fn true_constant() {
    assert_eq!(
        const_value("interface A {\n  const boolean X = true;\n};"),
        ConstValue::Boolean(true)
    );
}

#[test]
fn unrestricted_double_attribute() {
    match only_member("interface A {\n  readonly attribute unrestricted double n;\n};") {
        MemberKind::Attribute { ty, .. } => {
            assert_eq!(ty.kind, TypeKind::Double { unrestricted: true });
        }
        other => panic!("not an attribute: {other:?}"),
    }
}

#[test]
fn line_after_a_comment_of_several_lines() {
    let definitions = all("/* one\n   two\n*/\ninterface A {};\n");
    assert_eq!(names_and_lines(&definitions), [("A", 4)]);
}

#[test]
fn line_after_a_string_of_several_lines() {
    let definitions = all("enum E { \"a\nb\" };\ninterface A {};\n");
    assert_eq!(names_and_lines(&definitions), [("E", 1), ("A", 3)]);
}

#[test]
fn extraction_stops_at_the_next_h2() {
    let page = "<!DOCTYPE html><title>t</title>\
        <h2 id=\"idl-index\">IDL Index</h2><pre>interface A {};</pre>\
        <h2 id=\"next\">Next</h2><pre>interface B {};</pre>";
    let extracted =
        read_page(page.as_bytes(), "page.html").unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(extracted.index, "interface A {};\n");
    assert_eq!(names_and_lines(&extracted.definitions), [("A", 1)]);
}

#[test]
fn extraction_puts_each_pre_on_lines_of_its_own() {
    let page = "<!DOCTYPE html><title>t</title>\
        <h2 id=\"idl-index\">IDL Index</h2><pre>interface A {};</pre>\
        <div><pre>interface B {};</pre></div>";
    let extracted =
        read_page(page.as_bytes(), "page.html").unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(extracted.index, "interface A {};\ninterface B {};\n");
    assert_eq!(
        names_and_lines(&extracted.definitions),
        [("A", 1), ("B", 2)]
    );
}

#[test]
fn unresolved_inheritance() {
    let model = model("interface A : Missing {};\n");
    assert_eq!(unresolved_names(&model), ["Missing"]);
}

#[test]
fn a_partial_interface_defines_no_name() {
    let model = model("partial interface W {};\ninterface A {\n  attribute W w;\n};\n");
    // Since round 5 the partial's own name is listed too: W at line 1 and at line 3.
    assert_eq!(unresolved_names(&model), ["W", "W"]);
}

#[test]
fn inside_a_sequence_names_and_extended_attributes_are_seen() {
    let model = model("typedef sequence<[AllowShared] Missing> S;\n");
    assert_eq!(unresolved_names(&model), ["Missing"]);
    assert_eq!(
        model.extended_attribute_counts(FILE),
        BTreeMap::from([("AllowShared".to_string(), 1)])
    );
}
