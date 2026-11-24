//! The trust anchors, `surf_tls::anchors` (`story:tls-anchors`): the bundled root list of
//! Mozilla as curl publishes it against its `SOURCE` (acceptance 1), the extra PEM file of item
//! 8 accepted (acceptance 2) or refused with the error that names its block (acceptance 3), and
//! the two tasks of `taskfiles/tls-anchors.sh`, run with `sh` on local files under
//! `CARGO_TARGET_TMPDIR` (acceptance 4 and 5). No test reaches the network.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

use surf_tls::anchors::{AnchorError, Anchors, EXTRA_BYTES_MAX, EXTRA_CERTIFICATES_MAX};
use surf_tls::crypto::{self, HashAlgorithm};
use surf_tls::der;
use surf_tls::pem;
use surf_tls::x509::{self, Certificate, TrustAnchor};

use crate::x509_parse::Spec;

const SOURCE: &str = include_str!("../../src/anchors/SOURCE");
const BUNDLE: &[u8] = include_bytes!("../../src/anchors/cacert.pem");

/// The roots of the bundle the certificate reader refuses and that are still anchors of name
/// and key (docs/design/tls.md, item 11): the title curl gives each, and the reader's error. A
/// refresh that changes this list fails here and is read by a person.
const NAME_AND_KEY: [(&str, &str); 0] = [];

/// The roots of the bundle that give no anchor, with the error `TrustAnchor::from_der`
/// returns: a finding of `story:tls-anchors` for the coordinator, held here so that the
/// count of anchors is held too and a refresh that loses another root fails. e-Szigno TLS Root
/// CA 2023 is signed with ecdsa-with-SHA512 (the reader's `UnsupportedAlgorithm`) and its key
/// is on P-521, which the seam has no verification for, so neither its whole certificate nor
/// its name and key make an anchor.
const NO_ANCHOR: [(&str, &str); 1] = [("e-Szigno TLS Root CA 2023", "UnsupportedAlgorithm")];

fn test_certs(path: &str) -> Vec<u8> {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tls-test-certs/certs")
        .join(path);
    fs::read(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()))
}

fn source_field(source: &str, key: &str) -> String {
    let prefix = format!("{key}: ");
    let lines: Vec<&str> = source
        .lines()
        .filter_map(|line| line.strip_prefix(prefix.as_str()))
        .collect();
    assert_eq!(lines.len(), 1, "one line `{key}:` in SOURCE");
    lines[0].to_string()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Base64 of RFC 4648 with padding, in lines of 64 characters, as PEM writes it.
fn base64_lines(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                text.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                text.push('=');
            }
        }
    }
    let mut lines = String::new();
    for (i, c) in text.chars().enumerate() {
        if i > 0 && i % 64 == 0 {
            lines.push('\n');
        }
        lines.push(c);
    }
    lines
}

fn pem_block(label: &str, data: &[u8]) -> String {
    format!(
        "-----BEGIN {label}-----\n{}\n-----END {label}-----\n",
        base64_lines(data)
    )
}

/// The first block of a PEM file of the test certificates, as DER.
fn first_der(pem_text: &[u8]) -> Vec<u8> {
    pem::parse(pem_text).expect("PEM").remove(0).data
}

/// The titles curl writes above each block of the bundle (the line above a line of `=`), in
/// the order of the blocks.
fn bundle_titles() -> Vec<String> {
    let text = std::str::from_utf8(BUNDLE).expect("the bundle is UTF-8");
    let lines: Vec<&str> = text.lines().collect();
    let mut titles = Vec::new();
    let mut title = String::new();
    for (i, line) in lines.iter().enumerate() {
        if lines.get(i + 1).is_some_and(|next| next.starts_with("==")) {
            title = line.to_string();
        }
        if line.starts_with("-----BEGIN CERTIFICATE-----") {
            titles.push(title.clone());
        }
    }
    titles
}

// ---------------------------------------------------------------------------------------------
// Acceptance 1: the bundled list.

#[test]
fn the_bundled_count_equals_source() {
    let started = Instant::now();
    let anchors = Anchors::bundled();
    println!(
        "anchors: the bundle read into {} anchors in {:.3} s",
        anchors.count(),
        started.elapsed().as_secs_f64()
    );
    let count: usize = source_field(SOURCE, "count").parse().expect("a count");
    // Every certificate of SOURCE's count gives an anchor but the roots of NO_ANCHOR.
    assert_eq!(anchors.count(), count - NO_ANCHOR.len());
    assert_eq!(anchors.as_slice().len(), count - NO_ANCHOR.len());
    let blocks = pem::parse(BUNDLE).expect("the bundle is PEM");
    assert_eq!(
        blocks.len(),
        count,
        "BEGIN CERTIFICATE blocks of cacert.pem"
    );
    assert!(blocks.iter().all(|b| b.label == "CERTIFICATE"));
}

/// The line of `surf --anchors` counts the anchors, not the certificates of `SOURCE`.
#[test]
fn the_anchors_line_prints_the_count_of_anchors() {
    let count: usize = source_field(SOURCE, "count").parse().expect("a count");
    assert_eq!(
        Anchors::bundled_line(),
        format!(
            "anchors {} bundled {} sha256:{}",
            count - NO_ANCHOR.len(),
            source_field(SOURCE, "date"),
            source_field(SOURCE, "sha256")
        )
    );
    assert_eq!(
        Anchors::bundled_line().split(' ').nth(1),
        Some(Anchors::bundled().count().to_string().as_str())
    );
}

#[test]
fn the_sha256_of_the_included_bytes_equals_source() {
    assert_eq!(Anchors::bundled_pem(), BUNDLE);
    let digest = crypto::hash(HashAlgorithm::Sha256, Anchors::bundled_pem());
    let sha256 = source_field(SOURCE, "sha256");
    assert_eq!(hex(digest.as_ref()), sha256);
    assert_eq!(Anchors::bundled_sha256(), sha256);
    let bytes: usize = source_field(SOURCE, "bytes").parse().expect("bytes");
    assert_eq!(BUNDLE.len(), bytes);
}

