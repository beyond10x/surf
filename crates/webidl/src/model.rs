//! The model the reader builds: definitions, members, arguments, types and extended attributes,
//! and [`Model`], the merged view over all definitions read.

use std::collections::{BTreeMap, BTreeSet};

/// One definition of an IDL text, with the file and line it starts on (the line of its keyword,
/// or of its first name for `includes`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    /// The name it defines; for `includes`, the interface that includes.
    pub name: String,
    /// The file named to [`crate::read`].
    pub file: String,
    /// The line, counted from 1 in the text read.
    pub line: usize,
    /// The extended attributes before it.
    pub extended_attributes: Vec<ExtendedAttribute>,
    /// What it is.
    pub kind: DefinitionKind,
}

/// What a definition is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefinitionKind {
    /// `interface` or `partial interface`, with its inheritance (none for a partial).
    Interface {
        /// Whether it is a `partial interface`.
        partial: bool,
        /// The interface it inherits from.
        inherits: Option<String>,
        /// Its members.
        members: Vec<Member>,
    },
    /// `interface mixin`.
    InterfaceMixin {
        /// Its members.
        members: Vec<Member>,
    },
    /// `callback interface`.
    CallbackInterface {
        /// Its members.
        members: Vec<Member>,
    },
    /// `callback <name> = <return type> (<arguments>)`.
    Callback {
        /// The return type.
        return_type: Type,
        /// The arguments.
        arguments: Vec<Argument>,
    },
    /// `dictionary`, with its inheritance.
    Dictionary {
        /// The dictionary it inherits from.
        inherits: Option<String>,
        /// Its members.
        members: Vec<DictionaryMember>,
    },
    /// `enum`, with its values without quotes.
    Enum {
        /// The values.
        values: Vec<String>,
    },
    /// `typedef`.
    Typedef {
        /// The type it names.
        ty: Type,
    },
    /// `<interface> includes <mixin>`; the interface is [`Definition::name`].
    Includes {
        /// The mixin.
        mixin: String,
    },
}

/// A member of an interface, an interface mixin or a callback interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// Its name; `None` for a constructor, a bare `stringifier` and `iterable`.
    pub name: Option<String>,
    /// The line of its first token after its extended attributes.
    pub line: usize,
    /// The extended attributes before it.
    pub extended_attributes: Vec<ExtendedAttribute>,
    /// What it is.
    pub kind: MemberKind,
}

/// What a member is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberKind {
    /// `const`.
    Const {
        /// The type.
        ty: Type,
        /// The value.
        value: ConstValue,
    },
    /// `attribute`, `readonly attribute` or `stringifier attribute`.
    Attribute {
        /// The type, with the extended attributes written before it.
        ty: Type,
        /// Whether it is `readonly`.
        readonly: bool,
        /// Whether it is a `stringifier` attribute.
        stringifier: bool,
    },
    /// An operation: regular, `static` or a special `getter`.
    Operation {
        /// The return type.
        return_type: Type,
        /// The arguments.
        arguments: Vec<Argument>,
        /// Whether it is `static`.
        is_static: bool,
        /// Whether it is a special `getter` operation.
        getter: bool,
    },
    /// A bare `stringifier;`.
    Stringifier,
    /// `constructor(<arguments>)`.
    Constructor {
        /// The arguments.
        arguments: Vec<Argument>,
    },
    /// `iterable<T>`.
    Iterable {
        /// The value type.
        ty: Type,
    },
}

/// An argument of an operation, a constructor or a callback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument {
    /// Its name, a leading `_` removed.
    pub name: String,
    /// The line of its name.
    pub line: usize,
    /// The extended attributes before it (before `optional`, or before the type).
    pub extended_attributes: Vec<ExtendedAttribute>,
    /// The type; for `optional`, with the extended attributes written after `optional`.
    pub ty: Type,
    /// Whether it is `optional`.
    pub optional: bool,
    /// Its default value, for an optional argument that has one.
    pub default: Option<DefaultValue>,
    /// Whether it is variadic (`T... name`).
    pub variadic: bool,
}

/// A member of a dictionary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryMember {
    /// Its name.
    pub name: String,
    /// The line of its first token after its extended attributes.
    pub line: usize,
    /// The extended attributes before it.
    pub extended_attributes: Vec<ExtendedAttribute>,
    /// The type.
    pub ty: Type,
    /// Whether it is `required`.
    pub required: bool,
    /// Its default value.
    pub default: Option<DefaultValue>,
}

/// A type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Type {
    /// What type it is.
    pub kind: TypeKind,
    /// Whether it is nullable (`T?`).
    pub nullable: bool,
    /// The extended attributes on the type (`[LegacyNullToEmptyString] DOMString`).
    pub extended_attributes: Vec<ExtendedAttribute>,
}

