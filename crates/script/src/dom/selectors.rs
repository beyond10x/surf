//! The slice `selectors` (`story:scripts-dom-selectors`): `querySelector` and
//! `querySelectorAll` of `Element`, `Document` and `DocumentFragment`, and `closest`, `matches`
//! and `webkitMatchesSelector` of `Element`, over a subset of Selectors Level 4.
//!
//! The subset, each production with its section of Selectors Level 4 (the Editor's Draft of
//! 10 November 2026, as the story read it; the text is not in the tree):
//!
//! ```text
//! selector-list      = complex-selector [ "," complex-selector ]*         4.1, 16
//! complex-selector   = compound-selector [ combinator? compound-selector ]*  3.1, 16
//! combinator         = ">"                       the child combinator, 14.2
//!                    | whitespace alone          the descendant combinator, 14.1
//! compound-selector  = [ type-selector? subclass-selector* ]!            3.1, 16
//! type-selector      = ident                     a type selector, 5.1
//!                    | "*"                       the universal selector, 5.2
//! subclass-selector  = id-selector | class-selector | attribute-selector
//! id-selector        = hash-token of type "id"   6.7
//! class-selector     = "." ident                 6.6
//! attribute-selector = "[" ident "]"             presence, 6.1
//!                    | "[" ident "=" ( ident | string ) "]"   equality, 6.1
//! ```
//!
//! Whitespace is allowed around a combinator and a comma and inside the brackets, and nowhere
//! inside a compound (section 16); whitespace around the whole text is tokens of CSS Syntax. The
//! tokens are CSS Syntax Level 3's (the Editor's Draft of 20 October 2026): CR, FF and CR LF are
//! one newline and U+0000 is U+FFFD (3.3); an escape is up to six hex digits and one whitespace,
//! or any other code point but a newline (4.3.7); an identifier starts with a digit only through
//! an escape (4.3.8, 4.3.9); a string ends at its quote or at the end of the text, and a newline
//! inside it makes it bad (4.3.5); a comment is dropped and an unterminated one runs to the end
//! (4.3.2). The end of the text closes an open `[` or `(` (5.5.9, 2.2), so `[a=b` is `[a=b]`.
//!
//! Outside the subset, and valid, a text is refused with a `SyntaxError` that says surf does not
//! support it, naming the first such construct: a pseudo-class or a pseudo-element, recognised
//! by its shape (a colon or two, an identifier, an argument in balanced parentheses skipped by
//! token, its name and argument not validated; one colon before one of
//! [`LEGACY_PSEUDO_ELEMENTS`] is a pseudo-element, section 16), the combinators `+` and `~`, an
//! attribute matcher
//! other than `=`, a flag after the value, a namespace prefix of `*` or of none, and the nesting
//! selector `&`. A prefix that is an identifier is an undeclared namespace prefix (3.9) and not
//! valid, as is `||`. A text with any fault of the grammar is not valid whatever precedes it
//! (DOM 1.3, "scope-match a selectors string": a failure is a `SyntaxError`).
//!
//! The case rules are the HTML Standard's section "Case-sensitivity of selectors" (4.16.2, the
//! Living Standard of 26 November 2026, commit `0162cee0c42a82267354d38fbc6a2503ee295b77`, as
//! read): a type name and an attribute name are put in ASCII lower
//! case for an element of the HTML namespace and then compared exactly, and compared as given
//! for any other element; an attribute value is compared exactly, but ASCII case-insensitively
//! on an HTML element for the names of [`SurfSelectors::CASE_INSENSITIVE_VALUES`]; `#id` and
//! `.class` compare exactly, and ASCII case-insensitively in quirks mode (Selectors 6.6, 6.7).
//!
//! Bounds. A text is at most [`TEXT_BYTES`] bytes and holds at most [`SIMPLE_SELECTORS`] simple
//! selectors; past either it is refused before or where the bound is met. Nothing recurses: the
//! parser is one loop over tokens taken one at a time, the walk keeps a stack of its own
//! (`selector_reads::Frames`), and the matcher fills a table of two bytes for each compound and
//! each element of the chain of ancestors, row by row. Every read of the tree and every step
//! charged is a function of `selector_reads`; this file holds none.

use surf_html::NodeId;

use crate::bindings::{DomCx, DomEnd, DomException, DomResult, Selectors};

use super::reads;
use super::selector_reads::{
    cell, descendant_elements, gate, memo, next_ancestor, selector_name, selector_text,
    selector_token,
};

/// The most bytes of UTF-8 a selector text holds, counted as given.
pub(super) const TEXT_BYTES: usize = 4096;

/// The most simple selectors (a type, `*`, an id, a class or an attribute selector) a text
/// holds, counted after the parse.
pub(super) const SIMPLE_SELECTORS: usize = 512;

