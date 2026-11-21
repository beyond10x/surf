//! The state machine of 13.2.5: the markup states, written by `story:html-tokenizer-markup`.
//! Each state is written from its subsection of the standard, with the standard's name and its
//! parse error codes, in the standard's order.
//!
//! Not written here: the RCDATA, RAWTEXT, script data and PLAINTEXT states and the states they
//! lead to (story `html-tokenizer-text`), and the character reference states (story
//! `html-character-references`), of which only the character reference state's "anything else"
//! entry stands, as a stub.

use super::{ParseError, Token, TokenAttribute, Tokenized};
use crate::input::{self, InputStream};

const NULL: u32 = 0x0000;
const TAB: u32 = 0x0009;
const LF: u32 = 0x000A;
const FF: u32 = 0x000C;
const SPACE: u32 = 0x0020;
const EXCLAMATION_MARK: u32 = 0x0021;
const QUOTATION_MARK: u32 = 0x0022;
const AMPERSAND: u32 = 0x0026;
const APOSTROPHE: u32 = 0x0027;
const HYPHEN_MINUS: u32 = 0x002D;
const SOLIDUS: u32 = 0x002F;
const LESS_THAN_SIGN: u32 = 0x003C;
const EQUALS_SIGN: u32 = 0x003D;
const GREATER_THAN_SIGN: u32 = 0x003E;
const QUESTION_MARK: u32 = 0x003F;
const RIGHT_SQUARE_BRACKET: u32 = 0x005D;
const LOW_LINE: u32 = 0x005F;
const GRAVE_ACCENT: u32 = 0x0060;
const REPLACEMENT_CHARACTER: u32 = 0xFFFD;

/// A state of the tokenizer (13.2.5.1 and after), named as the standard names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum State {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    BogusComment,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLessThanSign,
    CommentLessThanSignBang,
    CommentLessThanSignBangDash,
    CommentLessThanSignBangDashDash,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    AfterDoctypePublicKeyword,
    BeforeDoctypePublicIdentifier,
    DoctypePublicIdentifierDoubleQuoted,
    DoctypePublicIdentifierSingleQuoted,
    AfterDoctypePublicIdentifier,
    BetweenDoctypePublicAndSystemIdentifiers,
    AfterDoctypeSystemKeyword,
    BeforeDoctypeSystemIdentifier,
    DoctypeSystemIdentifierDoubleQuoted,
    DoctypeSystemIdentifierSingleQuoted,
    AfterDoctypeSystemIdentifier,
    BogusDoctype,
    CdataSection,
    CdataSectionBracket,
    CdataSectionEnd,
    ProcessingInstructionOpen,
    ProcessingInstructionTarget,
    AfterProcessingInstructionTarget,
    ProcessingInstructionData,
    ProcessingInstructionQuestionable,
    CharacterReference,
}

/// The start or end tag token being built.
#[derive(Debug, Default)]
struct TagToken {
    end: bool,
    name: String,
    attributes: Vec<TokenAttribute>,
    self_closing: bool,
    /// The current attribute (the last of `attributes`) is a duplicate and is removed from the
    /// token when the next attribute starts or the token is emitted (13.2.5.33).
    current_attribute_removed: bool,
}

/// The DOCTYPE token being built.
#[derive(Debug, Default)]
struct DoctypeToken {
    name: Option<String>,
    public_id: Option<String>,
    system_id: Option<String>,
    force_quirks: bool,
}

/// The processing instruction token being built.
#[derive(Debug, Default)]
struct ProcessingInstructionToken {
    target: String,
    data: String,
}

/// Appends a code point of the input stream to a token's string.
///
/// Deliberate deviation: the token types hold `String`, which cannot hold a surrogate, so a
/// surrogate appended to a name, a value, a comment, an identifier or a processing instruction
/// is appended as U+FFFD REPLACEMENT CHARACTER. A character token keeps it
/// ([`Token::Character`] is a `u32`). Decoded text never holds a surrogate; only an input given
/// as code points to [`super::tokenize`] can.
fn push(s: &mut String, c: u32) {
    s.push(char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER));
}

fn is_ascii_upper_alpha(c: u32) -> bool {
    (0x41..=0x5A).contains(&c)
}

fn is_ascii_alpha(c: u32) -> bool {
    is_ascii_upper_alpha(c) || (0x61..=0x7A).contains(&c)
}

fn is_ascii_alphanumeric(c: u32) -> bool {
    is_ascii_alpha(c) || (0x30..=0x39).contains(&c)
}

/// The lowercase version of an ASCII upper alpha (add 0x0020 to the code point).
fn to_lowercase(c: u32) -> u32 {
    c + 0x20
}

/// The tokenizer: the input stream, the state and what the states share (13.2.5).
pub(super) struct Tokenizer {
    input: InputStream,
    state: State,
    return_state: State,
    /// The current input character; `None` is EOF.
    current: Option<u32>,
    /// The next consume gives the current input character again ("reconsume").
    reconsume: bool,
    /// The code points of the stream looked at so far, whose parse errors of 13.2.3.5 are
    /// reported.
    examined: usize,
    temporary_buffer: Vec<u32>,
    tag: TagToken,
    comment: String,
    doctype: DoctypeToken,
    processing_instruction: ProcessingInstructionToken,
    out: Tokenized,
    done: bool,
}

