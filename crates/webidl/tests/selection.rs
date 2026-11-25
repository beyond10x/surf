//! The selection `selection.txt` (`story:webidl-bindings`, decisions 1 to 6): its reader
//! refuses what its grammar does not hold, the committed file has the counts of decision 5, and
//! the generator refuses a definition not generated or a member the model lacks, naming the
//! row.

use std::collections::BTreeMap;
use std::path::Path;

use surf_webidl::{Model, Selection, Slice, generate, read};

fn committed() -> Selection {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("selection.txt");
    let text = std::fs::read_to_string(path).expect("read selection.txt");
    Selection::read(&text, "selection.txt").expect("the committed selection reads")
}

/// Member rows and constructor rows per slice, `iterable` left out.
fn per_slice<'s>(
    rows: impl Iterator<Item = &'s surf_webidl::Row>,
) -> BTreeMap<Slice, (usize, usize)> {
    let mut counts = BTreeMap::new();
    for row in rows {
        if row.is_iterable() {
            continue;
        }
        let entry = counts.entry(row.slice).or_insert((0, 0));
        if row.is_constructor() {
            entry.1 += 1;
        } else {
            entry.0 += 1;
        }
    }
    counts
}

#[test]
fn the_committed_selection_has_the_counts_of_decision_5() {
    let selection = committed();
    let all = per_slice(selection.rows.iter());
    let expected = BTreeMap::from([
        (Slice::Reading, (99, 0)),
        (Slice::Attributes, (14, 0)),
        (Slice::Mutation, (46, 4)),
        (Slice::Selectors, (9, 0)),
        (Slice::Throws, (30, 2)),
        (Slice::Exception, (28, 1)),
    ]);
    assert_eq!(all, expected);
    let members: usize = all.values().map(|counts| counts.0).sum();
    let constructors: usize = all.values().map(|counts| counts.1).sum();
    assert_eq!((members, constructors), (226, 7));
    let setters: Vec<String> = selection
        .rows
        .iter()
        .filter(|row| row.setter_slice() != row.slice)
        .map(|row| {
            format!(
                "{}.{} {}",
                row.interface,
                row.member,
                row.setter_slice().word()
            )
        })
        .collect();
    assert_eq!(
        setters,
        [
            "Node.nodeValue mutation",
            "Node.textContent mutation",
            "Document.title mutation",
            "CharacterData.data mutation",
        ]
    );
    // The four interfaces of the epic's table, without `Document.title` (S6).
    let table = per_slice(selection.rows.iter().filter(|row| {
        ["EventTarget", "Node", "Element", "Document"].contains(&row.interface.as_str())
            && !row.is_constructor()
            && !(row.interface == "Document" && row.member == "title")
    }));
    let table: Vec<(Slice, usize)> = table
        .into_iter()
        .map(|(slice, (members, _))| (slice, members))
        .collect();
    assert_eq!(
        table,
        [
            (Slice::Reading, 73),
            (Slice::Attributes, 14),
            (Slice::Mutation, 29),
            (Slice::Selectors, 7),
            (Slice::Throws, 23),
        ]
    );
    assert!(
        selection
            .rows
            .iter()
            .filter(|row| row.slice == Slice::Throws)
            .all(|row| row.reason().is_some_and(|reason| !reason.is_empty()))
    );
}

#[test]
fn the_reader_refuses_what_the_grammar_does_not_hold() {
    let head = "definitions: Node DOMException\nglobal: Node\n";
    let cases: [(&str, &str); 12] = [
        (
            "Node nodeType",
            "s.txt:3: not a row `<interface> <member> <slice> [<note>]`: Node nodeType",
        ),
        (
            "Node nodeType reads",
            "s.txt:3: `Node nodeType`: no slice `reads`",
        ),
        (
            "Node nodeType reading\nNode nodeType reading",
            "s.txt:4: a second row of `Node nodeType`",
        ),
        (
            "Node appendChild throws",
            "s.txt:3: `Node appendChild`: a row of throws names one `reason:`",
        ),
        (
            "Node appendChild mutation reason: none",
            "s.txt:3: `Node appendChild`: `reason:` on a row of mutation",
        ),
        (
            "Node nodeValue reading setter: throws",
            "s.txt:3: `Node nodeValue`: `setter:` names no slice with a trait: throws",
        ),
        (
            "Node nodeType reading colour: red",
            "s.txt:3: `Node nodeType`: no note `colour:`",
        ),
        (
            "Node nodeType reading deviation:",
            "s.txt:3: `Node nodeType`: a note clause is `<key>: <words>`: deviation:",
        ),
        (
            "Node nodeType reading\nglobal: Node",
            "s.txt:4: a second `global:`",
        ),
        ("definitions: Node", "s.txt:3: a second `definitions:`"),
        (
            "Attr name reading",
            "s.txt:3: `Attr name`: `Attr` is not among the definitions generated",
        ),
        (
            "Node node-type reading",
            "s.txt:3: not a row `<interface> <member> <slice> [<note>]`: Node node-type reading",
        ),
    ];
    for (rows, expected) in cases {
        let text = format!("{head}{rows}\n");
        let error = Selection::read(&text, "s.txt").expect_err(rows);
        assert_eq!(error.to_string(), expected, "{rows}");
    }
    let error = Selection::read("global: Node\n", "s.txt").expect_err("no definitions");
    assert_eq!(error.to_string(), "s.txt:0: no `definitions:` line");
    let error =
        Selection::read("definitions: Node\nglobal: Window\n", "s.txt").expect_err("global");
    assert_eq!(
        error.to_string(),
        "s.txt:2: the global `Window` is not among the definitions"
    );
}