/// The four pseudo-elements of Level 2, which Selectors 16 parses as pseudo-elements also after
/// one colon and never as pseudo-classes; their names are ASCII case-insensitive (3.6.1).
const LEGACY_PSEUDO_ELEMENTS: [&str; 4] = ["before", "after", "first-line", "first-letter"];

/// surf's members of `selectors`. It keeps nothing from one call to the next.
pub struct SurfSelectors;

impl SurfSelectors {
    /// The names of the attributes whose values a selector compares ASCII case-insensitively on
    /// an element of the HTML namespace: the list of the HTML Standard's section
    /// "Case-sensitivity of selectors" (4.16.2, the Living Standard of 26 November 2026, commit
    /// `0162cee0c42a82267354d38fbc6a2503ee295b77`, as read), in its order, which is sorted.
    pub const CASE_INSENSITIVE_VALUES: [&'static str; 46] = [
        "accept",
        "accept-charset",
        "align",
        "alink",
        "axis",
        "bgcolor",
        "charset",
        "checked",
        "clear",
        "codetype",
        "color",
        "compact",
        "declare",
        "defer",
        "dir",
        "direction",
        "disabled",
        "enctype",
        "face",
        "frame",
        "hreflang",
        "http-equiv",
        "lang",
        "language",
        "link",
        "media",
        "method",
        "multiple",
        "nohref",
        "noresize",
        "noshade",
        "nowrap",
        "readonly",
        "rel",
        "rev",
        "rules",
        "scope",
        "scrolling",
        "selected",
        "shape",
        "target",
        "text",
        "type",
        "valign",
        "valuetype",
        "vlink",
    ];
}

/// A type name or an attribute name of the selector: as decoded, and its ASCII lower case, made
/// once in the parse.
pub(super) struct Name {
    pub(super) given: String,
    pub(super) lower: String,
}

/// An attribute selector: its name, its value for `[a=v]`, and whether the lower case of the name
/// is in [`SurfSelectors::CASE_INSENSITIVE_VALUES`].
pub(super) struct AttributeSelector {
    pub(super) name: Name,
    pub(super) value: Option<String>,
    pub(super) folded: bool,
}

/// A simple selector of the subset.
pub(super) enum Simple {
    Universal,
    Type(Name),
    Id(String),
    Class(String),
    Attribute(AttributeSelector),
}

/// How a compound relates to the compound on its left.
#[derive(Clone, Copy)]
enum Combinator {
    Descendant,
    Child,
}

/// A compound selector: its simple selectors, `start..end` of the list's, and the combinator to
/// the compound on its left (none for the first of a complex selector).
struct Compound {
    start: usize,
    end: usize,
    left: Combinator,
}

/// A complex selector: its compounds, `start..end` of the list's, left to right.
struct Complex {
    start: usize,
    end: usize,
}

/// A parsed selector list.
struct SelectorList {
    simples: Vec<Simple>,
    compounds: Vec<Compound>,
    complexes: Vec<Complex>,
}

impl SelectorList {
    fn simples_of(&self, compound: &Compound) -> &[Simple] {
        &self.simples[compound.start..compound.end]
    }
}

/// A construct of a valid selector outside the subset, or a bound: the word of its message.
#[derive(Clone, Copy)]
enum Unsupported {
    PseudoClass,
    PseudoElement,
    SiblingCombinator,
    AttributeMatcher,
    AttributeFlag,
    NamespacePrefix,
    NestingSelector,
    TextBytes,
    SimpleSelectors,
}

impl Unsupported {
    fn word(self) -> &'static str {
        match self {
            Unsupported::PseudoClass => "pseudo-class",
            Unsupported::PseudoElement => "pseudo-element",
            Unsupported::SiblingCombinator => "sibling combinator",
            Unsupported::AttributeMatcher => "attribute matcher",
            Unsupported::AttributeFlag => "attribute flag",
            Unsupported::NamespacePrefix => "namespace prefix",
            Unsupported::NestingSelector => "nesting selector",
            Unsupported::TextBytes => "text over 4096 bytes",
            Unsupported::SimpleSelectors => "more than 512 simple selectors",
        }
    }

    /// The `SyntaxError` of a valid selector surf does not support.
    fn end(self) -> DomEnd {
        DomEnd::Throw(DomException::new(
            "SyntaxError",
            format!("selector is not supported by surf: {}", self.word()),
        ))
    }
}

/// The `SyntaxError` of a text that is not a selector list.
fn not_valid() -> DomEnd {
    DomEnd::Throw(DomException::new("SyntaxError", "selector is not valid"))
}

