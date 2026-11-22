//! The cases: for each a root, the certificates a server sends (the leaf first), the host
//! checked and the outcome expected at [`crate::VALIDATION_TIME`] (docs/design/tls.md, items 7,
//! 9 and 10). The names of the negative cases follow badssl.com where it has one
//! (https://github.com/chromium/badssl.com, Apache-2.0). `openssl/make.sh` makes three more,
//! [`crate::OPENSSL_CASES`], which ring cannot sign.

use surf_tls::crypto::Scheme;

use crate::Error;
use crate::cert::{self, CLIENT_AUTH, Date, Issuer, San, Template, issue, key_usage};
use crate::keys::{Key, Keys};

/// The outcome a validation of the case gives: `ok`, or a reason of item 7 and, for
/// `CertificateNotTrusted`, the check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The leaf's key.
    Ok,
    /// `CertificateExpired`: outside the validity, either side.
    Expired,
    /// `CertificateNameMismatch`.
    NameMismatch,
    /// `CertificateNotTrusted` with one check of the closed list of item 7.
    NotTrusted(&'static str),
}

impl Outcome {
    /// The outcome as a line of `INDEX` writes it.
    pub fn index_text(self) -> String {
        match self {
            Outcome::Ok => "ok".into(),
            Outcome::Expired => "CertificateExpired".into(),
            Outcome::NameMismatch => "CertificateNameMismatch".into(),
            Outcome::NotTrusted(check) => format!("CertificateNotTrusted {check}"),
        }
    }
}

/// A case: its name, the host checked and the outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    /// The name, also the directory of its files.
    pub name: &'static str,
    /// The host the leaf is checked against.
    pub host: &'static str,
    /// The outcome expected.
    pub outcome: Outcome,
}

const fn spec(name: &'static str, host: &'static str, outcome: Outcome) -> Spec {
    Spec {
        name,
        host,
        outcome,
    }
}

/// The cases this crate makes, in the order of `INDEX`.
pub fn specs() -> Vec<Spec> {
    use Outcome::{Expired, NameMismatch, NotTrusted, Ok};
    vec![
        spec("ok-rsa2048", "localhost", Ok),
        spec("ok-ecdsa-p256", "localhost", Ok),
        spec("ok-ecdsa-p384", "localhost", Ok),
        spec("ok-ed25519", "localhost", Ok),
        spec("ok-wildcard", "a.example.test", Ok),
        spec("expired", "localhost", Expired),
        spec("not-yet-valid", "localhost", Expired),
        spec("wrong-host", "localhost", NameMismatch),
        spec("untrusted-root", "localhost", NotTrusted("NoPathToAnchor")),
        spec("bad-signature", "localhost", NotTrusted("BadSignature")),
        spec(
            "intermediate-not-ca",
            "localhost",
            NotTrusted("IssuerNotCa"),
        ),
        spec("path-too-long", "localhost", NotTrusted("PathTooLong")),
        spec("pathlen-exceeded", "localhost", NotTrusted("PathTooLong")),
        spec(
            "leaf-no-digital-signature",
            "localhost",
            NotTrusted("KeyUsage"),
        ),
        spec(
            "issuer-no-key-cert-sign",
            "localhost",
            NotTrusted("KeyUsage"),
        ),
        spec("candidate-limit", "localhost", NotTrusted("CandidateLimit")),
        spec(
            "candidate-limit-no-anchor",
            "localhost",
            NotTrusted("CandidateLimit"),
        ),
        spec(
            "unknown-critical-extension",
            "localhost",
            NotTrusted("UnknownCriticalExtension"),
        ),
        spec("wildcard-two-labels", "a.b.example.test", NameMismatch),
        spec("common-name-only", "localhost", NameMismatch),
        spec("ip-mismatch", "127.0.0.1", NameMismatch),
        spec(
            "name-constraints-excluded",
            "localhost",
            NotTrusted("NameConstraint"),
        ),
        spec("eku-client-only", "localhost", NotTrusted("NoServerAuth")),
        spec("1000-sans", "localhost", Ok),
        spec("ok-rsa-pss", "localhost", Ok),
        spec("ok-rsa-pss-sha384", "localhost", Ok),
        spec("ok-rsa-pss-sha512", "localhost", Ok),
        spec("ok-rsa-pkcs1-sha384", "localhost", Ok),
        spec("ok-rsa-pkcs1-sha512", "localhost", Ok),
        spec("leaf-v1", "localhost", NotTrusted("Unreadable")),
    ]
}

