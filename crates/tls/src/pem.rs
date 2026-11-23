//! PEM (RFC 7468): the blocks between `-----BEGIN <label>-----` and `-----END <label>-----`
//! with equal labels, their content base64 of RFC 4648 with line breaks, its padding checked.
//! Text outside blocks is ignored. Each block is returned with its label. The reader looks at
//! each byte a bounded number of times and allocates only for the blocks it returns.

use std::fmt;

/// One block: its label and its decoded bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The label of the BEGIN and END lines, such as `CERTIFICATE`.
    pub label: String,
    /// The bytes the base64 decodes to.
    pub data: Vec<u8>,
}

/// Why PEM text was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Error {
    /// A block whose content is not base64 with correct padding.
    BadBase64,
    /// A BEGIN line without its END line (the text ends, or another BEGIN comes first).
    UnterminatedBlock,
    /// An END line with another label than its BEGIN line.
    LabelMismatch,
    /// A BEGIN or END line without its closing dashes, or a label with a character outside
    /// space to `~`.
    BadLabel,
}

impl Error {
    /// The name of the kind, as the variant is written.
    pub fn name(self) -> &'static str {
        match self {
            Error::BadBase64 => "BadBase64",
            Error::UnterminatedBlock => "UnterminatedBlock",
            Error::LabelMismatch => "LabelMismatch",
            Error::BadLabel => "BadLabel",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl std::error::Error for Error {}

/// Reads every block of `text`.
pub fn parse(text: &[u8]) -> Result<Vec<Block>, Error> {
    parse_counting(text).0
}

/// [`parse`], and the count of bytes it looked at, for the test that holds that count to the
/// length of the input.
#[doc(hidden)]
pub fn parse_counting(text: &[u8]) -> (Result<Vec<Block>, Error>, usize) {
    let mut work = 0;
    let result = blocks(text, &mut work);
    (result, work)
}

const BEGIN: &[u8] = b"-----BEGIN ";
const END: &[u8] = b"-----END ";
const DASHES: &[u8] = b"-----";

/// The label of a BEGIN or END line that starts with `prefix`.
fn label<'a>(line: &'a [u8], prefix: &[u8]) -> Result<&'a [u8], Error> {
    let rest = &line[prefix.len()..];
    let label = rest.strip_suffix(DASHES).ok_or(Error::BadLabel)?;
    if label.iter().all(|&b| (0x20..=0x7E).contains(&b)) {
        Ok(label)
    } else {
        Err(Error::BadLabel)
    }
}

fn blocks(text: &[u8], work: &mut usize) -> Result<Vec<Block>, Error> {
    let mut found = Vec::new();
    // The label of the open block and where its content starts.
    let mut open: Option<(&[u8], usize)> = None;
    let mut pos = 0;
    while pos < text.len() {
        let rest = &text[pos..];
        let (line_len, next) = match rest.iter().position(|&b| b == b'\n') {
            Some(i) => (i, pos + i + 1),
            None => (rest.len(), text.len()),
        };
        *work += next - pos;
        let mut line = &rest[..line_len];
        while let [head @ .., b'\r' | b' ' | b'\t'] = line {
            line = head;
        }
        match open {
            None => {
                if line.starts_with(BEGIN) {
                    open = Some((label(line, BEGIN)?, next));
                }
            }
            Some((begin, start)) => {
                if line.starts_with(END) {
                    if label(line, END)? != begin {
                        return Err(Error::LabelMismatch);
                    }
                    let data = crate::base64::decode(&text[start..pos], work)
                        .map_err(|_| Error::BadBase64)?;
                    found.push(Block {
                        // The label's bytes are ASCII.
                        label: begin.iter().map(|&b| char::from(b)).collect(),
                        data,
                    });
                    open = None;
                } else if line.starts_with(BEGIN) {
                    return Err(Error::UnterminatedBlock);
                }
            }
        }
        pos = next;
    }
    match open {
        Some(_) => Err(Error::UnterminatedBlock),
        None => Ok(found),
    }
}
