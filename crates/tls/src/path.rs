//! Path building and validation: [`PathVerifier`], the [`ServerCertVerifier`] of surf over a
//! set of trust anchors (RFC 5280 section 6.1, the parts `epic:tls-https` names;
//! docs/design/tls.md, items 6, 7, 9 and 13; `story:tls-x509-path`).
//!
//! # What a validation does
//!
//! 1. Every certificate the server sent is read; one the reader refuses ends the validation
//!    with `NotTrusted(Unreadable)` and the reader's error. A certificate sent twice (the same
//!    bytes) counts once.
//! 2. The leaf's own key is held to [`crypto::REFUSED`]: an RSA key under 2048 bits is
//!    `NotTrusted(WeakKey)` before any signature is verified.
//! 3. The path is searched depth-first from the leaf (item 9.4). For the certificate at the end
//!    of the path, its candidate issuers are the anchors, then the certificates sent in the
//!    order sent, that are not on the path, whose subject equals its issuer byte for byte and,
//!    when both carry one, whose subjectKeyIdentifier equals its authorityKeyIdentifier (item
//!    9.4; the anchor's when the anchor was made from its certificate). Each name comparison
//!    charges [`Work::considered`]. A sent candidate that would be one
//!    intermediate more than [`Limits::intermediates`] is not descended into (item 9.3). Before
//!    any other candidate is taken, the policy of [`crypto::REFUSED`] is applied to the
//!    signature and the candidate's key (`WeakAlgorithm`, `WeakKey`), then the signature is
//!    verified through the seam, which charges [`Work::signature_checks`]. A sent candidate
//!    whose signature verifies is descended into; an anchor whose signature verifies ends a
//!    path, and the checks below run on it: the first path they pass is the answer.
//! 4. The host is matched against the leaf's subjectAltName ([`crate::names`]):
//!    `NameMismatch` when no entry matches.
//!
//! When the search ends without a path, the answer is, in this order: a limit of the search
//! reached (`CandidateLimit`); a candidate cut by the bound of intermediates (`PathTooLong`);
//! a candidate refused by the policy (`WeakAlgorithm`, `WeakKey`, the first met); a name match
//! whose signature did not verify (`BadSignature`, the first met); a complete path that failed
//! a check (that check, of the last such path); else `NoPathToAnchor`.
//!
//! # The checks of a complete path
//!
//! In this order, each certificate from the leaf up, the anchor's certificate last when the
//! anchor was made from one ([`PathVerifier::add_root`]):
//!
//! | condition | rejection |
//! |---|---|
//! | an extension the reader does not know, marked critical; authorityKeyIdentifier or subjectKeyIdentifier marked critical (RFC 5280 sections 4.2.1.1 and 4.2.1.2: "MUST mark this extension as non-critical") | `UnknownCriticalExtension` |
//! | the anchor's certificate refused by the reader for its extensions (`BadExtension`, `DuplicateExtension`), so that what it constrains is not known; the anchor's authorityKeyIdentifier with `authorityCertIssuer` or `authorityCertSerialNumber`, or with a keyIdentifier other than its subjectKeyIdentifier | `Unreadable` with `BadExtension` (or the reader's error) |
//! | the leaf's subject empty and its subjectAltName not critical (RFC 5280 section 4.2.1.6) | `Unreadable` with `BadExtension` |
//! | an issuer without basicConstraints, with cA false, with basicConstraints not critical (section 4.2.1.9), or with an empty subject (section 4.1.2.6) | `IssuerNotCa` |
//! | an issuer whose keyUsage lacks keyCertSign | `KeyUsage` |
//! | an issuer with a pathLenConstraint below the intermediates under it that are not self-issued | `PathTooLong` |
//! | the leaf's keyUsage, when present, without digitalSignature | `KeyUsage` |
//! | the leaf's extKeyUsage, when present, without id-kp-serverAuth (anyExtendedKeyUsage alone does not do) | `NoServerAuth` |
//! | a certificate outside its validity at the time given, either side, both ends included | `Expired` |
//! | nameConstraints in a leaf that is no CA, or not marked critical in an issuer or the anchor's certificate (section 4.2.1.10) | `NameConstraint` |
//! | a name outside the nameConstraints of an issuer or of the anchor | `NameConstraint` |
//!
//! `<n>` of a rejection counts the certificates as the server sent them, the leaf 0; the anchor
//! is numbered after them (the count of certificates sent, after duplicates are dropped).
//!
//! # Name constraints
//!
//! The nameConstraints of each issuer on the path and of the anchor hold for every certificate
//! below it that is not self-issued, the leaf always (RFC 5280 section 6.1.3 (b) and (c)), over
//! the dNSName and iPAddress entries of their subjectAltName: a name of a form must lie in one
//! permitted subtree of that form when the extension permits any of that form, and in no
//! excluded subtree. A dNSName subtree holds the names made by adding labels to its left
//! (section 4.2.1.10); a subtree that is not a DNS name (a leading dot, a wildcard) fails every
//! name it is compared with. A wildcard entry `*.<suffix>` is held to every name it can match:
//! permitted when the suffix lies in the subtree, excluded when the suffix lies in it or the
//! subtree is one label plus the suffix. An iPAddress subtree holds the addresses of its family
//! whose octets under the mask equal its own. Names of another form (rfc822Name,
//! uniformResourceIdentifier, otherName, directoryName) are not checked: a constraint of such a
//! form over a subjectAltName entry of that form, or a directoryName constraint over a
//! certificate with a subject, fails closed (`NameConstraint`). The extension holds whether it
//! is marked critical or not. Each comparison charges [`Work::name_constraint_comparisons`];
//! at [`Limits::name_constraint_comparisons`] the path fails with `NameConstraint` naming that
//! work.
//!
//! # Bounds and cost
//!
//! [`INTERMEDIATES_MAX`] 6 (item 9.3), [`SIGNATURE_CHECKS_MAX`] 100 and [`CANDIDATES_MAX`]
//! 25,000 (item 9.4), and [`NAME_CONSTRAINT_COMPARISONS_MAX`] 250,000, the budget
//! rustls-webpki keeps for the same work (`name_constraint_comparisons`). A validation costs at
//! most 100 signature verifications, 25,000 name comparisons of the search and 250,000 of name
//! constraints, each counted in [`Work`] ([`PathVerifier::verify_counting`]).
//!
//! Not checked: revocation (no CRL, no OCSP, no network request of any kind), certificate
//! policies, and the self-signature of an anchor.

