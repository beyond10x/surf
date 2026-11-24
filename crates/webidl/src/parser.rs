//! The parser of the definitions: the constructs of the crate documentation, each other one
//! refused with its file and line.

use std::collections::BTreeSet;

use crate::lexer::{Kind, Token};
use crate::model::{
    Argument, ConstValue, DefaultValue, Definition, DefinitionKind, DictionaryMember,
    ExtendedAttribute, ExtendedAttributeValue, IntegerSize, Member, MemberKind, Reference, Type,
    TypeKind,
};
use crate::{EXTENDED_ATTRIBUTES, ReadError, TYPE_EXTENDED_ATTRIBUTES};

/// The keywords of the grammar of the Web IDL Standard: an identifier token with one of these
/// texts is no name.
const KEYWORDS: &[&str] = &[
    "ArrayBuffer",
    "BigInt64Array",
    "BigUint64Array",
    "ByteString",
    "DOMString",
    "DataView",
    "Float16Array",
    "Float32Array",
    "Float64Array",
    "FrozenArray",
    "Infinity",
    "Int16Array",
    "Int32Array",
    "Int8Array",
    "NaN",
    "ObservableArray",
    "Promise",
    "SharedArrayBuffer",
    "USVString",
    "Uint16Array",
    "Uint32Array",
    "Uint8Array",
    "Uint8ClampedArray",
    "any",
    "async_iterable",
    "async_sequence",
    "attribute",
    "bigint",
    "boolean",
    "byte",
    "callback",
    "const",
    "constructor",
    "deleter",
    "dictionary",
    "double",
    "enum",
    "false",
    "float",
    "getter",
    "includes",
    "inherit",
    "interface",
    "iterable",
    "long",
    "maplike",
    "mixin",
    "namespace",
    "null",
    "object",
    "octet",
    "optional",
    "or",
    "partial",
    "readonly",
    "record",
    "required",
    "sequence",
    "setlike",
    "setter",
    "short",
    "static",
    "stringifier",
    "symbol",
    "true",
    "typedef",
    "undefined",
    "unrestricted",
    "unsigned",
];

/// The keywords the grammar's `ArgumentNameKeyword` allows as the name of an argument, read as
/// written.
const ARGUMENT_NAME_KEYWORDS: &[&str] = &[
    "attribute",
    "callback",
    "const",
    "constructor",
    "deleter",
    "dictionary",
    "enum",
    "getter",
    "includes",
    "inherit",
    "interface",
    "iterable",
    "maplike",
    "mixin",
    "namespace",
    "partial",
    "readonly",
    "required",
    "setlike",
    "setter",
    "static",
    "stringifier",
    "typedef",
    "unrestricted",
];

/// The most levels a type nests: a named type alone is 1 level, `sequence<DOMString>` and
/// `(A or B)` are 2. Generic arguments and union members count alike. The vendored pages nest
/// 2 deep; a deeper type is refused, so the recursion of the type parser stays this shallow.
pub const MAX_TYPE_DEPTH: usize = 32;

/// The grammar's `BufferRelatedType`s (`ADDED`: "buffer related type").
const BUFFER_RELATED_TYPES: &[&str] = &[
    "ArrayBuffer",
    "SharedArrayBuffer",
    "DataView",
    "Int8Array",
    "Int16Array",
    "Int32Array",
    "Uint8Array",
    "Uint16Array",
    "Uint32Array",
    "Uint8ClampedArray",
    "BigInt64Array",
    "BigUint64Array",
    "Float16Array",
    "Float32Array",
    "Float64Array",
];

/// Where a list of members is read: what each kind of body allows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Body {
    Interface,
    Mixin,
    CallbackInterface,
}

impl Body {
    fn name(self) -> &'static str {
        match self {
            Body::Interface => "an interface",
            Body::Mixin => "an interface mixin",
            Body::CallbackInterface => "a callback interface",
        }
    }
}

pub(crate) struct Parser<'a> {
    tokens: Vec<Token>,
    at: usize,
    /// How many types are open around the one being read ([`MAX_TYPE_DEPTH`]).
    depth: usize,
    file: &'a str,
}

