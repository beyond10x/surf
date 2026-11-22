//! The handshake messages of RFC 8446 section 4 that the client writes and reads: the
//! ClientHello it builds, the facts it offered (read back from the hello's own bytes, so a
//! hello given by a trace is read the same way), the extension table of section 4.2, and the
//! parsers of what a server sends. Every parser reads a hostile message: its fields stay
//! inside the message, the message ends with its last field, and an extension block holds no
//! type twice, decided in linear work.

use crate::crypto::{self, Group, HashAlgorithm};
use crate::error::{AlertDescription, Violation};
use crate::key_schedule::CipherSuite;

use super::codec::{self, Malformed, Reader, TooLong};

/// The handshake types of RFC 8446 section 4 the client meets.
pub(crate) mod kind {
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
    /// `certificate_request`.
    pub const CERTIFICATE_REQUEST: u8 = 13;
    /// `certificate_verify`.
    pub const CERTIFICATE_VERIFY: u8 = 15;
    /// `finished`.
    pub const FINISHED: u8 = 20;
    /// `key_update`.
    pub const KEY_UPDATE: u8 = 24;
}

/// The extension types of RFC 8446 section 4.2, and two the client reads beside them.
pub(crate) mod ext {
    /// `server_name` (RFC 6066).
    pub const SERVER_NAME: u16 = 0;
    /// `max_fragment_length` (RFC 6066).
    pub const MAX_FRAGMENT_LENGTH: u16 = 1;
    /// `status_request` (RFC 6066).
    pub const STATUS_REQUEST: u16 = 5;
    /// `supported_groups` (RFC 8422, 7919).
    pub const SUPPORTED_GROUPS: u16 = 10;
    /// `signature_algorithms` (RFC 8446).
    pub const SIGNATURE_ALGORITHMS: u16 = 13;
    /// `use_srtp` (RFC 5764).
    pub const USE_SRTP: u16 = 14;
    /// `heartbeat` (RFC 6520).
    pub const HEARTBEAT: u16 = 15;
    /// `application_layer_protocol_negotiation` (RFC 7301).
    pub const ALPN: u16 = 16;
    /// `signed_certificate_timestamp` (RFC 6962).
    pub const SIGNED_CERTIFICATE_TIMESTAMP: u16 = 18;
    /// `client_certificate_type` (RFC 7250).
    pub const CLIENT_CERTIFICATE_TYPE: u16 = 19;
    /// `server_certificate_type` (RFC 7250).
    pub const SERVER_CERTIFICATE_TYPE: u16 = 20;
    /// `padding` (RFC 7685).
    pub const PADDING: u16 = 21;
    /// `pre_shared_key` (RFC 8446).
    pub const PRE_SHARED_KEY: u16 = 41;
    /// `early_data` (RFC 8446).
    pub const EARLY_DATA: u16 = 42;
    /// `supported_versions` (RFC 8446).
    pub const SUPPORTED_VERSIONS: u16 = 43;
    /// `cookie` (RFC 8446).
    pub const COOKIE: u16 = 44;
    /// `psk_key_exchange_modes` (RFC 8446).
    pub const PSK_KEY_EXCHANGE_MODES: u16 = 45;
    /// `certificate_authorities` (RFC 8446).
    pub const CERTIFICATE_AUTHORITIES: u16 = 47;
    /// `oid_filters` (RFC 8446).
    pub const OID_FILTERS: u16 = 48;
    /// `post_handshake_auth` (RFC 8446).
    pub const POST_HANDSHAKE_AUTH: u16 = 49;
    /// `signature_algorithms_cert` (RFC 8446).
    pub const SIGNATURE_ALGORITHMS_CERT: u16 = 50;
    /// `key_share` (RFC 8446).
    pub const KEY_SHARE: u16 = 51;
}

/// The message an extension stands in, as the table of RFC 8446 section 4.2 names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    /// CH.
    ClientHello,
    /// SH.
    ServerHello,
    /// HRR.
    HelloRetryRequest,
    /// EE.
    EncryptedExtensions,
    /// CR.
    CertificateRequest,
    /// CT.
    Certificate,
    /// NST.
    NewSessionTicket,
}

