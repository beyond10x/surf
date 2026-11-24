//! `story:tls-fetch-https`, case 22: each of the five TLS failures of `surf-fetch` is answered
//! with the `surf.net.FailureReason` of the same name (`surf::answer`).

use surf::{Answer, answer};
use surf_fetch::Failure;
use surf_types::net::FailureReason;

#[test]
fn each_tls_failure_maps_to_the_reason_of_the_same_name() {
    let detail = || "detail".to_string();
    let cases = [
        (
            Failure::CertificateNotTrusted(detail()),
            FailureReason::CertificateNotTrusted,
        ),
        (
            Failure::CertificateExpired(detail()),
            FailureReason::CertificateExpired,
        ),
        (
            Failure::CertificateNameMismatch(detail()),
            FailureReason::CertificateNameMismatch,
        ),
        (
            Failure::TlsVersionUnsupported(detail()),
            FailureReason::TlsVersionUnsupported,
        ),
        (
            Failure::TlsHandshakeFailed(detail()),
            FailureReason::TlsHandshakeFailed,
        ),
    ];
    for (failure, reason) in cases {
        let shown = format!("{failure:?}");
        assert_eq!(answer(Err(failure)), Answer::Failure(reason), "{shown}");
    }
}
