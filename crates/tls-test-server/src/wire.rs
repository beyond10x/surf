//! The bytes of the handshake messages the server reads and writes (RFC 8446 section 4): the
//! ClientHello read, and the server's own messages written. Written here, not taken from
//! `surf-tls`, whose handshake module is the client's (docs/design/tls.md item 6).

use surf_tls::crypto::{self, HashAlgorithm};

/// The handshake types the server meets.
pub mod kind {
    /// `client_hello`.
    pub const CLIENT_HELLO: u8 = 1;
    /// `server_hello`, also a HelloRetryRequest.
    pub const SERVER_HELLO: u8 = 2;
    /// `new_session_ticket`.
    pub const NEW_SESSION_TICKET: u8 = 4;
    /// `encrypted_extensions`.
    pub const ENCRYPTED_EXTENSIONS: u8 = 8;
    /// `certificate`.
    pub const CERTIFICATE: u8 = 11;
    /// `certificate_verify`.
    pub const CERTIFICATE_VERIFY: u8 = 15;
    /// `finished`.
    pub const FINISHED: u8 = 20;
}

/// The extension types the server reads or writes.
pub mod ext {
    /// `server_name`.
    pub const SERVER_NAME: u16 = 0;
    /// `supported_groups`.
    pub const SUPPORTED_GROUPS: u16 = 10;
    /// `signature_algorithms`.
    pub const SIGNATURE_ALGORITHMS: u16 = 13;
    /// `application_layer_protocol_negotiation`.
    pub const ALPN: u16 = 16;
    /// `supported_versions`.
    pub const SUPPORTED_VERSIONS: u16 = 43;
    /// `key_share`.
    pub const KEY_SHARE: u16 = 51;
}

/// TLS 1.3.
pub const TLS13: u16 = 0x0304;
/// TLS 1.2, the `legacy_version` of every hello.
pub const TLS12: u16 = 0x0303;
/// TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256, the suite of kind (c).
pub const TLS12_ECDHE_RSA_AES128_GCM: u16 = 0xc02f;
/// The suites a server limited to TLS 1.2 holds (kind (b)): the ECDHE GCM and ChaCha20 suites
/// of RFC 5289 and RFC 7905.
pub const TLS12_SUITES: [u16; 6] = [0xc02b, 0xc02c, 0xc02f, 0xc030, 0xcca8, 0xcca9];

/// A message that does not parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Malformed;

/// A reader over a slice that never reads past it.
pub struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A reader of `bytes`.
    pub fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes }
    }

    /// Whether every byte is read.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The next `n` bytes.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Malformed> {
        if n > self.bytes.len() {
            return Err(Malformed);
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }

    /// One byte.
    pub fn u8(&mut self) -> Result<u8, Malformed> {
        Ok(self.take(1)?[0])
    }

    /// A big-endian u16.
    pub fn u16(&mut self) -> Result<u16, Malformed> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    /// A vector with a one-byte length.
    pub fn vec8(&mut self) -> Result<&'a [u8], Malformed> {
        let n = usize::from(self.u8()?);
        self.take(n)
    }

    /// A vector with a two-byte length.
    pub fn vec16(&mut self) -> Result<&'a [u8], Malformed> {
        let n = usize::from(self.u16()?);
        self.take(n)
    }
}

/// Appends `data` with a one-byte length.
pub fn put_vec8(out: &mut Vec<u8>, data: &[u8]) {
    out.push(u8::try_from(data.len()).unwrap_or(u8::MAX));
    out.extend_from_slice(data);
}

/// Appends `data` with a two-byte length.
pub fn put_vec16(out: &mut Vec<u8>, data: &[u8]) {
    out.extend_from_slice(&u16::try_from(data.len()).unwrap_or(u16::MAX).to_be_bytes());
    out.extend_from_slice(data);
}

/// Appends `data` with a three-byte length.
pub fn put_vec24(out: &mut Vec<u8>, data: &[u8]) {
    out.extend_from_slice(&u24(data.len()));
    out.extend_from_slice(data);
}

/// The three bytes of a uint24.
pub fn u24(n: usize) -> [u8; 3] {
    let n = u32::try_from(n.min(0xff_ffff)).unwrap_or(0xff_ffff);
    let b = n.to_be_bytes();
    [b[1], b[2], b[3]]
}

