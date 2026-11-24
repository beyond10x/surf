//! The trust anchors a connection validates against (docs/design/tls.md, items 8 and 11;
//! `story:tls-anchors`): the root list of Mozilla as curl publishes it, compiled in, and one
//! extra PEM file.
//!
//! The bundled list is `cacert.pem` beside this file, unchanged, with its record `SOURCE`
//! (address, date of the list, sha256, count, licence). [`Anchors::bundled`] reads it once per
//! process into [`TrustAnchor`]s, in the order of the file. A root the certificate reader
//! refuses is taken as its subject and key only, when [`TrustAnchor::from_der`] takes it so
//! (item 11: only when it holds no nameConstraints). A root that gives no anchor is left out;
//! the tests name each such root and hold the count of anchors to `SOURCE`'s count less them.
//! In the list of 2026-11-05 that is one root of 118, e-Szigno TLS Root CA 2023: its key is on
//! P-521, which the seam cannot verify with.
//!
//! [`Anchors::with_extra_pem`] adds an extra file (item 8): at most [`EXTRA_BYTES_MAX`] bytes
//! and [`EXTRA_CERTIFICATES_MAX`] blocks, every block a `CERTIFICATE` the reader reads whole,
//! every certificate a CA (basicConstraints cA). It is refused whole with an [`AnchorError`]
//! that names the first block at fault, and then adds nothing. The size is checked before any
//! block is read, the count of blocks before any is decoded, and a block of another label is
//! refused by its BEGIN line, so its content (a private key, for one) is never decoded.
//! [`Anchors::as_slice`] hands the set to path validation.

use std::fmt;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::pem;
use crate::x509::{self, Certificate, TrustAnchor};

/// The most bytes an extra anchor file may hold: 32 KiB (item 8: the file travels quoted in an
/// `open` request of at most 64 KiB).
pub const EXTRA_BYTES_MAX: usize = 32 * 1024;

/// The most PEM blocks an extra anchor file may hold (item 8).
pub const EXTRA_CERTIFICATES_MAX: usize = 64;

const BUNDLE: &[u8] = include_bytes!("cacert.pem");
const SOURCE: &str = include_str!("SOURCE");

const BEGIN: &[u8] = b"-----BEGIN ";
const END: &[u8] = b"-----END ";
const DASHES: &[u8] = b"-----";
const CERTIFICATE: &[u8] = b"CERTIFICATE";

/// Why an extra anchor file was refused. `index` counts the PEM blocks of the file from 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnchorError {
    /// The file holds no PEM block.
    NoCertificate,
    /// A block whose label is not `CERTIFICATE`; its content was not decoded or kept.
    NotACertificate {
        /// The block.
        index: usize,
        /// The label of its BEGIN line.
        label: String,
    },
    /// A block that is not PEM: its base64 or padding is wrong, its END line is missing or has
    /// another label, or its BEGIN line is not in the form.
    BadPem {
        /// The block.
        index: usize,
    },
    /// A certificate the reader refuses.
    BadCertificate {
        /// The block.
        index: usize,
        /// The reader's error.
        error: x509::Error,
    },
    /// A certificate that is not a CA: no basicConstraints, or cA false.
    NotACa {
        /// The block.
        index: usize,
    },
    /// The file holds more than [`EXTRA_BYTES_MAX`] bytes.
    TooLarge,
    /// The file holds more than [`EXTRA_CERTIFICATES_MAX`] blocks.
    TooMany,
}

impl AnchorError {
    /// The name of the kind, as the variant is written.
    pub fn name(&self) -> &'static str {
        match self {
            AnchorError::NoCertificate => "NoCertificate",
            AnchorError::NotACertificate { .. } => "NotACertificate",
            AnchorError::BadPem { .. } => "BadPem",
            AnchorError::BadCertificate { .. } => "BadCertificate",
            AnchorError::NotACa { .. } => "NotACa",
            AnchorError::TooLarge => "TooLarge",
            AnchorError::TooMany => "TooMany",
        }
    }
}

impl fmt::Display for AnchorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        match self {
            AnchorError::NoCertificate => write!(f, "{name}: the file holds no PEM block"),
            AnchorError::NotACertificate { index, label } => {
                write!(
                    f,
                    "{name}: block {index} is labelled {}, not CERTIFICATE",
                    crate::error::excerpt(label.as_bytes())
                )
            }
            AnchorError::BadPem { index } => write!(f, "{name}: block {index} is not PEM"),
            AnchorError::BadCertificate { index, error } => {
                write!(f, "{name}: certificate {index} is refused: {error}")
            }
            AnchorError::NotACa { index } => write!(f, "{name}: certificate {index} is not a CA"),
            AnchorError::TooLarge => {
                write!(
                    f,
                    "{name}: the file holds more than {EXTRA_BYTES_MAX} bytes"
                )
            }
            AnchorError::TooMany => write!(
                f,
                "{name}: the file holds more than {EXTRA_CERTIFICATES_MAX} blocks"
            ),
        }
    }
}