/// The messages the table of RFC 8446 section 4.2 allows `extension` in; `None` for a type the
/// table does not name (an unknown one, or one of another RFC such as `record_size_limit`).
pub(crate) fn allowed_in(extension: u16) -> Option<&'static [Place]> {
    use Place::{
        Certificate as CT, CertificateRequest as CR, ClientHello as CH, EncryptedExtensions as EE,
        HelloRetryRequest as HRR, NewSessionTicket as NST, ServerHello as SH,
    };
    let places: &'static [Place] = match extension {
        ext::SERVER_NAME
        | ext::MAX_FRAGMENT_LENGTH
        | ext::SUPPORTED_GROUPS
        | ext::USE_SRTP
        | ext::HEARTBEAT
        | ext::ALPN
        | ext::CLIENT_CERTIFICATE_TYPE
        | ext::SERVER_CERTIFICATE_TYPE => &[CH, EE],
        ext::STATUS_REQUEST | ext::SIGNED_CERTIFICATE_TIMESTAMP => &[CH, CR, CT],
        ext::SIGNATURE_ALGORITHMS
        | ext::CERTIFICATE_AUTHORITIES
        | ext::SIGNATURE_ALGORITHMS_CERT => &[CH, CR],
        ext::PADDING | ext::PSK_KEY_EXCHANGE_MODES | ext::POST_HANDSHAKE_AUTH => &[CH],
        ext::KEY_SHARE | ext::SUPPORTED_VERSIONS => &[CH, SH, HRR],
        ext::PRE_SHARED_KEY => &[CH, SH],
        ext::EARLY_DATA => &[CH, EE, NST],
        ext::COOKIE => &[CH, HRR],
        ext::OID_FILTERS => &[CR],
        _ => return None,
    };
    Some(places)
}

/// A refusal of the server's handshake: the alert to send and the violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Abort {
    /// The alert.
    pub alert: AlertDescription,
    /// The violation.
    pub violation: Violation,
}

impl Abort {
    pub(crate) fn new(alert: AlertDescription, violation: Violation) -> Self {
        Abort { alert, violation }
    }

    /// `decode_error`, `Malformed { message }`.
    pub(crate) fn malformed(message: &'static str) -> Self {
        Abort::new(AlertDescription::DECODE_ERROR, Malformed::of(message))
    }

    /// `illegal_parameter` with `violation`.
    pub(crate) fn illegal(violation: Violation) -> Self {
        Abort::new(AlertDescription::ILLEGAL_PARAMETER, violation)
    }
}

/// The version of TLS 1.3 (RFC 8446 section 4.2.1).
pub(crate) const TLS13: u16 = 0x0304;
/// The `legacy_version` of a ClientHello and of a TLS 1.2 ServerHello.
pub(crate) const TLS12: u16 = 0x0303;

/// The random of a HelloRetryRequest (RFC 8446 section 4.1.3): SHA-256 of
/// "HelloRetryRequest".
pub(crate) fn hello_retry_request_random() -> [u8; 32] {
    let digest = crypto::hash(HashAlgorithm::Sha256, b"HelloRetryRequest");
    let mut out = [0u8; 32];
    out.copy_from_slice(digest.as_ref());
    out
}

/// The last 8 bytes of a ServerHello random that negotiates TLS 1.2 (`DOWNGRD` and 0x01) or
/// below (`DOWNGRD` and 0x00), RFC 8446 section 4.1.3.
pub(crate) const DOWNGRADE_TLS12: [u8; 8] = [0x44, 0x4f, 0x57, 0x4e, 0x47, 0x52, 0x44, 0x01];
/// See [`DOWNGRADE_TLS12`].
pub(crate) const DOWNGRADE_TLS11: [u8; 8] = [0x44, 0x4f, 0x57, 0x4e, 0x47, 0x52, 0x44, 0x00];

/// The code point of a group in `supported_groups` and `key_share` (RFC 8446 section 4.2.7).
pub(crate) fn group_code(group: Group) -> u16 {
    match group {
        Group::X25519 => 0x001d,
        Group::Secp256r1 => 0x0017,
    }
}

/// The group of a code point, if the seam agrees in it.
pub(crate) fn group_of(code: u16) -> Option<Group> {
    match code {
        0x001d => Some(Group::X25519),
        0x0017 => Some(Group::Secp256r1),
        _ => None,
    }
}

/// The groups of `supported_groups`, in the order offered: x25519, then secp256r1.
pub(crate) const GROUPS: [Group; 2] = [Group::X25519, Group::Secp256r1];