/// A handshake message: its type, the uint24 length and the body.
pub fn message(msg_type: u8, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + body.len());
    out.push(msg_type);
    put_vec24(&mut out, body);
    out
}

/// Appends one extension.
pub fn put_extension(out: &mut Vec<u8>, extension_type: u16, data: &[u8]) {
    out.extend_from_slice(&extension_type.to_be_bytes());
    put_vec16(out, data);
}

/// What the server reads of a ClientHello.
#[derive(Clone, Debug, Default)]
pub struct ClientHello {
    /// `legacy_version`.
    pub legacy_version: u16,
    /// `legacy_session_id`, echoed; at most 32 bytes (RFC 8446 section 4.1.2).
    pub session_id: Vec<u8>,
    /// `legacy_compression_methods`, as sent; a TLS 1.3 hello holds the one byte 0.
    pub compression: Vec<u8>,
    /// The cipher suites, in order.
    pub suites: Vec<u16>,
    /// The `server_name` of type `host_name`, if sent.
    pub server_name: Option<String>,
    /// `supported_groups`.
    pub groups: Vec<u16>,
    /// `signature_algorithms`.
    pub signature_algorithms: Vec<u16>,
    /// The ALPN protocols, in order.
    pub alpn: Vec<String>,
    /// `supported_versions`; empty without the extension.
    pub versions: Vec<u16>,
    /// The key shares: group and public key.
    pub shares: Vec<(u16, Vec<u8>)>,
}

fn u16_list(data: &[u8]) -> Result<Vec<u16>, Malformed> {
    if !data.len().is_multiple_of(2) {
        return Err(Malformed);
    }
    Ok(data
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect())
}

/// Reads a ClientHello message, its 4-byte header included.
pub fn client_hello(message: &[u8]) -> Result<ClientHello, Malformed> {
    if message.first() != Some(&kind::CLIENT_HELLO) || message.len() < 4 {
        return Err(Malformed);
    }
    let mut r = Reader::new(&message[4..]);
    let mut hello = ClientHello {
        legacy_version: r.u16()?,
        ..ClientHello::default()
    };
    r.take(32)?;
    hello.session_id = r.vec8()?.to_vec();
    if hello.session_id.len() > 32 {
        return Err(Malformed);
    }
    hello.suites = u16_list(r.vec16()?)?;
    hello.compression = r.vec8()?.to_vec();
    let mut extensions = Reader::new(r.vec16()?);
    if !r.is_empty() {
        return Err(Malformed);
    }
    while !extensions.is_empty() {
        let extension_type = extensions.u16()?;
        let data = extensions.vec16()?;
        let mut d = Reader::new(data);
        match extension_type {
            ext::SERVER_NAME => {
                let mut list = Reader::new(d.vec16()?);
                while !list.is_empty() {
                    let name_type = list.u8()?;
                    let name = list.vec16()?;
                    if name_type == 0 {
                        hello.server_name = Some(String::from_utf8_lossy(name).into_owned());
                    }
                }
            }
            ext::SUPPORTED_GROUPS => hello.groups = u16_list(d.vec16()?)?,
            ext::SIGNATURE_ALGORITHMS => hello.signature_algorithms = u16_list(d.vec16()?)?,
            ext::ALPN => {
                let mut list = Reader::new(d.vec16()?);
                while !list.is_empty() {
                    let protocol = list.vec8()?;
                    hello
                        .alpn
                        .push(String::from_utf8_lossy(protocol).into_owned());
                }
            }
            ext::SUPPORTED_VERSIONS => hello.versions = u16_list(d.vec8()?)?,
            ext::KEY_SHARE => {
                let mut list = Reader::new(d.vec16()?);
                while !list.is_empty() {
                    let group = list.u16()?;
                    let key = list.vec16()?;
                    hello.shares.push((group, key.to_vec()));
                }
            }
            _ => {}
        }
    }
    Ok(hello)
}

/// The random of a HelloRetryRequest (RFC 8446 section 4.1.3): SHA-256 of "HelloRetryRequest".
pub fn hello_retry_request_random() -> [u8; 32] {
    let digest = crypto::hash(HashAlgorithm::Sha256, b"HelloRetryRequest");
    let mut out = [0u8; 32];
    out.copy_from_slice(digest.as_ref());
    out
}