impl Tokenizer {
    pub(super) fn new(input: &[u32], state: State) -> Tokenizer {
        Tokenizer {
            input: InputStream::new(input),
            state,
            return_state: State::Data,
            current: None,
            reconsume: false,
            examined: 0,
            temporary_buffer: Vec::new(),
            tag: TagToken::default(),
            comment: String::new(),
            doctype: DoctypeToken::default(),
            processing_instruction: ProcessingInstructionToken::default(),
            out: Tokenized::default(),
            done: false,
        }
    }

    /// Runs the state machine until it emits the end-of-file token.
    pub(super) fn run(mut self) -> Tokenized {
        while !self.done {
            self.step();
        }
        self.out
    }

    fn error(&mut self, code: &'static str) {
        self.out.errors.push(ParseError { code });
    }

    /// Reports the parse errors of 13.2.3.5 for the code points before `end` that were not
    /// looked at before.
    fn examine(&mut self, end: usize) {
        while self.examined < end {
            if let Some(code) = self.input.get(self.examined).and_then(input::parse_error) {
                self.error(code);
            }
            self.examined += 1;
        }
    }

    /// Consumes the next input character, or gives the current one again after a reconsume.
    fn consume(&mut self) -> Option<u32> {
        if self.reconsume {
            self.reconsume = false;
            return self.current;
        }
        let c = self.input.consume();
        self.examine(self.input.position());
        self.current = c;
        c
    }

    /// Whether the next input characters are `word` (an ASCII case-insensitive match when
    /// `case_insensitive`), consuming nothing. The next input character's parse error of
    /// 13.2.3.5 is reported first, and none of a code point after it (see [`crate::input`]).
    fn next_characters_are(&mut self, word: &str, case_insensitive: bool) -> bool {
        self.examine(self.input.position() + 1);
        self.input.next_characters_are(word, case_insensitive)
    }

    /// Consumes `n` characters a lookahead has matched.
    fn consume_matched(&mut self, n: usize) {
        for _ in 0..n {
            self.consume();
        }
    }

    fn switch_to(&mut self, state: State) {
        self.state = state;
    }

    fn reconsume_in(&mut self, state: State) {
        self.reconsume = true;
        self.state = state;
    }

    fn emit(&mut self, token: Token) {
        self.out.tokens.push(token);
    }

    fn emit_character(&mut self, c: u32) {
        self.emit(Token::Character(c));
    }

    fn emit_end_of_file(&mut self) {
        self.emit(Token::EndOfFile);
        self.done = true;
    }

    fn create_tag(&mut self, end: bool) {
        self.tag = TagToken {
            end,
            ..TagToken::default()
        };
    }

    /// Removes the current attribute from the tag token when it was found a duplicate.
    fn finish_attribute(&mut self) {
        if self.tag.current_attribute_removed {
            self.tag.attributes.pop();
            self.tag.current_attribute_removed = false;
        }
    }

    fn start_attribute(&mut self) {
        self.finish_attribute();
        self.tag.attributes.push(TokenAttribute {
            name: String::new(),
            value: String::new(),
        });
    }

    fn current_attribute(&mut self) -> &mut TokenAttribute {
        if self.tag.attributes.is_empty() {
            self.start_attribute();
        }
        let last = self.tag.attributes.len() - 1;
        &mut self.tag.attributes[last]
    }

    /// Leaving the attribute name state: a name already on the token is a
    /// `duplicate-attribute` parse error, and the new attribute is removed (13.2.5.33).
    fn leave_attribute_name(&mut self) {
        if let Some((current, earlier)) = self.tag.attributes.split_last()
            && earlier.iter().any(|a| a.name == current.name)
        {
            self.error("duplicate-attribute");
            self.tag.current_attribute_removed = true;
        }
    }

    /// Emits the current tag token, with the parse errors 13.2.5 gives an end tag token emitted
    /// with attributes or with its self-closing flag set.
    fn emit_tag(&mut self) {
        self.finish_attribute();
        let tag = std::mem::take(&mut self.tag);
        if tag.end {
            if !tag.attributes.is_empty() {
                self.error("end-tag-with-attributes");
            }
            if tag.self_closing {
                self.error("end-tag-with-trailing-solidus");
            }
            self.emit(Token::EndTag { name: tag.name });
        } else {
            self.emit(Token::StartTag {
                name: tag.name,
                attributes: tag.attributes,
                self_closing: tag.self_closing,
            });
        }
    }

    fn create_comment(&mut self, data: &str) {
        self.comment = data.to_string();
    }

    fn emit_comment(&mut self) {
        let data = std::mem::take(&mut self.comment);
        self.emit(Token::Comment { data });
    }

    /// Creates a DOCTYPE token: name, public and system identifier missing, force-quirks off.
    fn create_doctype(&mut self) {
        self.doctype = DoctypeToken::default();
    }

    fn emit_doctype(&mut self) {
        let doctype = std::mem::take(&mut self.doctype);
        self.emit(Token::Doctype {
            name: doctype.name,
            public_id: doctype.public_id,
            system_id: doctype.system_id,
            force_quirks: doctype.force_quirks,
        });
    }

    fn doctype_name(&mut self) -> &mut String {
        self.doctype.name.get_or_insert_with(String::new)
    }

    fn doctype_public_id(&mut self) -> &mut String {
        self.doctype.public_id.get_or_insert_with(String::new)
    }

    fn doctype_system_id(&mut self) -> &mut String {
        self.doctype.system_id.get_or_insert_with(String::new)
    }

    /// "Convert the temporary buffer to a comment": a comment token whose data is "?" and the
    /// code points of the temporary buffer.
    fn convert_temporary_buffer_to_comment(&mut self) {
        let mut data = String::from("?");
        for &c in &self.temporary_buffer {
            push(&mut data, c);
        }
        self.comment = data;
    }