/// The definitions of `tokens`.
pub(crate) fn parse(tokens: Vec<Token>, file: &str) -> Result<Vec<Definition>, ReadError> {
    let mut parser = Parser {
        tokens,
        at: 0,
        depth: 0,
        file,
    };
    let mut definitions = Vec::new();
    while parser.at < parser.tokens.len() {
        definitions.push(parser.definition()?);
    }
    Ok(definitions)
}

impl Parser<'_> {
    fn peek(&self, ahead: usize) -> Option<&Token> {
        self.tokens.get(self.at + ahead)
    }

    /// Whether the token `ahead` of the next is the word or punctuation `text`.
    fn is(&self, ahead: usize, text: &str) -> bool {
        self.peek(ahead).is_some_and(|token| {
            token.text == text && matches!(token.kind, Kind::Identifier | Kind::Other)
        })
    }

    /// The line of the next token, or of the last one at the end of the text.
    fn line(&self) -> usize {
        self.peek(0)
            .or(self.tokens.last())
            .map_or(1, |token| token.line)
    }

    fn refuse<T>(&self, line: usize, construct: impl Into<String>) -> Result<T, ReadError> {
        Err(ReadError::new(self.file, line, construct))
    }

    /// The error for the next token where `expected` was expected.
    fn unexpected<T>(&self, expected: &str) -> Result<T, ReadError> {
        match self.peek(0) {
            Some(token) => self.refuse(
                token.line,
                format!("`{}` where {expected} was expected", token.text),
            ),
            None => self.refuse(
                self.line(),
                format!("the end of the text where {expected} was expected"),
            ),
        }
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        self.at += 1;
        token
    }

    fn expect(&mut self, text: &str) -> Result<usize, ReadError> {
        if self.is(0, text) {
            let line = self.line();
            self.at += 1;
            Ok(line)
        } else {
            self.unexpected(&format!("`{text}`"))
        }
    }

    fn eat(&mut self, text: &str) -> bool {
        let found = self.is(0, text);
        if found {
            self.at += 1;
        }
        found
    }

    /// A name: an identifier that is no keyword, its leading `_` removed.
    fn identifier(&mut self, what: &str) -> Result<(String, usize), ReadError> {
        match self.peek(0) {
            Some(token)
                if token.kind == Kind::Identifier
                    && !token.text.starts_with('-')
                    && !KEYWORDS.contains(&token.text.as_str()) =>
            {
                let token = self.next().expect("peeked");
                let name = token.text.strip_prefix('_').unwrap_or(&token.text);
                Ok((name.to_string(), token.line))
            }
            _ => self.unexpected(what),
        }
    }

    fn definition(&mut self) -> Result<Definition, ReadError> {
        let extended_attributes = self.extended_attributes()?;
        let line = self.line();
        let (name, kind) = if self.is(0, "callback") && self.is(1, "interface") {
            self.at += 2;
            let (name, _) = self.identifier("the name of a callback interface")?;
            let members = self.body(Body::CallbackInterface, &name)?;
            (name, DefinitionKind::CallbackInterface { members })
        } else if self.eat("callback") {
            let (name, _) = self.identifier("the name of a callback")?;
            self.expect("=")?;
            let return_type = self.ty()?;
            let arguments = self.arguments()?;
            self.expect(";")?;
            (
                name,
                DefinitionKind::Callback {
                    return_type,
                    arguments,
                },
            )
        } else if self.is(0, "interface") && self.is(1, "mixin") {
            self.at += 2;
            let (name, _) = self.identifier("the name of an interface mixin")?;
            let members = self.body(Body::Mixin, &name)?;
            (name, DefinitionKind::InterfaceMixin { members })
        } else if self.eat("interface") {
            let (name, _) = self.identifier("the name of an interface")?;
            let inherits = self.inheritance()?;
            let members = self.body(Body::Interface, &name)?;
            (
                name,
                DefinitionKind::Interface {
                    partial: false,
                    inherits,
                    members,
                },
            )
        } else if self.is(0, "partial") {
            if !self.is(1, "interface") || self.is(2, "mixin") {
                let what = match (self.peek(1), self.peek(2)) {
                    (Some(a), Some(b)) if b.text == "mixin" => format!("{} {}", a.text, b.text),
                    (Some(a), _) => a.text.clone(),
                    _ => String::new(),
                };
                return self.refuse(line, format!("`partial {what}`"));
            }
            self.at += 2;
            let (name, _) = self.identifier("the name of an interface")?;
            if self.is(0, ":") {
                return self.refuse(self.line(), "inheritance on a `partial interface`");
            }
            let members = self.body(Body::Interface, &name)?;
            (
                name,
                DefinitionKind::Interface {
                    partial: true,
                    inherits: None,
                    members,
                },
            )
        } else if self.eat("dictionary") {
            let (name, _) = self.identifier("the name of a dictionary")?;
            let inherits = self.inheritance()?;
            let members = self.dictionary_members()?;
            (name, DefinitionKind::Dictionary { inherits, members })
        } else if self.eat("enum") {
            let (name, _) = self.identifier("the name of an enum")?;
            let values = self.enum_values()?;
            (name, DefinitionKind::Enum { values })
        } else if self.eat("typedef") {
            let ty = self.type_with_extended_attributes()?;
            let (name, _) = self.identifier("the name of a typedef")?;
            self.expect(";")?;
            (name, DefinitionKind::Typedef { ty })
        } else if self.is(0, "namespace") {
            return self.refuse(line, "`namespace`");
        } else if self.is(1, "includes") {
            let (name, _) = self.identifier("the name of an interface")?;
            self.expect("includes")?;
            let (mixin, mixin_line) = self.identifier("the name of an interface mixin")?;
            self.expect(";")?;
            let mixin = Reference {
                name: mixin,
                line: mixin_line,
            };
            (name, DefinitionKind::Includes { mixin })
        } else {
            return self.unexpected("a definition");
        };
        Ok(Definition {
            name,
            file: self.file.to_string(),
            line,
            extended_attributes,
            kind,
        })
    }

    fn inheritance(&mut self) -> Result<Option<Reference>, ReadError> {
        if self.eat(":") {
            let (name, line) = self.identifier("the name inherited from")?;
            Ok(Some(Reference { name, line }))
        } else {
            Ok(None)
        }
    }

    /// `{ members } ;`, each operation name at most once (an overload is refused).
    fn body(&mut self, body: Body, owner: &str) -> Result<Vec<Member>, ReadError> {
        self.expect("{")?;
        let mut members = Vec::new();
        let mut operations = BTreeSet::new();
        while !self.is(0, "}") {
            if self.peek(0).is_none() {
                return self.unexpected("`}`");
            }
            let member = self.member(body)?;
            if let (MemberKind::Operation { .. }, Some(name)) = (&member.kind, &member.name)
                && !operations.insert(name.clone())
            {
                return self.refuse(
                    member.line,
                    format!("an overload: a second operation `{name}` in `{owner}`"),
                );
            }
            members.push(member);
        }
        self.expect("}")?;
        self.expect(";")?;
        Ok(members)
    }

    fn member(&mut self, body: Body) -> Result<Member, ReadError> {
        let mut extended_attributes = self.extended_attributes()?;
        let line = self.line();
        let first = self
            .peek(0)
            .map(|token| token.text.clone())
            .unwrap_or_default();
        let (name, kind) = match first.as_str() {
            "const" => {
                self.at += 1;
                let ty = self.ty()?;
                if ty.nullable
                    || !matches!(
                        ty.kind,
                        TypeKind::Boolean
                            | TypeKind::Integer { .. }
                            | TypeKind::Double { .. }
                            | TypeKind::Named { .. }
                    )
                {
                    return self
                        .refuse(line, "a `const` of a type other than a primitive or a name");
                }
                let (name, _) = self.identifier("the name of a constant")?;
                self.expect("=")?;
                let value = self.const_value()?;
                self.expect(";")?;
                (Some(name), MemberKind::Const { ty, value })
            }
            "constructor" => {
                if body != Body::Interface {
                    return self.not_in(body, line, "`constructor`");
                }
                self.at += 1;
                let arguments = self.arguments()?;
                self.expect(";")?;
                (None, MemberKind::Constructor { arguments })
            }
            "stringifier" => {
                if body == Body::CallbackInterface {
                    return self.not_in(body, line, "`stringifier`");
                }
                self.at += 1;
                if self.eat(";") {
                    (None, MemberKind::Stringifier)
                } else if self.is(0, "attribute") || self.is(0, "readonly") {
                    let (name, kind) = self.attribute(true)?;
                    (Some(name), kind)
                } else {
                    return self.refuse(line, "a `stringifier` operation");
                }
            }
            "static" => {
                if body != Body::Interface {
                    return self.not_in(body, line, "`static`");
                }
                if self.is(1, "attribute") || self.is(1, "readonly") {
                    return self.refuse(line, "a `static` attribute");
                }
                self.at += 1;
                let (name, kind) = self.operation(true, false)?;
                (Some(name), kind)
            }
            "readonly" | "attribute" => {
                if body == Body::CallbackInterface {
                    return self.not_in(body, line, "an attribute");
                }
                if self.is(1, "maplike") || self.is(1, "setlike") {
                    return self.refuse(line, format!("`{}`", self.peek(1).expect("peeked").text));
                }
                let (name, kind) = self.attribute(false)?;
                (Some(name), kind)
            }
            "getter" => {
                if body != Body::Interface {
                    return self.not_in(body, line, "`getter`");
                }
                self.at += 1;
                let (name, kind) = self.operation(false, true)?;
                (Some(name), kind)
            }
            "iterable" => {
                if body != Body::Interface {
                    return self.not_in(body, line, "`iterable`");
                }
                self.at += 1;
                self.expect("<")?;
                let ty = self.type_with_extended_attributes()?;
                if self.is(0, ",") {
                    return self.refuse(line, "`iterable` of a key and a value");
                }
                self.expect(">")?;
                self.expect(";")?;
                (None, MemberKind::Iterable { ty })
            }
            "async_iterable" => return self.refuse(line, "`async iterable`"),
            "async" if self.is(1, "iterable") => return self.refuse(line, "`async iterable`"),
            "maplike" | "setlike" | "legacycaller" | "setter" | "deleter" | "inherit" => {
                return self.refuse(line, format!("`{first}`"));
            }
            _ => {
                let (name, kind) = self.operation(false, false)?;
                (Some(name), kind)
            }
        };
        let mut kind = kind;
        match &mut kind {
            MemberKind::Attribute { ty, .. }
            | MemberKind::Operation {
                return_type: ty, ..
            } => onto_type(&mut extended_attributes, ty),
            _ => {}
        }
        Ok(Member {
            name,
            line,
            extended_attributes,
            kind,
        })
    }

    /// `[readonly] attribute TypeWithExtendedAttributes name ;`.
    fn attribute(&mut self, stringifier: bool) -> Result<(String, MemberKind), ReadError> {
        let readonly = self.eat("readonly");
        self.expect("attribute")?;
        let ty = self.type_with_extended_attributes()?;
        let (name, _) = self.identifier("the name of an attribute")?;
        self.expect(";")?;
        Ok((
            name,
            MemberKind::Attribute {
                ty,
                readonly,
                stringifier,
            },
        ))
    }

    /// `Type name ( arguments ) ;`.
    fn operation(
        &mut self,
        is_static: bool,
        getter: bool,
    ) -> Result<(String, MemberKind), ReadError> {
        let return_type = self.ty()?;
        let (name, _) = self.identifier("the name of an operation")?;
        let arguments = self.arguments()?;
        self.expect(";")?;
        Ok((
            name,
            MemberKind::Operation {
                return_type,
                arguments,
                is_static,
                getter,
            },
        ))
    }

    /// `( Argument, ... )`.
    fn arguments(&mut self) -> Result<Vec<Argument>, ReadError> {
        self.expect("(")?;
        let mut arguments = Vec::new();
        if self.eat(")") {
            return Ok(arguments);
        }
        loop {
            arguments.push(self.argument()?);
            if self.eat(")") {
                return Ok(arguments);
            }
            self.expect(",")?;
        }
    }

    fn argument(&mut self) -> Result<Argument, ReadError> {
        let mut extended_attributes = self.extended_attributes()?;
        let optional = self.eat("optional");
        let mut ty = if optional {
            self.type_with_extended_attributes()?
        } else {
            self.ty()?
        };
        onto_type(&mut extended_attributes, &mut ty);
        let variadic = !optional && self.eat("...");
        let (name, line) = self.argument_name()?;
        let default = if optional && self.eat("=") {
            Some(self.default_value()?)
        } else {
            None
        };
        Ok(Argument {
            name,
            line,
            extended_attributes,
            ty,
            optional,
            default,
            variadic,
        })
    }

    /// An identifier, or one of [`ARGUMENT_NAME_KEYWORDS`] (`ADDED`: "argument name keyword").
    fn argument_name(&mut self) -> Result<(String, usize), ReadError> {
        if let Some(token) = self.peek(0)
            && token.kind == Kind::Identifier
            && ARGUMENT_NAME_KEYWORDS.contains(&token.text.as_str())
        {
            let token = self.next().expect("peeked");
            return Ok((token.text, token.line));
        }
        self.identifier("the name of an argument")
    }

    fn dictionary_members(&mut self) -> Result<Vec<DictionaryMember>, ReadError> {
        self.expect("{")?;
        let mut members = Vec::new();
        while !self.eat("}") {
            if self.peek(0).is_none() {
                return self.unexpected("`}`");
            }
            let mut extended_attributes = self.extended_attributes()?;
            let line = self.line();
            let required = self.eat("required");
            let mut ty = if required {
                self.type_with_extended_attributes()?
            } else {
                self.ty()?
            };
            onto_type(&mut extended_attributes, &mut ty);
            let (name, _) = self.identifier("the name of a dictionary member")?;
            let default = if !required && self.eat("=") {
                Some(self.default_value()?)
            } else {
                None
            };
            self.expect(";")?;
            members.push(DictionaryMember {
                name,
                line,
                extended_attributes,
                ty,
                required,
                default,
            });
        }
        self.expect(";")?;
        Ok(members)
    }

    /// `{ "a", "b" }`, at least one string, a comma after the last allowed (the grammar's
    /// `EnumValueList`); an empty list and an empty place between two commas are refused.
    fn enum_values(&mut self) -> Result<Vec<String>, ReadError> {
        self.expect("{")?;
        let mut values = Vec::new();
        loop {
            match self.peek(0) {
                Some(token) if token.kind == Kind::String => {
                    let token = self.next().expect("peeked");
                    values.push(token.text[1..token.text.len() - 1].to_string());
                }
                _ => return self.unexpected("a string"),
            }
            if self.eat("}") {
                break;
            }
            self.expect(",")?;
            if self.eat("}") {
                break;
            }
        }
        self.expect(";")?;
        Ok(values)
    }

    fn integer(&mut self) -> Result<i64, ReadError> {
        let token = self.next().expect("an integer token");
        let (negative, digits) = match token.text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, token.text.as_str()),
        };
        let value = if let Some(hex) = digits.strip_prefix("0x").or(digits.strip_prefix("0X")) {
            i64::from_str_radix(hex, 16)
        } else if digits.len() > 1 && digits.starts_with('0') {
            i64::from_str_radix(&digits[1..], 8)
        } else {
            digits.parse()
        };
        match value {
            Ok(value) if negative => Ok(-value),
            Ok(value) => Ok(value),
            Err(_) => self.refuse(
                token.line,
                format!("the integer `{}`, out of range", token.text),
            ),
        }
    }

    fn const_value(&mut self) -> Result<ConstValue, ReadError> {
        let Some(token) = self.peek(0).cloned() else {
            return self.unexpected("a constant value");
        };
        if token.kind == Kind::Integer {
            return Ok(ConstValue::Integer(self.integer()?));
        }
        if self.eat("true") {
            return Ok(ConstValue::Boolean(true));
        }
        if self.eat("false") {
            return Ok(ConstValue::Boolean(false));
        }
        if token.kind == Kind::Decimal
            || ["Infinity", "-Infinity", "NaN"].contains(&token.text.as_str())
        {
            return self.refuse(
                token.line,
                format!("the floating point value `{}`", token.text),
            );
        }
        self.unexpected("a constant value")
    }

    /// The error for `what` in a kind of body that does not allow it.
    fn not_in<T>(&self, body: Body, line: usize, what: &str) -> Result<T, ReadError> {
        self.refuse(line, format!("{what} in {}", body.name()))
    }

    fn default_value(&mut self) -> Result<DefaultValue, ReadError> {
        let line = self.line();
        if self.is(0, "{") {
            self.at += 1;
            self.expect("}")?;
            return Ok(DefaultValue::EmptyDictionary);
        }
        if self.eat("null") {
            return Ok(DefaultValue::Null);
        }
        if self.is(0, "[") || self.is(0, "undefined") {
            let text = if self.is(0, "[") { "[]" } else { "undefined" };
            return self.refuse(line, format!("the default value `{text}`"));
        }
        if let Some(token) = self.peek(0)
            && token.kind == Kind::String
        {
            let token = self.next().expect("peeked");
            return Ok(DefaultValue::String(
                token.text[1..token.text.len() - 1].to_string(),
            ));
        }
        Ok(match self.const_value()? {
            ConstValue::Boolean(value) => DefaultValue::Boolean(value),
            ConstValue::Integer(value) => DefaultValue::Integer(value),
        })
    }

    /// `[A, B=C, D=*, E=(F, G)]`, or nothing; every name one of [`EXTENDED_ATTRIBUTES`].
    fn extended_attributes(&mut self) -> Result<Vec<ExtendedAttribute>, ReadError> {
        let mut list = Vec::new();
        if !self.eat("[") {
            return Ok(list);
        }
        loop {
            let line = self.line();
            let name = match self.peek(0) {
                Some(token) if token.kind == Kind::Identifier => token.text.clone(),
                _ => return self.unexpected("the name of an extended attribute"),
            };
            if !EXTENDED_ATTRIBUTES.contains(&name.as_str()) {
                return self.refuse(line, format!("the extended attribute `[{name}]`"));
            }
            self.at += 1;
            let value = if self.is(0, "(") {
                return self.refuse(line, format!("an argument list of `[{name}]`"));
            } else if self.eat("=") {
                if self.eat("*") {
                    ExtendedAttributeValue::Wildcard
                } else if self.eat("(") {
                    let mut names = vec![self.identifier("a name")?.0];
                    while self.eat(",") {
                        names.push(self.identifier("a name")?.0);
                    }
                    self.expect(")")?;
                    ExtendedAttributeValue::IdentifierList(names)
                } else {
                    let (value, _) = self.identifier("an identifier")?;
                    if self.is(0, "(") {
                        return self.refuse(line, format!("an argument list of `[{name}]`"));
                    }
                    ExtendedAttributeValue::Identifier(value)
                }
            } else {
                ExtendedAttributeValue::None
            };
            list.push(ExtendedAttribute { name, line, value });
            if self.eat("]") {
                return Ok(list);
            }
            self.expect(",")?;
        }
    }

    fn type_with_extended_attributes(&mut self) -> Result<Type, ReadError> {
        let extended_attributes = self.extended_attributes()?;
        let mut ty = self.ty()?;
        ty.extended_attributes = extended_attributes;
        Ok(ty)
    }

    /// A type: a union, `any`, or a distinguishable type, then `?` for nullable. Every nested
    /// type (a union member, a generic argument) passes through here, so the depth of the
    /// recursion is at most [`MAX_TYPE_DEPTH`] calls of `ty`.
    fn ty(&mut self) -> Result<Type, ReadError> {
        if self.depth == MAX_TYPE_DEPTH {
            return self.refuse(
                self.line(),
                format!("a type that nests deeper than {MAX_TYPE_DEPTH} levels"),
            );
        }
        self.depth += 1;
        let ty = self.ty_at_depth();
        self.depth -= 1;
        ty
    }

    fn ty_at_depth(&mut self) -> Result<Type, ReadError> {
        if self.is(0, "any") {
            self.at += 1;
            if self.is(0, "?") {
                return self.refuse(self.line(), "`any?`");
            }
            return Ok(plain(TypeKind::Any));
        }
        let kind = if self.eat("(") {
            let mut members = vec![self.union_member()?];
            while self.eat("or") {
                members.push(self.union_member()?);
            }
            self.expect(")")?;
            if members.len() < 2 {
                return self.unexpected("`or`");
            }
            TypeKind::Union(members)
        } else {
            self.distinguishable()?
        };
        let nullable = self.eat("?");
        Ok(Type {
            kind,
            nullable,
            extended_attributes: Vec::new(),
        })
    }

    fn union_member(&mut self) -> Result<Type, ReadError> {
        let extended_attributes = self.extended_attributes()?;
        if self.is(0, "any") {
            return self.refuse(self.line(), "`any` in a union");
        }
        let mut ty = self.ty()?;
        ty.extended_attributes = extended_attributes;
        Ok(ty)
    }

    fn distinguishable(&mut self) -> Result<TypeKind, ReadError> {
        let Some(token) = self.peek(0).cloned() else {
            return self.unexpected("a type");
        };
        let line = token.line;
        if token.kind != Kind::Identifier {
            return self.unexpected("a type");
        }
        let kind = match token.text.as_str() {
            "Promise" => return self.refuse(line, "`Promise<T>`"),
            "undefined" => TypeKind::Undefined,
            "boolean" => TypeKind::Boolean,
            "object" => TypeKind::Object,
            "DOMString" => TypeKind::DomString,
            "USVString" => TypeKind::UsvString,
            "double" => TypeKind::Double {
                unrestricted: false,
            },
            "unrestricted" => {
                if !self.is(1, "double") {
                    return self.refuse(line, "`unrestricted float`");
                }
                self.at += 1;
                TypeKind::Double { unrestricted: true }
            }
            "unsigned" => {
                let size = if self.is(1, "short") {
                    self.at += 1;
                    IntegerSize::Short
                } else if self.is(1, "long") && self.is(2, "long") {
                    self.at += 2;
                    IntegerSize::LongLong
                } else if self.is(1, "long") {
                    self.at += 1;
                    IntegerSize::Long
                } else {
                    return self.unexpected("`short` or `long` after `unsigned`");
                };
                TypeKind::Integer {
                    unsigned: true,
                    size,
                }
            }
            "short" => TypeKind::Integer {
                unsigned: false,
                size: IntegerSize::Short,
            },
            name if BUFFER_RELATED_TYPES.contains(&name) => TypeKind::Buffer(name.to_string()),
            "sequence" => {
                self.at += 1;
                self.expect("<")?;
                let inner = self.type_with_extended_attributes()?;
                if !self.is(0, ">") {
                    return self.unexpected("`>`");
                }
                TypeKind::Sequence(Box::new(inner))
            }
            name if KEYWORDS.contains(&name) => {
                return self.refuse(line, format!("the type `{name}`"));
            }
            _ => {
                let (name, line) = self.identifier("a type")?;
                return Ok(TypeKind::Named { name, line });
            }
        };
        self.at += 1;
        Ok(kind)
    }
}

fn plain(kind: TypeKind) -> Type {
    Type {
        kind,
        nullable: false,
        extended_attributes: Vec::new(),
    }
}

/// Moves the names of [`TYPE_EXTENDED_ATTRIBUTES`] in `written` (the list before an argument,
/// an attribute, an operation or a dictionary member) onto `ty`, ahead of those `ty` holds;
/// the others stay in `written`.
fn onto_type(written: &mut Vec<ExtendedAttribute>, ty: &mut Type) {
    let (on_type, stay): (Vec<_>, Vec<_>) = std::mem::take(written)
        .into_iter()
        .partition(|attribute| TYPE_EXTENDED_ATTRIBUTES.contains(&attribute.name.as_str()));
    *written = stay;
    let after = std::mem::replace(&mut ty.extended_attributes, on_type);
    ty.extended_attributes.extend(after);
}