#[test]
fn the_bundled_date_is_the_date_of_the_header_line() {
    // The date is computed here from the header line of the compiled-in bundle, not read back
    // from SOURCE, so a SOURCE whose date is not the list's fails.
    let text = std::str::from_utf8(BUNDLE).expect("UTF-8");
    let prefix = "## Certificate data from Mozilla as of: ";
    let headers: Vec<&str> = text.lines().filter(|l| l.starts_with(prefix)).collect();
    assert_eq!(headers.len(), 1, "one header line");
    let fields: Vec<&str> = headers[0][prefix.len()..].split_whitespace().collect();
    assert_eq!(fields.len(), 6, "{:?}", headers[0]);
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = months
        .iter()
        .position(|m| *m == fields[1])
        .expect("a month")
        + 1;
    let day: u32 = fields[2].parse().expect("a day");
    let date = format!("{}-{month:02}-{day:02}", fields[4]);
    assert_eq!(Anchors::bundled_date(), date);
    assert_eq!(source_field(SOURCE, "date"), date);
    assert_eq!(format!("## {}", source_field(SOURCE, "header")), headers[0]);
}

#[test]
fn every_root_of_the_bundle_gives_an_anchor_and_the_name_and_key_ones_are_named() {
    let titles = bundle_titles();
    let blocks = pem::parse(BUNDLE).expect("PEM");
    assert_eq!(titles.len(), blocks.len());
    let mut read_whole = 0;
    let mut name_and_key = Vec::new();
    let mut no_anchor = Vec::new();
    for (block, title) in blocks.iter().zip(&titles) {
        match (
            Certificate::from_der(&block.data),
            TrustAnchor::from_der(&block.data),
        ) {
            (Ok(_), Ok(anchor)) => {
                assert_eq!(anchor.refused, None);
                read_whole += 1;
            }
            (Err(e), Ok(anchor)) => {
                assert_eq!(anchor.refused, Some(e));
                name_and_key.push((title.clone(), e.name()));
            }
            (_, Err(e)) => no_anchor.push((title.clone(), e.name())),
        }
    }
    let report = Path::new(env!("CARGO_TARGET_TMPDIR")).join("anchors-bundle.txt");
    let mut lines = format!(
        "certificates {}\nread whole {read_whole}\nname and key {}\nno anchor {}\n",
        blocks.len(),
        name_and_key.len(),
        no_anchor.len()
    );
    for (title, e) in &name_and_key {
        lines.push_str(&format!("name-and-key {title}: {e}\n"));
    }
    for (title, e) in &no_anchor {
        lines.push_str(&format!("no-anchor {title}: {e}\n"));
    }
    fs::write(&report, &lines).expect("write the report");
    print!("{lines}");
    let held: Vec<(String, &str)> = NO_ANCHOR.iter().map(|&(t, e)| (t.to_string(), e)).collect();
    assert_eq!(no_anchor, held, "roots that give no anchor");
    let expected: Vec<(String, &str)> = NAME_AND_KEY
        .iter()
        .map(|&(t, e)| (t.to_string(), e))
        .collect();
    assert_eq!(name_and_key, expected, "roots taken as name and key only");
    let bundled = Anchors::bundled();
    assert_eq!(
        bundled
            .as_slice()
            .iter()
            .filter(|a| a.refused.is_some())
            .count(),
        NAME_AND_KEY.len()
    );
}

#[test]
fn the_bundle_is_read_once() {
    let first = Anchors::bundled();
    let second = Anchors::bundled();
    assert!(first.count() > 0);
    assert_eq!(first.as_slice(), second.as_slice());
    // Every certificate of the bundle once, also the ones that give no anchor.
    let blocks = pem::parse(BUNDLE).expect("PEM").len();
    assert_eq!(Anchors::bundled_certificates_read(), blocks);
    assert_eq!(first.count(), blocks - NO_ANCHOR.len());
}

#[test]
fn the_anchors_are_the_bundle_in_its_order() {
    let blocks = pem::parse(BUNDLE).expect("PEM");
    let expected: Vec<TrustAnchor> = blocks
        .iter()
        .filter_map(|block| TrustAnchor::from_der(&block.data).ok())
        .collect();
    let anchors = Anchors::bundled();
    assert_eq!(anchors.count(), blocks.len() - NO_ANCHOR.len());
    assert_eq!(anchors.as_slice(), expected.as_slice());
    assert_eq!(Anchors::from_list(BUNDLE), anchors);
}

/// The bundle holds no root taken as name and key and no other label, so the reading of a list
/// is shown on one written here: a version 1 root (name and key), a block of another label (left
/// out), a root that gives no anchor (left out) and a root the reader reads whole.
#[test]
fn a_list_takes_a_refused_root_as_name_and_key_and_leaves_out_what_gives_no_anchor() {
    let v1 = Spec {
        version: None,
        extensions: Vec::new(),
        ..Spec::p256()
    }
    .build();
    let cut = &first_der(&test_certs("ok-ed25519/root.pem"))[..100];
    let whole = first_der(&test_certs("ok-ed25519/root.pem"));
    let text = [
        pem_block("CERTIFICATE", &v1),
        pem_block("X509 CRL", &whole),
        pem_block("CERTIFICATE", cut),
        pem_block("CERTIFICATE", &whole),
    ]
    .concat();
    let list = Anchors::from_list(text.as_bytes());
    assert_eq!(list.count(), 2);
    assert_eq!(
        list.as_slice()[0].refused,
        Some(x509::Error::UnsupportedVersion)
    );
    assert_eq!(list.as_slice()[0], TrustAnchor::from_der(&v1).expect("v1"));
    let cert = Certificate::from_der(&whole).expect("reads");
    assert_eq!(list.as_slice()[1], TrustAnchor::from_certificate(&cert));
    assert_eq!(Anchors::from_list(b"no PEM").count(), 0);
    assert_eq!(
        Anchors::from_list(b"-----BEGIN CERTIFICATE-----\n").count(),
        0
    );
}

// ---------------------------------------------------------------------------------------------
// Acceptance 2: an extra file.