/// A token of CSS Syntax Level 3, as far as a selector reads it.
#[derive(Clone, PartialEq, Eq)]
enum Token {
    Whitespace,
    Comment,
    Ident(String),
    Function,
    /// A hash token's name, and whether its type flag is "id".
    Hash(String, bool),
    Str(String),
    BadStr,
    Delim(char),
    Colon,
    Comma,
    OpenBracket,
    CloseBracket,
    OpenParen,
    CloseParen,
    /// A token no production of a selector takes: a number, a dimension, a percentage, an
    /// at-keyword, `{`, `}`, `;`, `<!--` or `-->`.
    Other,
    /// The end of the text.
    End,
}

fn is_newline_or_space(c: Option<char>) -> bool {
    matches!(c, Some('\n' | '\t' | ' '))
}

fn is_digit(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_digit())
}

fn is_name_start(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || !c.is_ascii())
}

fn is_name(c: Option<char>) -> bool {
    is_name_start(c) || is_digit(c) || c == Some('-')
}

/// CSS Syntax 4.3.8: whether two code points are a valid escape.
fn valid_escape(first: Option<char>, second: Option<char>) -> bool {
    first == Some('\\') && second != Some('\n')
}

/// CSS Syntax 4.3.9: whether three code points would start an ident sequence.
fn starts_ident(first: Option<char>, second: Option<char>, third: Option<char>) -> bool {
    match first {
        Some('-') => is_name_start(second) || second == Some('-') || valid_escape(second, third),
        Some('\\') => valid_escape(first, second),
        other => is_name_start(other),
    }
}

/// CSS Syntax 4.3.10: whether three code points would start a number.
fn starts_number(first: Option<char>, second: Option<char>, third: Option<char>) -> bool {
    match first {
        Some('+' | '-') => is_digit(second) || (second == Some('.') && is_digit(third)),
        Some('.') => is_digit(second),
        other => is_digit(other),
    }
}

/// The tokenizer of CSS Syntax Level 3 over a selector text, one token at a time. It reads the
/// text after the preprocessing of 3.3 without making it: CR, FF and CR LF read as one LF and
/// U+0000 as U+FFFD.
struct Lexer<'t> {
    text: &'t str,
    at: usize,
}

impl<'t> Lexer<'t> {
    /// The code point at byte `at` after the preprocessing, and the byte after it.
    fn point(&self, at: usize) -> Option<(char, usize)> {
        let c = self.text.get(at..)?.chars().next()?;
        let next = at + c.len_utf8();
        Some(match c {
            '\r' if self.text.as_bytes().get(next) == Some(&b'\n') => ('\n', next + 1),
            '\r' | '\x0C' => ('\n', next),
            '\0' => ('\u{FFFD}', next),
            c => (c, next),
        })
    }

    /// The code point `ahead` places after the next one (0 for the next).
    fn peek(&self, ahead: usize) -> Option<char> {
        let mut at = self.at;
        for _ in 0..ahead {
            at = self.point(at)?.1;
        }
        self.point(at).map(|(c, _)| c)
    }

    fn bump(&mut self) -> Option<char> {
        let (c, next) = self.point(self.at)?;
        self.at = next;
        Some(c)
    }

    fn at_end(&self) -> bool {
        self.at >= self.text.len()
    }

    /// The next token; [`Token::End`] at the end of the text.
    fn token(&mut self) -> Token {
        let Some(c) = self.peek(0) else {
            return Token::End;
        };
        if c == '/' && self.peek(1) == Some('*') {
            self.bump();
            self.bump();
            while let Some(c) = self.bump() {
                if c == '*' && self.peek(0) == Some('/') {
                    self.bump();
                    break;
                }
            }
            return Token::Comment;
        }
        match c {
            '\n' | '\t' | ' ' => {
                while is_newline_or_space(self.peek(0)) {
                    self.bump();
                }
                Token::Whitespace
            }
            '"' | '\'' => {
                self.bump();
                self.string(c)
            }
            '#' => {
                if is_name(self.peek(1)) || valid_escape(self.peek(1), self.peek(2)) {
                    let id = starts_ident(self.peek(1), self.peek(2), self.peek(3));
                    self.bump();
                    Token::Hash(self.ident_sequence(), id)
                } else {
                    self.bump();
                    Token::Delim('#')
                }
            }
            '(' => self.single(Token::OpenParen),
            ')' => self.single(Token::CloseParen),
            ',' => self.single(Token::Comma),
            ':' => self.single(Token::Colon),
            '[' => self.single(Token::OpenBracket),
            ']' => self.single(Token::CloseBracket),
            ';' | '{' | '}' => self.single(Token::Other),
            '+' | '.' if starts_number(self.peek(0), self.peek(1), self.peek(2)) => self.numeric(),
            '-' if starts_number(self.peek(0), self.peek(1), self.peek(2)) => self.numeric(),
            '-' if self.peek(1) == Some('-') && self.peek(2) == Some('>') => {
                self.bump();
                self.bump();
                self.bump();
                Token::Other
            }
            '-' | '\\' if starts_ident(self.peek(0), self.peek(1), self.peek(2)) => {
                self.ident_like()
            }
            '<' if self.peek(1) == Some('!')
                && self.peek(2) == Some('-')
                && self.peek(3) == Some('-') =>
            {
                for _ in 0..4 {
                    self.bump();
                }
                Token::Other
            }
            '@' if starts_ident(self.peek(1), self.peek(2), self.peek(3)) => {
                self.bump();
                self.ident_sequence();
                Token::Other
            }
            c if c.is_ascii_digit() => self.numeric(),
            c if is_name_start(Some(c)) => self.ident_like(),
            c => self.single(Token::Delim(c)),
        }
    }

