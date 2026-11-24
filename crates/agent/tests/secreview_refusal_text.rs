//! Security review case of `story:tls-session-anchors`: the line by which the client refuses a
//! `--ca-file` file names the block's index and the kind of refusal, and repeats no byte of the
//! file, neither the label of a BEGIN line nor a byte of a certificate it cannot read.

use crate::common;

use std::fs;

use common::*;

/// `surf-agent open --ca-file <file holding text>`: exit 2 and the line on standard error.
fn refusal(dir: &TestDir, name: &str, text: &str) -> String {
    let file = dir.path.join(name);
    fs::write(&file, text).unwrap();
    let output = run(agent(dir).arg("open").arg("--ca-file").arg(&file));
    let err = stderr(&output);
    assert_eq!(output.status.code(), Some(2), "{err}");
    assert_eq!(dir.requests(), "", "{name}: a request was sent");
    err.lines().next().unwrap_or_default().to_string()
}

#[test]
fn secreview_refusal_text_a_label_of_the_file_is_not_repeated() {
    let dir = TestDir::new();
    let label = "api-token-7f3a9c41d2e85b60";
    let line = refusal(
        &dir,
        "not-a-ca.pem",
        &format!("-----BEGIN {label}-----\nZm9v\n-----END {label}-----\n"),
    );
    assert!(
        !line.contains(label),
        "the refusal repeats bytes of the file: {line}"
    );
    assert_eq!(
        line,
        "surf-agent: the --ca-file file is refused: NotACertificate: block 0 is not a CERTIFICATE"
    );
}

/// A `CERTIFICATE` block holding the bytes 0x31 0x00: the certificate reader finds the tag 0x31
/// (a SET) where a SEQUENCE belongs. The line names the reader's kind, not the byte.
#[test]
fn secreview_refusal_text_a_byte_of_an_unreadable_certificate_is_not_repeated() {
    let dir = TestDir::new();
    let line = refusal(
        &dir,
        "not-der.pem",
        "-----BEGIN CERTIFICATE-----\nMQA=\n-----END CERTIFICATE-----\n",
    );
    assert!(
        !line.contains("0x31"),
        "the refusal repeats bytes of the file: {line}"
    );
    assert_eq!(
        line,
        "surf-agent: the --ca-file file is refused: BadCertificate: certificate 0 is refused: \
         UnexpectedTag"
    );
}