/// A small model: an interface `Node` with two members, and `DOMException`.
fn small_model() -> Model {
    let idl = "interface EventTarget {};\n\
               interface Node : EventTarget {\n\
               \x20 readonly attribute unsigned short nodeType;\n\
               \x20 attribute double weight;\n\
               };\n\
               interface Attr : Node {};\n\
               interface DOMException { readonly attribute DOMString name; };\n";
    Model::new(read(idl, "small.idl").expect("read")).expect("model")
}

#[test]
fn outside_definition_refused() {
    let model = small_model();
    // A row of `Attr`, a definition the selection does not generate, is refused by the reader.
    let attr = "definitions: EventTarget Node DOMException\nglobal: Node\nNode nodeType reading\nAttr name reading\n";
    let error = Selection::read(attr, "s.txt").expect_err("Attr");
    assert_eq!(
        error.to_string(),
        "s.txt:4: `Attr name`: `Attr` is not among the definitions generated"
    );
    // A member the model lacks fails the generator, naming the row.
    let lacking = "definitions: EventTarget Node DOMException\nglobal: Node\nNode nodeType reading\nNode nodeName reading\n";
    let selection = Selection::read(lacking, "s.txt").expect("reads");
    let error = generate(&model, &selection, &[]).expect_err("nodeName");
    assert_eq!(
        error.to_string(),
        "s.txt:4: `Node nodeName`: the model holds no member `nodeName` of `Node`"
    );
    // A definition the model does not define, and a parent outside the definitions.
    let unknown = "definitions: EventTarget Node DOMException Window\nglobal: Node\n";
    let selection = Selection::read(unknown, "s.txt").expect("reads");
    let error = generate(&model, &selection, &[]).expect_err("Window");
    assert_eq!(
        error.to_string(),
        "s.txt:1: `Window` is no interface the model defines"
    );
    let orphan = "definitions: Node DOMException\nglobal: Node\n";
    let selection = Selection::read(orphan, "s.txt").expect("reads");
    let error = generate(&model, &selection, &[]).expect_err("EventTarget");
    assert_eq!(
        error.to_string(),
        "s.txt:1: `Node` inherits from `EventTarget`, which is not among the definitions"
    );
    // A type the generator does not convert.
    let double = "definitions: EventTarget Node DOMException\nglobal: Node\nNode weight reading\n";
    let selection = Selection::read(double, "s.txt").expect("reads");
    let error = generate(&model, &selection, &[]).expect_err("double");
    assert_eq!(
        error.to_string(),
        "s.txt:3: `Node weight`: a return type the generator does not convert: double"
    );
    // The same selection without the lacking row generates.
    let fine = "definitions: EventTarget Node DOMException\nglobal: Node\nNode nodeType reading\n";
    let selection = Selection::read(fine, "s.txt").expect("reads");
    let text = generate(&model, &selection, &[]).expect("generates");
    assert!(
        text.contains("fn node_node_type("),
        "the trait method of Node.nodeType"
    );
}

