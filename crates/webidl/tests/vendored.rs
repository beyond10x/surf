//! The two vendored pages read whole, once per test run: one test reads both and asserts every
//! count over the one model (the story's decision 5).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use surf_webidl::{DefinitionKind, MemberKind, Model, Page, TypeKind, read_page};

fn data(file: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data/whatwg")
        .join(file);
    std::fs::read(path).expect("read a vendored page")
}

/// The value of `index-lines:` in the block of `file` in `SOURCE`.
fn index_lines(file: &str) -> usize {
    let source =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("data/whatwg/SOURCE"))
            .expect("read SOURCE");
    let mut current = "";
    for line in source.lines() {
        if let Some(name) = line.strip_prefix("file: ") {
            current = name;
        }
        if current == file
            && let Some(count) = line.strip_prefix("index-lines: ")
        {
            return count.parse().expect("a count");
        }
    }
    panic!("SOURCE names no index-lines for {file}");
}

/// (own, from mixins, constants, attributes, operations, constructor).
fn counts(model: &Model, interface: &str) -> (usize, usize, usize, usize, usize, bool) {
    let c = model.counts(interface);
    (
        c.own,
        c.from_mixins,
        c.constants,
        c.attributes,
        c.operations,
        c.constructor,
    )
}

/// The mixins `interface` includes, in the order of the `includes` statements, each with the
/// number of its members.
fn includes(model: &Model, interface: &str) -> Vec<(String, usize)> {
    model
        .definitions()
        .iter()
        .filter(|definition| definition.name == interface)
        .filter_map(|definition| match &definition.kind {
            DefinitionKind::Includes { mixin } => Some(mixin.clone()),
            _ => None,
        })
        .map(|mixin| {
            let members = model
                .definitions()
                .iter()
                .filter(|definition| definition.name == mixin)
                .map(|definition| match &definition.kind {
                    DefinitionKind::InterfaceMixin { members } => members.len(),
                    _ => 0,
                })
                .sum();
            (mixin, members)
        })
        .collect()
}

fn pair(name: &str, members: usize) -> (String, usize) {
    (name.to_string(), members)
}

#[test]
fn the_indexes_read_whole() {
    let pages: Vec<(&str, Result<Page, _>)> = ["dom.html", "webidl.html"]
        .into_iter()
        .map(|file| (file, read_page(&data(file), file)))
        .collect();
    let errors: Vec<String> = pages
        .iter()
        .filter_map(|(_, page)| page.as_ref().err().map(ToString::to_string))
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));

    let mut definitions = Vec::new();
    let mut lines = Vec::new();
    for (file, page) in pages {
        let page = page.expect("checked above");
        lines.push((file, page.index.lines().count()));
        definitions.extend(page.definitions);
    }
    let model = Model::new(definitions);

    // Acceptance 3: the names no definition defines, and the four interfaces of the table.
    let unresolved: BTreeSet<String> = model
        .unresolved()
        .into_iter()
        .map(|unresolved| unresolved.name)
        .collect();
    let listed: BTreeSet<String> = include_str!("unresolved.txt")
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(unresolved, listed);
    for name in ["EventHandler", "DOMHighResTimeStamp", "TrustedType"] {
        assert!(unresolved.contains(name), "{name}");
    }
    let table = [
        ("EventTarget", (3, 0, 0, 0, 3, true)),
        ("Node", (47, 0, 18, 14, 15, false)),
        ("Element", (36, 17, 0, 18, 35, false)),
        ("Document", (28, 15, 0, 15, 28, true)),
    ];
    for (interface, expected) in table {
        assert_eq!(counts(&model, interface), expected, "{interface}");
    }
    let members: usize = table.iter().map(|(_, c)| c.0 + c.1).sum();
    let constructors = table.iter().filter(|(_, c)| c.5).count();
    assert_eq!((members, constructors), (146, 2));

    // Acceptance 4: the mixins of Element and Document, and the writable attributes.
    assert_eq!(
        includes(&model, "Element"),
        [
            pair("ParentNode", 10),
            pair("NonDocumentTypeChildNode", 2),
            pair("ChildNode", 4),
            pair("Slottable", 1)
        ]
    );
    assert_eq!(
        includes(&model, "Document"),
        [
            pair("NonElementParentNode", 1),
            pair("DocumentOrShadowRoot", 1),
            pair("ParentNode", 10),
            pair("XPathEvaluatorBase", 3)
        ]
    );
    let writable: Vec<String> = table
        .iter()
        .flat_map(|(interface, _)| model.members(interface))
        .filter(|member| {
            matches!(
                member.kind,
                MemberKind::Attribute {
                    readonly: false,
                    ..
                }
            )
        })
        .filter_map(|member| member.name.clone())
        .collect();
    assert_eq!(
        writable,
        ["nodeValue", "textContent", "id", "className", "slot"]
    );

    // Acceptance 5: the interfaces the four return.
    let returned = [
        ("CharacterData", 13, false),
        ("Text", 3, true),
        ("Comment", 0, true),
        ("ProcessingInstruction", 8, true),
        ("DocumentType", 7, false),
        ("DocumentFragment", 11, true),
        ("NodeList", 2, false),
        ("HTMLCollection", 3, false),
        ("DOMException", 28, true),
    ];
    for (interface, members, constructor) in returned {
        let c = model.counts(interface);
        assert_eq!(
            (c.own + c.from_mixins, c.constructor),
            (members, constructor),
            "{interface}"
        );
    }
    let exception = model.counts("DOMException");
    assert_eq!((exception.attributes, exception.constants), (3, 25));
    assert!(
        model
            .definitions()
            .iter()
            .any(|d| d.name == "DOMException" && d.file == "webidl.html")
    );
    let iterable: Vec<&TypeKind> = model
        .members("NodeList")
        .into_iter()
        .filter_map(|member| match &member.kind {
            MemberKind::Iterable { ty } => Some(&ty.kind),
            _ => None,
        })
        .collect();
    assert!(
        matches!(iterable.as_slice(), [TypeKind::Named { name, .. }] if name == "Node"),
        "{iterable:?}"
    );
    let members: usize = returned.iter().map(|r| r.1).sum();
    let constructors = returned.iter().filter(|r| r.2).count();
    assert_eq!((members, constructors), (75, 5));

    // Acceptance 6: the extended attributes of the DOM index.
    let expected: BTreeMap<String, usize> = [
        ("CEReactions", 49),
        ("Exposed", 37),
        ("NewObject", 27),
        ("SameObject", 11),
        ("Unscopable", 8),
        ("LegacyNullToEmptyString", 5),
        ("LegacyUnenumerableNamedProperties", 2),
        ("PutForwards", 1),
        ("LegacyUnforgeable", 1),
        ("Replaceable", 1),
        ("EnforceRange", 1),
    ]
    .into_iter()
    .map(|(name, count)| (name.to_string(), count))
    .collect();
    assert_eq!(model.extended_attribute_counts("dom.html"), expected);

    // Acceptance 3: the lines of each index, as `SOURCE` names them.
    assert_eq!(
        lines,
        [
            ("dom.html", index_lines("dom.html")),
            ("webidl.html", index_lines("webidl.html"))
        ]
    );
    assert_eq!(
        [index_lines("dom.html"), index_lines("webidl.html")],
        [673, 58]
    );
}