impl std::error::Error for AnchorError {}

/// A set of trust anchors: the bundled list and the anchors of extra files, in that order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anchors {
    anchors: Vec<TrustAnchor>,
}

/// The certificates of the bundle read so far in this process.
static BUNDLE_READS: AtomicUsize = AtomicUsize::new(0);

fn bundle() -> &'static [TrustAnchor] {
    static ANCHORS: OnceLock<Vec<TrustAnchor>> = OnceLock::new();
    ANCHORS.get_or_init(|| {
        let (anchors, read) = read_list(BUNDLE);
        BUNDLE_READS.fetch_add(read, Ordering::Relaxed);
        anchors
    })
}

/// The anchors of a list in the form of the bundle, and the certificates read: each
/// `CERTIFICATE` block through [`TrustAnchor::from_der`], so a root the reader refuses is taken
/// as name and key when item 11 allows; a block of another label and a root that gives no
/// anchor are left out.
///
/// A list that does not parse as PEM gives no anchor at all. For the bundle that means: a caller
/// of [`Anchors::bundled`] gets a set whose [`Anchors::count`] is 0, so every path ends at
/// `NoPathToAnchor` (it fails closed, and the library does not panic). It cannot happen silently
/// in a release: the bundle is compiled in, `task anchors-refresh` writes only a list whose
/// blocks are whole, and the tests parse the compiled-in bytes and hold the count of anchors to
/// `SOURCE`. The place that builds a verifier from the set refuses a count of 0
/// (`story:tls-fetch-https`).
fn read_list(text: &[u8]) -> (Vec<TrustAnchor>, usize) {
    let blocks = pem::parse(text).unwrap_or_default();
    let mut read = 0;
    let anchors = blocks
        .iter()
        .filter(|block| block.label.as_bytes() == CERTIFICATE)
        .filter_map(|block| {
            read += 1;
            TrustAnchor::from_der(&block.data).ok()
        })
        .collect();
    (anchors, read)
}

/// The value of the line `<key>: <value>` of `SOURCE`.
fn source_field(key: &str) -> &'static str {
    SOURCE
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix(": "))
        })
        .unwrap_or("")
}

impl Anchors {
    /// The bundled list: an anchor for each root of `cacert.pem`, read once per process.
    pub fn bundled() -> Self {
        Anchors {
            anchors: bundle().to_vec(),
        }
    }

    /// The number of anchors.
    pub fn count(&self) -> usize {
        self.anchors.len()
    }

    /// The anchors, as path validation takes them.
    pub fn as_slice(&self) -> &[TrustAnchor] {
        &self.anchors
    }

    /// Adds the anchors of the extra PEM file `pem` (item 8) and returns how many, or refuses
    /// the whole file and adds nothing.
    pub fn with_extra_pem(&mut self, pem: &[u8]) -> Result<usize, AnchorError> {
        self.take_extra(pem).map(|taken| taken.len())
    }

    /// [`Anchors::with_extra_pem`], giving back the text of the blocks it took instead of their
    /// count: each block from its BEGIN line to its END line and a line feed, in the order of
    /// the file. The lines before, between and after the blocks are left out. The text is ASCII,
    /// since a block the PEM reader takes holds nothing else, and it is taken again whole by
    /// [`Anchors::with_extra_pem`].
    pub fn with_extra_pem_blocks(&mut self, pem: &[u8]) -> Result<String, AnchorError> {
        let taken = self.take_extra(pem)?;
        let mut text = String::new();
        for block in taken {
            text.extend(block.iter().map(|&b| char::from(b)));
            text.push('\n');
        }
        Ok(text)
    }