/// `[LegacyUnforgeable]` on a member of an interface that is not the global is refused naming
/// the row: the bindings put such a member on the global object only, where Web IDL puts it on
/// each instance. On the global's own member it is generated.
#[test]
fn legacy_unforgeable_off_the_global_is_refused() {
    let idl = "interface EventTarget {};\n\
               interface Node : EventTarget {\n\
               \x20 [LegacyUnforgeable] readonly attribute unsigned short nodeType;\n\
               };\n\
               interface Window : EventTarget {\n\
               \x20 [LegacyUnforgeable] readonly attribute unsigned short length;\n\
               };\n\
               interface DOMException { readonly attribute DOMString name; };\n";
    let model = Model::new(read(idl, "small.idl").expect("read")).expect("model");
    let off = "definitions: EventTarget Node Window DOMException\nglobal: Window\n\
               Node nodeType reading\n";
    let selection = Selection::read(off, "s.txt").expect("reads");
    let error = generate(&model, &selection, &[]).expect_err("Node.nodeType");
    assert_eq!(
        error.to_string(),
        "s.txt:3: `Node nodeType`: `[LegacyUnforgeable]` off the global interface: the \
         bindings put such a member on the global object only, and Web IDL puts it on each \
         instance"
    );
    let on = "definitions: EventTarget Node Window DOMException\nglobal: Window\n\
              Window length reading\n";
    let selection = Selection::read(on, "s.txt").expect("reads");
    let text = generate(&model, &selection, &[]).expect("generates");
    assert!(text.contains("name: \"length\""), "Window.length generated");
}

/// The rule of the generator's names: `-` is written `_`, a keyword with a raw form is raw, and
/// `self` gets a `_` at its end (no raw `self` exists).
#[test]
fn argument_names_follow_the_rule_of_rust_identifiers() {
    let idl = "interface EventTarget {};\n\
               interface Node : EventTarget {\n\
               \x20 undefined foo(DOMString my-arg, DOMString self, DOMString type);\n\
               };\n\
               interface DOMException { readonly attribute DOMString name; };\n";
    let model = Model::new(read(idl, "small.idl").expect("read")).expect("model");
    let text = "definitions: EventTarget Node DOMException\nglobal: Node\nNode foo mutation\n";
    let selection = Selection::read(text, "s.txt").expect("reads");
    let text = generate(&model, &selection, &[]).expect("generates");
    assert!(
        text.contains("my_arg: String, self_: String, r#type: String"),
        "{}",
        text.lines()
            .filter(|line| line.contains("fn node_foo("))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Two arguments of one member whose names give the same Rust identifier are refused naming
/// the row.
#[test]
fn two_arguments_with_one_rust_name_are_refused() {
    let idl = "interface EventTarget {};\n\
               interface Node : EventTarget {\n\
               \x20 undefined foo(DOMString my-arg, DOMString my_arg);\n\
               };\n\
               interface DOMException { readonly attribute DOMString name; };\n";
    let model = Model::new(read(idl, "small.idl").expect("read")).expect("model");
    let text = "definitions: EventTarget Node DOMException\nglobal: Node\nNode foo mutation\n";
    let selection = Selection::read(text, "s.txt").expect("reads");
    let error = generate(&model, &selection, &[]).expect_err("my_arg twice");
    assert_eq!(
        error.to_string(),
        "s.txt:3: `Node foo`: the arguments `my-arg` and `my_arg` are both written `my_arg`"
    );
}

/// The model of the bindings, the only test that reads the cut of the HTML Standard.
#[test]
fn rows_match_the_model() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/whatwg");
    let page = |name: &str| std::fs::read(data.join(name)).expect("read a page");
    let selection = committed();
    let (model, _) = surf_webidl::bindings_model(
        &page("dom.html"),
        &page("webidl.html"),
        &page("html-idl.html"),
        &selection,
    )
    .expect("the model of the bindings");
    selection
        .check(&model)
        .expect("every row names a member of the model");
    let mut wrong = Vec::new();
    for name in &selection.definitions {
        for member in model.members(name) {
            let key = match (&member.kind, &member.name) {
                (surf_webidl::MemberKind::Constructor { .. }, _) => "constructor".to_string(),
                (surf_webidl::MemberKind::Iterable { .. }, _) => "iterable".to_string(),
                (_, Some(member)) => member.clone(),
                (_, None) => "stringifier".to_string(),
            };
            let rows = selection
                .rows_of(name)
                .filter(|row| row.member == key)
                .count();
            if rows != 1 {
                wrong.push(format!("{name} {key}: {rows} rows"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "members of the model without exactly one row: {wrong:?}"
    );
    let rows: usize = selection
        .rows
        .iter()
        .filter(|row| !row.is_iterable())
        .count();
    assert_eq!(rows, 233, "226 member rows and 7 constructor rows");
}