    fn emit_processing_instruction(&mut self) {
        let pi = std::mem::take(&mut self.processing_instruction);
        self.emit(Token::ProcessingInstruction {
            target: pi.target,
            data: pi.data,
        });
    }

    /// Whether the return state is one of the attribute value states: a character reference is
    /// then "consumed as part of an attribute".
    fn consumed_as_part_of_an_attribute(&self) -> bool {
        matches!(
            self.return_state,
            State::AttributeValueDoubleQuoted
                | State::AttributeValueSingleQuoted
                | State::AttributeValueUnquoted
        )
    }

    /// "Flush code points consumed as a character reference".
    fn flush_code_points_consumed_as_a_character_reference(&mut self) {
        let buffer = std::mem::take(&mut self.temporary_buffer);
        if self.consumed_as_part_of_an_attribute() {
            for &c in &buffer {
                push(&mut self.current_attribute().value, c);
            }
        } else {
            for &c in &buffer {
                self.emit_character(c);
            }
        }
    }

    /// One step of the state machine: the current state's subsection, once.
    fn step(&mut self) {
        match self.state {
            // 13.2.5.1 Data state
            State::Data => match self.consume() {
                Some(AMPERSAND) => {
                    self.return_state = State::Data;
                    self.switch_to(State::CharacterReference);
                }
                Some(LESS_THAN_SIGN) => self.switch_to(State::TagOpen),
                Some(NULL) => {
                    self.error("unexpected-null-character");
                    self.emit_character(NULL);
                }
                None => self.emit_end_of_file(),
                Some(c) => self.emit_character(c),
            },
            // 13.2.5.6 Tag open state
            State::TagOpen => match self.consume() {
                Some(EXCLAMATION_MARK) => self.switch_to(State::MarkupDeclarationOpen),
                Some(SOLIDUS) => self.switch_to(State::EndTagOpen),
                Some(c) if is_ascii_alpha(c) => {
                    self.create_tag(false);
                    self.reconsume_in(State::TagName);
                }
                Some(QUESTION_MARK) => {
                    self.temporary_buffer.clear();
                    self.switch_to(State::ProcessingInstructionOpen);
                }
                None => {
                    self.error("eof-before-tag-name");
                    self.emit_character(LESS_THAN_SIGN);
                    self.emit_end_of_file();
                }
                Some(_) => {
                    self.error("invalid-first-character-of-tag-name");
                    self.emit_character(LESS_THAN_SIGN);
                    self.reconsume_in(State::Data);
                }
            },
            // 13.2.5.7 End tag open state
            State::EndTagOpen => match self.consume() {
                Some(c) if is_ascii_alpha(c) => {
                    self.create_tag(true);
                    self.reconsume_in(State::TagName);
                }
                Some(GREATER_THAN_SIGN) => {
                    self.error("missing-end-tag-name");
                    self.switch_to(State::Data);
                }
                None => {
                    self.error("eof-before-tag-name");
                    self.emit_character(LESS_THAN_SIGN);
                    self.emit_character(SOLIDUS);
                    self.emit_end_of_file();
                }
                Some(_) => {
                    self.error("invalid-first-character-of-tag-name");
                    self.create_comment("");
                    self.reconsume_in(State::BogusComment);
                }
            },
            // 13.2.5.8 Tag name state
            State::TagName => match self.consume() {
                Some(TAB | LF | FF | SPACE) => self.switch_to(State::BeforeAttributeName),
                Some(SOLIDUS) => self.switch_to(State::SelfClosingStartTag),
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_tag();
                }
                Some(c) if is_ascii_upper_alpha(c) => push(&mut self.tag.name, to_lowercase(c)),
                Some(NULL) => {
                    self.error("unexpected-null-character");
                    push(&mut self.tag.name, REPLACEMENT_CHARACTER);
                }
                None => {
                    self.error("eof-in-tag");
                    self.emit_end_of_file();
                }
                Some(c) => push(&mut self.tag.name, c),
            },
            // 13.2.5.32 Before attribute name state
            State::BeforeAttributeName => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {}
                Some(SOLIDUS | GREATER_THAN_SIGN) | None => {
                    self.reconsume_in(State::AfterAttributeName)
                }
                Some(EQUALS_SIGN) => {
                    self.error("unexpected-equals-sign-before-attribute-name");
                    self.start_attribute();
                    push(&mut self.current_attribute().name, EQUALS_SIGN);
                    self.switch_to(State::AttributeName);
                }
                Some(_) => {
                    self.start_attribute();
                    self.reconsume_in(State::AttributeName);
                }
            },
            // 13.2.5.33 Attribute name state
            State::AttributeName => match self.consume() {
                Some(TAB | LF | FF | SPACE | SOLIDUS | GREATER_THAN_SIGN) | None => {
                    self.leave_attribute_name();
                    self.reconsume_in(State::AfterAttributeName);
                }
                Some(EQUALS_SIGN) => {
                    self.leave_attribute_name();
                    self.switch_to(State::BeforeAttributeValue);
                }
                Some(c) if is_ascii_upper_alpha(c) => {
                    push(&mut self.current_attribute().name, to_lowercase(c))
                }
                Some(NULL) => {
                    self.error("unexpected-null-character");
                    push(&mut self.current_attribute().name, REPLACEMENT_CHARACTER);
                }
                Some(c) => {
                    if matches!(c, QUOTATION_MARK | APOSTROPHE | LESS_THAN_SIGN) {
                        self.error("unexpected-character-in-attribute-name");
                    }
                    push(&mut self.current_attribute().name, c);
                }
            },
            // 13.2.5.34 After attribute name state
            State::AfterAttributeName => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {}
                Some(SOLIDUS) => self.switch_to(State::SelfClosingStartTag),
                Some(EQUALS_SIGN) => self.switch_to(State::BeforeAttributeValue),
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_tag();
                }
                None => {
                    self.error("eof-in-tag");
                    self.emit_end_of_file();
                }
                Some(_) => {
                    self.start_attribute();
                    self.reconsume_in(State::AttributeName);
                }
            },
            // 13.2.5.35 Before attribute value state
            State::BeforeAttributeValue => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {}
                Some(QUOTATION_MARK) => self.switch_to(State::AttributeValueDoubleQuoted),
                Some(APOSTROPHE) => self.switch_to(State::AttributeValueSingleQuoted),
                Some(GREATER_THAN_SIGN) => {
                    self.error("missing-attribute-value");
                    self.switch_to(State::Data);
                    self.emit_tag();
                }
                _ => self.reconsume_in(State::AttributeValueUnquoted),
            },
            // 13.2.5.36 Attribute value (double-quoted) state
            // 13.2.5.37 Attribute value (single-quoted) state
            State::AttributeValueDoubleQuoted | State::AttributeValueSingleQuoted => {
                let quote = if self.state == State::AttributeValueDoubleQuoted {
                    QUOTATION_MARK
                } else {
                    APOSTROPHE
                };
                match self.consume() {
                    Some(c) if c == quote => self.switch_to(State::AfterAttributeValueQuoted),
                    Some(AMPERSAND) => {
                        self.return_state = self.state;
                        self.switch_to(State::CharacterReference);
                    }
                    Some(NULL) => {
                        self.error("unexpected-null-character");
                        push(&mut self.current_attribute().value, REPLACEMENT_CHARACTER);
                    }
                    None => {
                        self.error("eof-in-tag");
                        self.emit_end_of_file();
                    }
                    Some(c) => push(&mut self.current_attribute().value, c),
                }
            }
            // 13.2.5.38 Attribute value (unquoted) state
            State::AttributeValueUnquoted => match self.consume() {
                Some(TAB | LF | FF | SPACE) => self.switch_to(State::BeforeAttributeName),
                Some(AMPERSAND) => {
                    self.return_state = State::AttributeValueUnquoted;
                    self.switch_to(State::CharacterReference);
                }
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_tag();
                }
                Some(NULL) => {
                    self.error("unexpected-null-character");
                    push(&mut self.current_attribute().value, REPLACEMENT_CHARACTER);
                }
                None => {
                    self.error("eof-in-tag");
                    self.emit_end_of_file();
                }
                Some(c) => {
                    if matches!(
                        c,
                        QUOTATION_MARK | APOSTROPHE | LESS_THAN_SIGN | EQUALS_SIGN | GRAVE_ACCENT
                    ) {
                        self.error("unexpected-character-in-unquoted-attribute-value");
                    }
                    push(&mut self.current_attribute().value, c);
                }
            },
            // 13.2.5.39 After attribute value (quoted) state
            State::AfterAttributeValueQuoted => match self.consume() {
                Some(TAB | LF | FF | SPACE) => self.switch_to(State::BeforeAttributeName),
                Some(SOLIDUS) => self.switch_to(State::SelfClosingStartTag),
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_tag();
                }
                None => {
                    self.error("eof-in-tag");
                    self.emit_end_of_file();
                }
                Some(_) => {
                    self.error("missing-whitespace-between-attributes");
                    self.reconsume_in(State::BeforeAttributeName);
                }
            },
            // 13.2.5.40 Self-closing start tag state
            State::SelfClosingStartTag => match self.consume() {
                Some(GREATER_THAN_SIGN) => {
                    self.tag.self_closing = true;
                    self.switch_to(State::Data);
                    self.emit_tag();
                }
                None => {
                    self.error("eof-in-tag");
                    self.emit_end_of_file();
                }
                Some(_) => {
                    self.error("unexpected-solidus-in-tag");
                    self.reconsume_in(State::BeforeAttributeName);
                }
            },
            // 13.2.5.41 Bogus comment state
            State::BogusComment => match self.consume() {
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_comment();
                }
                None => {
                    self.emit_comment();
                    self.emit_end_of_file();
                }
                Some(NULL) => {
                    self.error("unexpected-null-character");
                    push(&mut self.comment, REPLACEMENT_CHARACTER);
                }
                Some(c) => push(&mut self.comment, c),
            },
            // 13.2.5.42 Markup declaration open state
            State::MarkupDeclarationOpen => self.markup_declaration_open(),
            // 13.2.5.43 Comment start state
            State::CommentStart => match self.consume() {
                Some(HYPHEN_MINUS) => self.switch_to(State::CommentStartDash),
                Some(GREATER_THAN_SIGN) => {
                    self.error("abrupt-closing-of-empty-comment");
                    self.switch_to(State::Data);
                    self.emit_comment();
                }
                _ => self.reconsume_in(State::Comment),
            },
            // 13.2.5.44 Comment start dash state
            State::CommentStartDash => match self.consume() {
                Some(HYPHEN_MINUS) => self.switch_to(State::CommentEnd),
                Some(GREATER_THAN_SIGN) => {
                    self.error("abrupt-closing-of-empty-comment");
                    self.switch_to(State::Data);
                    self.emit_comment();
                }
                None => self.eof_in_comment(),
                Some(_) => {
                    push(&mut self.comment, HYPHEN_MINUS);
                    self.reconsume_in(State::Comment);
                }
            },
            // 13.2.5.45 Comment state
            State::Comment => match self.consume() {
                Some(LESS_THAN_SIGN) => {
                    push(&mut self.comment, LESS_THAN_SIGN);
                    self.switch_to(State::CommentLessThanSign);
                }
                Some(HYPHEN_MINUS) => self.switch_to(State::CommentEndDash),
                Some(NULL) => {
                    self.error("unexpected-null-character");
                    push(&mut self.comment, REPLACEMENT_CHARACTER);
                }
                None => self.eof_in_comment(),
                Some(c) => push(&mut self.comment, c),
            },
            // 13.2.5.46 Comment less-than sign state
            State::CommentLessThanSign => match self.consume() {
                Some(EXCLAMATION_MARK) => {
                    push(&mut self.comment, EXCLAMATION_MARK);
                    self.switch_to(State::CommentLessThanSignBang);
                }
                Some(LESS_THAN_SIGN) => push(&mut self.comment, LESS_THAN_SIGN),
                _ => self.reconsume_in(State::Comment),
            },
            // 13.2.5.47 Comment less-than sign bang state
            State::CommentLessThanSignBang => match self.consume() {
                Some(HYPHEN_MINUS) => self.switch_to(State::CommentLessThanSignBangDash),
                _ => self.reconsume_in(State::Comment),
            },
            // 13.2.5.48 Comment less-than sign bang dash state
            State::CommentLessThanSignBangDash => match self.consume() {
                Some(HYPHEN_MINUS) => self.switch_to(State::CommentLessThanSignBangDashDash),
                _ => self.reconsume_in(State::CommentEndDash),
            },
            // 13.2.5.49 Comment less-than sign bang dash dash state
            State::CommentLessThanSignBangDashDash => match self.consume() {
                Some(GREATER_THAN_SIGN) | None => self.reconsume_in(State::CommentEnd),
                Some(_) => {
                    self.error("nested-comment");
                    self.reconsume_in(State::CommentEnd);
                }
            },
            // 13.2.5.50 Comment end dash state
            State::CommentEndDash => match self.consume() {
                Some(HYPHEN_MINUS) => self.switch_to(State::CommentEnd),
                None => self.eof_in_comment(),
                Some(_) => {
                    push(&mut self.comment, HYPHEN_MINUS);
                    self.reconsume_in(State::Comment);
                }
            },
            // 13.2.5.51 Comment end state
            State::CommentEnd => match self.consume() {
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_comment();
                }
                Some(EXCLAMATION_MARK) => self.switch_to(State::CommentEndBang),
                Some(HYPHEN_MINUS) => push(&mut self.comment, HYPHEN_MINUS),
                None => self.eof_in_comment(),
                Some(_) => {
                    self.comment.push_str("--");
                    self.reconsume_in(State::Comment);
                }
            },
            // 13.2.5.52 Comment end bang state
            State::CommentEndBang => match self.consume() {
                Some(HYPHEN_MINUS) => {
                    self.comment.push_str("--!");
                    self.switch_to(State::CommentEndDash);
                }
                Some(GREATER_THAN_SIGN) => {
                    self.error("incorrectly-closed-comment");
                    self.switch_to(State::Data);
                    self.emit_comment();
                }
                None => self.eof_in_comment(),
                Some(_) => {
                    self.comment.push_str("--!");
                    self.reconsume_in(State::Comment);
                }
            },
            // 13.2.5.53 DOCTYPE state
            State::Doctype => match self.consume() {
                Some(TAB | LF | FF | SPACE) => self.switch_to(State::BeforeDoctypeName),
                Some(GREATER_THAN_SIGN) => self.reconsume_in(State::BeforeDoctypeName),
                None => {
                    self.error("eof-in-doctype");
                    self.create_doctype();
                    self.doctype.force_quirks = true;
                    self.emit_doctype();
                    self.emit_end_of_file();
                }
                Some(_) => {
                    self.error("missing-whitespace-before-doctype-name");
                    self.reconsume_in(State::BeforeDoctypeName);
                }
            },
            // 13.2.5.54 Before DOCTYPE name state
            State::BeforeDoctypeName => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {}
                Some(c) if is_ascii_upper_alpha(c) => {
                    self.create_doctype();
                    push(self.doctype_name(), to_lowercase(c));
                    self.switch_to(State::DoctypeName);
                }
                Some(NULL) => {
                    self.error("unexpected-null-character");
                    self.create_doctype();
                    push(self.doctype_name(), REPLACEMENT_CHARACTER);
                    self.switch_to(State::DoctypeName);
                }
                Some(GREATER_THAN_SIGN) => {
                    self.error("missing-doctype-name");
                    self.create_doctype();
                    self.doctype.force_quirks = true;
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                None => {
                    self.error("eof-in-doctype");
                    self.create_doctype();
                    self.doctype.force_quirks = true;
                    self.emit_doctype();
                    self.emit_end_of_file();
                }
                Some(c) => {
                    self.create_doctype();
                    push(self.doctype_name(), c);
                    self.switch_to(State::DoctypeName);
                }
            },
            // 13.2.5.55 DOCTYPE name state
            State::DoctypeName => match self.consume() {
                Some(TAB | LF | FF | SPACE) => self.switch_to(State::AfterDoctypeName),
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                Some(c) if is_ascii_upper_alpha(c) => push(self.doctype_name(), to_lowercase(c)),
                Some(NULL) => {
                    self.error("unexpected-null-character");
                    push(self.doctype_name(), REPLACEMENT_CHARACTER);
                }
                None => self.eof_in_doctype(),
                Some(c) => push(self.doctype_name(), c),
            },
            // 13.2.5.56 After DOCTYPE name state
            State::AfterDoctypeName => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {}
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                None => self.eof_in_doctype(),
                Some(c) => {
                    if self.six_characters_match(c, "PUBLIC") {
                        self.consume_matched(5);
                        self.switch_to(State::AfterDoctypePublicKeyword);
                    } else if self.six_characters_match(c, "SYSTEM") {
                        self.consume_matched(5);
                        self.switch_to(State::AfterDoctypeSystemKeyword);
                    } else {
                        self.error("invalid-character-sequence-after-doctype-name");
                        self.doctype.force_quirks = true;
                        self.reconsume_in(State::BogusDoctype);
                    }
                }
            },
            // 13.2.5.57 After DOCTYPE public keyword state
            State::AfterDoctypePublicKeyword => match self.consume() {
                Some(TAB | LF | FF | SPACE) => self.switch_to(State::BeforeDoctypePublicIdentifier),
                Some(QUOTATION_MARK) => {
                    self.error("missing-whitespace-after-doctype-public-keyword");
                    self.doctype.public_id = Some(String::new());
                    self.switch_to(State::DoctypePublicIdentifierDoubleQuoted);
                }
                Some(APOSTROPHE) => {
                    self.error("missing-whitespace-after-doctype-public-keyword");
                    self.doctype.public_id = Some(String::new());
                    self.switch_to(State::DoctypePublicIdentifierSingleQuoted);
                }
                Some(GREATER_THAN_SIGN) => {
                    self.error("missing-doctype-public-identifier");
                    self.doctype.force_quirks = true;
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                None => self.eof_in_doctype(),
                Some(_) => {
                    self.error("missing-quote-before-doctype-public-identifier");
                    self.doctype.force_quirks = true;
                    self.reconsume_in(State::BogusDoctype);
                }
            },
            // 13.2.5.58 Before DOCTYPE public identifier state
            State::BeforeDoctypePublicIdentifier => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {}
                Some(QUOTATION_MARK) => {
                    self.doctype.public_id = Some(String::new());
                    self.switch_to(State::DoctypePublicIdentifierDoubleQuoted);
                }
                Some(APOSTROPHE) => {
                    self.doctype.public_id = Some(String::new());
                    self.switch_to(State::DoctypePublicIdentifierSingleQuoted);
                }
                Some(GREATER_THAN_SIGN) => {
                    self.error("missing-doctype-public-identifier");
                    self.doctype.force_quirks = true;
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                None => self.eof_in_doctype(),
                Some(_) => {
                    self.error("missing-quote-before-doctype-public-identifier");
                    self.doctype.force_quirks = true;
                    self.reconsume_in(State::BogusDoctype);
                }
            },
            // 13.2.5.59 DOCTYPE public identifier (double-quoted) state
            // 13.2.5.60 DOCTYPE public identifier (single-quoted) state
            State::DoctypePublicIdentifierDoubleQuoted
            | State::DoctypePublicIdentifierSingleQuoted => {
                let quote = if self.state == State::DoctypePublicIdentifierDoubleQuoted {
                    QUOTATION_MARK
                } else {
                    APOSTROPHE
                };
                match self.consume() {
                    Some(c) if c == quote => self.switch_to(State::AfterDoctypePublicIdentifier),
                    Some(NULL) => {
                        self.error("unexpected-null-character");
                        push(self.doctype_public_id(), REPLACEMENT_CHARACTER);
                    }
                    Some(GREATER_THAN_SIGN) => {
                        self.error("abrupt-doctype-public-identifier");
                        self.doctype.force_quirks = true;
                        self.switch_to(State::Data);
                        self.emit_doctype();
                    }
                    None => self.eof_in_doctype(),
                    Some(c) => push(self.doctype_public_id(), c),
                }
            }
            // 13.2.5.61 After DOCTYPE public identifier state
            State::AfterDoctypePublicIdentifier => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {
                    self.switch_to(State::BetweenDoctypePublicAndSystemIdentifiers)
                }
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                Some(QUOTATION_MARK) => {
                    self.error("missing-whitespace-between-doctype-public-and-system-identifiers");
                    self.doctype.system_id = Some(String::new());
                    self.switch_to(State::DoctypeSystemIdentifierDoubleQuoted);
                }
                Some(APOSTROPHE) => {
                    self.error("missing-whitespace-between-doctype-public-and-system-identifiers");
                    self.doctype.system_id = Some(String::new());
                    self.switch_to(State::DoctypeSystemIdentifierSingleQuoted);
                }
                None => self.eof_in_doctype(),
                Some(_) => {
                    self.error("missing-quote-before-doctype-system-identifier");
                    self.doctype.force_quirks = true;
                    self.reconsume_in(State::BogusDoctype);
                }
            },
            // 13.2.5.62 Between DOCTYPE public and system identifiers state
            State::BetweenDoctypePublicAndSystemIdentifiers => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {}
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                Some(QUOTATION_MARK) => {
                    self.doctype.system_id = Some(String::new());
                    self.switch_to(State::DoctypeSystemIdentifierDoubleQuoted);
                }
                Some(APOSTROPHE) => {
                    self.doctype.system_id = Some(String::new());
                    self.switch_to(State::DoctypeSystemIdentifierSingleQuoted);
                }
                None => self.eof_in_doctype(),
                Some(_) => {
                    self.error("missing-quote-before-doctype-system-identifier");
                    self.doctype.force_quirks = true;
                    self.reconsume_in(State::BogusDoctype);
                }
            },
            // 13.2.5.63 After DOCTYPE system keyword state
            State::AfterDoctypeSystemKeyword => match self.consume() {
                Some(TAB | LF | FF | SPACE) => self.switch_to(State::BeforeDoctypeSystemIdentifier),
                Some(QUOTATION_MARK) => {
                    self.error("missing-whitespace-after-doctype-system-keyword");
                    self.doctype.system_id = Some(String::new());
                    self.switch_to(State::DoctypeSystemIdentifierDoubleQuoted);
                }
                Some(APOSTROPHE) => {
                    self.error("missing-whitespace-after-doctype-system-keyword");
                    self.doctype.system_id = Some(String::new());
                    self.switch_to(State::DoctypeSystemIdentifierSingleQuoted);
                }
                Some(GREATER_THAN_SIGN) => {
                    self.error("missing-doctype-system-identifier");
                    self.doctype.force_quirks = true;
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                None => self.eof_in_doctype(),
                Some(_) => {
                    self.error("missing-quote-before-doctype-system-identifier");
                    self.doctype.force_quirks = true;
                    self.reconsume_in(State::BogusDoctype);
                }
            },
            // 13.2.5.64 Before DOCTYPE system identifier state
            State::BeforeDoctypeSystemIdentifier => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {}
                Some(QUOTATION_MARK) => {
                    self.doctype.system_id = Some(String::new());
                    self.switch_to(State::DoctypeSystemIdentifierDoubleQuoted);
                }
                Some(APOSTROPHE) => {
                    self.doctype.system_id = Some(String::new());
                    self.switch_to(State::DoctypeSystemIdentifierSingleQuoted);
                }
                Some(GREATER_THAN_SIGN) => {
                    self.error("missing-doctype-system-identifier");
                    self.doctype.force_quirks = true;
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                None => self.eof_in_doctype(),
                Some(_) => {
                    self.error("missing-quote-before-doctype-system-identifier");
                    self.doctype.force_quirks = true;
                    self.reconsume_in(State::BogusDoctype);
                }
            },
            // 13.2.5.65 DOCTYPE system identifier (double-quoted) state
            // 13.2.5.66 DOCTYPE system identifier (single-quoted) state
            State::DoctypeSystemIdentifierDoubleQuoted
            | State::DoctypeSystemIdentifierSingleQuoted => {
                let quote = if self.state == State::DoctypeSystemIdentifierDoubleQuoted {
                    QUOTATION_MARK
                } else {
                    APOSTROPHE
                };
                match self.consume() {
                    Some(c) if c == quote => self.switch_to(State::AfterDoctypeSystemIdentifier),
                    Some(NULL) => {
                        self.error("unexpected-null-character");
                        push(self.doctype_system_id(), REPLACEMENT_CHARACTER);
                    }
                    Some(GREATER_THAN_SIGN) => {
                        self.error("abrupt-doctype-system-identifier");
                        self.doctype.force_quirks = true;
                        self.switch_to(State::Data);
                        self.emit_doctype();
                    }
                    None => self.eof_in_doctype(),
                    Some(c) => push(self.doctype_system_id(), c),
                }
            }
            // 13.2.5.67 After DOCTYPE system identifier state
            State::AfterDoctypeSystemIdentifier => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {}
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                None => self.eof_in_doctype(),
                Some(_) => {
                    // This does not set the force-quirks flag.
                    self.error("unexpected-character-after-doctype-system-identifier");
                    self.reconsume_in(State::BogusDoctype);
                }
            },
            // 13.2.5.68 Bogus DOCTYPE state
            State::BogusDoctype => match self.consume() {
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_doctype();
                }
                Some(NULL) => self.error("unexpected-null-character"),
                None => {
                    self.emit_doctype();
                    self.emit_end_of_file();
                }
                Some(_) => {}
            },
            // 13.2.5.69 CDATA section state. U+0000 NULL is the tree builder's (foreign
            // content), not this state's.
            State::CdataSection => match self.consume() {
                Some(RIGHT_SQUARE_BRACKET) => self.switch_to(State::CdataSectionBracket),
                None => {
                    self.error("eof-in-cdata");
                    self.emit_end_of_file();
                }
                Some(c) => self.emit_character(c),
            },
            // 13.2.5.70 CDATA section bracket state
            State::CdataSectionBracket => match self.consume() {
                Some(RIGHT_SQUARE_BRACKET) => self.switch_to(State::CdataSectionEnd),
                _ => {
                    self.emit_character(RIGHT_SQUARE_BRACKET);
                    self.reconsume_in(State::CdataSection);
                }
            },
            // 13.2.5.71 CDATA section end state
            State::CdataSectionEnd => match self.consume() {
                Some(RIGHT_SQUARE_BRACKET) => self.emit_character(RIGHT_SQUARE_BRACKET),
                Some(GREATER_THAN_SIGN) => self.switch_to(State::Data),
                _ => {
                    self.emit_character(RIGHT_SQUARE_BRACKET);
                    self.emit_character(RIGHT_SQUARE_BRACKET);
                    self.reconsume_in(State::CdataSection);
                }
            },
            // 13.2.5.72 Processing instruction open state
            State::ProcessingInstructionOpen => match self.consume() {
                Some(c) if is_ascii_alpha(c) || c == LOW_LINE => {
                    self.reconsume_in(State::ProcessingInstructionTarget)
                }
                None => {
                    self.error("eof-in-processing-instruction");
                    self.emit_end_of_file();
                }
                Some(_) => {
                    self.error("invalid-first-character-of-processing-instruction-target");
                    self.convert_temporary_buffer_to_comment();
                    self.reconsume_in(State::BogusComment);
                }
            },
            // 13.2.5.73 Processing instruction target state
            State::ProcessingInstructionTarget => match self.consume() {
                Some(TAB | LF | FF | SPACE | QUESTION_MARK | GREATER_THAN_SIGN) => {
                    let mut target = String::new();
                    for &c in &self.temporary_buffer {
                        push(&mut target, c);
                    }
                    if target.eq_ignore_ascii_case("xml")
                        || target.eq_ignore_ascii_case("xml-stylesheet")
                    {
                        self.error("disallowed-processing-instruction-target");
                        self.convert_temporary_buffer_to_comment();
                        self.reconsume_in(State::BogusComment);
                    } else {
                        self.processing_instruction = ProcessingInstructionToken {
                            target,
                            data: String::new(),
                        };
                        self.reconsume_in(State::AfterProcessingInstructionTarget);
                    }
                }
                Some(c) if is_ascii_alphanumeric(c) || c == HYPHEN_MINUS || c == LOW_LINE => {
                    self.temporary_buffer.push(c)
                }
                None => {
                    self.error("eof-in-processing-instruction");
                    self.emit_end_of_file();
                }
                Some(_) => {
                    self.error("invalid-processing-instruction-target");
                    self.convert_temporary_buffer_to_comment();
                    self.reconsume_in(State::BogusComment);
                }
            },
            // 13.2.5.74 After processing instruction target state
            State::AfterProcessingInstructionTarget => match self.consume() {
                Some(TAB | LF | FF | SPACE) => {}
                _ => self.reconsume_in(State::ProcessingInstructionData),
            },
            // 13.2.5.75 Processing instruction data state
            State::ProcessingInstructionData => match self.consume() {
                Some(QUESTION_MARK) => self.switch_to(State::ProcessingInstructionQuestionable),
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_processing_instruction();
                }
                None => {
                    self.error("eof-in-processing-instruction");
                    self.emit_end_of_file();
                }
                Some(c) => push(&mut self.processing_instruction.data, c),
            },
            // 13.2.5.76 Processing instruction questionable state
            State::ProcessingInstructionQuestionable => match self.consume() {
                Some(GREATER_THAN_SIGN) => {
                    self.switch_to(State::Data);
                    self.emit_processing_instruction();
                }
                None => {
                    self.error("eof-in-processing-instruction");
                    self.emit_end_of_file();
                }
                Some(_) => {
                    push(&mut self.processing_instruction.data, QUESTION_MARK);
                    self.reconsume_in(State::ProcessingInstructionData);
                }
            },
            // 13.2.5.77 Character reference state, as a stub: only its "anything else" entry,
            // so that `&` stays a character. Story `html-character-references` writes the
            // state and the ones after it.
            State::CharacterReference => {
                self.temporary_buffer = vec![AMPERSAND];
                self.consume();
                self.flush_code_points_consumed_as_a_character_reference();
                self.reconsume_in(self.return_state);
            }
        }
    }

    /// 13.2.5.42 Markup declaration open state: it looks at the next few characters and
    /// consumes only those it matches.
    fn markup_declaration_open(&mut self) {
        if self.next_characters_are("--", false) {
            self.consume_matched(2);
            self.create_comment("");
            self.switch_to(State::CommentStart);
        } else if self.next_characters_are("DOCTYPE", true) {
            self.consume_matched(7);
            self.switch_to(State::Doctype);
        } else if self.next_characters_are("[CDATA[", false) {
            self.consume_matched(7);
            // "If there is an adjusted current node and it is not an element in the HTML
            // namespace": the tokenizer has no tree builder yet, so the answer is no, as for
            // the HTML namespace. Story `html-tree-foreign` adds the hook the tree builder
            // answers.
            self.error("cdata-in-html-content");
            self.create_comment("[CDATA[");
            self.switch_to(State::BogusComment);
        } else {
            self.error("incorrectly-opened-comment");
            self.create_comment("");
            self.switch_to(State::BogusComment);
        }
    }

    /// Whether the six characters starting from the current input character `c` are an ASCII
    /// case-insensitive match for `word` (13.2.5.56).
    fn six_characters_match(&mut self, c: u32, word: &str) -> bool {
        let first = u32::from(word.as_bytes()[0]);
        (c == first || c == first + 0x20) && self.next_characters_are(&word[1..], true)
    }

    /// The EOF entry of the comment states: an `eof-in-comment` parse error, the comment token
    /// and an end-of-file token emitted.
    fn eof_in_comment(&mut self) {
        self.error("eof-in-comment");
        self.emit_comment();
        self.emit_end_of_file();
    }

    /// The EOF entry of the DOCTYPE states after the token is created: an `eof-in-doctype`
    /// parse error, force-quirks on, the DOCTYPE token and an end-of-file token emitted.
    fn eof_in_doctype(&mut self) {
        self.error("eof-in-doctype");
        self.doctype.force_quirks = true;
        self.emit_doctype();
        self.emit_end_of_file();
    }
}