    /// Adds the anchors of `pem` as [`Anchors::with_extra_pem`] tells, and returns the text of
    /// each block taken, without the line feed after its END line.
    fn take_extra<'a>(&mut self, pem: &'a [u8]) -> Result<Vec<&'a [u8]>, AnchorError> {
        if pem.len() > EXTRA_BYTES_MAX {
            return Err(AnchorError::TooLarge);
        }
        let blocks = split_blocks(pem);
        if blocks.is_empty() {
            return Err(AnchorError::NoCertificate);
        }
        if blocks.len() > EXTRA_CERTIFICATES_MAX {
            return Err(AnchorError::TooMany);
        }
        let mut added = Vec::with_capacity(blocks.len());
        let mut taken = Vec::with_capacity(blocks.len());
        for (index, block) in blocks.into_iter().enumerate() {
            if let Some(label) = other_label(block) {
                return Err(AnchorError::NotACertificate { index, label });
            }
            let der = match pem::parse(block) {
                Ok(mut found) if found.len() == 1 => found.remove(0).data,
                _ => return Err(AnchorError::BadPem { index }),
            };
            let cert = Certificate::from_der(&der)
                .map_err(|error| AnchorError::BadCertificate { index, error })?;
            if !cert.extensions.basic_constraints.is_some_and(|b| b.ca) {
                return Err(AnchorError::NotACa { index });
            }
            added.push(TrustAnchor::from_certificate(&cert));
            // The PEM reader took the block whole, so its first END line after the BEGIN line
            // closes it; what follows up to the next BEGIN line is outside every block.
            let end = lines(block)
                .skip(1)
                .find(|(_, line)| line.starts_with(END))
                .map_or(block.len(), |(start, line)| start + line.len());
            taken.push(&block[..end]);
        }
        self.anchors.extend(added);
        Ok(taken)
    }

    /// The date of the bundled list, `yyyy-mm-dd`, from its header line.
    pub fn bundled_date() -> &'static str {
        source_field("date")
    }

    /// The sha256 of the bundled list, 64 lower-case hex digits, as `SOURCE` records it.
    pub fn bundled_sha256() -> &'static str {
        source_field("sha256")
    }

    /// The bytes of the bundled list, as compiled in.
    pub fn bundled_pem() -> &'static [u8] {
        BUNDLE
    }

    /// The certificates of the bundle this process has read: the count of the bundle after
    /// any number of [`Anchors::bundled`], since it is read once.
    #[doc(hidden)]
    pub fn bundled_certificates_read() -> usize {
        BUNDLE_READS.load(Ordering::Relaxed)
    }

    /// The line `surf --anchors` prints for the bundled list (item 8): `anchors <n> bundled
    /// <yyyy-mm-dd> sha256:<64 hex>`, where `<n>` is the [`Anchors::count`] of
    /// [`Anchors::bundled`] (117 for the list of 2026-11-05, whose `SOURCE` counts 118
    /// certificates), the date and sha256 those of `SOURCE`.
    pub fn bundled_line() -> String {
        format!(
            "anchors {} bundled {} sha256:{}",
            bundle().len(),
            Anchors::bundled_date(),
            Anchors::bundled_sha256()
        )
    }

    /// The anchors of `text` read as the bundle is read, for the tests of that reading (the
    /// bundle itself holds no root that is taken as name and key, and no other label). Behind
    /// the crate's test seam, the feature `test-tooling`: no build of a release has it.
    #[cfg(feature = "test-tooling")]
    #[doc(hidden)]
    pub fn from_list(text: &[u8]) -> Self {
        Anchors {
            anchors: read_list(text).0,
        }
    }
}

/// The lines of `text` as the PEM reader cuts them: at each line feed, with the carriage
/// returns, spaces and tabs at the end of a line not part of it.
fn lines(text: &[u8]) -> impl Iterator<Item = (usize, &[u8])> {
    let mut pos = 0;
    std::iter::from_fn(move || {
        if pos >= text.len() {
            return None;
        }
        let start = pos;
        let rest = &text[start..];
        let len = rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
        pos = (start + len + 1).min(text.len());
        let mut line = &rest[..len];
        while let [head @ .., b'\r' | b' ' | b'\t'] = line {
            line = head;
        }
        Some((start, line))
    })
}

/// The blocks of `text`: each from a BEGIN line to the next BEGIN line or the end. The PEM
/// reader reads each alone, so that a refusal names its block.
fn split_blocks(text: &[u8]) -> Vec<&[u8]> {
    let starts: Vec<usize> = lines(text)
        .filter(|(_, line)| line.starts_with(BEGIN))
        .map(|(start, _)| start)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(i, &start)| &text[start..starts.get(i + 1).copied().unwrap_or(text.len())])
        .collect()
}

/// The label of the BEGIN line of `block` when it is in the PEM reader's form and is not
/// `CERTIFICATE`.
fn other_label(block: &[u8]) -> Option<String> {
    let (_, line) = lines(block).next()?;
    let label = line.strip_prefix(BEGIN)?.strip_suffix(DASHES)?;
    if label == CERTIFICATE || !label.iter().all(|&b| (0x20..=0x7E).contains(&b)) {
        return None;
    }
    Some(label.iter().map(|&b| char::from(b)).collect())
}
