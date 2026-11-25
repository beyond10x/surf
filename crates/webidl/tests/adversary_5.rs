//! Fourth review of `story:webidl-bindings`, on its fix round 3: the one registry of the Rust
//! names the generator writes (`generate.rs`, `fixed_items` and `take_items`), filled with the
//! items the prelude defines at the start of a line. A name the file holds without defining it
//! there is not in it: the names the prelude imports with `use`, the names of Rust's prelude
//! (`String`, `Option`), and `Self`, which no item may take.

use std::collections::BTreeSet;

use super::adversary_4::generated_with;

/// The names `text` imports with `use` (the last segment, or each name of a `{...}` list).
fn imported(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut statement = String::new();
    for line in text.lines() {
        if statement.is_empty() && !line.starts_with("use ") && !line.starts_with("pub use ") {
            continue;
        }
        statement.push_str(line);
        statement.push(' ');
        if !line.trim_end().ends_with(';') {
            continue;
        }
        let body = statement.trim().trim_end_matches(';');
        let list = match (body.find('{'), body.rfind('}')) {
            (Some(open), Some(close)) => body[open + 1..close].to_string(),
            _ => body.rsplit("::").next().unwrap_or("").to_string(),
        };
        for name in list.split(',') {
            let name = name.trim();
            let name = name.rsplit(" as ").next().unwrap_or(name).trim();
            let name = name.rsplit("::").next().unwrap_or(name).trim();
            if !name.is_empty() {
                names.insert(name.to_string());
            }
        }
        statement.clear();
    }
    names
}

/// The names of the structs, enums and traits `text` defines at the start of a line.
fn defined_types(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let line = line.strip_prefix("pub ").unwrap_or(line);
            let rest = line
                .strip_prefix("struct ")
                .or_else(|| line.strip_prefix("enum "))
                .or_else(|| line.strip_prefix("trait "))?;
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            Some(name)
        })
        .collect()
}

/// Names of Rust's prelude the file uses as types (edition 2024).
const RUST_PRELUDE: &[&str] = &[
    "String", "Vec", "Option", "Result", "Box", "Clone", "Copy", "Default", "Drop", "Iterator",
];

/// The bindings of a dictionary `name` that an operation of `Node` takes.
fn dictionary_named(name: &str) -> Result<String, String> {
    let extra = format!(
        "dictionary {name} {{\n  boolean a = false;\n}};\n\
         interface Node : EventTarget {{\n  undefined a(optional {name} d = {{}});\n}};\n"
    );
    generated_with(&extra, "Node", "Node a mutation\n")
}