#[test]
fn the_root_of_ok_rsa2048_is_accepted_and_adds_one() {
    let pem_text = test_certs("ok-rsa2048/root.pem");
    let mut anchors = Anchors::bundled();
    let before = anchors.count();
    assert_eq!(anchors.with_extra_pem(&pem_text), Ok(1));
    assert_eq!(anchors.count(), before + 1);
    let der = first_der(&pem_text);
    let cert = Certificate::from_der(&der).expect("the root reads");
    assert_eq!(
        anchors.as_slice().last(),
        Some(&TrustAnchor::from_certificate(&cert))
    );
}

#[test]
fn a_file_at_its_byte_bound_is_accepted() {
    let mut text = test_certs("ok-ed25519/root.pem");
    text.resize(EXTRA_BYTES_MAX, b'\n');
    assert_eq!(text.len(), 32_768);
    let mut anchors = Anchors::bundled();
    let before = anchors.count();
    assert_eq!(anchors.with_extra_pem(&text), Ok(1));
    assert_eq!(anchors.count(), before + 1);
}

#[test]
fn a_file_of_64_certificates_is_accepted() {
    let one = test_certs("ok-ed25519/root.pem");
    let text = one.repeat(EXTRA_CERTIFICATES_MAX);
    assert!(text.len() <= EXTRA_BYTES_MAX, "{} bytes", text.len());
    let mut anchors = Anchors::bundled();
    let before = anchors.count();
    assert_eq!(anchors.with_extra_pem(&text), Ok(64));
    assert_eq!(anchors.count(), before + 64);
}

#[test]
fn the_text_around_the_blocks_is_ignored() {
    let root = test_certs("ok-rsa2048/root.pem");
    let ed = test_certs("ok-ed25519/root.pem");
    let mut text = b"a comment line\nok-rsa2048\n".to_vec();
    text.extend_from_slice(&root);
    text.extend_from_slice(b"\nbetween\n-----END CERTIFICATE-----\n\n");
    text.extend_from_slice(&ed);
    text.extend_from_slice(b"after");
    let mut anchors = Anchors::bundled();
    assert_eq!(anchors.with_extra_pem(&text), Ok(2));
}

/// `with_extra_pem_blocks` gives back the blocks it took and nothing of the text around them:
/// not the lines before, between and after them (bytes that are not UTF-8 among them), nor a
/// stray END line, nor the white space after an END line. The text it gives is taken again
/// whole, to the same anchors; a refused file gives the error of `with_extra_pem` and adds
/// nothing.
#[test]
fn the_blocks_taken_are_given_back_without_the_text_around_them() {
    let root = test_certs("ok-rsa2048/root.pem");
    let ed = test_certs("ok-ed25519/root.pem");
    let mut text = b"a comment line\n\xff\xfe not UTF-8\n".to_vec();
    text.extend_from_slice(&root);
    text.extend_from_slice(b"\nbetween \xc3\x28\n-----END CERTIFICATE-----\n\n");
    let mut ed_trailing = ed.clone();
    ed_trailing.truncate(ed_trailing.len() - 1);
    text.extend_from_slice(&ed_trailing);
    text.extend_from_slice(b" \t\r\nafter\n");
    let mut anchors = Anchors::bundled();
    let blocks = anchors
        .with_extra_pem_blocks(&text)
        .expect("the file is taken");
    let mut expected = root.clone();
    expected.extend_from_slice(&ed);
    assert_eq!(blocks.as_bytes(), expected.as_slice());
    let mut again = Anchors::bundled();
    assert_eq!(again.with_extra_pem(blocks.as_bytes()), Ok(2));
    assert_eq!(again, anchors);

    let mut leaf = root.clone();
    leaf.extend_from_slice(&first_block(&test_certs("ok-rsa2048/chain.pem")));
    let mut refused_here = Anchors::bundled();
    assert_eq!(
        refused_here.with_extra_pem_blocks(&leaf),
        Err(refused(&leaf))
    );
    assert_eq!(refused_here, Anchors::bundled());
}

/// The text of the first PEM block of `text`, its END line and line feed included.
fn first_block(text: &[u8]) -> Vec<u8> {
    let end = b"-----END CERTIFICATE-----\n";
    let at = text
        .windows(end.len())
        .position(|window| window == end)
        .expect("a certificate block");
    text[..at + end.len()].to_vec()
}

#[test]
fn the_bounds_are_32_kib_and_64_certificates() {
    assert_eq!(EXTRA_BYTES_MAX, 32 * 1024);
    assert_eq!(EXTRA_CERTIFICATES_MAX, 64);
}

// ---------------------------------------------------------------------------------------------
// Acceptance 3: hostile extra files, each refused, nothing added.

/// Refuses `text` and checks that the set did not change.
fn refused(text: &[u8]) -> AnchorError {
    let mut anchors = Anchors::bundled();
    let before = anchors.as_slice().to_vec();
    let error = anchors.with_extra_pem(text).expect_err("refused");
    assert_eq!(anchors.as_slice(), before.as_slice(), "nothing added");
    error
}

#[test]
fn random_bytes_hold_no_certificate() {
    let mut state = 0x5EED_A9C4_0B5E_u64;
    let bytes: Vec<u8> = (0..1024)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect();
    assert_eq!(refused(&bytes), AnchorError::NoCertificate);
    assert_eq!(refused(b""), AnchorError::NoCertificate);
    assert_eq!(
        refused(b"-----END CERTIFICATE-----\n"),
        AnchorError::NoCertificate
    );
}

#[test]
fn a_private_key_block_is_not_a_certificate_and_is_not_kept() {
    // The label is put together here so that this file holds no line a secret scan reads as
    // a private key (brief: a short body and no second such line).
    let label = ["PRIVATE", "KEY"].join(" ");
    let body = "c3VyZi10bHMgdGVzdA==";
    let text = format!("-----BEGIN {label}-----\n{body}\n-----END {label}-----\n");
    let error = refused(text.as_bytes());
    assert_eq!(
        error,
        AnchorError::NotACertificate {
            index: 0,
            label: label.clone()
        }
    );
    for shown in [format!("{error}"), format!("{error:?}")] {
        assert!(!shown.contains(body), "{shown}");
        assert!(!shown.contains("surf-tls test"), "{shown}");
    }
}

