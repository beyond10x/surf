//! Tokenization (13.2.5). Written by `story:html-tokenizer-markup` and
//! `story:html-tokenizer-text`; the character reference states by
//! `story:html-character-references`.

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
/// Written: the data state, the tag, attribute, comment, DOCTYPE, CDATA section and processing
/// instruction states. The character reference state is a stub that leaves `&` a character
/// (story `html-character-references`). Not written yet (story `html-tokenizer-text`): the
/// RCDATA, RAWTEXT, script data and PLAINTEXT states; started in one of them, it returns no
/// token and no parse error, and `last_start_tag`, which only those states read, is unused.
pub fn tokenize(input: &[u32], state: State, last_start_tag: Option<&str>) -> Tokenized {
    let _ = last_start_tag;
    let state = match state {
        State::Data => machine::State::Data,
        State::CdataSection => machine::State::CdataSection,
        State::Rcdata | State::Rawtext | State::ScriptData | State::Plaintext => {
            return Tokenized::default();
        }
    };
    machine::Tokenizer::new(input, state).run()
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

    #[test]
    fn a_text_state_is_not_written_yet() {
        let input: Vec<u32> = "a".chars().map(u32::from).collect();
        assert_eq!(
            tokenize(&input, State::Rcdata, Some("title")),
            Tokenized::default()
        );
    }
}