/// Red (should). Note 1 of the third review as ruled for fix round 3: two names that give one
/// Rust name are refused, among them the names the prelude uses. The registry holds what the
/// prelude defines, not what it imports: a dictionary `Value` (or `Cx`, `Root`, `Tree`,
/// `NodeId`, `Limits`, `Cell`, `StringRef`...) is written `pub struct Value` beside
/// `use swell::{..., Value, ...}` (rustc E0255), and a dictionary `String` or `Option` is
/// written as a struct that every `String` and `Option` of the prelude then names. Refused
/// naming the name, as `Copies` is.
#[test]
fn a_dictionary_named_like_a_name_the_prelude_imports_is_refused() {
    let names = [
        "Value",
        "Completion",
        "Cx",
        "Root",
        "Tree",
        "NodeId",
        "Limit",
        "Limits",
        "Cell",
        "Brand",
        "StringRef",
        "ObjectRef",
        "ScriptHost",
        "String",
        "Option",
        "Vec",
    ];
    let mut wrong = Vec::new();
    for name in names {
        match dictionary_named(name) {
            Ok(text) => {
                let clash: Vec<String> = defined_types(&text)
                    .intersection(&imported(&text))
                    .cloned()
                    .chain(
                        RUST_PRELUDE
                            .iter()
                            .filter(|std| defined_types(&text).contains(**std))
                            .map(|std| format!("{std} (Rust's prelude)")),
                    )
                    .collect();
                if !clash.is_empty() {
                    wrong.push(format!("dictionary {name}: generated, defines {clash:?}"));
                }
            }
            Err(error) if error.contains("both written") && error.contains(name) => {}
            Err(error) => wrong.push(format!("dictionary {name}: {error}")),
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Red (note). `identifier` writes a member, an argument or a field named `self` or `Self` as
/// `self_`, a name Rust keeps for itself; the names of interfaces and dictionaries are written
/// by `camel` without that rule: an interface `Self` is the variant `Interface::Self` and a
/// dictionary `Self` (or `self`) the item `pub struct Self`, which rustc refuses (`Self` is a
/// keyword). Refused naming it, or written as another name.
#[test]
fn an_interface_or_a_dictionary_named_self_is_refused() {
    let mut wrong = Vec::new();
    for name in ["Self", "self"] {
        match dictionary_named(name) {
            Ok(text) if text.contains("struct Self ") || text.contains("struct Self{") => {
                wrong.push(format!("dictionary {name}: generated `struct Self`"));
            }
            Ok(_) => {}
            Err(error) if error.contains(name) => {}
            Err(error) => wrong.push(format!("dictionary {name}: {error}")),
        }
        let extra = format!("interface {name} {{\n  readonly attribute DOMString x;\n}};\n");
        match generated_with(&extra, name, &format!("{name} x reading\n")) {
            Ok(text) if text.lines().any(|line| line.trim() == "Self,") => {
                wrong.push(format!(
                    "interface {name}: generated the variant `Interface::Self`"
                ));
            }
            Ok(_) => {}
            Err(error) if error.contains(name) => {}
            Err(error) => wrong.push(format!("interface {name}: {error}")),
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Held. The registry's own kinds hold across kinds: an interface `Of` (the prelude's
/// `fn interface_of`), a dictionary `RawD` beside a dictionary `D` (both `RawD`), a dictionary
/// `Totals` (a `pub struct` of the prelude) and a dictionary `Interface` (the generator's
/// enum): each refused naming both. The
/// implementor's mutant list calls the claim of `value interface_x` equivalent; the first case
/// is a pair only that claim refuses.
#[test]
fn names_of_two_kinds_that_meet_are_refused() {
    let cases = [
        (
            "interface Of {\n  readonly attribute DOMString x;\n};\n".to_string(),
            "Of",
            "Of x reading\n",
        ),
        (
            "dictionary D {\n  boolean a = false;\n};\ndictionary RawD {\n  boolean b = false;\n};\n\
             interface Node : EventTarget {\n  undefined a(optional D d = {});\n  undefined b(optional RawD d = {});\n};\n"
                .to_string(),
            "Node",
            "Node a mutation\nNode b mutation\n",
        ),
        (
            "dictionary Totals {\n  boolean a = false;\n};\n\
             interface Node : EventTarget {\n  undefined a(optional Totals d = {});\n};\n"
                .to_string(),
            "Node",
            "Node a mutation\n",
        ),
        (
            "dictionary Interface {\n  boolean a = false;\n};\n\
             interface Node : EventTarget {\n  undefined a(optional Interface d = {});\n};\n"
                .to_string(),
            "Node",
            "Node a mutation\n",
        ),
    ];
    let mut wrong = Vec::new();
    for (extra, definitions, rows) in &cases {
        match generated_with(extra, definitions, rows) {
            Ok(_) => wrong.push(format!("{rows}: generated")),
            Err(error) if error.contains("both written") => {}
            Err(error) => wrong.push(format!("{rows}: {error}")),
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Red (note). Web IDL's identifier allows a hyphen (`/[_-]?[A-Za-z][0-9A-Z_a-z-]*/`);
/// `identifier` writes it `_` for a member, an argument or a field, but `camel` and `snake` keep
/// it in the names of a dictionary: `dictionary Foo-Bar` is written `pub struct Foo-bar` and
/// `fn convert_foo-bar`, which rustc refuses. Refused naming it, or written as a Rust name.
#[test]
fn a_dictionary_named_with_a_hyphen_is_refused() {
    match dictionary_named("Foo-Bar") {
        Ok(text) => {
            let written: Vec<&str> = text
                .lines()
                .filter(|line| line.contains("Foo-bar") || line.contains("foo-bar"))
                .collect();
            assert!(written.is_empty(), "{written:#?}");
        }
        Err(error) => assert!(error.contains("Foo-Bar"), "{error}"),
    }
}