#[test]
fn a_certificate_cut_after_100_bytes_is_truncated() {
    let der = first_der(&test_certs("ok-rsa2048/root.pem"));
    let text = pem_block("CERTIFICATE", &der[..100]);
    assert_eq!(
        refused(text.as_bytes()),
        AnchorError::BadCertificate {
            index: 0,
            error: x509::Error::Der(der::Error::Truncated)
        }
    );
}

#[test]
fn the_leaf_of_ok_rsa2048_is_not_a_ca() {
    let chain = test_certs("ok-rsa2048/chain.pem");
    let leaf = first_der(&chain);
    let cert = Certificate::from_der(&leaf).expect("the leaf reads");
    assert!(!cert.extensions.basic_constraints.is_some_and(|b| b.ca));
    let text = pem_block("CERTIFICATE", &leaf);
    assert_eq!(refused(text.as_bytes()), AnchorError::NotACa { index: 0 });
}

#[test]
fn base64_cut_in_the_middle_is_bad_pem() {
    let text = String::from_utf8(test_certs("ok-rsa2048/root.pem")).expect("ASCII");
    let lines: Vec<&str> = text.lines().collect();
    let body: String = lines[1..lines.len() - 1].concat();
    // An odd count of base64 characters never decodes.
    let cut = &body[..(body.len() / 2) | 1];
    let text = format!("{}\n{cut}\n{}\n", lines[0], lines[lines.len() - 1]);
    assert_eq!(refused(text.as_bytes()), AnchorError::BadPem { index: 0 });
}

#[test]
fn a_begin_without_its_end_is_bad_pem() {
    let root = String::from_utf8(test_certs("ok-ed25519/root.pem")).expect("ASCII");
    let cut = root.replace("-----END CERTIFICATE-----", "");
    assert_eq!(refused(cut.as_bytes()), AnchorError::BadPem { index: 0 });
    let two = format!("{root}{cut}");
    assert_eq!(refused(two.as_bytes()), AnchorError::BadPem { index: 1 });
    let mismatch = root.replace("-----END CERTIFICATE-----", "-----END X509 CRL-----");
    assert_eq!(
        refused(mismatch.as_bytes()),
        AnchorError::BadPem { index: 0 }
    );
}

#[test]
fn one_byte_over_32_kib_is_too_large_before_any_block_is_read() {
    let mut text = test_certs("ok-ed25519/root.pem");
    text.resize(EXTRA_BYTES_MAX + 1, b'\n');
    assert_eq!(refused(&text), AnchorError::TooLarge);
    // A file whose first block would be refused is still TooLarge: the size comes first.
    let mut text = b"-----BEGIN CERTIFICATE-----\n!\n-----END CERTIFICATE-----\n".to_vec();
    text.resize(EXTRA_BYTES_MAX + 1, b' ');
    assert_eq!(refused(&text), AnchorError::TooLarge);
}

#[test]
fn sixty_five_small_ca_certificates_are_too_many() {
    let one = test_certs("ok-ed25519/root.pem");
    let text = one.repeat(EXTRA_CERTIFICATES_MAX + 1);
    assert!(text.len() <= EXTRA_BYTES_MAX, "{} bytes", text.len());
    assert_eq!(refused(&text), AnchorError::TooMany);
    // The count comes before the blocks: a bad first block among 65 is still TooMany.
    let mut text = b"-----BEGIN CERTIFICATE-----\n!\n-----END CERTIFICATE-----\n".to_vec();
    text.extend_from_slice(&one.repeat(EXTRA_CERTIFICATES_MAX));
    assert_eq!(refused(&text), AnchorError::TooMany);
}

#[test]
fn a_good_root_then_a_bad_block_is_refused_at_index_1_and_adds_nothing() {
    let root = test_certs("ok-rsa2048/root.pem");
    let leaf = pem_block(
        "CERTIFICATE",
        &first_der(&test_certs("ok-rsa2048/chain.pem")),
    );
    let mut text = root.clone();
    text.extend_from_slice(leaf.as_bytes());
    assert_eq!(refused(&text), AnchorError::NotACa { index: 1 });
    let mut text = root.clone();
    text.extend_from_slice(b"-----BEGIN CERTIFICATE-----\nZm9v!\n-----END CERTIFICATE-----\n");
    assert_eq!(refused(&text), AnchorError::BadPem { index: 1 });
    let mut text = root.clone();
    text.extend_from_slice(b"-----BEGIN X509 CRL-----\nZm9v\n-----END X509 CRL-----\n");
    assert_eq!(
        refused(&text),
        AnchorError::NotACertificate {
            index: 1,
            label: "X509 CRL".to_string()
        }
    );
    let mut text = root;
    text.extend_from_slice(pem_block("CERTIFICATE", b"\x30\x03\x02\x01\x01").as_bytes());
    assert!(matches!(
        refused(&text),
        AnchorError::BadCertificate { index: 1, .. }
    ));
}

#[test]
fn a_refused_extra_file_leaves_an_earlier_one_in_place() {
    let mut anchors = Anchors::bundled();
    let before = anchors.count();
    assert_eq!(
        anchors.with_extra_pem(&test_certs("ok-rsa2048/root.pem")),
        Ok(1)
    );
    let kept = anchors.as_slice().to_vec();
    assert_eq!(
        anchors.with_extra_pem(b"no block"),
        Err(AnchorError::NoCertificate)
    );
    assert_eq!(anchors.as_slice(), kept.as_slice());
    assert_eq!(anchors.count(), before + 1);
}

#[test]
fn a_certificate_with_ca_false_or_without_basic_constraints_is_not_a_ca() {
    // Spec::p256 holds basicConstraints with cA left out, which is false.
    let ca_false = Spec::p256().build();
    let none = Spec {
        extensions: Vec::new(),
        ..Spec::p256()
    }
    .build();
    for der_bytes in [ca_false, none] {
        assert!(Certificate::from_der(&der_bytes).is_ok());
        let text = pem_block("CERTIFICATE", &der_bytes);
        assert_eq!(refused(text.as_bytes()), AnchorError::NotACa { index: 0 });
    }
}