/// The schemes of `signature_algorithms` (docs/design/tls.md item 14): CertificateVerify may
/// use one of these.
pub(crate) const SIGNATURE_ALGORITHMS: [u16; 6] = [0x0403, 0x0503, 0x0807, 0x0804, 0x0805, 0x0806];

/// The schemes of `signature_algorithms_cert` (item 14): the six above and RSA PKCS#1.
pub(crate) const SIGNATURE_ALGORITHMS_CERT: [u16; 9] = [
    0x0403, 0x0503, 0x0807, 0x0804, 0x0805, 0x0806, 0x0401, 0x0501, 0x0601,
];

/// The one ALPN protocol offered.
pub(crate) const HTTP11: &[u8] = b"http/1.1";

/// The longest name surf sends in `server_name`: a DNS name is at most 255 bytes.
pub(crate) const MAX_SERVER_NAME: usize = 255;

/// The most bytes the extensions of a second ClientHello other than `cookie` take, each with
/// its 4-byte header: `server_name` of the longest name (264), `supported_groups` (10),
/// `signature_algorithms` (18), `signature_algorithms_cert` (24), ALPN (15),
/// `supported_versions` (7) and `key_share` of one secp256r1 share of 65 bytes (75): 413.
const MAX_OTHER_EXTENSIONS: usize = (4 + 2 + 1 + 2 + MAX_SERVER_NAME)
    + (4 + 2 + 2 * GROUPS.len())
    + (4 + 2 + 2 * SIGNATURE_ALGORITHMS.len())
    + (4 + 2 + 2 * SIGNATURE_ALGORITHMS_CERT.len())
    + (4 + 2 + 1 + HTTP11.len())
    + (4 + 1 + 2)
    + (4 + 2 + 2 + 2 + 65);

/// The longest cookie of a HelloRetryRequest surf takes: the second ClientHello echoes it in a
/// `cookie` extension (4 + 2 bytes around it) beside the other extensions, all inside
/// `extensions<8..2^16-1>`, so 65,535 - 413 - 6 = 65,116 bytes. A longer one is refused with
/// `illegal_parameter` before any second hello is built.
pub(crate) const MAX_COOKIE: usize = 0xffff - MAX_OTHER_EXTENSIONS - (4 + 2);

/// Whether `host` is an IP literal (RFC 6066 section 3 allows none in `server_name`): an IPv4
/// or IPv6 address, the latter also in brackets.
pub(crate) fn is_ip_literal(host: &str) -> bool {
    let bare = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    bare.parse::<std::net::IpAddr>().is_ok()
}

/// The `server_name` of `host`: none for an IP literal or an empty host, else the name
/// without a trailing dot (RFC 6066 section 3).
pub(crate) fn server_name(host: &str) -> Option<&str> {
    let name = host.strip_suffix('.').unwrap_or(host);
    if name.is_empty() || is_ip_literal(host) {
        None
    } else {
        Some(name)
    }
}

/// What surf puts in a ClientHello.
pub(crate) struct HelloParts<'a> {
    /// The random, 32 bytes.
    pub random: &'a [u8; 32],
    /// The legacy session id, 32 bytes (compatibility mode).
    pub session_id: &'a [u8; 32],
    /// The `server_name`, at most 255 bytes, or none.
    pub server_name: Option<&'a str>,
    /// The key shares, each its group and public key.
    pub shares: &'a [(Group, &'a [u8])],
    /// The cookie of a HelloRetryRequest, in the second hello.
    pub cookie: Option<&'a [u8]>,
}

fn extension(out: &mut Vec<u8>, extension_type: u16, data: &[u8]) -> Result<(), TooLong> {
    out.extend_from_slice(&extension_type.to_be_bytes());
    codec::put_vec16(out, data)
}