use std::fmt;

use crate::crypto::{
    self, PublicKey, PublicKeyKind, ShippedVerifier, SignatureError, SignatureVerifier,
};
use crate::der::{self, Time};
use crate::names;
use crate::verify::{
    CertificateRejection, Check, LeafKey, Limit, PathTooLong, ServerCertVerifier, TrustAnchor,
};
use crate::x509::{self, Certificate, GeneralName, KeyUsage, NameConstraints};

/// The most intermediates on a path (item 9.3; rustls-webpki's `MAX_SUB_CA_COUNT`).
pub const INTERMEDIATES_MAX: usize = 6;
/// The most signature verifications of one validation (item 9.4).
pub const SIGNATURE_CHECKS_MAX: usize = 100;
/// The most candidates one validation compares by name (item 9.4).
pub const CANDIDATES_MAX: usize = 25_000;
/// The most comparisons of a name with a subtree of name constraints in one validation.
pub const NAME_CONSTRAINT_COMPARISONS_MAX: usize = 250_000;

/// The OBJECT IDENTIFIERs, dotted, that a rejection names for a critical extension the profile
/// requires to be non-critical.
const AUTHORITY_KEY_ID_OID: &str = "2.5.29.35";
const SUBJECT_KEY_ID_OID: &str = "2.5.29.14";

/// The bounds of one validation. [`Limits::default`] holds those of item 9; a test sets them
/// small.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most intermediates on a path.
    pub intermediates: usize,
    /// The most signature verifications.
    pub signature_checks: usize,
    /// The most candidates compared by name.
    pub candidates: usize,
    /// The most comparisons of a name with a subtree of name constraints.
    pub name_constraint_comparisons: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            intermediates: INTERMEDIATES_MAX,
            signature_checks: SIGNATURE_CHECKS_MAX,
            candidates: CANDIDATES_MAX,
            name_constraint_comparisons: NAME_CONSTRAINT_COMPARISONS_MAX,
        }
    }
}