#[test]
fn a_file_with_crlf_line_ends_reads_as_with_lf() {
    let root = String::from_utf8(test_certs("ok-rsa2048/root.pem")).expect("ASCII");
    let crlf = root.replace('\n', "\r\n");
    let mut anchors = Anchors::bundled();
    assert_eq!(anchors.with_extra_pem(crlf.as_bytes()), Ok(1));
    let label = ["PRIVATE", "KEY"].join(" ");
    let text = format!("-----BEGIN {label}-----\r\nZm9v\r\n-----END {label}-----\r\n");
    assert_eq!(
        refused(text.as_bytes()),
        AnchorError::NotACertificate { index: 0, label }
    );
}

#[test]
fn a_label_outside_space_to_tilde_is_bad_pem() {
    let text = b"-----BEGIN X\x01Y-----\nZm9v\n-----END X\x01Y-----\n";
    assert_eq!(refused(text), AnchorError::BadPem { index: 0 });
    let text = b"-----BEGIN CERTIFICATE\nZm9v\n-----END CERTIFICATE-----\n";
    assert_eq!(refused(text), AnchorError::BadPem { index: 0 });
}

#[test]
fn a_root_the_reader_refuses_is_not_taken_from_an_extra_file() {
    // The extra file takes only what the reader reads whole: a version 1 root, which the
    // bundle would take as name and key, is BadCertificate here.
    let chain = test_certs("leaf-v1/chain.pem");
    let blocks = pem::parse(&chain).expect("PEM");
    let refused_by_reader = blocks
        .iter()
        .find_map(|b| Certificate::from_der(&b.data).err().map(|e| (b, e)));
    let (block, error) = refused_by_reader.expect("leaf-v1 holds a certificate the reader refuses");
    let text = pem_block("CERTIFICATE", &block.data);
    assert_eq!(
        refused(text.as_bytes()),
        AnchorError::BadCertificate { index: 0, error }
    );
}

#[test]
fn every_anchor_error_has_its_name_and_display() {
    let cases = [
        (
            AnchorError::NoCertificate,
            "NoCertificate",
            "NoCertificate: the file holds no PEM block",
        ),
        (
            AnchorError::NotACertificate {
                index: 2,
                label: "X509 CRL".to_string(),
            },
            "NotACertificate",
            "NotACertificate: block 2 is labelled X509 CRL, not CERTIFICATE",
        ),
        (
            AnchorError::BadPem { index: 3 },
            "BadPem",
            "BadPem: block 3 is not PEM",
        ),
        (
            AnchorError::BadCertificate {
                index: 4,
                error: x509::Error::Der(der::Error::Truncated),
            },
            "BadCertificate",
            "BadCertificate: certificate 4 is refused: Truncated",
        ),
        (
            AnchorError::NotACa { index: 5 },
            "NotACa",
            "NotACa: certificate 5 is not a CA",
        ),
        (
            AnchorError::TooLarge,
            "TooLarge",
            "TooLarge: the file holds more than 32768 bytes",
        ),
        (
            AnchorError::TooMany,
            "TooMany",
            "TooMany: the file holds more than 64 blocks",
        ),
    ];
    for (error, name, shown) in cases {
        assert_eq!(error.name(), name);
        assert_eq!(error.to_string(), shown);
    }
}

// ---------------------------------------------------------------------------------------------
// Acceptance 4 and 5: the tasks of taskfiles/tls-anchors.sh, on local files.

/// A case of the refusals of `refresh`: its name, the arguments and the environment.
type RefreshCase<'a> = (&'a str, Vec<&'a Path>, Vec<(&'a str, &'a Path)>);

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../taskfiles/tls-anchors.sh")
}

/// A fresh directory under `CARGO_TARGET_TMPDIR` holding a copy of the committed `cacert.pem`
/// and `SOURCE`.
fn anchors_copy(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tls-anchors-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the directory");
    fs::write(dir.join("cacert.pem"), BUNDLE).expect("write");
    fs::write(dir.join("SOURCE"), SOURCE).expect("write");
    dir
}

