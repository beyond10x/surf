//! The model the reader builds: definitions, members, arguments, types and extended attributes,
//! and [`Model`], the merged view over all definitions read.

use std::collections::{BTreeMap, BTreeSet};

use crate::ReadError;

/// One definition of an IDL text, with the file and line it starts on (the line of its keyword,
/// or of its first name for `includes`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    /// The name it defines; for `includes`, the interface that includes.
    pub name: String,
    /// The file named to [`crate::read`].
    pub file: String,
    /// The line of its first token after its extended attributes: a line of the extracted IDL
    /// index, counted from 1, not a line of the page.
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
        /// The interface it inherits from, with the line its name is written on.
        inherits: Option<Reference>,
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
        /// The dictionary it inherits from, with the line its name is written on.
        inherits: Option<Reference>,
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
        /// The mixin, with the line its name is written on.
        mixin: Reference,
    },
}

/// A member of an interface, an interface mixin or a callback interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// Its name; `None` for a constructor, a bare `stringifier` and `iterable`.
    pub name: Option<String>,
    /// The line of its first token after its extended attributes: a line of the extracted IDL
    /// index, counted from 1, not a line of the page.
    pub line: usize,
    /// The extended attributes before it; a name of [`crate::TYPE_EXTENDED_ATTRIBUTES`] there is
    /// refused.
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
        /// The type, with the extended attributes written on it (`attribute [X] T`).
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
    /// The line of its name: a line of the extracted IDL index, counted from 1, not a line of
    /// the page.
    pub line: usize,
    /// The extended attributes before it (before `optional`, or before the type), but those of
    /// [`crate::TYPE_EXTENDED_ATTRIBUTES`], which are on [`Argument::ty`].
    pub extended_attributes: Vec<ExtendedAttribute>,
    /// The type, with the extended attributes written after `optional` and the names of
    /// [`crate::TYPE_EXTENDED_ATTRIBUTES`] written before the argument.
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
    /// The line of its first token after its extended attributes: a line of the extracted IDL
    /// index, counted from 1, not a line of the page.
    pub line: usize,
    /// The extended attributes before it, but those of [`crate::TYPE_EXTENDED_ATTRIBUTES`],
    /// which are on [`DictionaryMember::ty`].
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
        /// The line it is written on: a line of the extracted IDL index, counted from 1, not a
        /// line of the page.
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
    /// The line of its name: a line of the extracted IDL index, counted from 1, not a line of
    /// the page.
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

/// A name a definition refers to (the one it inherits from, the mixin of `includes`) and the
/// line it is written on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    /// The name, a leading `_` removed.
    pub name: String,
    /// The line the name is written on: a line of the extracted IDL index, counted from 1, not
    /// a line of the page.
    pub line: usize,
}

/// A name a type, an inheritance or an `includes` names that no definition read defines.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unresolved {
    /// The name.
    pub name: String,
    /// The file it is written in.
    pub file: String,
    /// The line it is written on: a line of the extracted IDL index, counted from 1, not a line
    /// of the page.
    pub line: usize,
}

/// The definitions of one or more IDL texts, read together.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    definitions: Vec<Definition>,
    /// For each name a definition defines ([`defines`]), the index of that definition.
    defined: BTreeMap<String, usize>,
    /// For each name, the indices of every definition of that name, in the order given.
    named: BTreeMap<String, Vec<usize>>,
}

/// Whether `definition` defines its name: all but a `partial interface` and `includes`.
fn defines(definition: &Definition) -> bool {
    !matches!(
        definition.kind,
        DefinitionKind::Includes { .. } | DefinitionKind::Interface { partial: true, .. }
    )
}

/// What a definition is, as the refusals name it.
fn kind_name(kind: &DefinitionKind) -> &'static str {
    match kind {
        DefinitionKind::Dictionary { .. } => "dictionary",
        _ => "interface",
    }
}

impl Model {
    /// The model of `definitions`, in the order given. It builds one map from each name to its
    /// definitions, then refuses, in this order, each check over all definitions before the
    /// next:
    /// 1. a name two definitions define, naming the second and the first;
    /// 2. a `partial interface` of a name defined as something other than an interface;
    /// 3. `A includes B;` where `B` is defined as no interface mixin or `A` as no interface;
    /// 4. an interface or a dictionary that inherits from a definition of another kind, then a
    ///    cycle of inheritance of any length;
    /// 5. a name twice among the members of an interface merged with its partials and the
    ///    mixins it includes ([`Model::members`]): two operations are an overload; an
    ///    operation, an attribute or a constant of one name with any other member of that
    ///    name is a second member.
    ///
    /// A name no definition defines is not refused; [`Model::unresolved`] lists it.
    pub fn new(definitions: Vec<Definition>) -> Result<Model, ReadError> {
        let mut model = Model {
            definitions,
            defined: BTreeMap::new(),
            named: BTreeMap::new(),
        };
        model.index()?;
        model.check_partials()?;
        model.check_includes()?;
        model.check_inheritance()?;
        model.check_members()?;
        Ok(model)
    }