    fn single(&mut self, token: Token) -> Token {
        self.bump();
        token
    }

    /// CSS Syntax 4.3.7, after the backslash: the code point of an escape.
    fn escape(&mut self) -> char {
        match self.bump() {
            None => '\u{FFFD}',
            Some(c) if c.is_ascii_hexdigit() => {
                let mut value = c.to_digit(16).unwrap_or(0);
                for _ in 1..6 {
                    match self.peek(0).and_then(|c| c.to_digit(16)) {
                        Some(digit) => {
                            value = value * 16 + digit;
                            self.bump();
                        }
                        None => break,
                    }
                }
                if is_newline_or_space(self.peek(0)) {
                    self.bump();
                }
                if value == 0 {
                    '\u{FFFD}'
                } else {
                    char::from_u32(value).unwrap_or('\u{FFFD}')
                }
            }
            Some(c) => c,
        }
    }

    /// CSS Syntax 4.3.12: an ident sequence, its escapes decoded.
    fn ident_sequence(&mut self) -> String {
        let mut name = String::new();
        loop {
            let c = self.peek(0);
            if is_name(c) {
                if let Some(c) = self.bump() {
                    name.push(c);
                }
            } else if valid_escape(c, self.peek(1)) {
                self.bump();
                name.push(self.escape());
            } else {
                return name;
            }
        }
    }

    /// CSS Syntax 4.3.4: an identifier, or a function token when a `(` follows.
    fn ident_like(&mut self) -> Token {
        let name = self.ident_sequence();
        if self.peek(0) == Some('(') {
            self.bump();
            Token::Function
        } else {
            Token::Ident(name)
        }
    }

    /// CSS Syntax 4.3.5, after the opening quote: the string up to `quote` or the end of the
    /// text; a newline makes a bad string and is not consumed.
    fn string(&mut self, quote: char) -> Token {
        let mut value = String::new();
        loop {
            match self.peek(0) {
                None => return Token::Str(value),
                Some('\n') => return Token::BadStr,
                Some('\\') => {
                    self.bump();
                    match self.peek(0) {
                        None => {}
                        Some('\n') => {
                            self.bump();
                        }
                        Some(_) => value.push(self.escape()),
                    }
                }
                Some(c) => {
                    self.bump();
                    if c == quote {
                        return Token::Str(value);
                    }
                    value.push(c);
                }
            }
        }
    }

    /// CSS Syntax 4.3.3 and 4.3.12: a number, a percentage or a dimension, which no selector
    /// takes.
    fn numeric(&mut self) -> Token {
        if matches!(self.peek(0), Some('+' | '-')) {
            self.bump();
        }
        while is_digit(self.peek(0)) {
            self.bump();
        }
        if self.peek(0) == Some('.') && is_digit(self.peek(1)) {
            self.bump();
            while is_digit(self.peek(0)) {
                self.bump();
            }
        }
        let exponent = matches!(self.peek(0), Some('e' | 'E'))
            && (is_digit(self.peek(1))
                || (matches!(self.peek(1), Some('+' | '-')) && is_digit(self.peek(2))));
        if exponent {
            self.bump();
            self.bump();
            while is_digit(self.peek(0)) {
                self.bump();
            }
        }
        if starts_ident(self.peek(0), self.peek(1), self.peek(2)) {
            self.ident_sequence();
        } else if self.peek(0) == Some('%') {
            self.bump();
        }
        Token::Other
    }
}

/// Why a parse stopped before its end.
enum Stop {
    /// A fault of the grammar: the text is not valid.
    Fault,
    /// The simple selector past [`SIMPLE_SELECTORS`].
    Bound,
    /// A charge ended the member.
    Charge(DomEnd),
}