fn run(args: &[&Path], env: &[(&str, &Path)], date: Option<&str>) -> Output {
    let mut command = Command::new("sh");
    command.arg(script()).args(args);
    for var in ["FILE", "SHA256", "DATE", "ANCHORS_REMOVED_MAX"] {
        command.env_remove(var);
    }
    for (key, value) in env {
        command.env(key, value);
    }
    if let Some(date) = date {
        command.env("DATE", date);
    }
    command.output().expect("run sh")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A small list in the form of curl's: its header lines and the root of ok-rsa2048.
fn small_list() -> Vec<u8> {
    let mut text = b"##\n## Bundle of CA Root Certificates\n##\n## Certificate data from Mozilla as of: Mon Feb  2 10:00:00 2026 GMT\n##\n\nok-rsa2048\n==========\n".to_vec();
    text.extend_from_slice(&test_certs("ok-rsa2048/root.pem"));
    text
}

fn sha256_line(data: &[u8]) -> String {
    format!(
        "{}  cacert.pem\n",
        hex(crypto::hash(HashAlgorithm::Sha256, data).as_ref())
    )
}

#[test]
fn refresh_with_a_sha256_that_does_not_match_changes_nothing() {
    let dir = anchors_copy("refresh-mismatch");
    let file = dir.join("new.pem");
    let sum = dir.join("new.pem.sha256");
    let list = small_list();
    fs::write(&file, &list).expect("write");
    let mut other = list.clone();
    other.push(b'\n');
    fs::write(&sum, sha256_line(&other)).expect("write");
    let out = run(
        &[Path::new("refresh"), &dir],
        &[("FILE", &file), ("SHA256", &sum)],
        None,
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("sha256") || stderr(&out).contains("sha256"),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(fs::read(dir.join("cacert.pem")).expect("read"), BUNDLE);
    assert_eq!(
        fs::read_to_string(dir.join("SOURCE")).expect("read"),
        SOURCE
    );
    let mut names: Vec<String> = fs::read_dir(&dir)
        .expect("list")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["SOURCE", "cacert.pem", "new.pem", "new.pem.sha256"]);
}

#[test]
fn refresh_with_a_matching_pair_writes_both() {
    let dir = anchors_copy("refresh-match");
    let file = dir.join("new.pem");
    let sum = dir.join("new.pem.sha256");
    let list = small_list();
    fs::write(&file, &list).expect("write");
    fs::write(&sum, sha256_line(&list)).expect("write");
    // The small list removes the 118 titles of the bundle in place: the bound is named.
    let out = run(
        &[Path::new("refresh"), &dir],
        &[
            ("FILE", &file),
            ("SHA256", &sum),
            ("ANCHORS_REMOVED_MAX", Path::new("1000")),
        ],
        None,
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(fs::read(dir.join("cacert.pem")).expect("read"), list);
    let source = fs::read_to_string(dir.join("SOURCE")).expect("read");
    let digest = hex(crypto::hash(HashAlgorithm::Sha256, &list).as_ref());
    assert_eq!(source_field(&source, "sha256"), digest);
    assert_eq!(source_field(&source, "count"), "1");
    assert_eq!(source_field(&source, "date"), "2026-02-02");
    assert_eq!(source_field(&source, "bytes"), list.len().to_string());
    assert_eq!(
        source_field(&source, "header"),
        "Certificate data from Mozilla as of: Mon Feb  2 10:00:00 2026 GMT"
    );
    assert_eq!(source_field(&source, "url"), source_field(SOURCE, "url"));
    assert_eq!(
        source_field(&source, "licence"),
        source_field(SOURCE, "licence")
    );
    // The committed SOURCE has the form the task writes: the same keys in the same order.
    let keys = |s: &str| -> Vec<String> {
        s.lines()
            .filter_map(|l| l.split_once(": ").map(|(k, _)| k.to_string()))
            .collect()
    };
    assert_eq!(keys(&source), keys(SOURCE));
    assert!(
        stdout(&out).contains("anchors-refresh: 1 certificates of 2026-02-02"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn refresh_refuses_what_it_cannot_take_with_one_line() {
    let dir = anchors_copy("refresh-refusals");
    let list = small_list();
    let good = dir.join("good.pem");
    fs::write(&good, &list).expect("write");
    let good_sum = dir.join("good.sha256");
    fs::write(&good_sum, sha256_line(&list)).expect("write");
    let no_header = dir.join("no-header.pem");
    let body = test_certs("ok-rsa2048/root.pem");
    fs::write(&no_header, &body).expect("write");
    let no_header_sum = dir.join("no-header.sha256");
    fs::write(&no_header_sum, sha256_line(&body)).expect("write");
    let no_cert = dir.join("no-cert.pem");
    let header_only = b"## Certificate data from Mozilla as of: Mon Feb  2 10:00:00 2026 GMT\n";
    fs::write(&no_cert, header_only).expect("write");
    let no_cert_sum = dir.join("no-cert.sha256");
    fs::write(&no_cert_sum, sha256_line(header_only)).expect("write");
    let bad_sum = dir.join("bad.sha256");
    fs::write(&bad_sum, "not a sum\n").expect("write");
    let missing = dir.join("missing.pem");
    let refresh = Path::new("refresh");
    let cases: Vec<RefreshCase> = vec![
        ("FILE alone", vec![refresh, &dir], vec![("FILE", &good)]),
        (
            "SHA256 alone",
            vec![refresh, &dir],
            vec![("SHA256", &good_sum)],
        ),
        (
            "a missing FILE",
            vec![refresh, &dir],
            vec![("FILE", &missing), ("SHA256", &good_sum)],
        ),
        (
            "a sha256 file not in its form",
            vec![refresh, &dir],
            vec![("FILE", &good), ("SHA256", &bad_sum)],
        ),
        (
            "no header line",
            vec![refresh, &dir],
            vec![("FILE", &no_header), ("SHA256", &no_header_sum)],
        ),
        (
            "no certificate",
            vec![refresh, &dir],
            vec![("FILE", &no_cert), ("SHA256", &no_cert_sum)],
        ),
        (
            "a missing directory",
            vec![refresh, &missing],
            vec![("FILE", &good), ("SHA256", &good_sum)],
        ),
        ("no directory", vec![refresh], vec![]),
        ("no task", vec![], vec![]),
        ("another task", vec![Path::new("other"), &dir], vec![]),
    ];
    for (case, args, env) in cases {
        let out = run(&args, &env, None);
        let text = format!("{}{}", stdout(&out), stderr(&out));
        assert_eq!(out.status.code(), Some(1), "{case}: {text}");
        assert_eq!(text.lines().count(), 1, "{case}: one line: {text}");
        assert_eq!(
            fs::read(dir.join("cacert.pem")).expect("read"),
            BUNDLE,
            "{case}"
        );
        assert_eq!(
            fs::read_to_string(dir.join("SOURCE")).expect("read"),
            SOURCE,
            "{case}"
        );
    }
    let mut names: Vec<String> = fs::read_dir(&dir)
        .expect("list")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "SOURCE",
            "bad.sha256",
            "cacert.pem",
            "good.pem",
            "good.sha256",
            "no-cert.pem",
            "no-cert.sha256",
            "no-header.pem",
            "no-header.sha256",
        ],
        "no file left behind"
    );
}

#[test]
fn refresh_refuses_a_sha256_file_not_in_its_form() {
    let dir = anchors_copy("refresh-sha256-form");
    let list = small_list();
    let file = dir.join("new.pem");
    fs::write(&file, &list).expect("write");
    let digest = hex(crypto::hash(HashAlgorithm::Sha256, &list).as_ref());
    let sum = dir.join("new.sha256");
    for (case, text) in [
        ("not a sum", "not a sum\n".to_string()),
        ("a short sum", format!("{}  cacert.pem\n", &digest[..63])),
        (
            "upper case",
            format!("{}  cacert.pem\n", digest.to_uppercase()),
        ),
        ("no two spaces", format!("{digest}\n")),
        ("empty", String::new()),
    ] {
        fs::write(&sum, text).expect("write");
        let out = run(
            &[Path::new("refresh"), &dir],
            &[("FILE", &file), ("SHA256", &sum)],
            None,
        );
        let text = format!("{}{}", stdout(&out), stderr(&out));
        assert_eq!(out.status.code(), Some(1), "{case}: {text}");
        assert_eq!(text.lines().count(), 1, "{case}: {text}");
        assert!(
            text.contains("does not start with a sha256") || text.contains("is empty"),
            "{case}: {text}"
        );
        assert_eq!(fs::read(dir.join("cacert.pem")).expect("read"), BUNDLE);
    }
}

/// A list in curl's form whose blocks are the root of ok-ed25519 under the titles `titles`.
fn titled_list(titles: &[String]) -> Vec<u8> {
    let root = test_certs("ok-ed25519/root.pem");
    let mut text =
        b"## Certificate data from Mozilla as of: Mon Feb  2 10:00:00 2026 GMT\n\n".to_vec();
    for title in titles {
        text.extend_from_slice(format!("{title}\n{}\n", "=".repeat(title.len())).as_bytes());
        text.extend_from_slice(&root);
        text.push(b'\n');
    }
    text
}

/// Runs `refresh` of `new` over a directory whose `cacert.pem` is `old`, with the bound
/// `ANCHORS_REMOVED_MAX` when given; the output and whether the old pair is still in place.
fn refresh_over(name: &str, old: &[u8], new: &[u8], removed_max: Option<&str>) -> (Output, bool) {
    let dir = anchors_copy(name);
    fs::write(dir.join("cacert.pem"), old).expect("write");
    let input = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tls-anchors-{name}-in"));
    let _ = fs::remove_dir_all(&input);
    fs::create_dir_all(&input).expect("create");
    let file = input.join("new.pem");
    let sum = input.join("new.sha256");
    fs::write(&file, new).expect("write");
    fs::write(&sum, sha256_line(new)).expect("write");
    let mut env: Vec<(&str, &Path)> = vec![("FILE", &file), ("SHA256", &sum)];
    if let Some(max) = removed_max {
        env.push(("ANCHORS_REMOVED_MAX", Path::new(max)));
    }
    let out = run(&[Path::new("refresh"), &dir], &env, None);
    let unchanged = fs::read(dir.join("cacert.pem")).expect("read") == old
        && fs::read_to_string(dir.join("SOURCE")).expect("read") == SOURCE;
    (out, unchanged)
}

#[test]
fn refresh_prints_the_titles_and_refuses_more_than_10_removed() {
    let names = |range: std::ops::Range<usize>| -> Vec<String> {
        range.map(|i| format!("Root {i:02}")).collect()
    };
    let new = titled_list(&[names(0..1), vec!["New Root".to_string()]].concat());
    // 10 removed: accepted, each title printed before the list is written.
    let (out, unchanged) = refresh_over("removed-10", &titled_list(&names(0..11)), &new, None);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(!unchanged);
    let lines: Vec<String> = stdout(&out).lines().map(str::to_string).collect();
    let mut expected = vec!["anchors-refresh: added New Root".to_string()];
    expected.extend((1..11).map(|i| format!("anchors-refresh: removed Root {i:02}")));
    expected.push("anchors-refresh: 2 certificates of 2026-02-02, sha256 ".to_string());
    assert_eq!(lines.len(), expected.len(), "{lines:?}");
    for (line, want) in lines.iter().zip(&expected) {
        assert!(line.starts_with(want.as_str()), "{line:?} {want:?}");
    }
    // 11 removed: refused, nothing changed, the titles still printed and one refusal line.
    let old = titled_list(&names(0..12));
    let (out, unchanged) = refresh_over("removed-11", &old, &new, None);
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(unchanged);
    assert_eq!(
        stdout(&out)
            .lines()
            .filter(|l| l.contains("removed"))
            .count(),
        11
    );
    assert_eq!(stderr(&out).lines().count(), 1, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("removes 11 titles, more than 10"),
        "{}",
        stderr(&out)
    );
    // 11 removed with the bound named: accepted.
    let (out, unchanged) = refresh_over("removed-11-named", &old, &new, Some("11"));
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(!unchanged);
    // A bound that is not a number is refused.
    let (out, unchanged) = refresh_over("removed-bad-bound", &old, &new, Some("eleven"));
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(unchanged);
}

#[test]
fn refresh_refuses_a_target_that_is_not_a_regular_file() {
    let dir = anchors_copy("refresh-target-dir");
    fs::remove_file(dir.join("SOURCE")).expect("remove");
    fs::create_dir(dir.join("SOURCE")).expect("a directory named SOURCE");
    let list = small_list();
    let input = Path::new(env!("CARGO_TARGET_TMPDIR")).join("tls-anchors-target-dir-in");
    let _ = fs::remove_dir_all(&input);
    fs::create_dir_all(&input).expect("create");
    let file = input.join("new.pem");
    let sum = input.join("new.sha256");
    fs::write(&file, &list).expect("write");
    fs::write(&sum, sha256_line(&list)).expect("write");
    let out = run(
        &[Path::new("refresh"), &dir],
        &[
            ("FILE", &file),
            ("SHA256", &sum),
            ("ANCHORS_REMOVED_MAX", Path::new("1000")),
        ],
        None,
    );
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(text.contains("is not a regular file"), "{text}");
    assert_eq!(fs::read(dir.join("cacert.pem")).expect("read"), BUNDLE);
    assert!(dir.join("SOURCE").is_dir());
}

#[test]
fn refresh_refuses_blocks_out_of_order_and_a_header_not_spaced_as_curl_writes_it() {
    let root = String::from_utf8(test_certs("ok-ed25519/root.pem")).expect("ASCII");
    let header = "## Certificate data from Mozilla as of: Mon Feb  2 10:00:00 2026 GMT\n";
    let end_first = root.replace("-----BEGIN CERTIFICATE-----\n", "");
    let begin_twice = root.replace("-----END CERTIFICATE-----\n", "") + &root;
    for (case, list) in [
        (
            "an END before its BEGIN",
            format!("{header}{end_first}{root}"),
        ),
        ("a BEGIN inside a block", format!("{header}{begin_twice}")),
        (
            "a day not padded",
            format!("## Certificate data from Mozilla as of: Mon Feb 2 10:00:00 2026 GMT\n{root}"),
        ),
        (
            "a time of 24 hours",
            format!("## Certificate data from Mozilla as of: Mon Feb  2 24:00:00 2026 GMT\n{root}"),
        ),
        (
            "a year 0999",
            format!("## Certificate data from Mozilla as of: Mon Feb  2 10:00:00 0999 GMT\n{root}"),
        ),
    ] {
        let (out, unchanged) =
            refresh_over("order-and-form", BUNDLE, list.as_bytes(), Some("1000"));
        let text = format!("{}{}", stdout(&out), stderr(&out));
        assert_eq!(out.status.code(), Some(1), "{case}: {text}");
        assert_eq!(text.lines().count(), 1, "{case}: {text}");
        assert!(unchanged, "{case}");
    }
}

/// A directory with a `SOURCE` of the list date `date` and 118 certificates.
fn source_of(name: &str, date: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tls-anchors-age-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create");
    let source: String = SOURCE
        .lines()
        .map(|line| {
            if line.starts_with("date: ") {
                format!("date: {date}\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    fs::write(dir.join("SOURCE"), source).expect("write");
    dir
}

#[test]
fn age_91_days_after_the_list_date_fails_naming_the_age() {
    let count = source_field(SOURCE, "count");
    let dir = source_of("91", "2026-11-05");
    let out = run(&[Path::new("age"), &dir], &[], Some("2027-02-04"));
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(
        text.contains(&format!(
            "anchors: {count} certificates of 2026-11-05, 91 days old"
        )),
        "{text}"
    );
    assert!(text.contains("older than 90 days"), "{text}");
}

#[test]
fn age_90_days_after_the_list_date_passes() {
    let count = source_field(SOURCE, "count");
    let dir = source_of("90", "2026-11-05");
    let out = run(&[Path::new("age"), &dir], &[], Some("2027-02-03"));
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(
        stdout(&out),
        format!("anchors: {count} certificates of 2026-11-05, 90 days old\n")
    );
}

#[test]
fn age_takes_february_29_only_in_a_leap_year() {
    let dir = source_of("february-29", "2000-01-01");
    for (date, code) in [
        ("2000-02-29", 0),
        ("2028-02-29", 1),
        ("2027-02-29", 1),
        ("2100-02-29", 1),
        ("2400-02-29", 1),
    ] {
        let out = run(&[Path::new("age"), &dir], &[], Some(date));
        let text = format!("{}{}", stdout(&out), stderr(&out));
        assert_eq!(out.status.code(), Some(code), "{date}: {text}");
        let days_old = text.contains("days old");
        let no_day = text.contains("has no day 29");
        match date {
            // 2000 is a leap year (divisible by 400): 59 days.
            "2000-02-29" => assert!(text.contains(" 59 days old"), "{date}: {text}"),
            // Leap years, too old for the list of 2000.
            "2028-02-29" | "2400-02-29" => assert!(days_old && !no_day, "{date}: {text}"),
            // Not leap years: refused as a date.
            _ => assert!(no_day && !days_old, "{date}: {text}"),
        }
    }
}

#[test]
fn age_counts_days_across_leap_years_and_months() {
    let dir = source_of("leap", "2028-01-01");
    for (date, days, code) in [
        ("2028-01-01", 0, 0),
        ("2028-03-31", 90, 0),
        ("2028-04-01", 91, 1),
    ] {
        let out = run(&[Path::new("age"), &dir], &[], Some(date));
        assert_eq!(out.status.code(), Some(code), "{date}");
        assert!(
            stdout(&out).contains(&format!("of 2028-01-01, {days} days old")),
            "{date}: {}",
            stdout(&out)
        );
    }
    let dir = source_of("century", "2099-12-01");
    let out = run(&[Path::new("age"), &dir], &[], Some("2100-03-01"));
    // 2100 is not a leap year: 31 + 31 + 28 = 90 days (91 if it were).
    assert!(stdout(&out).contains("90 days old"), "{}", stdout(&out));
}

#[test]
fn age_refuses_what_it_cannot_read_with_one_line() {
    let dir = source_of("refusals", "2026-11-05");
    let no_date = Path::new(env!("CARGO_TARGET_TMPDIR")).join("tls-anchors-age-no-date");
    let _ = fs::remove_dir_all(&no_date);
    fs::create_dir_all(&no_date).expect("create");
    fs::write(no_date.join("SOURCE"), "count: 3\n").expect("write");
    let missing = dir.join("missing");
    let age = Path::new("age");
    for (case, args, date) in [
        (
            "a DATE not yyyy-mm-dd",
            vec![age, dir.as_path()],
            "2027-2-4",
        ),
        ("a month 13", vec![age, dir.as_path()], "2027-13-01"),
        ("a year 0000", vec![age, dir.as_path()], "0000-01-01"),
        ("a year 0800", vec![age, dir.as_path()], "0800-01-01"),
        (
            "a day 30 of February",
            vec![age, dir.as_path()],
            "2028-02-30",
        ),
        (
            "a DATE before the list",
            vec![age, dir.as_path()],
            "2026-11-04",
        ),
        (
            "a SOURCE without date",
            vec![age, no_date.as_path()],
            "2027-01-01",
        ),
        (
            "a missing directory",
            vec![age, missing.as_path()],
            "2027-01-01",
        ),
    ] {
        let out = run(&args, &[], Some(date));
        let text = format!("{}{}", stdout(&out), stderr(&out));
        assert_eq!(out.status.code(), Some(1), "{case}: {text}");
        assert_eq!(text.lines().count(), 1, "{case}: one line: {text}");
    }
}
