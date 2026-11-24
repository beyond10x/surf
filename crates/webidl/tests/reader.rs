//! The reader, one case per item of the model: each reads an IDL text of a few lines written
//! here.

use surf_webidl::{
    ADDED, Argument, ConstValue, DefaultValue, Definition, DefinitionKind, EXTENDED_ATTRIBUTES,
    ExtendedAttributeValue, IntegerSize, Member, MemberKind, Model, Reference,
    TYPE_EXTENDED_ATTRIBUTES, Type, TypeKind, read,
};

const FILE: &str = "test.idl";

fn all(text: &str) -> Vec<Definition> {
    read(text, FILE).unwrap_or_else(|error| panic!("{error}"))
}

fn model(definitions: Vec<Definition>) -> Model {
    Model::new(definitions).unwrap_or_else(|error| panic!("{error}"))
}

fn one(text: &str) -> Definition {
    let mut definitions = all(text);
    assert_eq!(definitions.len(), 1, "{definitions:?}");
    definitions.remove(0)
}

fn members(definition: &Definition) -> &[Member] {
    match &definition.kind {
        DefinitionKind::Interface { members, .. }
        | DefinitionKind::InterfaceMixin { members }
        | DefinitionKind::CallbackInterface { members } => members,
        other => panic!("no members: {other:?}"),
    }
}

/// The one member of the interface `text`.
fn member(text: &str) -> Member {
    let definition = one(text);
    let list = members(&definition);
    assert_eq!(list.len(), 1, "{list:?}");
    list[0].clone()
}

fn arguments(member: &Member) -> &[Argument] {
    match &member.kind {
        MemberKind::Operation { arguments, .. } | MemberKind::Constructor { arguments } => {
            arguments
        }
        other => panic!("no arguments: {other:?}"),
    }
}

fn named(name: &str) -> TypeKind {
    TypeKind::Named {
        name: name.to_string(),
        line: 0,
    }
}