/// The ClientHello of RFC 8446 section 4.1.2 with its 4-byte header: `legacy_version`
/// 0x0303, the random, the session id, the three TLS 1.3 suites, null compression, and the
/// extensions `server_name` (when given), `supported_groups`, `signature_algorithms`,
/// `signature_algorithms_cert`, ALPN `http/1.1`, `supported_versions` 0x0304, `key_share`, and
/// `cookie` (when given), in this order.
pub(crate) fn client_hello(parts: &HelloParts<'_>) -> Result<Vec<u8>, TooLong> {
    let mut body = Vec::with_capacity(512);
    body.extend_from_slice(&TLS12.to_be_bytes());
    body.extend_from_slice(parts.random);
    codec::put_vec8(&mut body, parts.session_id)?;
    let mut suites = Vec::with_capacity(6);
    for suite in CipherSuite::ALL {
        suites.extend_from_slice(&suite.code().to_be_bytes());
    }
    codec::put_vec16(&mut body, &suites)?;
    codec::put_vec8(&mut body, &[0])?;

    let mut extensions = Vec::with_capacity(400);
    if let Some(name) = parts.server_name {
        let mut entry = vec![0u8];
        codec::put_vec16(&mut entry, name.as_bytes())?;
        let mut list = Vec::with_capacity(entry.len() + 2);
        codec::put_vec16(&mut list, &entry)?;
        extension(&mut extensions, ext::SERVER_NAME, &list)?;
    }
    let mut groups = Vec::new();
    for group in GROUPS {
        groups.extend_from_slice(&group_code(group).to_be_bytes());
    }
    let mut list = Vec::new();
    codec::put_vec16(&mut list, &groups)?;
    extension(&mut extensions, ext::SUPPORTED_GROUPS, &list)?;
    for (extension_type, schemes) in [
        (ext::SIGNATURE_ALGORITHMS, &SIGNATURE_ALGORITHMS[..]),
        (
            ext::SIGNATURE_ALGORITHMS_CERT,
            &SIGNATURE_ALGORITHMS_CERT[..],
        ),
    ] {
        let mut codes = Vec::new();
        for scheme in schemes {
            codes.extend_from_slice(&scheme.to_be_bytes());
        }
        let mut list = Vec::new();
        codec::put_vec16(&mut list, &codes)?;
        extension(&mut extensions, extension_type, &list)?;
    }
    let mut name = Vec::new();
    codec::put_vec8(&mut name, HTTP11)?;
    let mut list = Vec::new();
    codec::put_vec16(&mut list, &name)?;
    extension(&mut extensions, ext::ALPN, &list)?;
    let mut versions = Vec::new();
    codec::put_vec8(&mut versions, &TLS13.to_be_bytes())?;
    extension(&mut extensions, ext::SUPPORTED_VERSIONS, &versions)?;
    let mut shares = Vec::new();
    for (group, public) in parts.shares {
        shares.extend_from_slice(&group_code(*group).to_be_bytes());
        codec::put_vec16(&mut shares, public)?;
    }
    let mut list = Vec::new();
    codec::put_vec16(&mut list, &shares)?;
    extension(&mut extensions, ext::KEY_SHARE, &list)?;
    if let Some(cookie) = parts.cookie {
        let mut value = Vec::new();
        codec::put_vec16(&mut value, cookie)?;
        extension(&mut extensions, ext::COOKIE, &value)?;
    }
    codec::put_vec16(&mut body, &extensions)?;
    codec::handshake_message(kind::CLIENT_HELLO, &body)
}

/// What a ClientHello offered, read from its bytes.
#[derive(Clone, Debug, Default)]
pub(crate) struct Offered {
    /// The legacy session id.
    pub session_id: Vec<u8>,
    /// The cipher suites.
    pub suites: Vec<u16>,
    /// The extension types.
    pub extensions: Vec<u16>,
    /// The groups of `supported_groups`.
    pub groups: Vec<u16>,
    /// The groups of the key shares, in their order.
    pub shares: Vec<u16>,
    /// The schemes of `signature_algorithms`.
    pub signature_algorithms: Vec<u16>,
    /// The ALPN protocols.
    pub alpn: Vec<Vec<u8>>,
}

impl Offered {
    /// Whether the hello offered `extension`.
    pub(crate) fn offers(&self, extension: u16) -> bool {
        self.extensions.contains(&extension)
    }
}

fn u16_list(data: &[u8], min: usize) -> Result<Vec<u16>, Malformed> {
    let mut reader = Reader::new(data);
    let list = reader.vec16(min, 0xffff)?;
    reader.finish()?;
    if list.len() % 2 != 0 {
        return Err(Malformed);
    }
    Ok(list
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect())
}

