//! The bound on nested types: a named type alone is 1 deep, `sequence<DOMString>` and
//! `(A or B)` 2 deep; past 32 levels a type is refused, never read by deeper recursion. And the
//! definitions a model of many definitions looks up.

use std::time::Instant;

use surf_webidl::{Model, definition_visits, read};

const FILE: &str = "test.idl";

/// A typedef of a type `depth` levels deep: `sequence<` nested `depth - 1` times around
/// `DOMString`.
fn sequences(depth: usize) -> String {
    format!(
        "typedef {}DOMString{} T;\n",
        "sequence<".repeat(depth - 1),
        ">".repeat(depth - 1)
    )
}

/// A typedef of a union `depth` levels deep: `(((DOMString or A) or A) ...)`.
fn unions(depth: usize) -> String {
    format!(
        "typedef {}DOMString{} T;\n",
        "(".repeat(depth - 1),
        " or A)".repeat(depth - 1)
    )
}

fn refused(text: &str) -> String {
    match read(text, FILE) {
        Ok(_) => panic!("read a type nested past the bound"),
        Err(error) => error.to_string(),
    }
}

const REFUSED: &str = "test.idl:1: refused: a type that nests deeper than 32 levels";

#[test]
fn types_32_deep_are_read() {
    read(&sequences(32), FILE).unwrap_or_else(|error| panic!("{error}"));
    read(&unions(32), FILE).unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn types_33_deep_are_refused() {
    assert_eq!(refused(&sequences(33)), REFUSED);
    assert_eq!(refused(&unions(33)), REFUSED);
}

/// The definitions `Model::new` looks up for `interfaces` interfaces, each with one `includes`
/// of one mixin: the difference of [`definition_visits`] around it (the read of the text is
/// outside it), with the time it took, printed.
fn visits_of_a_model(interfaces: usize) -> usize {
    let mut text = String::new();
    for i in 0..interfaces {
        text.push_str(&format!("interface A{i} {{}};\nA{i} includes M;\n"));
    }
    text.push_str("interface mixin M {\n  undefined f();\n};\n");
    let definitions = read(&text, FILE).unwrap_or_else(|error| panic!("{error}"));
    let start = Instant::now();
    let before = definition_visits();
    let model = Model::new(definitions).unwrap_or_else(|error| panic!("{error}"));
    let visits = definition_visits() - before;
    println!(
        "{interfaces} interfaces: {visits} definitions visited, model {:?}",
        start.elapsed()
    );
    let last = format!("A{}", interfaces - 1);
    assert_eq!(model.counts(&last).from_mixins, 1);
    visits
}

/// `Model::new` finds each name through one map built once, not by a scan of every definition
/// per statement: the definitions it looks up grow with the statements. Twice the interfaces,
/// each with one `includes` of one mixin, visit at most 3 times the definitions; a scan per
/// statement would visit 4 times as many. A lookup that does not go through the counted
/// accessor is not seen.
#[test]
fn the_definitions_a_model_visits_double_when_its_interfaces_double() {
    let (half, whole) = (visits_of_a_model(8_000), visits_of_a_model(16_000));
    assert!(half > 0, "no definition visited at 8000 interfaces");
    assert!(
        whole <= 3 * half,
        "{half} definitions visited at 8000 interfaces, {whole} at 16000"
    );
}

#[test]
fn types_5000_deep_are_refused_on_the_test_threads_stack() {
    assert_eq!(refused(&sequences(5000)), REFUSED);
    assert_eq!(refused(&unions(5000)), REFUSED);
    let mixed = format!(
        "typedef {}DOMString{} T;\n",
        "(sequence<".repeat(2500),
        "> or A)".repeat(2500)
    );
    assert_eq!(refused(&mixed), REFUSED);
}