/// A case made: the spec, the certificates sent (DER, the leaf first), the root, which is the
/// one trust anchor of the case, and the leaf's private key (PKCS#8 DER).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    /// The spec.
    pub spec: Spec,
    /// The certificates a server sends, the leaf first.
    pub chain: Vec<Vec<u8>>,
    /// The root: the trust anchor a validation of the case is given.
    pub root: Vec<u8>,
    /// The private key of the leaf.
    pub leaf_key: Vec<u8>,
}

/// Every case of [`specs`], made from `keys`.
pub fn build_all(keys: &Keys) -> Result<Vec<Case>, Error> {
    let families = Families::read(keys)?;
    specs()
        .into_iter()
        .map(|spec| {
            let (chain, root, leaf_key) = make(&spec, keys, &families)?;
            Ok(Case {
                spec,
                chain,
                root,
                leaf_key,
            })
        })
        .collect()
}

/// A certificate made, and the issuer it is for the certificates below it.
struct Made {
    der: Vec<u8>,
    issuer: Issuer,
}

fn self_signed(template: Template) -> Result<Made, Error> {
    let issuer = Issuer::itself(&template);
    let der = issue(&template, &issuer)?;
    Ok(Made { der, issuer })
}

fn issued(template: Template, by: &Issuer) -> Result<Made, Error> {
    let der = issue(&template, by)?;
    let issuer = Issuer::itself(&template);
    Ok(Made { der, issuer })
}

/// A root, an intermediate and a leaf key of one type each: the chains of the `ok-*` cases and
/// of the negative cases built on them.
struct Family {
    root: Template,
    intermediate: Template,
    leaf: Key,
}

impl Family {
    fn read(keys: &Keys, label: &str, names: [&str; 3]) -> Result<Family, Error> {
        Ok(Family {
            root: Template::ca(&format!("surf test root {label}"), &keys.key(names[0])?),
            intermediate: Template::ca(
                &format!("surf test intermediate {label}"),
                &keys.key(names[1])?,
            ),
            leaf: keys.key(names[2])?,
        })
    }

    fn root(&self) -> Result<Made, Error> {
        self_signed(self.root.clone())
    }

    /// The root and the intermediate, as the `ok-*` case of the family has them.
    fn root_and_intermediate(&self) -> Result<(Made, Made), Error> {
        let root = self.root()?;
        let intermediate = issued(self.intermediate.clone(), &root.issuer)?;
        Ok((root, intermediate))
    }
}

struct Families {
    rsa: Family,
    p256: Family,
    p384: Family,
    ed25519: Family,
}

impl Families {
    fn read(keys: &Keys) -> Result<Families, Error> {
        Ok(Families {
            rsa: Family::read(keys, "RSA", ["rsa4096", "rsa2048-0", "rsa2048-1"])?,
            p256: Family::read(keys, "P-256", ["p256-00", "p256-01", "p256-02"])?,
            p384: Family::read(keys, "P-384", ["p384-0", "p384-1", "p384-2"])?,
            ed25519: Family::read(keys, "Ed25519", ["ed25519-0", "ed25519-1", "ed25519-2"])?,
        })
    }
}

fn localhost() -> Vec<San> {
    vec![San::Dns("localhost".into()), San::Ip([127, 0, 0, 1])]
}

type Parts = (Vec<Vec<u8>>, Vec<u8>, Vec<u8>);