/// What the ClientHello `message` (with its header) offered. Our own hellos and the given
/// hellos of a trace are read so; a hello that does not parse is refused.
pub(crate) fn offered(message: &[u8]) -> Result<Offered, Malformed> {
    let mut reader = Reader::new(message);
    if reader.u8()? != kind::CLIENT_HELLO {
        return Err(Malformed);
    }
    let body = reader.vec24(0, 0xff_ffff)?;
    reader.finish()?;
    let mut reader = Reader::new(body);
    reader.u16()?;
    reader.take(32)?;
    let session_id = reader.vec8(0, 32)?.to_vec();
    let suites = reader.vec16(2, 0xfffe)?;
    if suites.len() % 2 != 0 {
        return Err(Malformed);
    }
    let suites = suites
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    reader.vec8(1, 255)?;
    let block = reader.vec16(0, 0xffff)?;
    reader.finish()?;
    let mut out = Offered {
        session_id,
        suites,
        ..Offered::default()
    };
    let extensions = extension_block(block).map_err(|_| Malformed)?;
    for (extension_type, data) in extensions.iter() {
        out.extensions.push(extension_type);
        match extension_type {
            ext::SUPPORTED_GROUPS => out.groups = u16_list(data, 2)?,
            ext::SIGNATURE_ALGORITHMS => out.signature_algorithms = u16_list(data, 2)?,
            ext::KEY_SHARE => {
                let mut reader = Reader::new(data);
                let mut shares = Reader::new(reader.vec16(0, 0xffff)?);
                reader.finish()?;
                while !shares.is_empty() {
                    out.shares.push(shares.u16()?);
                    shares.vec16(1, 0xffff)?;
                }
            }
            ext::ALPN => {
                let mut reader = Reader::new(data);
                let mut names = Reader::new(reader.vec16(2, 0xffff)?);
                reader.finish()?;
                while !names.is_empty() {
                    out.alpn.push(names.vec8(1, 255)?.to_vec());
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// One extension: its type and its data.
pub(crate) type Extension<'a> = (u16, &'a [u8]);

/// Why an extension block was refused: it does not parse, or a type stands twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockError {
    /// It does not parse.
    Malformed,
    /// A type stands twice.
    Repeated,
}

/// The extensions of a block that parsed, read in place: nothing is copied or collected, so
/// what a block makes the client hold does not grow with the number of its extensions.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Extensions<'a> {
    block: &'a [u8],
}

impl<'a> Extensions<'a> {
    /// Each extension in the order sent.
    pub(crate) fn iter(self) -> ExtensionIter<'a> {
        ExtensionIter {
            reader: Reader::new(self.block),
        }
    }

    /// The data of `extension_type`, if present.
    pub(crate) fn get(self, extension_type: u16) -> Option<&'a [u8]> {
        self.iter()
            .find(|(t, _)| *t == extension_type)
            .map(|(_, d)| d)
    }
}

/// The iterator of [`Extensions`].
pub(crate) struct ExtensionIter<'a> {
    reader: Reader<'a>,
}

impl<'a> Iterator for ExtensionIter<'a> {
    type Item = Extension<'a>;

    fn next(&mut self) -> Option<Extension<'a>> {
        // The block parsed in `extension_block`, so a read fails only past its end.
        let extension_type = self.reader.u16().ok()?;
        let data = self.reader.vec16(0, 0xffff).ok()?;
        Some((extension_type, data))
    }
}

/// The extensions of a block (the bytes inside its 2-byte length): each a type and data of a
/// 2-byte length, to the end of the block. Two of one type are refused; the check keeps one bit
/// per type, so its work is linear in the block, and it allocates nothing.
pub(crate) fn extension_block(block: &[u8]) -> Result<Extensions<'_>, BlockError> {
    let mut seen = [0u64; 1024];
    let mut reader = Reader::new(block);
    while !reader.is_empty() {
        let extension_type = reader.u16().map_err(|_| BlockError::Malformed)?;
        reader.vec16(0, 0xffff).map_err(|_| BlockError::Malformed)?;
        let (word, bit) = (usize::from(extension_type / 64), extension_type % 64);
        if seen[word] & (1 << bit) != 0 {
            return Err(BlockError::Repeated);
        }
        seen[word] |= 1 << bit;
    }
    Ok(Extensions { block })
}

