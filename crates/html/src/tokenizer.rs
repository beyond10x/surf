//! Tokenization (13.2.5). Written by `story:html-tokenizer-markup` and
//! `story:html-tokenizer-text`; the character reference states by
//! `story:html-character-references`, with the named reference lookup and the numeric
//! replacement table of [`crate::charref`].

mod machine;

/// The state the tokenizer starts in: the ones the tree builder switches it to, and the ones
/// the html5lib-tests data starts it in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The data state.
    Data,
    /// The RCDATA state.
    Rcdata,
    /// The RAWTEXT state.
    Rawtext,
    /// The script data state.
    ScriptData,
    /// The PLAINTEXT state.
    Plaintext,
    /// The CDATA section state.
    CdataSection,
}

/// An attribute of a start or end tag token, as the tokenizer leaves it: a name and a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenAttribute {
    /// The attribute's name.
    pub name: String,
    /// The attribute's value.
    pub value: String,
}

/// A token the tokenizer emits (13.2.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// A DOCTYPE token. A missing name or identifier is `None`.
    Doctype {
        /// The name.
        name: Option<String>,
        /// The public identifier.
        public_id: Option<String>,
        /// The system identifier.
        system_id: Option<String>,
        /// The force-quirks flag.
        force_quirks: bool,
    },
    /// A start tag token.
    StartTag {
        /// The tag name.
        name: String,
        /// The attributes, in source order, duplicates dropped.
        attributes: Vec<TokenAttribute>,
        /// The self-closing flag.
        self_closing: bool,
    },
    /// An end tag token.
    EndTag {
        /// The tag name.
        name: String,
    },
    /// A comment token.
    Comment {
        /// The data.
        data: String,
    },
    /// A processing instruction token, with the target and the data the data's three-member
    /// form `["ProcessingInstruction", <target>, <data>]` compares.
    ProcessingInstruction {
        /// The target.
        target: String,
        /// The data.
        data: String,
    },
    /// A character token: one code point. A surrogate (U+D800 to U+DFFF) is a code point of the
    /// input stream too, so the value is a `u32` and not a `char`.
    Character(u32),
    /// The end-of-file token.
    EndOfFile,
}

/// A parse error, named by the code 13.2.2 gives it (`unexpected-null-character` and so on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The code.
    pub code: &'static str,
}

/// What [`tokenize`] returns.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tokenized {
    /// The tokens, in the order they were emitted.
    pub tokens: Vec<Token>,
    /// The parse errors, in the order they occurred.
    pub errors: Vec<ParseError>,
}