/// root, intermediate, leaf: the chain sent is the leaf and the intermediate.
fn three(family: &Family, intermediate: Option<Template>, leaf: Template) -> Result<Parts, Error> {
    let root = family.root()?;
    let intermediate = issued(
        intermediate.unwrap_or_else(|| family.intermediate.clone()),
        &root.issuer,
    )?;
    let leaf_key = leaf.key.pkcs8.clone();
    let leaf = issued(leaf, &intermediate.issuer)?;
    Ok((vec![leaf.der, intermediate.der], root.der, leaf_key))
}

/// A leaf of the RSA 2048 intermediate signed in `scheme`.
fn rsa_leaf_in(families: &Families, scheme: Scheme) -> Result<Parts, Error> {
    let family = &families.rsa;
    let (root, intermediate) = family.root_and_intermediate()?;
    let mut by = intermediate.issuer.clone();
    by.scheme = scheme;
    let leaf = Template::leaf("localhost", &family.leaf, localhost());
    let leaf_key = leaf.key.pkcs8.clone();
    let leaf = issue(&leaf, &by)?;
    Ok((vec![leaf, intermediate.der], root.der, leaf_key))
}

fn make(spec: &Spec, keys: &Keys, families: &Families) -> Result<Parts, Error> {
    let p256 = &families.p256;
    let leaf = |sans: Vec<San>| Template::leaf("localhost", &p256.leaf, sans);
    match spec.name {
        "ok-rsa2048" => three(
            &families.rsa,
            None,
            Template::leaf("localhost", &families.rsa.leaf, localhost()),
        ),
        "ok-ecdsa-p256" => three(p256, None, leaf(localhost())),
        "ok-ecdsa-p384" => three(
            &families.p384,
            None,
            Template::leaf("localhost", &families.p384.leaf, localhost()),
        ),
        "ok-ed25519" => three(
            &families.ed25519,
            None,
            Template::leaf("localhost", &families.ed25519.leaf, localhost()),
        ),
        "ok-wildcard" => three(
            p256,
            None,
            Template::leaf(
                "*.example.test",
                &p256.leaf,
                vec![San::Dns("*.example.test".into())],
            ),
        ),
        "expired" => {
            let mut t = leaf(localhost());
            t.validity = (Date::new(2000, 1, 1), Date::new(2001, 1, 1));
            three(p256, None, t)
        }
        "not-yet-valid" => {
            let mut t = leaf(localhost());
            t.validity = (Date::new(2090, 1, 1), cert::VALID.1);
            three(p256, None, t)
        }
        "wrong-host" => three(
            p256,
            None,
            Template::leaf(
                "other.test",
                &p256.leaf,
                vec![San::Dns("other.test".into())],
            ),
        ),
        "untrusted-root" => {
            // The chain ends at a root of its own, sent with it; the anchor is the P-256 root,
            // whose name no certificate of the chain names as its issuer.
            let other = self_signed(Template::ca(
                "surf test untrusted root",
                &keys.key("p256-03")?,
            ))?;
            let intermediate = issued(
                Template::ca(
                    "surf test intermediate under the untrusted root",
                    &p256.intermediate.key,
                ),
                &other.issuer,
            )?;
            let leaf_t = leaf(localhost());
            let leaf_key = leaf_t.key.pkcs8.clone();
            let leaf = issued(leaf_t, &intermediate.issuer)?;
            Ok((
                vec![leaf.der, intermediate.der, other.der],
                p256.root()?.der,
                leaf_key,
            ))
        }
        "bad-signature" => {
            let (mut chain, root, key) = three(p256, None, leaf(localhost()))?;
            chain[0] = cert::flip_signature_bit(chain[0].clone());
            Ok((chain, root, key))
        }
        "intermediate-not-ca" => {
            let mut t = p256.intermediate.clone();
            t.basic_constraints = Some((false, None));
            three(p256, Some(t), leaf(localhost()))
        }
        "path-too-long" => {
            // Seven intermediates, one more than item 9.3 allows.
            let root = p256.root()?;
            let mut sent = Vec::new();
            let mut by = root.issuer.clone();
            for i in 1..=7 {
                let key = keys.key(&format!("p256-{:02}", i + 3))?;
                let made = issued(
                    Template::ca(&format!("surf test intermediate {i} of 7"), &key),
                    &by,
                )?;
                by = made.issuer;
                sent.push(made.der);
            }
            let leaf_t = leaf(localhost());
            let leaf_key = leaf_t.key.pkcs8.clone();
            let leaf = issued(leaf_t, &by)?;
            sent.push(leaf.der);
            sent.reverse();
            Ok((sent, root.der, leaf_key))
        }
        "pathlen-exceeded" => {
            // An intermediate with pathLenConstraint 0 issues a second intermediate, which
            // issues the leaf.
            let root = p256.root()?;
            let mut upper = Template::ca(
                "surf test intermediate with pathLenConstraint 0",
                &keys.key("p256-04")?,
            );
            upper.basic_constraints = Some((true, Some(0)));
            let upper = issued(upper, &root.issuer)?;
            let lower = issued(
                Template::ca(
                    "surf test intermediate below pathLenConstraint 0",
                    &keys.key("p256-05")?,
                ),
                &upper.issuer,
            )?;
            let leaf_t = leaf(localhost());
            let leaf_key = leaf_t.key.pkcs8.clone();
            let leaf = issued(leaf_t, &lower.issuer)?;
            Ok((vec![leaf.der, lower.der, upper.der], root.der, leaf_key))
        }
        "leaf-no-digital-signature" => {
            let mut t = leaf(localhost());
            t.key_usage = Some(vec![key_usage::KEY_ENCIPHERMENT]);
            three(p256, None, t)
        }
        "issuer-no-key-cert-sign" => {
            let mut t = p256.intermediate.clone();
            t.key_usage = Some(vec![key_usage::DIGITAL_SIGNATURE]);
            three(p256, Some(t), leaf(localhost()))
        }
        "candidate-limit" => candidate_limit(keys, p256),
        "candidate-limit-no-anchor" => candidate_limit_no_anchor(keys, p256),
        "unknown-critical-extension" => {
            let mut t = leaf(localhost());
            t.unknown_critical = true;
            three(p256, None, t)
        }
        "wildcard-two-labels" => three(
            p256,
            None,
            Template::leaf(
                "*.example.test",
                &p256.leaf,
                vec![San::Dns("*.example.test".into())],
            ),
        ),
        "common-name-only" => {
            let mut t = leaf(Vec::new());
            t.subject_alt_name = None;
            three(p256, None, t)
        }
        "ip-mismatch" => three(
            p256,
            None,
            Template::leaf("127.0.0.2", &p256.leaf, vec![San::Ip([127, 0, 0, 2])]),
        ),
        "name-constraints-excluded" => {
            let mut t = p256.intermediate.clone();
            t.excluded_dns = Some("localhost".into());
            three(p256, Some(t), leaf(localhost()))
        }
        "eku-client-only" => {
            let mut t = leaf(localhost());
            t.ext_key_usage = Some(vec![CLIENT_AUTH]);
            three(p256, None, t)
        }
        "1000-sans" => {
            // 999 names that do not match, then the one that does.
            let mut sans: Vec<San> = (1..1000)
                .map(|i| San::Dns(format!("name-{i:04}.example.test")))
                .collect();
            sans.push(San::Dns("localhost".into()));
            three(
                p256,
                None,
                Template::leaf("surf test 1000 names", &p256.leaf, sans),
            )
        }
        "ok-rsa-pss" => rsa_leaf_in(families, Scheme::RsaPssRsaeSha256),
        "ok-rsa-pss-sha384" => rsa_leaf_in(families, Scheme::RsaPssRsaeSha384),
        "ok-rsa-pss-sha512" => rsa_leaf_in(families, Scheme::RsaPssRsaeSha512),
        "ok-rsa-pkcs1-sha384" => rsa_leaf_in(families, Scheme::RsaPkcs1Sha384),
        "ok-rsa-pkcs1-sha512" => rsa_leaf_in(families, Scheme::RsaPkcs1Sha512),
        "leaf-v1" => {
            let mut t = leaf(localhost());
            t.version1 = true;
            three(p256, None, t)
        }
        other => Err(Error(format!("no case {other}"))),
    }
}