    /// Builds [`Model::defined`] and [`Model::named`]; refuses a name that two definitions
    /// define, naming the second and the first.
    fn index(&mut self) -> Result<(), ReadError> {
        for (index, definition) in self.definitions.iter().enumerate() {
            self.named
                .entry(definition.name.clone())
                .or_default()
                .push(index);
            if !defines(definition) {
                continue;
            }
            if let Some(&first) = self.defined.get(&definition.name) {
                let earlier = &self.definitions[first];
                return Err(ReadError::new(
                    &definition.file,
                    definition.line,
                    format!(
                        "a second definition of `{}`, the first at {}:{}",
                        definition.name, earlier.file, earlier.line
                    ),
                ));
            }
            self.defined.insert(definition.name.clone(), index);
        }
        Ok(())
    }

    /// The definition that defines `name`, if one does.
    fn definition_of(&self, name: &str) -> Option<&Definition> {
        self.defined
            .get(name)
            .map(|&index| &self.definitions[index])
    }

    /// Refuses a `partial interface D` where `D` is defined as something other than an
    /// interface, naming the partial and the definition.
    fn check_partials(&self) -> Result<(), ReadError> {
        for partial in &self.definitions {
            if !matches!(
                partial.kind,
                DefinitionKind::Interface { partial: true, .. }
            ) {
                continue;
            }
            if let Some(full) = self.definition_of(&partial.name)
                && !matches!(full.kind, DefinitionKind::Interface { .. })
            {
                return Err(ReadError::new(
                    &partial.file,
                    partial.line,
                    format!(
                        "`partial interface {name}`, where `{name}` is no interface, defined at {}:{}",
                        full.file,
                        full.line,
                        name = partial.name
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Refuses `A includes B;` where `B` is defined and is no interface mixin, or `A` is defined
    /// and is no interface (a `partial interface` defines no name), naming the statement. An
    /// undefined name is left to [`Model::unresolved`].
    fn check_includes(&self) -> Result<(), ReadError> {
        for statement in &self.definitions {
            let DefinitionKind::Includes { mixin } = &statement.kind else {
                continue;
            };
            let refuse = |what: String| {
                Err(ReadError::new(
                    &statement.file,
                    statement.line,
                    format!("`{} includes {}`, where {what}", statement.name, mixin.name),
                ))
            };
            if let Some(definition) = self.definition_of(&mixin.name)
                && !matches!(definition.kind, DefinitionKind::InterfaceMixin { .. })
            {
                return refuse(format!("`{}` is no interface mixin", mixin.name));
            }
            if let Some(definition) = self.definition_of(&statement.name)
                && !matches!(definition.kind, DefinitionKind::Interface { .. })
            {
                return refuse(format!("`{}` is no interface", statement.name));
            }
        }
        Ok(())
    }

    /// The index of the definition `definitions[index]` inherits from, if it inherits and that
    /// name is defined.
    fn parent_of(&self, index: usize) -> Option<usize> {
        match &self.definitions[index].kind {
            DefinitionKind::Interface {
                inherits: Some(parent),
                ..
            }
            | DefinitionKind::Dictionary {
                inherits: Some(parent),
                ..
            } => self.defined.get(&parent.name).copied(),
            _ => None,
        }
    }

    /// Refuses an interface or a dictionary whose parent is defined as another kind, naming
    /// both; then a cycle of inheritance, naming each definition of the cycle from the one
    /// read first. Each chain is walked once, in a loop.
    fn check_inheritance(&self) -> Result<(), ReadError> {
        for (index, child) in self.definitions.iter().enumerate() {
            let Some(parent) = self
                .parent_of(index)
                .map(|parent| &self.definitions[parent])
            else {
                continue;
            };
            let same = matches!(
                (&child.kind, &parent.kind),
                (
                    DefinitionKind::Interface { .. },
                    DefinitionKind::Interface { .. }
                ) | (
                    DefinitionKind::Dictionary { .. },
                    DefinitionKind::Dictionary { .. }
                )
            );
            if !same {
                return Err(ReadError::new(
                    &child.file,
                    child.line,
                    format!(
                        "`{} : {}`, where `{}` is no {}, defined at {}:{}",
                        child.name,
                        parent.name,
                        parent.name,
                        kind_name(&child.kind),
                        parent.file,
                        parent.line
                    ),
                ));
            }
        }
        // 0: not walked yet; 1: on the chain being walked; 2: its chain ends without a cycle.
        let mut state = vec![0u8; self.definitions.len()];
        for start in 0..self.definitions.len() {
            let mut chain = Vec::new();
            let mut at = Some(start);
            while let Some(index) = at {
                match state[index] {
                    0 => {
                        state[index] = 1;
                        chain.push(index);
                        at = self.parent_of(index);
                    }
                    1 => {
                        let from = chain
                            .iter()
                            .position(|&on| on == index)
                            .expect("on the chain");
                        return Err(self.cycle(&chain[from..]));
                    }
                    _ => break,
                }
            }
            for index in chain {
                state[index] = 2;
            }
        }
        Ok(())
    }

    /// The refusal of the inheritance cycle `cycle` (each inherits from the next, the last
    /// from the first), named from the definition read first.
    fn cycle(&self, cycle: &[usize]) -> ReadError {
        let first = (0..cycle.len())
            .min_by_key(|&at| cycle[at])
            .expect("a cycle has a definition");
        let mut names = Vec::new();
        for step in 0..cycle.len() {
            let definition = &self.definitions[cycle[(first + step) % cycle.len()]];
            names.push(format!(
                "`{}` at {}:{}",
                definition.name, definition.file, definition.line
            ));
        }
        let head = &self.definitions[cycle[first]];
        ReadError::new(
            &head.file,
            head.line,
            format!(
                "a cycle of inheritance: {}, then `{}` again",
                names.join(", "),
                head.name
            ),
        )
    }

    /// Refuses a name twice among the merged members of an interface (its definitions and
    /// the mixins it includes, [`Model::members`]), naming the file and line of the second:
    /// two operations as an overload, any other pair as a second member with the first's
    /// place.
    fn check_members(&self) -> Result<(), ReadError> {
        let mut interfaces: Vec<&str> = Vec::new();
        let mut seen = BTreeSet::new();
        for definition in &self.definitions {
            if matches!(
                definition.kind,
                DefinitionKind::Interface { .. } | DefinitionKind::Includes { .. }
            ) && seen.insert(definition.name.as_str())
            {
                interfaces.push(&definition.name);
            }
        }
        for interface in interfaces {
            let (own, mixins) = self.located_parts(interface);
            let mut first: BTreeMap<&str, (&str, &Member)> = BTreeMap::new();
            for (file, member) in own.into_iter().chain(mixins) {
                let Some(name) = &member.name else {
                    continue;
                };
                let Some((first_file, earlier)) = first.get(name.as_str()) else {
                    first.insert(name, (file, member));
                    continue;
                };
                let operation =
                    |member: &Member| matches!(member.kind, MemberKind::Operation { .. });
                let construct = if operation(earlier) && operation(member) {
                    format!("an overload: a second operation `{name}` in `{interface}`")
                } else {
                    format!(
                        "a second member `{name}` in `{interface}`, the first at {first_file}:{}",
                        earlier.line
                    )
                };
                return Err(ReadError::new(file, member.line, construct));
            }
        }
        Ok(())
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
        let (own, mixins) = self.located_parts(interface);
        fn strip(list: Located<'_>) -> Vec<&Member> {
            list.into_iter().map(|(_, member)| member).collect()
        }
        (strip(own), strip(mixins))
    }

    /// The own and the mixin members of `interface`, each with the file of its definition.
    fn located_parts(&self, interface: &str) -> (Located<'_>, Located<'_>) {
        let mut own = Vec::new();
        let mut mixins = Vec::new();
        for definition in self.all_named(interface) {
            match &definition.kind {
                DefinitionKind::Interface { members, .. } => own.extend(
                    members
                        .iter()
                        .map(|member| (definition.file.as_str(), member)),
                ),
                DefinitionKind::Includes { mixin } => {
                    mixins.extend(self.mixin_members(&mixin.name))
                }
                _ => {}
            }
        }
        (own, mixins)
    }

    /// Every definition of `name`, in the order given, found through [`Model::named`].
    fn all_named(&self, name: &str) -> impl Iterator<Item = &Definition> {
        self.named
            .get(name)
            .into_iter()
            .flatten()
            .map(|&index| &self.definitions[index])
    }

    fn mixin_members(&self, mixin: &str) -> Vec<(&str, &Member)> {
        self.all_named(mixin)
            .flat_map(|definition| {
                let members = match &definition.kind {
                    DefinitionKind::InterfaceMixin { members } => members.as_slice(),
                    _ => &[],
                };
                members
                    .iter()
                    .map(move |member| (definition.file.as_str(), member))
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
                } => names.push((parent.name.clone(), parent.line)),
                DefinitionKind::Includes { mixin } => {
                    names.push((definition.name.clone(), definition.line));
                    names.push((mixin.name.clone(), mixin.line));
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
                if !self.defined.contains_key(&name) {
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

/// Members, each with the file of the definition that holds it.
type Located<'a> = Vec<(&'a str, &'a Member)>;

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