/// Tokenizes an input stream on its own, with no tree builder: the code points as they are
/// before preprocessing (13.2.3.5), the state to start in, and the tag name of the last start
/// tag emitted, if any (the "appropriate end tag token" of the RCDATA, RAWTEXT and script data
/// states compares against it).
///
/// The input is code points and not a `&str`, because the stream can hold surrogates
/// (`surrogate-in-input-stream`). It is preprocessed first ([`crate::input`]); the parse errors
/// of preprocessing are reported as the tokenizer consumes the code point they are for. The
/// tokens end with one [`Token::EndOfFile`].
///
/// Written: the data, RCDATA, RAWTEXT, script data and PLAINTEXT states, the tag, attribute,
/// comment, DOCTYPE, CDATA section and processing instruction states, the less-than sign, end
/// tag and script data escape states the text states lead to, and the character reference
/// states (13.2.5.77 to 13.2.5.84) of data, RCDATA and attribute values. `last_start_tag` is
/// the last start tag until the tokenizer emits one of its own; with none, no end tag is
/// appropriate and every end tag in a text state is text. A character reference becomes
/// character tokens, or code points of the attribute value; a numeric one that names no
/// allowed code point becomes U+FFFD or the code point the standard's table puts in its place.
/// The character reference states are not states a caller starts in, so [`State`] does not
/// name them.
pub fn tokenize(input: &[u32], state: State, last_start_tag: Option<&str>) -> Tokenized {
    let state = match state {
        State::Data => machine::State::Data,
        State::Rcdata => machine::State::Rcdata,
        State::Rawtext => machine::State::Rawtext,
        State::ScriptData => machine::State::ScriptData,
        State::Plaintext => machine::State::Plaintext,
        State::CdataSection => machine::State::CdataSection,
    };
    machine::Tokenizer::new(input, state, last_start_tag).run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(input: &str) -> Tokenized {
        let input: Vec<u32> = input.chars().map(u32::from).collect();
        tokenize(&input, State::Data, None)
    }

    fn codes(t: &Tokenized) -> Vec<&'static str> {
        t.errors.iter().map(|e| e.code).collect()
    }

    #[test]
    fn a_start_tag_with_attributes_drops_the_duplicate() {
        let t = run("<A b=1 B='2' c>");
        assert_eq!(
            t.tokens,
            vec![
                Token::StartTag {
                    name: "a".to_string(),
                    attributes: vec![
                        TokenAttribute {
                            name: "b".to_string(),
                            value: "1".to_string()
                        },
                        TokenAttribute {
                            name: "c".to_string(),
                            value: String::new()
                        },
                    ],
                    self_closing: false,
                },
                Token::EndOfFile,
            ]
        );
        assert_eq!(codes(&t), vec!["duplicate-attribute"]);
    }

    #[test]
    fn preprocessing_errors_take_their_place_among_the_tokenizer_errors() {
        let t = run("<\u{1}\r\n");
        assert_eq!(
            codes(&t),
            vec![
                "control-character-in-input-stream",
                "invalid-first-character-of-tag-name"
            ]
        );
        assert_eq!(
            t.tokens,
            vec![
                Token::Character(0x3C),
                Token::Character(0x01),
                Token::Character(0x0A),
                Token::EndOfFile
            ]
        );
        // The next input character's error is reported before the markup declaration open
        // state decides; one further ahead only when it is consumed.
        assert_eq!(
            codes(&run("<!\u{B}")),
            vec![
                "control-character-in-input-stream",
                "incorrectly-opened-comment"
            ]
        );
        assert_eq!(
            codes(&run("<!doc\u{80}")),
            vec![
                "incorrectly-opened-comment",
                "control-character-in-input-stream"
            ]
        );
    }

    #[test]
    fn a_processing_instruction_and_a_disallowed_target() {
        let t = run("<?foo bar??><?xml x?>");
        assert_eq!(
            t.tokens,
            vec![
                Token::ProcessingInstruction {
                    target: "foo".to_string(),
                    data: "bar?".to_string()
                },
                Token::Comment {
                    data: "?xml x?".to_string()
                },
                Token::EndOfFile,
            ]
        );
        assert_eq!(codes(&t), vec!["disallowed-processing-instruction-target"]);
    }

    #[test]
    fn cdata_in_html_content_is_a_bogus_comment() {
        let t = run("<![CDATA[x]]>");
        assert_eq!(
            t.tokens,
            vec![
                Token::Comment {
                    data: "[CDATA[x]]".to_string()
                },
                Token::EndOfFile
            ]
        );
        assert_eq!(codes(&t), vec!["cdata-in-html-content"]);
    }

    fn run_in(input: &str, state: State, last_start_tag: Option<&str>) -> Tokenized {
        let input: Vec<u32> = input.chars().map(u32::from).collect();
        tokenize(&input, state, last_start_tag)
    }

    fn characters(s: &str) -> Vec<Token> {
        s.chars().map(|c| Token::Character(u32::from(c))).collect()
    }

    fn end_tag(name: &str) -> Token {
        Token::EndTag {
            name: name.to_string(),
        }
    }

    #[test]
    fn an_appropriate_end_tag_ends_rcdata() {
        let t = run_in("a<b></TITLE>c", State::Rcdata, Some("title"));
        let mut expected = characters("a<b>");
        expected.push(end_tag("title"));
        expected.extend(characters("c"));
        expected.push(Token::EndOfFile);
        assert_eq!(t.tokens, expected);
        assert_eq!(codes(&t), Vec::<&str>::new());
    }

    #[test]
    fn an_inappropriate_end_tag_in_rcdata_is_text() {
        let mut expected = characters("a</tit></titles>");
        expected.push(Token::EndOfFile);
        assert_eq!(
            run_in("a</tit></titles>", State::Rcdata, Some("title")).tokens,
            expected
        );
        // With no last start tag, no end tag is appropriate.
        let mut expected = characters("</title>");
        expected.push(Token::EndOfFile);
        assert_eq!(run_in("</title>", State::Rcdata, None).tokens, expected);
    }

    #[test]
    fn a_script_end_tag_inside_a_double_escape_does_not_end_script_data() {
        let t = run_in(
            "<!--<script></script>--></script>",
            State::ScriptData,
            Some("script"),
        );
        let mut expected = characters("<!--<script></script>-->");
        expected.push(end_tag("script"));
        expected.push(Token::EndOfFile);
        assert_eq!(t.tokens, expected);
        assert_eq!(codes(&t), Vec::<&str>::new());
    }

    #[test]
    fn an_empty_escape_goes_back_to_script_data() {
        // `<!-->` reaches the script data escaped dash dash state, whose `>` leaves the escape:
        // the `<script>` after it starts no double escape.
        let t = run_in("<!--><script></script>", State::ScriptData, Some("script"));
        let mut expected = characters("<!--><script>");
        expected.push(end_tag("script"));
        expected.push(Token::EndOfFile);
        assert_eq!(t.tokens, expected);
    }

    #[test]
    fn eof_in_escaped_script_data_and_a_null_in_plaintext() {
        let t = run_in("<!--x", State::ScriptData, Some("script"));
        assert_eq!(codes(&t), vec!["eof-in-script-html-comment-like-text"]);
        let t = run_in("<a>\u{0}", State::Plaintext, None);
        let mut expected = characters("<a>\u{FFFD}");
        expected.push(Token::EndOfFile);
        assert_eq!(t.tokens, expected);
        assert_eq!(codes(&t), vec!["unexpected-null-character"]);
    }
}
