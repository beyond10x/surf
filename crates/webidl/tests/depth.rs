//! The bound on nested types: a named type alone is 1 deep, `sequence<DOMString>` and
//! `(A or B)` 2 deep; past 32 levels a type is refused, never read by deeper recursion.

use surf_webidl::read;

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