/// The work of one validation, counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    /// Candidates compared by name.
    pub considered: usize,
    /// Signatures verified, whether they verified or not.
    pub signature_checks: usize,
    /// Names compared with a subtree of name constraints.
    pub name_constraint_comparisons: usize,
}

/// An anchor and, when it was made from a certificate the reader reads, that certificate.
#[derive(Clone, Debug)]
struct Anchor {
    anchor: TrustAnchor,
    certificate: Option<Vec<u8>>,
}

/// The [`ServerCertVerifier`] of surf: path building and validation over a set of trust
/// anchors, with the signature checks of `V` ([`ShippedVerifier`] unless a test gives its own).
#[derive(Clone)]
pub struct PathVerifier<V = ShippedVerifier> {
    anchors: Vec<Anchor>,
    limits: Limits,
    verifier: V,
}

impl<V> fmt::Debug for PathVerifier<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathVerifier")
            .field("anchors", &self.anchors.len())
            .field("limits", &self.limits)
            .finish()
    }
}

impl PathVerifier {
    /// A verifier over `anchors`, each its name, key and nameConstraints only, with the limits
    /// of item 9 and the seam's signature checks.
    pub fn new(anchors: Vec<TrustAnchor>) -> Self {
        PathVerifier {
            anchors: anchors
                .into_iter()
                .map(|anchor| Anchor {
                    anchor,
                    certificate: None,
                })
                .collect(),
            limits: Limits::default(),
            verifier: ShippedVerifier,
        }
    }
}

impl<V: SignatureVerifier> PathVerifier<V> {
    /// Adds the root certificate `der` as an anchor ([`TrustAnchor::from_der`]). When the reader
    /// reads it, its certificate is kept and checked as an issuer of every path that ends at it
    /// (its extensions and validity); else the anchor is its name and key. The reader's error
    /// when the root gives no anchor.
    pub fn add_root(&mut self, der: &[u8]) -> Result<(), x509::Error> {
        let anchor = TrustAnchor::from_der(der)?;
        let certificate = anchor.refused.is_none().then(|| der.to_vec());
        self.anchors.push(Anchor {
            anchor,
            certificate,
        });
        Ok(())
    }

    /// The same verifier with other limits.
    pub fn with_limits(self, limits: Limits) -> Self {
        PathVerifier { limits, ..self }
    }

    /// The same verifier with another signature verifier.
    pub fn with_verifier<W: SignatureVerifier>(self, verifier: W) -> PathVerifier<W> {
        PathVerifier {
            anchors: self.anchors,
            limits: self.limits,
            verifier,
        }
    }

    /// The limits.
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// [`ServerCertVerifier::verify`], and the work it took.
    pub fn verify_counting(
        &self,
        certificates: &[&[u8]],
        host: &str,
        now: Time,
    ) -> (Result<LeafKey, CertificateRejection>, Work) {
        let mut work = Work::default();
        let result = self.validate(certificates, host, now, &mut work);
        (result, work)
    }

