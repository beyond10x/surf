//! The private keys of `keys/`, PKCS#8 DER files made once by `openssl genpkey` (`keys/SOURCE`)
//! and never regenerated.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use surf_tls::crypto::{PublicKeyKind, Random, Scheme, SigningKey};

use crate::Error;

/// The key files of a directory, by name without `.pk8`.
#[derive(Debug)]
pub struct Keys {
    files: BTreeMap<String, Vec<u8>>,
}

impl Keys {
    /// Reads every `*.pk8` file of `dir`.
    pub fn read(dir: &Path) -> Result<Keys, Error> {
        let mut files = BTreeMap::new();
        let entries = fs::read_dir(dir).map_err(|e| Error(format!("{}: {e}", dir.display())))?;
        for entry in entries {
            let path = entry.map_err(|e| Error(e.to_string()))?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("pk8") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| Error(format!("{}: no name", path.display())))?
                .to_string();
            let bytes = fs::read(&path).map_err(|e| Error(format!("{}: {e}", path.display())))?;
            files.insert(name, bytes);
        }
        Ok(Keys { files })
    }

    /// The key of `name` (`p256-00`, `rsa2048-1`, ...), read through the seam.
    pub fn key(&self, name: &str) -> Result<Key, Error> {
        let pkcs8 = self
            .files
            .get(name)
            .ok_or_else(|| Error(format!("keys/{name}.pk8 is missing")))?
            .clone();
        let scheme = if name.starts_with("rsa") {
            Scheme::RsaPkcs1Sha256
        } else if name.starts_with("p256-") {
            Scheme::EcdsaSecp256r1Sha256
        } else if name.starts_with("p384-") {
            Scheme::EcdsaSecp384r1Sha384
        } else if name.starts_with("ed25519-") {
            Scheme::Ed25519
        } else {
            return Err(Error(format!("keys/{name}.pk8: no scheme for its name")));
        };
        let random = fixed_random();
        let signer = SigningKey::from_pkcs8(scheme, &pkcs8, &random)
            .map_err(|e| Error(format!("keys/{name}.pk8: {e}")))?;
        let public = signer.public_key();
        Ok(Key {
            name: name.to_string(),
            kind: public.kind,
            public: public.bytes.to_vec(),
            pkcs8,
            scheme,
        })
    }
}

/// The fixed random source every key and signature of this crate draws from, so that the same
/// key signs the same bytes every run (docs/design/tls.md, item 5).
pub fn fixed_random() -> Random<'static> {
    Random::fixed_byte(0x5a)
}

/// A key: its PKCS#8 bytes, its public key and the scheme it signs in by default.
#[derive(Clone, Debug)]
pub struct Key {
    /// The file name without `.pk8`.
    pub name: String,
    /// The kind of the public key.
    pub kind: PublicKeyKind,
    /// The public key in the form of [`PublicKeyKind`].
    pub public: Vec<u8>,
    /// The PKCS#8 file.
    pub pkcs8: Vec<u8>,
    /// The scheme an issuer with this key signs in unless a case names another.
    pub scheme: Scheme,
}

impl Key {
    /// The signature of `message` in `scheme`, under the fixed source.
    pub fn sign(&self, scheme: Scheme, message: &[u8]) -> Result<Vec<u8>, Error> {
        let random = fixed_random();
        let signer = SigningKey::from_pkcs8(scheme, &self.pkcs8, &random)
            .map_err(|e| Error(format!("keys/{}.pk8 in {}: {e}", self.name, scheme.name())))?;
        signer.sign(message, &random).map_err(|e| {
            Error(format!(
                "keys/{}.pk8 signs in {}: {e}",
                self.name,
                scheme.name()
            ))
        })
    }
}