/// What type a [`Type`] is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeKind {
    /// `any`.
    Any,
    /// `undefined`.
    Undefined,
    /// `boolean`.
    Boolean,
    /// `object`.
    Object,
    /// `short`, `unsigned short`, `unsigned long` or `unsigned long long`.
    Integer {
        /// Whether it is `unsigned`.
        unsigned: bool,
        /// Its size.
        size: IntegerSize,
    },
    /// `double` or `unrestricted double`.
    Double {
        /// Whether it is `unrestricted`.
        unrestricted: bool,
    },
    /// `DOMString`.
    DomString,
    /// `USVString`.
    UsvString,
    /// A buffer related type (`ArrayBuffer`, `DataView`, `Int8Array`, ...), by its name.
    Buffer(String),
    /// `sequence<T>`.
    Sequence(Box<Type>),
    /// A union, `(A or B ...)`, of its member types.
    Union(Vec<Type>),
    /// A name: an interface, dictionary, enum, typedef or callback, or a name no definition
    /// read defines ([`Model::unresolved`]).
    Named {
        /// The name, a leading `_` removed.
        name: String,
        /// The line it is written on.
        line: usize,
    },
}

/// The size of an integer type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerSize {
    /// `short`.
    Short,
    /// `long`.
    Long,
    /// `long long`.
    LongLong,
}

/// The value of a `const`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstValue {
    /// `true` or `false`.
    Boolean(bool),
    /// An integer, decimal, hexadecimal or octal.
    Integer(i64),
}

/// A default value of an optional argument or a dictionary member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefaultValue {
    /// `true` or `false`.
    Boolean(bool),
    /// An integer, decimal, hexadecimal or octal.
    Integer(i64),
    /// A string, without its quotes.
    String(String),
    /// `null`.
    Null,
    /// `{}`.
    EmptyDictionary,
}

/// An extended attribute, `[Name]`, `[Name=Value]`, `[Name=*]` or `[Name=(A, B)]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtendedAttribute {
    /// Its name, one of [`crate::EXTENDED_ATTRIBUTES`].
    pub name: String,
    /// The line of its name.
    pub line: usize,
    /// Its value.
    pub value: ExtendedAttributeValue,
}

/// The value of an extended attribute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtendedAttributeValue {
    /// None: `[Name]`.
    None,
    /// `[Name=Identifier]`.
    Identifier(String),
    /// `[Name=*]`.
    Wildcard,
    /// `[Name=(A, B)]`.
    IdentifierList(Vec<String>),
}

/// The member counts of an interface over its merged view ([`Model::members`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Members of the interface and its partials.
    pub own: usize,
    /// Members of the mixins it includes, each mixin counted once per `includes`.
    pub from_mixins: usize,
    /// Constants, own and from mixins.
    pub constants: usize,
    /// Attributes, own and from mixins.
    pub attributes: usize,
    /// Operations (regular, `static`, `getter`) and bare stringifiers, own and from mixins.
    pub operations: usize,
    /// Whether it has a constructor.
    pub constructor: bool,
}

/// A name a type, an inheritance or an `includes` names that no definition read defines.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unresolved {
    /// The name.
    pub name: String,
    /// The file it is written in.
    pub file: String,
    /// The line it is written on.
    pub line: usize,
}

/// The definitions of one or more IDL texts, read together.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    definitions: Vec<Definition>,
}

impl Model {
    /// The model of `definitions`, in the order given.
    pub fn new(definitions: Vec<Definition>) -> Model {
        Model { definitions }
    }

    /// The definitions, in the order given.
    pub fn definitions(&self) -> &[Definition] {
        &self.definitions
    }

    /// The members of `interface` merged: those of its definitions (`interface` and every
    /// `partial interface` of that name, in the order read), then those of each mixin it
    /// includes, in the order of the `includes` statements. Constructors and `iterable` are
    /// among them; [`Counts`] does not count them.
    pub fn members(&self, interface: &str) -> Vec<&Member> {
        let (own, mixins) = self.member_parts(interface);
        own.into_iter().chain(mixins).collect()
    }

    fn member_parts(&self, interface: &str) -> (Vec<&Member>, Vec<&Member>) {
        let mut own = Vec::new();
        let mut mixins = Vec::new();
        for definition in &self.definitions {
            if definition.name != interface {
                continue;
            }
            match &definition.kind {
                DefinitionKind::Interface { members, .. } => own.extend(members),
                DefinitionKind::Includes { mixin } => mixins.extend(self.mixin_members(mixin)),
                _ => {}
            }
        }
        (own, mixins)
    }

    fn mixin_members(&self, mixin: &str) -> Vec<&Member> {
        self.definitions
            .iter()
            .filter(|definition| definition.name == mixin)
            .flat_map(|definition| match &definition.kind {
                DefinitionKind::InterfaceMixin { members } => members.as_slice(),
                _ => &[],
            })
            .collect()
    }

    /// The counts of `interface`'s merged members. A constructor and `iterable` are not
    /// members; a member of a mixin counts on each interface that includes it.
    pub fn counts(&self, interface: &str) -> Counts {
        let (own, mixins) = self.member_parts(interface);
        let mut counts = Counts::default();
        for (member, from_mixin) in own
            .iter()
            .map(|member| (member, false))
            .chain(mixins.iter().map(|member| (member, true)))
        {
            match member.kind {
                MemberKind::Constructor { .. } => {
                    counts.constructor = true;
                    continue;
                }
                MemberKind::Iterable { .. } => continue,
                MemberKind::Const { .. } => counts.constants += 1,
                MemberKind::Attribute { .. } => counts.attributes += 1,
                MemberKind::Operation { .. } | MemberKind::Stringifier => counts.operations += 1,
            }
            if from_mixin {
                counts.from_mixins += 1;
            } else {
                counts.own += 1;
            }
        }
        counts
    }