    fn validate(
        &self,
        certificates: &[&[u8]],
        host: &str,
        now: Time,
        work: &mut Work,
    ) -> Result<LeafKey, CertificateRejection> {
        let not_trusted = |check| Err(CertificateRejection::NotTrusted(check));
        if certificates.is_empty() {
            return not_trusted(Check::NoPathToAnchor);
        }
        let mut sent: Vec<Certificate<'_>> = Vec::with_capacity(certificates.len());
        for (n, der) in certificates.iter().enumerate() {
            match Certificate::from_der(der) {
                Ok(cert) => sent.push(cert),
                Err(error) => {
                    return not_trusted(Check::Unreadable {
                        certificate: n,
                        error,
                    });
                }
            }
        }
        // A certificate sent twice counts once: the first of equal bytes is kept.
        let mut kept: Vec<Certificate<'_>> = Vec::with_capacity(sent.len());
        for cert in sent {
            if !kept.iter().any(|k| k.der == cert.der) {
                kept.push(cert);
            }
        }
        let sent = kept;
        let leaf = &sent[0];
        if let PublicKeyKind::Rsa { bits } = leaf.public_key.kind
            && bits < crypto::REFUSED.rsa_bits_min
        {
            return not_trusted(Check::WeakKey {
                certificate: 0,
                bits,
            });
        }
        let mut search = Search {
            verifier: self,
            sent: &sent,
            now,
            work,
            limit: None,
            cut: None,
            refused: None,
            bad_signature: None,
            failed: None,
        };
        let mut path = vec![0];
        match search.extend(&mut path) {
            Step::Found => {}
            Step::Stop | Step::Exhausted => return Err(search.answer()),
        }
        if !names::certificate_matches(leaf, host) {
            return Err(CertificateRejection::NameMismatch {
                host: host.to_string(),
            });
        }
        Ok(LeafKey::from_certificate(leaf))
    }
}

impl<V: SignatureVerifier> ServerCertVerifier for PathVerifier<V> {
    fn verify(
        &self,
        certificates: &[&[u8]],
        host: &str,
        now: Time,
    ) -> Result<LeafKey, CertificateRejection> {
        self.verify_counting(certificates, host, now).0
    }
}

/// How a step of the search ended.
enum Step {
    /// A path passed every check.
    Found,
    /// A limit was reached: the search ends.
    Stop,
    /// No candidate led to a path.
    Exhausted,
}

/// What a signature check of a candidate gave.
enum Signed {
    Verified,
    Refused(Check),
    Failed,
    Limit,
}

struct Search<'s, 'c, V> {
    verifier: &'s PathVerifier<V>,
    sent: &'s [Certificate<'c>],
    now: Time,
    work: &'s mut Work,
    limit: Option<Limit>,
    cut: Option<PathTooLong>,
    refused: Option<Check>,
    bad_signature: Option<usize>,
    failed: Option<CertificateRejection>,
}

impl<V: SignatureVerifier> Search<'_, '_, V> {
    /// The answer when the search found no path (item 9.4).
    fn answer(&mut self) -> CertificateRejection {
        let check = if let Some(limit) = self.limit {
            Check::CandidateLimit(limit)
        } else if let Some(cut) = self.cut {
            Check::PathTooLong(cut)
        } else if let Some(refused) = self.refused.take() {
            refused
        } else if let Some(certificate) = self.bad_signature {
            Check::BadSignature { certificate }
        } else if let Some(failed) = self.failed.take() {
            return failed;
        } else {
            Check::NoPathToAnchor
        };
        CertificateRejection::NotTrusted(check)
    }

    /// Charges one name comparison; `false` when the limit is reached.
    fn consider(&mut self) -> bool {
        let bound = self.verifier.limits.candidates;
        if self.work.considered >= bound {
            self.limit = Some(Limit::CandidatesConsidered { bound });
            return false;
        }
        self.work.considered += 1;
        true
    }

    /// The policy of `REFUSED`, then the signature of `child` (the certificate numbered
    /// `child_n`) under `key`, the key of the candidate numbered `key_n`.
    fn signed(&mut self, child_n: usize, key: &PublicKey<'_>, key_n: usize) -> Signed {
        let sent = self.sent;
        let child = &sent[child_n];
        if let Some(refusal) = crypto::refusal(child.signature_algorithm, key.kind) {
            return Signed::Refused(match refusal {
                crypto::Refusal::WeakAlgorithm { algorithm } => Check::WeakAlgorithm {
                    certificate: child_n,
                    algorithm,
                },
                crypto::Refusal::WeakKey { bits } => Check::WeakKey {
                    certificate: key_n,
                    bits,
                },
            });
        }
        let bound = self.verifier.limits.signature_checks;
        if self.work.signature_checks >= bound {
            self.limit = Some(Limit::SignatureChecks { bound });
            return Signed::Limit;
        }
        self.work.signature_checks += 1;
        match crypto::check_signature(
            &self.verifier.verifier,
            child.signature_algorithm,
            key,
            child.tbs,
            child.signature,
        ) {
            Ok(_) => Signed::Verified,
            Err(SignatureError::Refused(_)) | Err(SignatureError::Unsupported) => Signed::Failed,
            Err(SignatureError::BadSignature) => Signed::Failed,
        }
    }