/// A ServerHello or HelloRetryRequest: `legacy_version` 0x0303, the random, the echoed
/// session id, the suite, null compression, and `extensions` (none at all when `None`, as a
/// TLS 1.2 server that negotiated nothing writes it).
pub fn server_hello(
    random: &[u8; 32],
    session_id: &[u8],
    suite: u16,
    extensions: Option<&[u8]>,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(128);
    body.extend_from_slice(&TLS12.to_be_bytes());
    body.extend_from_slice(random);
    put_vec8(&mut body, session_id);
    body.extend_from_slice(&suite.to_be_bytes());
    body.push(0);
    if let Some(extensions) = extensions {
        put_vec16(&mut body, extensions);
    }
    message(kind::SERVER_HELLO, &body)
}

/// The extensions of a TLS 1.3 ServerHello: `supported_versions` 0x0304 and the key share.
pub fn server_hello_extensions(group: u16, public_key: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    put_extension(&mut out, ext::SUPPORTED_VERSIONS, &TLS13.to_be_bytes());
    let mut share = group.to_be_bytes().to_vec();
    put_vec16(&mut share, public_key);
    put_extension(&mut out, ext::KEY_SHARE, &share);
    out
}

/// The extensions of a HelloRetryRequest: `supported_versions` 0x0304 and the group asked for.
pub fn hello_retry_request_extensions(group: u16) -> Vec<u8> {
    let mut out = Vec::new();
    put_extension(&mut out, ext::SUPPORTED_VERSIONS, &TLS13.to_be_bytes());
    put_extension(&mut out, ext::KEY_SHARE, &group.to_be_bytes());
    out
}

/// EncryptedExtensions: an empty `server_name` when the hello named one, and the ALPN
/// protocol selected, if any.
pub fn encrypted_extensions(server_name: bool, alpn: Option<&str>) -> Vec<u8> {
    let mut extensions = Vec::new();
    if server_name {
        put_extension(&mut extensions, ext::SERVER_NAME, &[]);
    }
    if let Some(protocol) = alpn {
        let mut name = Vec::new();
        put_vec8(&mut name, protocol.as_bytes());
        let mut list = Vec::new();
        put_vec16(&mut list, &name);
        put_extension(&mut extensions, ext::ALPN, &list);
    }
    let mut body = Vec::new();
    put_vec16(&mut body, &extensions);
    message(kind::ENCRYPTED_EXTENSIONS, &body)
}

/// A TLS 1.3 Certificate of `chain`, the leaf first, with an empty request context and no
/// extensions on any entry.
pub fn certificate(chain: &[Vec<u8>]) -> Vec<u8> {
    let mut list = Vec::new();
    for cert in chain {
        put_vec24(&mut list, cert);
        put_vec16(&mut list, &[]);
    }
    let mut body = vec![0u8];
    put_vec24(&mut body, &list);
    message(kind::CERTIFICATE, &body)
}

/// The content a server's CertificateVerify signs (RFC 8446 section 4.4.3).
pub fn certificate_verify_content(transcript_hash: &[u8]) -> Vec<u8> {
    let context = b"TLS 1.3, server CertificateVerify";
    let mut content = Vec::with_capacity(64 + context.len() + 1 + transcript_hash.len());
    content.extend_from_slice(&[0x20; 64]);
    content.extend_from_slice(context);
    content.push(0);
    content.extend_from_slice(transcript_hash);
    content
}

/// CertificateVerify: the scheme and the signature.
pub fn certificate_verify(scheme: u16, signature: &[u8]) -> Vec<u8> {
    let mut body = scheme.to_be_bytes().to_vec();
    put_vec16(&mut body, signature);
    message(kind::CERTIFICATE_VERIFY, &body)
}

/// One NewSessionTicket (RFC 8446 section 4.6.1): a lifetime of an hour, `ticket` as both the
/// nonce's source and the ticket, no extensions. The client keeps none (no resumption).
pub fn new_session_ticket(age_add: u32, ticket: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&3600u32.to_be_bytes());
    body.extend_from_slice(&age_add.to_be_bytes());
    put_vec8(&mut body, &[0]);
    put_vec16(&mut body, ticket);
    put_vec16(&mut body, &[]);
    message(kind::NEW_SESSION_TICKET, &body)
}