impl From<DomEnd> for Stop {
    fn from(end: DomEnd) -> Stop {
        Stop::Charge(end)
    }
}

/// The parser of the subset: one loop over the lexer's tokens, which it takes one at a time and
/// charges before each is taken; comments are dropped once charged. It looks at most two tokens
/// ahead.
struct Parser<'t> {
    lexer: Lexer<'t>,
    ahead: [Option<Token>; 2],
    count: usize,
    unsupported: Option<Unsupported>,
    list: SelectorList,
}

impl Parser<'_> {
    /// The next token that is not a comment, each token charged one step before it is taken.
    fn lex(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<Token> {
        loop {
            if self.lexer.at_end() {
                return Ok(Token::End);
            }
            selector_token(cx)?;
            let token = self.lexer.token();
            if token != Token::Comment {
                return Ok(token);
            }
        }
    }

    /// The token `ahead` places on (0 or 1), taken from the lexer if not yet.
    fn peek(&mut self, cx: &mut DomCx<'_, '_>, ahead: usize) -> DomResult<&Token> {
        for at in 0..=ahead {
            if self.ahead[at].is_none() {
                let token = self.lex(cx)?;
                self.ahead[at] = Some(token);
            }
        }
        Ok(self.ahead[ahead].as_ref().unwrap_or(&Token::End))
    }

    fn take(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<Token> {
        self.peek(cx, 0)?;
        let token = self.ahead[0].take().unwrap_or(Token::End);
        self.ahead[0] = self.ahead[1].take();
        Ok(token)
    }

    /// Takes the whitespace tokens next; whether there was one.
    fn whitespace(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<bool> {
        let mut any = false;
        while *self.peek(cx, 0)? == Token::Whitespace {
            self.take(cx)?;
            any = true;
        }
        Ok(any)
    }

    /// The first construct outside the subset is the one named.
    fn outside(&mut self, construct: Unsupported) {
        self.unsupported.get_or_insert(construct);
    }

    /// The start of a simple selector: the one past the bound ends the parse here.
    fn simple(&mut self) -> Result<(), Stop> {
        if self.count == SIMPLE_SELECTORS {
            return Err(Stop::Bound);
        }
        self.count += 1;
        Ok(())
    }

    /// A type name or an attribute name: its fold to ASCII lower case, charged before it.
    fn name(cx: &mut DomCx<'_, '_>, given: String) -> DomResult<Name> {
        selector_name(cx, given.len())?;
        let lower = given.to_ascii_lowercase();
        Ok(Name { given, lower })
    }

    /// Whether the two tokens next are `|` and an identifier or `*`: a namespace prefix.
    fn prefix_follows(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<bool> {
        if *self.peek(cx, 0)? != Token::Delim('|') {
            return Ok(false);
        }
        Ok(matches!(
            self.peek(cx, 1)?,
            Token::Ident(_) | Token::Delim('*')
        ))
    }

    /// `selector-list`.
    fn selector_list(&mut self, cx: &mut DomCx<'_, '_>) -> Result<(), Stop> {
        self.whitespace(cx)?;
        loop {
            self.complex(cx)?;
            match self.take(cx)? {
                Token::Comma => {
                    self.whitespace(cx)?;
                }
                Token::End => return Ok(()),
                _ => return Err(Stop::Fault),
            }
        }
    }

    /// `complex-selector`, up to the comma or the end that follows it, which is not taken.
    fn complex(&mut self, cx: &mut DomCx<'_, '_>) -> Result<(), Stop> {
        let start = self.list.compounds.len();
        self.compound(cx, Combinator::Descendant)?;
        loop {
            let spaced = self.whitespace(cx)?;
            match self.peek(cx, 0)? {
                Token::End | Token::Comma => break,
                Token::Delim('>') => {
                    self.take(cx)?;
                    self.whitespace(cx)?;
                    self.compound(cx, Combinator::Child)?;
                }
                Token::Delim('+' | '~') => {
                    self.take(cx)?;
                    self.outside(Unsupported::SiblingCombinator);
                    self.whitespace(cx)?;
                    self.compound(cx, Combinator::Descendant)?;
                }
                _ if spaced => self.compound(cx, Combinator::Descendant)?,
                _ => return Err(Stop::Fault),
            }
        }
        let end = self.list.compounds.len();
        self.list.complexes.push(Complex { start, end });
        Ok(())
    }

    /// `compound-selector`, with the constructs outside the subset that a compound may hold.
    fn compound(&mut self, cx: &mut DomCx<'_, '_>, left: Combinator) -> Result<(), Stop> {
        let start = self.list.simples.len();
        let mut any = false;
        let mut pseudo_element = false;
        if matches!(self.peek(cx, 0)?, Token::Ident(_) | Token::Delim('*' | '|')) {
            self.type_selector(cx)?;
            any = true;
        }
        loop {
            match self.peek(cx, 0)? {
                Token::Hash(..) if !pseudo_element => {
                    self.simple()?;
                    let Token::Hash(id, flag_id) = self.take(cx)? else {
                        return Err(Stop::Fault);
                    };
                    if !flag_id {
                        return Err(Stop::Fault);
                    }
                    self.list.simples.push(Simple::Id(id));
                }
                Token::Delim('.') if !pseudo_element => {
                    self.simple()?;
                    self.take(cx)?;
                    let Token::Ident(class) = self.take(cx)? else {
                        return Err(Stop::Fault);
                    };
                    self.list.simples.push(Simple::Class(class));
                }
                Token::OpenBracket if !pseudo_element => {
                    self.simple()?;
                    self.take(cx)?;
                    self.attribute(cx)?;
                }
                Token::Delim('&') if !pseudo_element => {
                    self.take(cx)?;
                    self.outside(Unsupported::NestingSelector);
                }
                Token::Colon => {
                    self.take(cx)?;
                    if *self.peek(cx, 0)? == Token::Colon {
                        self.take(cx)?;
                        self.pseudo(cx)?;
                        self.outside(Unsupported::PseudoElement);
                        pseudo_element = true;
                    } else if matches!(
                        self.peek(cx, 0)?,
                        Token::Ident(name) if LEGACY_PSEUDO_ELEMENTS
                            .iter()
                            .any(|legacy| name.eq_ignore_ascii_case(legacy))
                    ) {
                        self.pseudo(cx)?;
                        self.outside(Unsupported::PseudoElement);
                        pseudo_element = true;
                    } else {
                        self.pseudo(cx)?;
                        self.outside(Unsupported::PseudoClass);
                    }
                }
                Token::Hash(..)
                | Token::Delim('.' | '&')
                | Token::OpenBracket
                | Token::Ident(_)
                | Token::Delim('*' | '|')
                    if pseudo_element =>
                {
                    return Err(Stop::Fault);
                }
                _ => break,
            }
            any = true;
        }
        if !any {
            return Err(Stop::Fault);
        }
        let end = self.list.simples.len();
        self.list.compounds.push(Compound { start, end, left });
        Ok(())
    }

    /// `type-selector`, or a namespace prefix before it.
    fn type_selector(&mut self, cx: &mut DomCx<'_, '_>) -> Result<(), Stop> {
        self.simple()?;
        match self.take(cx)? {
            Token::Ident(given) => {
                if self.prefix_follows(cx)? {
                    // An identifier prefix is a namespace prefix no style sheet declared.
                    return Err(Stop::Fault);
                }
                let name = Parser::name(cx, given)?;
                self.list.simples.push(Simple::Type(name));
            }
            Token::Delim('*') => {
                if self.prefix_follows(cx)? {
                    self.take(cx)?;
                    self.take(cx)?;
                    self.outside(Unsupported::NamespacePrefix);
                } else {
                    self.list.simples.push(Simple::Universal);
                }
            }
            Token::Delim('|') => match self.take(cx)? {
                Token::Ident(_) | Token::Delim('*') => self.outside(Unsupported::NamespacePrefix),
                _ => return Err(Stop::Fault),
            },
            _ => return Err(Stop::Fault),
        }
        Ok(())
    }

    /// `attribute-selector` after its `[`, up to its `]` or the end of the text, which closes it.
    fn attribute(&mut self, cx: &mut DomCx<'_, '_>) -> Result<(), Stop> {
        self.whitespace(cx)?;
        let name = match self.take(cx)? {
            Token::Ident(given) => {
                if *self.peek(cx, 0)? == Token::Delim('|')
                    && *self.peek(cx, 1)? != Token::Delim('=')
                {
                    // `[ns|a]`: an identifier prefix, or a `|` that is no matcher.
                    return Err(Stop::Fault);
                }
                Some(Parser::name(cx, given)?)
            }
            Token::Delim('*') => {
                if self.take(cx)? != Token::Delim('|') {
                    return Err(Stop::Fault);
                }
                let Token::Ident(_) = self.take(cx)? else {
                    return Err(Stop::Fault);
                };
                self.outside(Unsupported::NamespacePrefix);
                None
            }
            Token::Delim('|') => {
                let Token::Ident(_) = self.take(cx)? else {
                    return Err(Stop::Fault);
                };
                self.outside(Unsupported::NamespacePrefix);
                None
            }
            _ => return Err(Stop::Fault),
        };
        self.whitespace(cx)?;
        let value = match self.take(cx)? {
            Token::CloseBracket | Token::End => None,
            Token::Delim('=') => Some(self.value(cx)?),
            Token::Delim('~' | '|' | '^' | '$' | '*') => {
                if self.take(cx)? != Token::Delim('=') {
                    return Err(Stop::Fault);
                }
                self.outside(Unsupported::AttributeMatcher);
                Some(self.value(cx)?)
            }
            _ => return Err(Stop::Fault),
        };
        if let Some(name) = name {
            let folded = SurfSelectors::CASE_INSENSITIVE_VALUES
                .binary_search(&name.lower.as_str())
                .is_ok();
            self.list.simples.push(Simple::Attribute(AttributeSelector {
                name,
                value,
                folded,
            }));
        }
        Ok(())
    }

    /// The value of an attribute selector after its matcher, then its flag, if any, and its `]`
    /// or the end of the text.
    fn value(&mut self, cx: &mut DomCx<'_, '_>) -> Result<String, Stop> {
        self.whitespace(cx)?;
        let value = match self.take(cx)? {
            Token::Ident(value) | Token::Str(value) => value,
            _ => return Err(Stop::Fault),
        };
        self.whitespace(cx)?;
        match self.take(cx)? {
            Token::CloseBracket | Token::End => {}
            Token::Ident(flag)
                if flag.eq_ignore_ascii_case("i") || flag.eq_ignore_ascii_case("s") =>
            {
                self.outside(Unsupported::AttributeFlag);
                self.whitespace(cx)?;
                match self.take(cx)? {
                    Token::CloseBracket | Token::End => {}
                    _ => return Err(Stop::Fault),
                }
            }
            _ => return Err(Stop::Fault),
        }
        Ok(value)
    }

    /// A pseudo-class or pseudo-element by its shape, after its colons: an identifier, or a
    /// function token and its argument up to the `)` that balances it or the end of the text,
    /// skipped by token.
    fn pseudo(&mut self, cx: &mut DomCx<'_, '_>) -> Result<(), Stop> {
        match self.take(cx)? {
            Token::Ident(_) => Ok(()),
            Token::Function => {
                let mut open = 1usize;
                loop {
                    match self.take(cx)? {
                        Token::End => return Ok(()),
                        Token::Function | Token::OpenParen => open += 1,
                        Token::CloseParen => {
                            open -= 1;
                            if open == 0 {
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => Err(Stop::Fault),
        }
    }
}

/// Parses `text` into a selector list of the subset, or answers the `SyntaxError` that refuses
/// it. A text over [`TEXT_BYTES`] is refused before anything is read or charged.
fn parse(cx: &mut DomCx<'_, '_>, text: &str) -> DomResult<SelectorList> {
    if text.len() > TEXT_BYTES {
        return Err(Unsupported::TextBytes.end());
    }
    selector_text(cx, text.len())?;
    let mut parser = Parser {
        lexer: Lexer { text, at: 0 },
        ahead: [None, None],
        count: 0,
        unsupported: None,
        list: SelectorList {
            simples: Vec::new(),
            compounds: Vec::new(),
            complexes: Vec::new(),
        },
    };
    match parser.selector_list(cx) {
        Err(Stop::Charge(end)) => Err(end),
        Err(Stop::Fault) => Err(not_valid()),
        Err(Stop::Bound) => Err(parser
            .unsupported
            .unwrap_or(Unsupported::SimpleSelectors)
            .end()),
        Ok(()) => match parser.unsupported {
            Some(construct) => Err(construct.end()),
            None => Ok(parser.list),
        },
    }
}

/// Whether the element `node` matches any complex selector of `list`. The chain of `node`'s
/// element ancestors is read once, for the first complex selector that passes its gate and has
/// more than one compound.
fn matches_list(
    cx: &mut DomCx<'_, '_>,
    list: &SelectorList,
    node: NodeId,
    quirks: bool,
) -> DomResult<bool> {
    let mut chain: Option<Vec<NodeId>> = None;
    for complex in &list.complexes {
        if matches_complex(cx, list, complex, node, quirks, &mut chain)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether the element `node` matches `complex`. The gate tests its rightmost compound at the
/// element; past it, with `m` compounds and a chain of `n` elements (the element first), a memo
/// of two bytes a pair (compound `i`, position `p`) is filled row by row, `p` from the outermost
/// ancestor down: `yes`, `Ci` holds at `Pp` with everything left of it, and `some`, `yes` at `p`
/// or at a later position. Every cell is filled, so the cost is the selector's and the chain's
/// alone, apart from what the tests read; nothing recurses and nothing backtracks.
fn matches_complex(
    cx: &mut DomCx<'_, '_>,
    list: &SelectorList,
    complex: &Complex,
    node: NodeId,
    quirks: bool,
    chain: &mut Option<Vec<NodeId>>,
) -> DomResult<bool> {
    let compounds = &list.compounds[complex.start..complex.end];
    let Some(last) = compounds.last() else {
        return Ok(false);
    };
    if !gate(cx, node, list.simples_of(last), quirks)? {
        return Ok(false);
    }
    if compounds.len() == 1 {
        return Ok(true);
    }
    let chain: &[NodeId] = match chain {
        Some(chain) => chain,
        None => chain.insert(chain_of(cx, node)?),
    };
    let (m, n) = (compounds.len(), chain.len());
    let mut memo = memo(cx, m, n)?;
    let at = |i: usize, p: usize| 2 * (i * n + p);
    for (i, compound) in compounds.iter().enumerate() {
        for p in (0..n).rev() {
            let left = if i == 0 {
                true
            } else if p + 1 < n {
                let below = at(i - 1, p + 1);
                match compound.left {
                    Combinator::Child => memo[below] != 0,
                    Combinator::Descendant => memo[below + 1] != 0,
                }
            } else {
                false
            };
            let yes = cell(cx, chain[p], list.simples_of(compound), quirks, left)?;
            let later = p + 1 < n && memo[at(i, p + 1) + 1] != 0;
            memo[at(i, p)] = u8::from(yes);
            memo[at(i, p) + 1] = u8::from(yes || later);
        }
    }
    Ok(memo[at(m - 1, 0)] != 0)
}

/// `node` and its element ancestors, upward, each read through `next_ancestor`.
fn chain_of(cx: &mut DomCx<'_, '_>, node: NodeId) -> DomResult<Vec<NodeId>> {
    let mut chain = vec![node];
    let mut at = node;
    while let Some(parent) = next_ancestor(cx, at)? {
        chain.push(parent);
        at = parent;
    }
    Ok(chain)
}

/// The first element under `scope` in tree order that matches `selectors`, or none (DOM 4.2.6,
/// `querySelector`).
fn first(cx: &mut DomCx<'_, '_>, scope: NodeId, selectors: &str) -> DomResult<Option<NodeId>> {
    let list = parse(cx, selectors)?;
    let quirks = reads::quirks(cx);
    let mut found = None;
    descendant_elements(cx, scope, |cx, element| {
        let hit = matches_list(cx, &list, element, quirks)?;
        if hit {
            found = Some(element);
        }
        Ok(hit)
    })?;
    Ok(found)
}

/// The elements under `scope` in tree order that match `selectors` (DOM 4.2.6,
/// `querySelectorAll`: the static result).
fn all(cx: &mut DomCx<'_, '_>, scope: NodeId, selectors: &str) -> DomResult<Vec<NodeId>> {
    let list = parse(cx, selectors)?;
    let quirks = reads::quirks(cx);
    let mut found = Vec::new();
    descendant_elements(cx, scope, |cx, element| {
        if matches_list(cx, &list, element, quirks)? {
            found.push(element);
        }
        Ok(false)
    })?;
    Ok(found)
}

/// Whether `this` matches `selectors` (DOM 4.9, `matches`).
fn matches(cx: &mut DomCx<'_, '_>, this: NodeId, selectors: &str) -> DomResult<bool> {
    let list = parse(cx, selectors)?;
    let quirks = reads::quirks(cx);
    matches_list(cx, &list, this, quirks)
}

impl Selectors for SurfSelectors {
    /// DOM 4.9: `this`, then its element ancestors upward; the first that matches.
    fn element_closest(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Option<NodeId>> {
        let list = parse(cx, &selectors)?;
        let quirks = reads::quirks(cx);
        let mut candidate = this;
        loop {
            if matches_list(cx, &list, candidate, quirks)? {
                return Ok(Some(candidate));
            }
            match next_ancestor(cx, candidate)? {
                Some(parent) => candidate = parent,
                None => return Ok(None),
            }
        }
    }

    fn element_matches(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<bool> {
        matches(cx, this, &selectors)
    }

    fn element_webkit_matches_selector(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<bool> {
        matches(cx, this, &selectors)
    }

    fn element_query_selector(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Option<NodeId>> {
        first(cx, this, &selectors)
    }

    fn element_query_selector_all(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Vec<NodeId>> {
        all(cx, this, &selectors)
    }

    fn document_query_selector(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Option<NodeId>> {
        first(cx, this, &selectors)
    }

    fn document_query_selector_all(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Vec<NodeId>> {
        all(cx, this, &selectors)
    }

    fn document_fragment_query_selector(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Option<NodeId>> {
        first(cx, this, &selectors)
    }

    fn document_fragment_query_selector_all(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Vec<NodeId>> {
        all(cx, this, &selectors)
    }
}