    /// Records what a signature check that did not verify leaves for the answer; `true` when
    /// the search must stop.
    fn note(&mut self, signed: Signed, child_n: usize) -> bool {
        match signed {
            Signed::Verified => false,
            Signed::Refused(check) => {
                self.refused.get_or_insert(check);
                false
            }
            Signed::Failed => {
                self.bad_signature.get_or_insert(child_n);
                false
            }
            Signed::Limit => true,
        }
    }

    /// Extends `path` (indexes into `sent`, the leaf first) by every candidate issuer of its
    /// last certificate, depth first.
    fn extend(&mut self, path: &mut Vec<usize>) -> Step {
        let child_n = *path.last().expect("a path holds the leaf");
        let sent = self.sent;
        let verifier = self.verifier;
        let child = &sent[child_n];
        let anchor_n = sent.len();
        for (a, anchor) in verifier.anchors.iter().enumerate() {
            if !self.consider() {
                return Step::Stop;
            }
            if anchor.anchor.subject() != child.issuer {
                continue;
            }
            if let Some(cert) = anchor
                .certificate
                .as_deref()
                .and_then(|d| Certificate::from_der(d).ok())
                && !key_ids_agree(child, &cert)
            {
                continue;
            }
            let key = anchor.anchor.key.public_key();
            match self.signed(child_n, &key, anchor_n) {
                Signed::Verified => {}
                other => {
                    if self.note(other, child_n) {
                        return Step::Stop;
                    }
                    continue;
                }
            }
            match self.complete(path, a) {
                Ok(()) => return Step::Found,
                Err(rejection) => self.failed = Some(rejection),
            }
        }
        for (j, candidate) in sent.iter().enumerate().skip(1) {
            if path.contains(&j) {
                continue;
            }
            if !self.consider() {
                return Step::Stop;
            }
            if candidate.subject != child.issuer || !key_ids_agree(child, candidate) {
                continue;
            }
            let intermediates = path.len();
            if intermediates > self.verifier.limits.intermediates {
                self.cut = Some(PathTooLong::Intermediates {
                    count: intermediates,
                    bound: self.verifier.limits.intermediates,
                });
                continue;
            }
            let key = candidate.public_key;
            let signed = self.signed(child_n, &key, j);
            if !matches!(signed, Signed::Verified) {
                if self.note(signed, child_n) {
                    return Step::Stop;
                }
                continue;
            }
            path.push(j);
            let step = self.extend(path);
            path.pop();
            match step {
                Step::Exhausted => {}
                found_or_stop => return found_or_stop,
            }
        }
        Step::Exhausted
    }

