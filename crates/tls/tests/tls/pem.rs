//! PEM and base64, `surf_tls::pem` (`story:tls-x509`, acceptance 4): blocks between equal BEGIN
//! and END labels, base64 of RFC 4648 with line breaks and its padding checked, text outside
//! blocks ignored, each block with its label.

#![forbid(unsafe_code)]

use std::time::Instant;

use surf_tls::pem::{self, Error};

use crate::alloc::peak_while;

fn labels_and_data(text: &str) -> Vec<(String, Vec<u8>)> {
    pem::parse(text.as_bytes())
        .expect("PEM")
        .into_iter()
        .map(|b| (b.label, b.data))
        .collect()
}

fn block(body: &str) -> Result<Vec<u8>, Error> {
    let text = format!("-----BEGIN DATA-----\n{body}\n-----END DATA-----\n");
    pem::parse(text.as_bytes()).map(|mut blocks| blocks.remove(0).data)
}

// ---------------------------------------------------------------------------------------------
// Acceptance 4.

#[test]
fn a_block_with_wrong_padding_is_bad_base64() {
    for body in [
        "Zm9vYg=",  // one pad of two
        "Zm9vYg",   // no padding
        "Zm9vY===", // three pads
        "Zm9vYh==", // bits left over that are not zero
        "Zm9vYmF=", // bits left over that are not zero
        "Zg==Zg==", // data after padding
        "Z===", "=", "Zm9v!",
    ] {
        assert_eq!(block(body), Err(Error::BadBase64), "{body:?}");
    }
}

#[test]
fn a_begin_without_its_end_is_unterminated() {
    let text = "-----BEGIN CERTIFICATE-----\nZm9v\n";
    assert_eq!(pem::parse(text.as_bytes()), Err(Error::UnterminatedBlock));
    let text = "-----BEGIN CERTIFICATE-----\nZm9v\n-----BEGIN CERTIFICATE-----\nZm9v\n";
    assert_eq!(pem::parse(text.as_bytes()), Err(Error::UnterminatedBlock));
}

/// 16 MiB of `A` with no marker: no block, every byte looked at a bounded number of times
/// (the count of `parse_counting`), and nothing allocated by it. The time is printed, not
/// asserted.
#[test]
fn sixteen_mebibytes_of_a_without_a_marker_hold_no_block() {
    let text = vec![b'A'; 16 << 20];
    let started = Instant::now();
    let ((result, work), peak) = peak_while(|| pem::parse_counting(&text));
    let elapsed = started.elapsed();
    println!(
        "pem: 16 MiB of A in {:.3} s, {work} bytes looked at, peak {peak} bytes",
        elapsed.as_secs_f64()
    );
    assert_eq!(result, Ok(Vec::new()));
    assert!(work <= 2 * text.len(), "{work} bytes looked at");
    assert!(peak < 64 * 1024, "peak {peak} bytes");
}

#[test]
fn a_mebibyte_of_a_inside_a_block_is_looked_at_a_bounded_number_of_times() {
    let mut text = b"-----BEGIN DATA-----\n".to_vec();
    text.extend(std::iter::repeat_n(b'A', 1 << 20));
    text.extend_from_slice(b"\n-----END DATA-----\n");
    let (result, work) = pem::parse_counting(&text);
    let blocks = result.expect("one block");
    assert_eq!(blocks[0].data.len(), 3 << 18);
    assert!(blocks[0].data.iter().all(|&b| b == 0));
    assert!(work <= 3 * text.len(), "{work} bytes looked at");
}

// ---------------------------------------------------------------------------------------------
// Base64 (RFC 4648 section 10) and the lines.

#[test]
fn the_test_vectors_of_rfc_4648_decode() {
    for (body, data) in [
        ("Zg==", "f"),
        ("Zm8=", "fo"),
        ("Zm9v", "foo"),
        ("Zm9vYg==", "foob"),
        ("Zm9vYmE=", "fooba"),
        ("Zm9vYmFy", "foobar"),
        ("+/+/", "\u{fb}\u{ff}\u{bf}"),
    ] {
        let expected: Vec<u8> = if body == "+/+/" {
            vec![0xFB, 0xFF, 0xBF]
        } else {
            data.as_bytes().to_vec()
        };
        assert_eq!(block(body), Ok(expected), "{body}");
    }
}

#[test]
fn an_empty_block_holds_no_bytes() {
    let text = "-----BEGIN DATA-----\n-----END DATA-----\n";
    assert_eq!(
        labels_and_data(text),
        vec![("DATA".to_string(), Vec::new())]
    );
}

#[test]
fn line_breaks_and_crlf_inside_the_base64_are_skipped() {
    assert_eq!(block("Zm9v\nYmFy"), Ok(b"foobar".to_vec()));
    let text = "-----BEGIN DATA-----\r\nZm9v\r\nYmE=\r\n-----END DATA-----\r\n";
    assert_eq!(
        labels_and_data(text),
        vec![("DATA".to_string(), b"fooba".to_vec())]
    );
}

#[test]
fn text_outside_blocks_is_ignored_and_each_block_keeps_its_label() {
    let text = "Subject: an example\n\
                -----BEGIN CERTIFICATE-----\nZm9v\n-----END CERTIFICATE-----\n\
                between -----END CERTIFICATE----- them\n\
                -----BEGIN X509 CRL-----\nYmFy\n-----END X509 CRL-----\n\
                trailing text";
    assert_eq!(
        labels_and_data(text),
        vec![
            ("CERTIFICATE".to_string(), b"foo".to_vec()),
            ("X509 CRL".to_string(), b"bar".to_vec()),
        ]
    );
}

#[test]
fn the_last_end_line_needs_no_line_break() {
    let text = "-----BEGIN DATA-----\nZm9v\n-----END DATA-----";
    assert_eq!(
        labels_and_data(text),
        vec![("DATA".to_string(), b"foo".to_vec())]
    );
}

#[test]
fn an_end_with_another_label_is_a_mismatch() {
    let text = "-----BEGIN CERTIFICATE-----\nZm9v\n-----END PRIVATE KEY-----\n";
    assert_eq!(pem::parse(text.as_bytes()), Err(Error::LabelMismatch));
}

#[test]
fn a_begin_line_without_its_closing_dashes_is_a_bad_label() {
    let text = "-----BEGIN CERTIFICATE\nZm9v\n-----END CERTIFICATE-----\n";
    assert_eq!(pem::parse(text.as_bytes()), Err(Error::BadLabel));
    let text = "-----BEGIN CERT\u{1}-----\nZm9v\n-----END CERT\u{1}-----\n";
    assert_eq!(pem::parse(text.as_bytes()), Err(Error::BadLabel));
}

#[test]
fn every_error_has_its_name() {
    assert_eq!(Error::BadBase64.name(), "BadBase64");
    assert_eq!(Error::UnterminatedBlock.name(), "UnterminatedBlock");
    assert_eq!(Error::LabelMismatch.name(), "LabelMismatch");
    assert_eq!(Error::BadLabel.name(), "BadLabel");
}