/// `ty`'s kind with the line of every name set to 0, for comparison.
fn shape(ty: &Type) -> TypeKind {
    match &ty.kind {
        TypeKind::Named { name, .. } => named(name),
        TypeKind::Sequence(inner) => TypeKind::Sequence(Box::new(Type {
            kind: shape(inner),
            nullable: inner.nullable,
            extended_attributes: inner.extended_attributes.clone(),
        })),
        TypeKind::Union(list) => TypeKind::Union(
            list.iter()
                .map(|member| Type {
                    kind: shape(member),
                    nullable: member.nullable,
                    extended_attributes: member.extended_attributes.clone(),
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

fn attribute_type(member: &Member) -> &Type {
    match &member.kind {
        MemberKind::Attribute { ty, .. } => ty,
        other => panic!("no attribute: {other:?}"),
    }
}

#[test]
fn definition_interface() {
    let definition = one("\n[Exposed=Window]\ninterface Thing {\n};\n");
    assert_eq!(definition.name, "Thing");
    assert_eq!((definition.file.as_str(), definition.line), (FILE, 3));
    assert_eq!(definition.extended_attributes[0].name, "Exposed");
    assert_eq!(
        definition.kind,
        DefinitionKind::Interface {
            partial: false,
            inherits: None,
            members: Vec::new(),
        }
    );
}

#[test]
fn definition_interface_inherits() {
    let definition = one("interface Node : EventTarget {};");
    assert!(matches!(
        definition.kind,
        DefinitionKind::Interface { partial: false, inherits: Some(ref parent), .. } if parent.name == "EventTarget"
    ));
}

#[test]
fn definition_partial_interface() {
    let definition = one("partial interface Window {\n  readonly attribute any event;\n};");
    assert!(matches!(
        definition.kind,
        DefinitionKind::Interface {
            partial: true,
            inherits: None,
            ..
        }
    ));
    assert_eq!(members(&definition)[0].name.as_deref(), Some("event"));
}

#[test]
fn definition_interface_mixin() {
    let definition =
        one("interface mixin Slottable {\n  readonly attribute Element? assignedSlot;\n};");
    assert_eq!(definition.name, "Slottable");
    assert!(matches!(
        definition.kind,
        DefinitionKind::InterfaceMixin { .. }
    ));
    assert_eq!(members(&definition).len(), 1);
}

#[test]
fn definition_includes() {
    let definition = one("Element includes Slottable;");
    assert_eq!(definition.name, "Element");
    assert_eq!(
        definition.kind,
        DefinitionKind::Includes {
            mixin: Reference {
                name: "Slottable".to_string(),
                line: 1
            }
        }
    );
}

#[test]
fn definition_callback_interface() {
    let definition = one(
        "callback interface NodeFilter {\n  const unsigned short FILTER_ACCEPT = 1;\n  unsigned short acceptNode(Node node);\n};",
    );
    assert!(matches!(
        definition.kind,
        DefinitionKind::CallbackInterface { .. }
    ));
    let names: Vec<_> = members(&definition)
        .iter()
        .map(|member| member.name.as_deref())
        .collect();
    assert_eq!(names, [Some("FILTER_ACCEPT"), Some("acceptNode")]);
}

#[test]
fn definition_callback() {
    let definition = one(
        "callback MutationCallback = undefined (sequence<MutationRecord> mutations, MutationObserver observer);",
    );
    let DefinitionKind::Callback {
        return_type,
        arguments,
    } = definition.kind
    else {
        panic!("not a callback");
    };
    assert_eq!(return_type.kind, TypeKind::Undefined);
    let names: Vec<_> = arguments.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["mutations", "observer"]);
}

#[test]
fn definition_dictionary() {
    let definition = one(
        "dictionary ShadowRootInit : Base {\n  required ShadowRootMode mode;\n  boolean delegatesFocus = false;\n  SlotAssignmentMode slotAssignment = \"named\";\n  CustomElementRegistry? customElementRegistry;\n};",
    );
    let DefinitionKind::Dictionary { inherits, members } = definition.kind else {
        panic!("not a dictionary");
    };
    assert_eq!(
        inherits.map(|parent| (parent.name, parent.line)),
        Some(("Base".to_string(), 1))
    );
    let summary: Vec<_> = members
        .iter()
        .map(|m| (m.name.as_str(), m.required, m.default.clone(), m.line))
        .collect();
    assert_eq!(
        summary,
        [
            ("mode", true, None, 2),
            (
                "delegatesFocus",
                false,
                Some(DefaultValue::Boolean(false)),
                3
            ),
            (
                "slotAssignment",
                false,
                Some(DefaultValue::String("named".to_string())),
                4
            ),
            ("customElementRegistry", false, None, 5),
        ]
    );
    assert!(members[3].ty.nullable);
}

#[test]
fn definition_enum() {
    let definition = one("enum ShadowRootMode { \"open\", \"closed\" };");
    assert_eq!(
        definition.kind,
        DefinitionKind::Enum {
            values: vec!["open".to_string(), "closed".to_string()]
        }
    );
}

#[test]
fn definition_enum_trailing_comma() {
    let definition = one("enum E { \"a\", \"b\", };");
    assert_eq!(
        definition.kind,
        DefinitionKind::Enum {
            values: vec!["a".to_string(), "b".to_string()]
        }
    );
}

#[test]
fn definition_typedef() {
    let definition = one("typedef (ArrayBufferView or Thing) BufferSource;");
    assert_eq!(definition.name, "BufferSource");
    let DefinitionKind::Typedef { ty } = &definition.kind else {
        panic!("not a typedef");
    };
    assert!(matches!(&ty.kind, TypeKind::Union(list) if list.len() == 2));
}

#[test]
fn member_const() {
    let member =
        member("interface Node {\n  const unsigned short DOCUMENT_POSITION_CONTAINS = 0x08;\n};");
    assert_eq!(member.name.as_deref(), Some("DOCUMENT_POSITION_CONTAINS"));
    assert_eq!(member.line, 2);
    let MemberKind::Const { ty, value } = member.kind else {
        panic!("not a const");
    };
    assert_eq!(
        ty.kind,
        TypeKind::Integer {
            unsigned: true,
            size: IntegerSize::Short
        }
    );
    assert_eq!(value, ConstValue::Integer(8));
}

#[test]
fn member_attribute() {
    let member = member("interface Element {\n  [CEReactions] attribute DOMString id;\n};");
    assert_eq!(member.name.as_deref(), Some("id"));
    assert_eq!(member.extended_attributes[0].name, "CEReactions");
    assert!(matches!(
        member.kind,
        MemberKind::Attribute {
            readonly: false,
            stringifier: false,
            ref ty,
        } if ty.kind == TypeKind::DomString
    ));
}

#[test]
fn member_readonly_attribute() {
    let member = member("interface Node {\n  readonly attribute unsigned long long count;\n};");
    assert!(matches!(
        member.kind,
        MemberKind::Attribute {
            readonly: true,
            ref ty,
            ..
        } if ty.kind == TypeKind::Integer { unsigned: true, size: IntegerSize::LongLong }
    ));
}

#[test]
fn member_stringifier() {
    let definition = one(
        "interface Range {\n  stringifier;\n  [CEReactions] stringifier attribute DOMString value;\n};",
    );
    let list = members(&definition);
    assert_eq!(list[0].kind, MemberKind::Stringifier);
    assert_eq!(list[0].name, None);
    assert!(matches!(
        list[1].kind,
        MemberKind::Attribute {
            stringifier: true,
            readonly: false,
            ..
        }
    ));
    assert_eq!(list[1].name.as_deref(), Some("value"));
    let model = model(vec![definition]);
    assert_eq!(model.counts("Range").operations, 1);
    assert_eq!(model.counts("Range").attributes, 1);
}

#[test]
fn member_operation() {
    let member =
        member("interface Node {\n  [CEReactions] Node insertBefore(Node node, Node? child);\n};");
    assert_eq!(member.name.as_deref(), Some("insertBefore"));
    let MemberKind::Operation {
        return_type,
        arguments,
        is_static,
        getter,
    } = &member.kind
    else {
        panic!("not an operation");
    };
    assert_eq!(shape(return_type), named("Node"));
    assert!(!is_static && !getter);
    assert_eq!(arguments.len(), 2);
    assert!(arguments[1].ty.nullable && !arguments[0].ty.nullable);
}

#[test]
fn member_static_operation() {
    let member = member(
        "interface AbortSignal {\n  [NewObject] static AbortSignal _any(sequence<AbortSignal> signals);\n};",
    );
    assert_eq!(member.name.as_deref(), Some("any"));
    assert!(matches!(
        member.kind,
        MemberKind::Operation {
            is_static: true,
            getter: false,
            ..
        }
    ));
}

#[test]
fn member_constructor() {
    let member = member("interface Text {\n  constructor(optional DOMString data = \"\");\n};");
    assert_eq!(member.name, None);
    let list = arguments(&member);
    assert_eq!(list[0].default, Some(DefaultValue::String(String::new())));
    let model = model(vec![one("interface Text {\n  constructor();\n};")]);
    let counts = model.counts("Text");
    assert!(counts.constructor);
    assert_eq!(counts.own, 0);
}

#[test]
fn member_getter() {
    let member = member("interface NodeList {\n  getter Node? item(unsigned long index);\n};");
    assert_eq!(member.name.as_deref(), Some("item"));
    assert!(matches!(
        member.kind,
        MemberKind::Operation {
            getter: true,
            is_static: false,
            ..
        }
    ));
}

#[test]
fn member_iterable() {
    let member = member("interface NodeList {\n  iterable<Node>;\n};");
    assert_eq!(member.name, None);
    let MemberKind::Iterable { ty } = &member.kind else {
        panic!("not iterable");
    };
    assert_eq!(shape(ty), named("Node"));
    let model = model(vec![one("interface NodeList {\n  iterable<Node>;\n};")]);
    assert_eq!(model.counts("NodeList").own, 0);
}

#[test]
fn argument_optional_defaults() {
    let member = member(
        "interface D {\n  undefined f(optional Options a = {}, optional boolean b = false, optional DOMString c = \"\", optional Node? d = null, optional unsigned long e = 0xFFFFFFFF, optional any g);\n};",
    );
    let defaults: Vec<_> = arguments(&member)
        .iter()
        .map(|a| (a.name.as_str(), a.optional, a.default.clone()))
        .collect();
    assert_eq!(
        defaults,
        [
            ("a", true, Some(DefaultValue::EmptyDictionary)),
            ("b", true, Some(DefaultValue::Boolean(false))),
            ("c", true, Some(DefaultValue::String(String::new()))),
            ("d", true, Some(DefaultValue::Null)),
            ("e", true, Some(DefaultValue::Integer(0xFFFF_FFFF))),
            ("g", true, None),
        ]
    );
}

#[test]
fn argument_variadic_union() {
    let member = member(
        "interface mixin ParentNode {\n  [CEReactions, Unscopable] undefined append((Node or DOMString)... nodes);\n};",
    );
    let list = arguments(&member);
    assert_eq!(list.len(), 1);
    assert!(list[0].variadic && !list[0].optional);
    assert_eq!(list[0].name, "nodes");
    let TypeKind::Union(types) = &list[0].ty.kind else {
        panic!("not a union");
    };
    assert_eq!(types[1].kind, TypeKind::DomString);
}

#[test]
fn type_nullable() {
    let member = member("interface Node {\n  readonly attribute Document? ownerDocument;\n};");
    let ty = attribute_type(&member);
    assert!(ty.nullable);
    assert_eq!(shape(ty), named("Document"));
}

#[test]
fn type_sequence() {
    let member = member("interface Element {\n  sequence<DOMString> getAttributeNames();\n};");
    let MemberKind::Operation { return_type, .. } = &member.kind else {
        panic!("not an operation");
    };
    let TypeKind::Sequence(inner) = &return_type.kind else {
        panic!("not a sequence");
    };
    assert_eq!(inner.kind, TypeKind::DomString);
}

#[test]
fn type_union() {
    let member = member("interface Window {\n  readonly attribute (Event or undefined) event;\n};");
    let ty = attribute_type(&member);
    assert_eq!(
        shape(ty),
        TypeKind::Union(vec![
            Type {
                kind: named("Event"),
                nullable: false,
                extended_attributes: Vec::new()
            },
            Type {
                kind: TypeKind::Undefined,
                nullable: false,
                extended_attributes: Vec::new()
            },
        ])
    );
}

#[test]
fn type_extended_attribute() {
    let member = member(
        "interface CharacterData {\n  attribute [LegacyNullToEmptyString] DOMString data;\n};",
    );
    let ty = attribute_type(&member);
    assert_eq!(ty.kind, TypeKind::DomString);
    assert_eq!(ty.extended_attributes[0].name, "LegacyNullToEmptyString");
    assert!(member.extended_attributes.is_empty());
    let model = model(vec![one(
        "interface CharacterData {\n  attribute [LegacyNullToEmptyString] DOMString data;\n};",
    )]);
    assert_eq!(
        model
            .extended_attribute_counts(FILE)
            .get("LegacyNullToEmptyString"),
        Some(&1)
    );
}

#[test]
fn extended_attribute_arguments() {
    let definition = one(
        "[Exposed=*, LegacyUnenumerableNamedProperties]\ninterface A {\n  [Exposed=(Window,Worker), NewObject] static A timeout([EnforceRange] unsigned long long ms);\n  [SameObject, PutForwards=value] readonly attribute B list;\n};",
    );
    let values: Vec<_> = definition
        .extended_attributes
        .iter()
        .map(|a| (a.name.as_str(), a.value.clone()))
        .collect();
    assert_eq!(
        values,
        [
            ("Exposed", ExtendedAttributeValue::Wildcard),
            (
                "LegacyUnenumerableNamedProperties",
                ExtendedAttributeValue::None
            ),
        ]
    );
    let list = members(&definition);
    assert_eq!(
        list[0].extended_attributes[0].value,
        ExtendedAttributeValue::IdentifierList(vec!["Window".to_string(), "Worker".to_string()])
    );
    assert_eq!(
        arguments(&list[0])[0].ty.extended_attributes[0].name,
        "EnforceRange"
    );
    assert_eq!(
        list[1].extended_attributes[1].value,
        ExtendedAttributeValue::Identifier("value".to_string())
    );
}

/// The names of `list`.
fn attribute_names(list: &[surf_webidl::ExtendedAttribute]) -> Vec<&str> {
    list.iter()
        .map(|attribute| attribute.name.as_str())
        .collect()
}

#[test]
fn type_extended_attribute_on_an_argument() {
    let member = member(
        "interface A {\n  undefined f([EnforceRange] unsigned long a, [EnforceRange] optional unsigned long b);\n};",
    );
    for argument in arguments(&member) {
        assert!(argument.extended_attributes.is_empty(), "{argument:?}");
        assert_eq!(
            attribute_names(&argument.ty.extended_attributes),
            ["EnforceRange"]
        );
    }
}

#[test]
fn type_extended_attribute_on_an_attribute() {
    let member =
        member("interface A {\n  [CEReactions] attribute [EnforceRange] unsigned long x;\n};");
    assert_eq!(
        attribute_names(&member.extended_attributes),
        ["CEReactions"]
    );
    assert_eq!(
        attribute_names(&attribute_type(&member).extended_attributes),
        ["EnforceRange"]
    );
}

#[test]
fn type_extended_attribute_on_a_dictionary_member() {
    let definition = one("dictionary D {\n  [EnforceRange] unsigned long x = 0;\n};");
    let DefinitionKind::Dictionary { members, .. } = &definition.kind else {
        panic!("not a dictionary");
    };
    assert!(members[0].extended_attributes.is_empty());
    assert_eq!(
        attribute_names(&members[0].ty.extended_attributes),
        ["EnforceRange"]
    );
}

#[test]
fn type_extended_attributes_are_known_ones() {
    for name in TYPE_EXTENDED_ATTRIBUTES {
        assert!(EXTENDED_ATTRIBUTES.contains(name), "{name}");
    }
    assert_eq!(
        TYPE_EXTENDED_ATTRIBUTES,
        ["AllowShared", "EnforceRange", "LegacyNullToEmptyString"]
    );
}

#[test]
fn merged_mixin_counted_on_each() {
    let model = model(all(
        "interface mixin ChildNode {\n  undefined remove();\n  readonly attribute Element? next;\n};\ninterface A { const short X = 1; };\ninterface B {};\nA includes ChildNode;\nB includes ChildNode;\n",
    ));
    for (interface, own) in [("A", 1), ("B", 0)] {
        let counts = model.counts(interface);
        assert_eq!(
            (
                counts.own,
                counts.from_mixins,
                counts.attributes,
                counts.operations
            ),
            (own, 2, 1, 1),
            "{interface}"
        );
    }
    assert_eq!(model.members("B").len(), 2);
}

#[test]
fn merged_partial() {
    let model = model(all(
        "partial interface Document {\n  readonly attribute DOMString title;\n};\ninterface Document : Node {\n  constructor();\n  Element? getElementById(DOMString id);\n};\n",
    ));
    let counts = model.counts("Document");
    assert_eq!(
        (
            counts.own,
            counts.from_mixins,
            counts.attributes,
            counts.operations,
            counts.constructor
        ),
        (2, 0, 1, 1, true)
    );
    let names: Vec<_> = model
        .members("Document")
        .iter()
        .filter_map(|m| m.name.as_deref())
        .collect();
    assert_eq!(names, ["title", "getElementById"]);
}

fn added_argument_name_keyword() {
    let member = member(
        "interface EventTarget {\n  undefined f(DOMString interface, EventListener? callback, DOMString? namespace);\n};",
    );
    let names: Vec<_> = arguments(&member).iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["interface", "callback", "namespace"]);
}

fn added_short() {
    let member = member("interface Range {\n  short comparePoint(Node node);\n};");
    let MemberKind::Operation { return_type, .. } = &member.kind else {
        panic!("not an operation");
    };
    assert_eq!(
        return_type.kind,
        TypeKind::Integer {
            unsigned: false,
            size: IntegerSize::Short
        }
    );
}

fn added_buffer_related_type() {
    let definition =
        one("typedef (ArrayBuffer or SharedArrayBuffer or DataView or Float16Array) B;");
    let DefinitionKind::Typedef { ty } = &definition.kind else {
        panic!("not a typedef");
    };
    let TypeKind::Union(list) = &ty.kind else {
        panic!("not a union");
    };
    let names: Vec<_> = list
        .iter()
        .map(|member| match &member.kind {
            TypeKind::Buffer(name) => name.as_str(),
            other => panic!("not a buffer related type: {other:?}"),
        })
        .collect();
    assert_eq!(
        names,
        [
            "ArrayBuffer",
            "SharedArrayBuffer",
            "DataView",
            "Float16Array"
        ]
    );
    assert!(model(vec![definition]).unresolved().is_empty());
}

fn added_serializable() {
    let definition = one("[Exposed=*, Serializable]\ninterface DOMException {};");
    assert_eq!(definition.extended_attributes[1].name, "Serializable");
}

fn added_allow_shared() {
    let definition = one("typedef (ArrayBuffer or [AllowShared] ArrayBufferView) S;");
    let DefinitionKind::Typedef { ty } = &definition.kind else {
        panic!("not a typedef");
    };
    let TypeKind::Union(list) = &ty.kind else {
        panic!("not a union");
    };
    assert_eq!(list[1].extended_attributes[0].name, "AllowShared");
}

#[test]
fn unresolved_names_carry_the_line_they_are_written_on() {
    let model = model(all(
        "interface A\n  : Missing {\n};\ndictionary D\n\n  : Absent {};\nA\n  includes\n  Gone;\n",
    ));
    let lines: Vec<(String, usize)> = model
        .unresolved()
        .into_iter()
        .map(|unresolved| (unresolved.name, unresolved.line))
        .collect();
    assert_eq!(
        lines,
        [
            ("Absent".to_string(), 6),
            ("Gone".to_string(), 9),
            ("Missing".to_string(), 2)
        ]
    );
}

/// One case per entry of [`ADDED`]: the entry and the function that tests it.
const ADDED_CASES: &[(&str, fn())] = &[
    ("argument name keyword", added_argument_name_keyword),
    ("short", added_short),
    ("buffer related type", added_buffer_related_type),
    ("[Serializable]", added_serializable),
    ("[AllowShared]", added_allow_shared),
];

#[test]
fn added_cases() {
    let names: Vec<&str> = ADDED_CASES.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ADDED);
    for (_, case) in ADDED_CASES {
        case();
    }
}