/// The subject of the 48 intermediates of `candidate-limit`.
const CANDIDATE: &str = "surf test candidate CA";

/// `candidate-limit`: 48 intermediates I0 to I47 of one subject, each with its own P-256 key
/// (`p256-12` to `p256-59`) and that subject as its issuer, without key identifiers, so that
/// every one matches every issuer name. I(n) is signed by the key of I(n+1) for n up to 45; I46
/// and I47 sign each other, a loop. The leaf is signed by the key of I0. The 50th certificate,
/// a bridge with the key of I2 issued by the P-256 root, makes a path of three intermediates
/// (I0, I1, the bridge) to the anchor. Sent: the leaf, I47 down to I0, the bridge. A search in
/// the order of item 9.4 checks 48 signatures for the leaf's issuer (I0 last), 47 for I0's (I1
/// last), and meets the limit of 100 five checks into the level of I1, before it reaches the
/// bridge.
fn candidate_limit(keys: &Keys, p256: &Family) -> Result<Parts, Error> {
    let root = p256.root()?;
    let keys: Vec<Key> = (12..60)
        .map(|i| keys.key(&format!("p256-{i:02}")))
        .collect::<Result<_, _>>()?;
    let template = |n: usize| {
        let mut t = Template::ca(CANDIDATE, &keys[n]);
        t.key_ids = false;
        t
    };
    let issuer = |n: usize| Issuer {
        name: CANDIDATE.into(),
        key: keys[n].clone(),
        scheme: keys[n].scheme,
        key_id: false,
    };
    let mut intermediates = Vec::new();
    for n in 0..48 {
        let signer = match n {
            46 => 47,
            47 => 46,
            n => n + 1,
        };
        intermediates.push(issue(&template(n), &issuer(signer))?);
    }
    let mut bridge = template(2);
    bridge.serial_label = "bridge".into();
    let bridge = issue(&bridge, &root.issuer)?;
    let mut leaf = Template::leaf("localhost", &p256.leaf, localhost());
    leaf.key_ids = false;
    let leaf_key = leaf.key.pkcs8.clone();
    let leaf = issue(&leaf, &issuer(0))?;
    let mut sent = vec![leaf];
    sent.extend(intermediates.into_iter().rev());
    sent.push(bridge);
    Ok((sent, root.der, leaf_key))
}

/// `candidate-limit-no-anchor` (docs/design/tls.md item 10.4): 48 intermediates of one subject
/// and one P-256 key (`p256-11`), each issued by that subject and signed with that key, and a
/// leaf issued by the same subject. Every candidate matches and verifies, and no path reaches
/// the anchor, the P-256 root: the limit of 100 signature checks ends the search.
fn candidate_limit_no_anchor(keys: &Keys, p256: &Family) -> Result<Parts, Error> {
    const LOOP: &str = "surf test loop CA";
    let key = keys.key("p256-11")?;
    let issuer = Issuer {
        name: LOOP.into(),
        key: key.clone(),
        scheme: key.scheme,
        key_id: false,
    };
    let mut leaf = Template::leaf("localhost", &p256.leaf, localhost());
    leaf.key_ids = false;
    let leaf_key = leaf.key.pkcs8.clone();
    let mut sent = vec![issue(&leaf, &issuer)?];
    for n in 0..48 {
        let mut t = Template::ca(LOOP, &key);
        t.key_ids = false;
        t.serial_label = format!("loop {n}");
        sent.push(issue(&t, &issuer)?);
    }
    Ok((sent, p256.root()?.der, leaf_key))
}