    /// The checks of a complete path: `path` (indexes into `sent`, the leaf first) and the
    /// anchor numbered `a`.
    fn complete(&mut self, path: &[usize], a: usize) -> Result<(), CertificateRejection> {
        let sent = self.sent;
        let verifier = self.verifier;
        let anchor = &verifier.anchors[a];
        let anchor_n = sent.len();
        let anchor_cert = anchor
            .certificate
            .as_deref()
            .and_then(|d| Certificate::from_der(d).ok());
        // The certificates of the path with their numbers, the leaf first, the anchor's last.
        let mut chain: Vec<(usize, &Certificate<'_>)> =
            path.iter().map(|&n| (n, &sent[n])).collect();
        if let Some(cert) = &anchor_cert {
            chain.push((anchor_n, cert));
        }
        let not_trusted = |check| Err(CertificateRejection::NotTrusted(check));

        for &(n, cert) in &chain {
            if let Some(oid) = cert.extensions.unknown_critical {
                return not_trusted(Check::UnknownCriticalExtension {
                    certificate: n,
                    oid: der::oid_to_string(oid),
                });
            }
            for (critical, oid) in [
                (
                    cert.extensions.critical.authority_key_id,
                    AUTHORITY_KEY_ID_OID,
                ),
                (cert.extensions.critical.subject_key_id, SUBJECT_KEY_ID_OID),
            ] {
                if critical {
                    return not_trusted(Check::UnknownCriticalExtension {
                        certificate: n,
                        oid: oid.to_string(),
                    });
                }
            }
        }
        let bad_extension = |certificate| {
            not_trusted(Check::Unreadable {
                certificate,
                error: x509::Error::BadExtension,
            })
        };
        if let Some(error @ (x509::Error::BadExtension | x509::Error::DuplicateExtension)) =
            anchor.anchor.refused
        {
            return not_trusted(Check::Unreadable {
                certificate: anchor_n,
                error,
            });
        }
        if let Some(cert) = &anchor_cert {
            // The root's authorityKeyIdentifier: no authorityCertIssuer, no
            // authorityCertSerialNumber, and a keyIdentifier equal to its own
            // subjectKeyIdentifier when both are present.
            let ext = &cert.extensions;
            let ids_differ = matches!(
                (ext.authority_key_id, ext.subject_key_id),
                (Some(aki), Some(ski)) if aki != ski
            );
            if ext.authority_cert_issuer || ext.authority_cert_serial || ids_differ {
                return bad_extension(anchor_n);
            }
        }
        let leaf = &sent[0];
        if leaf.subject.der() == EMPTY_NAME && !leaf.extensions.critical.subject_alt_name {
            return bad_extension(0);
        }
        for (i, &(n, cert)) in chain.iter().enumerate().skip(1) {
            let ca = match cert.extensions.basic_constraints {
                Some(bc) => bc.ca && cert.extensions.critical.basic_constraints,
                None => false,
            };
            if !ca || cert.subject.der() == EMPTY_NAME {
                return not_trusted(Check::IssuerNotCa { certificate: n });
            }
            if let Some(ku) = cert.extensions.key_usage
                && !ku.has(KeyUsage::KEY_CERT_SIGN)
            {
                return not_trusted(Check::KeyUsage {
                    certificate: n,
                    lacks: "keyCertSign",
                });
            }
            if let Some(limit) = cert.extensions.basic_constraints.and_then(|bc| bc.path_len) {
                // The intermediates between this issuer and the leaf that are not self-issued.
                let below = chain[1..i].iter().filter(|(_, c)| !self_issued(c)).count();
                if below > limit as usize {
                    return not_trusted(Check::PathTooLong(PathTooLong::PathLenConstraint {
                        certificate: n,
                    }));
                }
            }
        }
        if let Some(ku) = leaf.extensions.key_usage
            && !ku.has(KeyUsage::DIGITAL_SIGNATURE)
        {
            return not_trusted(Check::KeyUsage {
                certificate: 0,
                lacks: "digitalSignature",
            });
        }
        if let Some(eku) = &leaf.extensions.ext_key_usage
            && !eku.server_auth()
        {
            return not_trusted(Check::NoServerAuth);
        }
        for &(n, cert) in &chain {
            let v = cert.validity;
            if self.now < v.not_before || self.now > v.not_after {
                return Err(CertificateRejection::Expired {
                    certificate: n,
                    not_before: v.not_before,
                    not_after: v.not_after,
                    at: self.now,
                });
            }
        }
        // Name constraints: of each issuer on the path and of the anchor, over the
        // certificates below it.
        let path_certs: Vec<(usize, &Certificate<'_>)> =
            path.iter().map(|&n| (n, &sent[n])).collect();
        let nc_refused = |certificate, name: &str| {
            not_trusted(Check::NameConstraint {
                certificate,
                name: name.to_string(),
            })
        };
        // RFC 5280 section 4.2.1.10: "used only in a CA certificate", "MUST mark this extension
        // as critical".
        if leaf.extensions.name_constraints.is_some()
            && !leaf.extensions.basic_constraints.is_some_and(|bc| bc.ca)
        {
            return nc_refused(0, "nameConstraints in an end-entity certificate");
        }
        for &(n, cert) in &chain[1..] {
            if cert.extensions.name_constraints.is_some()
                && !cert.extensions.critical.name_constraints
            {
                return nc_refused(n, "nameConstraints not marked critical");
            }
        }
        for i in 1..path_certs.len() {
            let (n, cert) = path_certs[i];
            if let Some(nc) = &cert.extensions.name_constraints {
                self.constrain(n, nc, &path_certs[..i])?;
            }
        }
        match anchor.anchor.name_constraints() {
            Ok(Some(nc)) => self.constrain(anchor_n, &nc, &path_certs)?,
            Ok(None) => {}
            Err(error) => {
                return not_trusted(Check::Unreadable {
                    certificate: anchor_n,
                    error,
                });
            }
        }
        Ok(())
    }

    /// The nameConstraints `nc` of the certificate numbered `issuer_n` over `below` (the leaf
    /// first): each not self-issued, and the leaf, held to them.
    fn constrain(
        &mut self,
        issuer_n: usize,
        nc: &NameConstraints<'_>,
        below: &[(usize, &Certificate<'_>)],
    ) -> Result<(), CertificateRejection> {
        let refuse = |name: String| {
            Err(CertificateRejection::NotTrusted(Check::NameConstraint {
                certificate: issuer_n,
                name,
            }))
        };
        let permitted = nc.permitted.as_deref().unwrap_or(&[]);
        let excluded = nc.excluded.as_deref().unwrap_or(&[]);
        let has_form = |list: &[GeneralName<'_>], tag: u8| list.iter().any(|g| form(g) == tag);
        for (i, &(_, cert)) in below.iter().enumerate() {
            if i > 0 && self_issued(cert) {
                continue;
            }
            // directoryName constraints over the subject: not checked, so they fail closed.
            if (has_form(permitted, DIRECTORY) || has_form(excluded, DIRECTORY))
                && cert.subject.der() != EMPTY_NAME
            {
                return refuse(format!("the subject of certificate {}", below[i].0));
            }
            let Some(sans) = &cert.extensions.subject_alt_name else {
                continue;
            };
            for san in sans {
                let tag = form(san);
                let forms_checked = matches!(san, GeneralName::Dns(_) | GeneralName::Ip(_));
                let constrained_form = has_form(permitted, tag) || has_form(excluded, tag);
                if !constrained_form {
                    continue;
                }
                if !forms_checked {
                    return refuse(name_text(san));
                }
                if has_form(permitted, tag) {
                    let mut inside = false;
                    for base in permitted.iter().filter(|b| form(b) == tag) {
                        self.compare(issuer_n)?;
                        if within(san, base) {
                            inside = true;
                            break;
                        }
                    }
                    if !inside {
                        return refuse(name_text(san));
                    }
                }
                for base in excluded.iter().filter(|b| form(b) == tag) {
                    self.compare(issuer_n)?;
                    if meets_excluded(san, base) {
                        return refuse(name_text(san));
                    }
                }
            }
        }
        Ok(())
    }

    /// Charges one comparison of name constraints.
    fn compare(&mut self, issuer_n: usize) -> Result<(), CertificateRejection> {
        let bound = self.verifier.limits.name_constraint_comparisons;
        if self.work.name_constraint_comparisons >= bound {
            return Err(CertificateRejection::NotTrusted(Check::NameConstraint {
                certificate: issuer_n,
                name: format!("the work of {bound} name comparisons"),
            }));
        }
        self.work.name_constraint_comparisons += 1;
        Ok(())
    }
}

/// The DER of an empty Name: an empty SEQUENCE.
const EMPTY_NAME: &[u8] = &[0x30, 0x00];

/// The context tag of directoryName.
const DIRECTORY: u8 = 0xA4;

/// When both carry one, whether the child's authorityKeyIdentifier is the candidate's
/// subjectKeyIdentifier (item 9.4).
fn key_ids_agree(child: &Certificate<'_>, candidate: &Certificate<'_>) -> bool {
    match (
        child.extensions.authority_key_id,
        candidate.extensions.subject_key_id,
    ) {
        (Some(aki), Some(ski)) => aki == ski,
        _ => true,
    }
}

/// Whether the certificate's issuer and subject are the same name.
fn self_issued(cert: &Certificate<'_>) -> bool {
    cert.subject == cert.issuer
}

/// The context tag of a GeneralName's form.
fn form(name: &GeneralName<'_>) -> u8 {
    match name {
        GeneralName::Dns(_) => 0x82,
        GeneralName::Ip(_) => 0x87,
        GeneralName::Other { tag, .. } => *tag,
    }
}

/// A name as a rejection prints it.
fn name_text(name: &GeneralName<'_>) -> String {
    match name {
        GeneralName::Dns(dns) => (*dns).to_string(),
        GeneralName::Ip(octets) => names::ip_text(octets),
        GeneralName::Other { tag, .. } => format!("a name of the form 0x{tag:02x}"),
    }
}

/// Whether the DNS name `name` lies in the dNSName subtree `base`: equal, or `base` with labels
/// added to its left; the empty subtree holds every name.
fn dns_within(name: &str, base: &str) -> bool {
    if base.is_empty() {
        return true;
    }
    if name.len() == base.len() {
        return name.eq_ignore_ascii_case(base);
    }
    name.len() > base.len()
        && name.as_bytes()[name.len() - base.len() - 1] == b'.'
        && name[name.len() - base.len()..].eq_ignore_ascii_case(base)
}

/// Whether a dNSName subtree is one surf can compare: empty, or a DNS name.
fn dns_base_ok(base: &str) -> bool {
    base.is_empty() || names::is_dns_name(base)
}

/// Whether `name` lies in the permitted subtree `base` of the same form: for a wildcard, every
/// name it can match.
fn within(name: &GeneralName<'_>, base: &GeneralName<'_>) -> bool {
    match (name, base) {
        (GeneralName::Dns(n), GeneralName::Dns(b)) => {
            if !dns_base_ok(b) {
                return false;
            }
            match names::wildcard_suffix(n) {
                Some(suffix) => dns_within(suffix, b),
                None => names::is_dns_name(n) && dns_within(n, b),
            }
        }
        (GeneralName::Ip(ip), GeneralName::Ip(range)) => ip_in_range(ip, range) == Some(true),
        _ => false,
    }
}

/// Whether `name` meets the excluded subtree `base` of the same form: for a wildcard, whether
/// any name it can match lies in it. A subtree surf cannot compare, or a name outside the
/// preferred syntax, counts as met.
fn meets_excluded(name: &GeneralName<'_>, base: &GeneralName<'_>) -> bool {
    match (name, base) {
        (GeneralName::Dns(n), GeneralName::Dns(b)) => {
            if !dns_base_ok(b) {
                return true;
            }
            match names::wildcard_suffix(n) {
                Some(suffix) => {
                    dns_within(suffix, b)
                        || b.split_once('.')
                            .is_some_and(|(_, rest)| rest.eq_ignore_ascii_case(suffix))
                }
                None => !names::is_dns_name(n) || dns_within(n, b),
            }
        }
        (GeneralName::Ip(ip), GeneralName::Ip(range)) => ip_in_range(ip, range) != Some(false),
        _ => false,
    }
}

/// Where the address `ip` (4 or 16 octets) stands to `range` (the address and its mask, 8 or
/// 32 octets): `Some(true)` inside, `Some(false)` outside or of the other family, `None` when
/// the mask's bits are not one run of ones from the top, a range surf cannot compare.
fn ip_in_range(ip: &[u8], range: &[u8]) -> Option<bool> {
    if range.len() != 2 * ip.len() {
        return Some(false);
    }
    let (base, mask) = range.split_at(ip.len());
    let mut ones = true;
    for &m in mask {
        if ones {
            if m != 0xFF {
                if m.leading_ones() + m.trailing_zeros() != 8 {
                    return None;
                }
                ones = false;
            }
        } else if m != 0 {
            return None;
        }
    }
    Some(
        ip.iter()
            .zip(base)
            .zip(mask)
            .all(|((i, b), m)| i & m == b & m),
    )
}
