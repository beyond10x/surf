//! Tokenization (13.2.5). Written by `story:html-tokenizer-markup` and
//! `story:html-tokenizer-text`; the character reference states by
//! `story:html-character-references`.

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
/// (`surrogate-in-input-stream`).
///
/// Not written yet: returns no token and no parse error.
pub fn tokenize(input: &[u32], state: State, last_start_tag: Option<&str>) -> Tokenized {
    let _ = (input, state, last_start_tag);
    Tokenized::default()
}
