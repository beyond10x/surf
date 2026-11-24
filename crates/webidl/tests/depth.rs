//! The bound on nested types: a named type alone is 1 deep, `sequence<DOMString>` and
//! `(A or B)` 2 deep; past 32 levels a type is refused, never read by deeper recursion. And the
//! time of a model of many definitions.

use std::time::{Duration, Instant};

use surf_webidl::{Model, read};

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

/// `Model::new` finds each name through one map built once, not by a scan of every definition
/// per statement: 16,000 interfaces, each with one `includes` of one mixin, read and modelled
/// within 2 s.
#[test]
fn a_model_of_16000_interfaces_is_built_within_2_s() {
    let mut text = String::new();
    for i in 0..16_000 {
        text.push_str(&format!("interface A{i} {{}};\nA{i} includes M;\n"));
    }
    text.push_str("interface mixin M {\n  undefined f();\n};\n");
    let start = Instant::now();
    let definitions = read(&text, FILE).unwrap_or_else(|error| panic!("{error}"));
    let read_time = start.elapsed();
    let model = Model::new(definitions).unwrap_or_else(|error| panic!("{error}"));
    let total = start.elapsed();
    println!(
        "16000 interfaces: read {read_time:?}, model {:?}, together {total:?}",
        total - read_time
    );
    assert_eq!(model.counts("A15999").from_mixins, 1);
    assert!(total < Duration::from_secs(2), "{total:?}");
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