/// The extension block of `message`, refused as `Malformed { message }` or
/// `RepeatedExtension`.
pub(crate) fn extensions_of<'a>(
    block: &'a [u8],
    message: &'static str,
) -> Result<Extensions<'a>, Abort> {
    extension_block(block).map_err(|e| match e {
        BlockError::Malformed => Abort::malformed(message),
        BlockError::Repeated => Abort::illegal(Violation::RepeatedExtension),
    })
}

/// Whether the table of RFC 8446 section 4.2 lets `extension_type` stand in `place`: a type
/// it names only where it names it; a type it does not name (an unknown one) anywhere.
fn allowed_here(extension_type: u16, place: Place) -> bool {
    allowed_in(extension_type).is_none_or(|places| places.contains(&place))
}

/// The first half of the rule of RFC 8446 section 4.2, for a message whose unknown and
/// unrequested extensions are ignored (CertificateRequest, NewSessionTicket): a type the table
/// names but does not allow in `place` is `illegal_parameter`, `ExtensionNotAllowedHere`.
pub(crate) fn check_allowed(extensions: Extensions<'_>, place: Place) -> Result<(), Abort> {
    if extensions.iter().all(|(t, _)| allowed_here(t, place)) {
        Ok(())
    } else {
        Err(Abort::illegal(Violation::ExtensionNotAllowedHere))
    }
}

/// The rule of RFC 8446 section 4.2 for the extensions of a message the server sent in
/// `place`: a type the table names but does not allow there is `illegal_parameter`,
/// `ExtensionNotAllowedHere`, checked first; then one the hello did not offer is
/// `unsupported_extension`, `UnofferedExtension`.
pub(crate) fn check_extensions(
    extensions: Extensions<'_>,
    place: Place,
    offered: &dyn Fn(u16) -> bool,
) -> Result<(), Abort> {
    for (extension_type, _) in extensions.iter() {
        if !allowed_here(extension_type, place) {
            return Err(Abort::illegal(Violation::ExtensionNotAllowedHere));
        }
        if !offered(extension_type) {
            return Err(Abort::new(
                AlertDescription::UNSUPPORTED_EXTENSION,
                Violation::UnofferedExtension,
            ));
        }
    }
    Ok(())
}

/// A ServerHello or HelloRetryRequest as read: its fields, the extensions not yet judged.
#[derive(Debug)]
pub(crate) struct ServerHello<'a> {
    /// `legacy_version`.
    pub legacy_version: u16,
    /// The random.
    pub random: &'a [u8],
    /// `legacy_session_id_echo`.
    pub session_id: &'a [u8],
    /// `cipher_suite`.
    pub suite: u16,
    /// `legacy_compression_method`.
    pub compression: u8,
    /// The extensions; none when the message ends after the compression method (TLS 1.2 and
    /// below may send none).
    pub extensions: Extensions<'a>,
}

impl ServerHello<'_> {
    /// The data of `extension_type`, if present.
    pub(crate) fn extension(&self, extension_type: u16) -> Option<&[u8]> {
        self.extensions.get(extension_type)
    }
}

/// Reads the body of a ServerHello or HelloRetryRequest (`message` names it in a refusal).
pub(crate) fn server_hello<'a>(
    body: &'a [u8],
    message: &'static str,
) -> Result<ServerHello<'a>, Abort> {
    let malformed = |_| Abort::malformed(message);
    let mut reader = Reader::new(body);
    let legacy_version = reader.u16().map_err(malformed)?;
    let random = reader.take(32).map_err(malformed)?;
    let session_id = reader.vec8(0, 32).map_err(malformed)?;
    let suite = reader.u16().map_err(malformed)?;
    let compression = reader.u8().map_err(malformed)?;
    let extensions = if reader.is_empty() {
        Extensions::default()
    } else {
        let block = reader.vec16(0, 0xffff).map_err(malformed)?;
        reader.finish().map_err(malformed)?;
        extensions_of(block, message)?
    };
    Ok(ServerHello {
        legacy_version,
        random,
        session_id,
        suite,
        compression,
        extensions,
    })
}

/// The selected version of a `supported_versions` in a ServerHello or HelloRetryRequest: two
/// bytes exactly.
pub(crate) fn selected_version(data: &[u8], message: &'static str) -> Result<u16, Abort> {
    let mut reader = Reader::new(data);
    let version = reader.u16().map_err(|_| Abort::malformed(message))?;
    reader.finish().map_err(|_| Abort::malformed(message))?;
    Ok(version)
}