    /// How often each extended attribute is written in the definitions read from `file`: on
    /// definitions, members, arguments, dictionary members and types, nested types included.
    pub fn extended_attribute_counts(&self, file: &str) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for definition in self.definitions.iter().filter(|d| d.file == file) {
            visit(definition, &mut |seen| {
                let list = match seen {
                    Seen::List(list) => list,
                    Seen::Type(ty) => &ty.extended_attributes,
                };
                for attribute in list {
                    *counts.entry(attribute.name.clone()).or_insert(0) += 1;
                }
            });
        }
        counts
    }

    /// Every name a type, an inheritance or an `includes` statement names that no definition
    /// read defines (a `partial interface` and an `includes` define no name), one entry per
    /// place it is written, sorted by name, file and line.
    pub fn unresolved(&self) -> Vec<Unresolved> {
        let defined: BTreeSet<&str> = self
            .definitions
            .iter()
            .filter(|definition| {
                !matches!(
                    definition.kind,
                    DefinitionKind::Includes { .. }
                        | DefinitionKind::Interface { partial: true, .. }
                )
            })
            .map(|definition| definition.name.as_str())
            .collect();
        let mut out = Vec::new();
        for definition in &self.definitions {
            let mut names: Vec<(String, usize)> = Vec::new();
            match &definition.kind {
                DefinitionKind::Interface {
                    inherits: Some(parent),
                    ..
                }
                | DefinitionKind::Dictionary {
                    inherits: Some(parent),
                    ..
                } => names.push((parent.clone(), definition.line)),
                DefinitionKind::Includes { mixin } => {
                    names.push((definition.name.clone(), definition.line));
                    names.push((mixin.clone(), definition.line));
                }
                _ => {}
            }
            visit(definition, &mut |seen| {
                if let Seen::Type(Type {
                    kind: TypeKind::Named { name, line },
                    ..
                }) = seen
                {
                    names.push((name.clone(), *line));
                }
            });
            for (name, line) in names {
                if !defined.contains(name.as_str()) {
                    out.push(Unresolved {
                        name,
                        file: definition.file.clone(),
                        line,
                    });
                }
            }
        }
        out.sort();
        out
    }
}

/// What [`visit`] hands its callback.
enum Seen<'a> {
    /// The extended attributes of a definition, a member, an argument or a dictionary member.
    List(&'a [ExtendedAttribute]),
    /// A type, nested types included.
    Type(&'a Type),
}

/// Calls `on` for the extended attributes of `definition` and of each of its members,
/// arguments and dictionary members, and for each of its types, nested types included.
fn visit<'a>(definition: &'a Definition, on: &mut dyn FnMut(Seen<'a>)) {
    on(Seen::List(&definition.extended_attributes));
    match &definition.kind {
        DefinitionKind::Interface { members, .. }
        | DefinitionKind::InterfaceMixin { members }
        | DefinitionKind::CallbackInterface { members } => {
            for member in members {
                on(Seen::List(&member.extended_attributes));
                match &member.kind {
                    MemberKind::Const { ty, .. }
                    | MemberKind::Attribute { ty, .. }
                    | MemberKind::Iterable { ty } => visit_type(ty, on),
                    MemberKind::Operation {
                        return_type,
                        arguments,
                        ..
                    } => {
                        visit_type(return_type, on);
                        visit_arguments(arguments, on);
                    }
                    MemberKind::Constructor { arguments } => visit_arguments(arguments, on),
                    MemberKind::Stringifier => {}
                }
            }
        }
        DefinitionKind::Callback {
            return_type,
            arguments,
        } => {
            visit_type(return_type, on);
            visit_arguments(arguments, on);
        }
        DefinitionKind::Dictionary { members, .. } => {
            for member in members {
                on(Seen::List(&member.extended_attributes));
                visit_type(&member.ty, on);
            }
        }
        DefinitionKind::Typedef { ty } => visit_type(ty, on),
        DefinitionKind::Enum { .. } | DefinitionKind::Includes { .. } => {}
    }
}

fn visit_arguments<'a>(arguments: &'a [Argument], on: &mut dyn FnMut(Seen<'a>)) {
    for argument in arguments {
        on(Seen::List(&argument.extended_attributes));
        visit_type(&argument.ty, on);
    }
}

fn visit_type<'a>(ty: &'a Type, on: &mut dyn FnMut(Seen<'a>)) {
    on(Seen::Type(ty));
    match &ty.kind {
        TypeKind::Sequence(inner) => visit_type(inner, on),
        TypeKind::Union(members) => {
            for member in members {
                visit_type(member, on);
            }
        }
        _ => {}
    }
}