/// The words of a version a server selected.
pub(crate) fn version_name(version: u16) -> String {
    match version {
        0x0300 => "SSL 3.0".to_string(),
        0x0301 => "TLS 1.0".to_string(),
        0x0302 => "TLS 1.1".to_string(),
        0x0303 => "TLS 1.2".to_string(),
        0x0304 => "TLS 1.3".to_string(),
        other => format!("version 0x{other:04x}"),
    }
}

/// The share of a ServerHello `key_share`: its group and key, to the end of the data.
pub(crate) fn server_share(data: &[u8]) -> Result<(u16, &[u8]), Abort> {
    let malformed = |_| Abort::malformed("ServerHello");
    let mut reader = Reader::new(data);
    let group = reader.u16().map_err(malformed)?;
    let key = reader.vec16(1, 0xffff).map_err(malformed)?;
    reader.finish().map_err(malformed)?;
    Ok((group, key))
}

/// The `selected_group` of a HelloRetryRequest `key_share`: two bytes exactly.
pub(crate) fn selected_group(data: &[u8]) -> Result<u16, Abort> {
    selected_version(data, "HelloRetryRequest")
}

/// The cookie of a HelloRetryRequest: `opaque cookie<1..2^16-1>`, to the end of the data.
pub(crate) fn cookie(data: &[u8]) -> Result<&[u8], Abort> {
    let malformed = |_| Abort::malformed("HelloRetryRequest");
    let mut reader = Reader::new(data);
    let cookie = reader.vec16(1, 0xffff).map_err(malformed)?;
    reader.finish().map_err(malformed)?;
    Ok(cookie)
}

/// The one protocol of an ALPN extension the server sent: a list of exactly one name.
pub(crate) fn alpn_protocol(data: &[u8]) -> Result<&[u8], Abort> {
    let malformed = |_| Abort::malformed("EncryptedExtensions");
    let mut reader = Reader::new(data);
    let mut names = Reader::new(reader.vec16(2, 0xffff).map_err(malformed)?);
    reader.finish().map_err(malformed)?;
    let name = names.vec8(1, 255).map_err(malformed)?;
    let mut more = false;
    while !names.is_empty() {
        names.vec8(1, 255).map_err(malformed)?;
        more = true;
    }
    if more {
        return Err(Abort::illegal(Violation::UnofferedAlpn));
    }
    Ok(name)
}

/// The most certificates in a Certificate message (docs/design/tls.md item 9.2).
pub(crate) const MAX_CERTIFICATES: usize = 64;

/// The `cert_data` of each entry of a Certificate body, the leaf first; each entry's
/// extensions judged by `entry_extensions`. A list of more than [`MAX_CERTIFICATES`] entries
/// is refused at the first entry past it.
pub(crate) fn certificate_list<'a>(
    body: &'a [u8],
    entry_extensions: &dyn Fn(Extensions<'_>) -> Result<(), Abort>,
) -> Result<Vec<&'a [u8]>, Abort> {
    let malformed = |_| Abort::malformed("Certificate");
    let mut reader = Reader::new(body);
    let context = reader.vec8(0, 255).map_err(malformed)?;
    let list = reader.vec24(0, 0xff_ffff).map_err(malformed)?;
    reader.finish().map_err(malformed)?;
    if !context.is_empty() {
        return Err(Abort::illegal(Violation::NonEmptyRequestContext));
    }
    let mut entries = Reader::new(list);
    let mut out = Vec::new();
    while !entries.is_empty() {
        if out.len() == MAX_CERTIFICATES {
            return Err(Abort::new(
                AlertDescription::BAD_CERTIFICATE,
                Violation::TooManyCertificates,
            ));
        }
        let data = entries.vec24(1, 0xff_ffff).map_err(malformed)?;
        let block = entries.vec16(0, 0xffff).map_err(malformed)?;
        let extensions = extensions_of(block, "Certificate")?;
        entry_extensions(extensions)?;
        out.push(data);
    }
    if out.is_empty() {
        return Err(Abort::new(
            AlertDescription::DECODE_ERROR,
            Violation::EmptyCertificate,
        ));
    }
    Ok(out)
}

/// A CertificateVerify body: the scheme's code point and the signature.
pub(crate) fn certificate_verify(body: &[u8]) -> Result<(u16, &[u8]), Abort> {
    let malformed = |_| Abort::malformed("CertificateVerify");
    let mut reader = Reader::new(body);
    let scheme = reader.u16().map_err(malformed)?;
    let signature = reader.vec16(0, 0xffff).map_err(malformed)?;
    reader.finish().map_err(malformed)?;
    Ok((scheme, signature))
}

/// The content a server's CertificateVerify signs (RFC 8446 section 4.4.3): 64 spaces, the
/// context string, a zero byte and the transcript hash.
pub(crate) fn certificate_verify_content(transcript_hash: &[u8]) -> Vec<u8> {
    let context = b"TLS 1.3, server CertificateVerify";
    let mut content = Vec::with_capacity(64 + context.len() + 1 + transcript_hash.len());
    content.extend_from_slice(&[0x20; 64]);
    content.extend_from_slice(context);
    content.push(0);
    content.extend_from_slice(transcript_hash);
    content
}

/// A CertificateRequest body: refused with `NonEmptyRequestContext` for a context that is not
/// empty, `ExtensionNotAllowedHere` for an extension the table of section 4.2 names and does
/// not allow here, `MissingExtension` without `signature_algorithms`, `Malformed` when that
/// does not parse; unknown extensions are read and ignored (RFC 8446 sections 4.2, 4.3.2,
/// 9.3).
pub(crate) fn certificate_request(body: &[u8]) -> Result<(), Abort> {
    let malformed = |_| Abort::malformed("CertificateRequest");
    let mut reader = Reader::new(body);
    let context = reader.vec8(0, 255).map_err(malformed)?;
    let block = reader.vec16(2, 0xffff).map_err(malformed)?;
    reader.finish().map_err(malformed)?;
    let extensions = extensions_of(block, "CertificateRequest")?;
    if !context.is_empty() {
        return Err(Abort::illegal(Violation::NonEmptyRequestContext));
    }
    check_allowed(extensions, Place::CertificateRequest)?;
    let Some(schemes) = extensions.get(ext::SIGNATURE_ALGORITHMS) else {
        return Err(Abort::new(
            AlertDescription::MISSING_EXTENSION,
            Violation::MissingExtension,
        ));
    };
    scheme_list(schemes).map_err(malformed)?;
    Ok(())
}

/// Checks `SignatureScheme supported_signature_algorithms<2..2^16-2>` (RFC 8446 section
/// 4.2.3) to the end of `data`, without collecting it: surf answers no CertificateRequest with a
/// signature, so the schemes are read and not kept.
fn scheme_list(data: &[u8]) -> Result<(), Malformed> {
    let mut reader = Reader::new(data);
    let list = reader.vec16(2, 0xfffe)?;
    reader.finish()?;
    if list.len() % 2 == 0 {
        Ok(())
    } else {
        Err(Malformed)
    }
}

/// A NewSessionTicket body (RFC 8446 section 4.6.1), read whole and dropped: lifetime, age
/// add, nonce, a ticket of at least one byte; an extension the table of section 4.2 names and
/// does not allow here is `ExtensionNotAllowedHere`, unknown ones are read and ignored.
pub(crate) fn new_session_ticket(body: &[u8]) -> Result<(), Abort> {
    let malformed = |_| Abort::malformed("NewSessionTicket");
    let mut reader = Reader::new(body);
    reader.u32().map_err(malformed)?;
    reader.u32().map_err(malformed)?;
    reader.vec8(0, 255).map_err(malformed)?;
    reader.vec16(1, 0xffff).map_err(malformed)?;
    let block = reader.vec16(0, 0xfffe).map_err(malformed)?;
    reader.finish().map_err(malformed)?;
    let extensions = extensions_of(block, "NewSessionTicket")?;
    check_allowed(extensions, Place::NewSessionTicket)
}

/// A KeyUpdate body: one byte, 0 (`update_not_requested`) or 1 (`update_requested`).
pub(crate) fn key_update(body: &[u8]) -> Result<bool, Abort> {
    match body {
        [0] => Ok(false),
        [1] => Ok(true),
        [_] => Err(Abort::illegal(Violation::BadKeyUpdate)),
        _ => Err(Abort::malformed("KeyUpdate")),
    }
}

/// The KeyUpdate message surf writes.
pub(crate) fn key_update_message(update_requested: bool) -> Vec<u8> {
    vec![kind::KEY_UPDATE, 0, 0, 1, u8::from(update_requested)]
}
