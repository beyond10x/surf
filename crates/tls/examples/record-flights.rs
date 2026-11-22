//! Records the flights of an independent TLS 1.3 server against surf's client
//! (`story:tls-handshake`, docs/design/tls.md item 10.5). Run only by the lane `slow`
//! (`task tls-flights-slow`), never by a test or the gate: it starts `openssl s_server` on
//! 127.0.0.1, one process per recording, and connects to it over loopback.
//!
//! For each configuration it writes `<out>/<name>.bin`: the client's draws (its random, its
//! session id, its x25519 key and its secp256r1 key, 32 bytes each, from `/dev/urandom`), every
//! byte the client wrote and every byte it read. A test replays the file: the same draws and the
//! same server bytes make the client write the same bytes. Beside them `<out>/SOURCE`: the
//! implementation and its version, each command line, the date, the sha256 of each recording.
//!
//! Usage: `cargo run -p surf-tls --example record-flights -- <out dir> <work dir>`; the work
//! directory receives the certificate files the server reads.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use surf_tls::connection::Connection;
use surf_tls::crypto::{self, HashAlgorithm, Random};
use surf_tls::der::Time;
use surf_tls::pem;
use surf_tls::verify::{CertificateRejection, Check, LeafKey, ServerCertVerifier};
use surf_tls::x509::Certificate;

/// The request the client sends after its handshake; `s_server -www` answers it with a status
/// page.
const REQUEST: &[u8] = b"GET / HTTP/1.0\r\n\r\n";

/// The first bytes of a recording file, then its format version.
const MAGIC: &[u8; 4] = b"SFLT";
const VERSION: u8 = 1;

/// One recording: its name, the certificate case of `crates/tls-test-certs/certs/`, and the
/// options of `s_server` beyond the common ones.
struct Recording {
    name: &'static str,
    case: &'static str,
    options: &'static [&'static str],
}

const RECORDINGS: &[Recording] = &[
    Recording {
        name: "aes128-x25519-ecdsa",
        case: "ok-ecdsa-p256",
        options: &[
            "-tls1_3",
            "-ciphersuites",
            "TLS_AES_128_GCM_SHA256",
            "-groups",
            "X25519",
        ],
    },
    Recording {
        name: "aes256-x25519-ecdsa",
        case: "ok-ecdsa-p256",
        options: &[
            "-tls1_3",
            "-ciphersuites",
            "TLS_AES_256_GCM_SHA384",
            "-groups",
            "X25519",
        ],
    },
    Recording {
        name: "chacha20-x25519-ecdsa",
        case: "ok-ecdsa-p256",
        options: &[
            "-tls1_3",
            "-ciphersuites",
            "TLS_CHACHA20_POLY1305_SHA256",
            "-groups",
            "X25519",
        ],
    },
    Recording {
        name: "retry-secp256r1-ecdsa",
        case: "ok-ecdsa-p256",
        options: &[
            "-tls1_3",
            "-ciphersuites",
            "TLS_AES_128_GCM_SHA256",
            "-groups",
            "P-256",
        ],
    },
    Recording {
        name: "aes128-x25519-rsa",
        case: "ok-rsa2048",
        options: &[
            "-tls1_3",
            "-ciphersuites",
            "TLS_AES_128_GCM_SHA256",
            "-groups",
            "X25519",
        ],
    },
    Recording {
        name: "tls12-only",
        case: "ok-ecdsa-p256",
        options: &["-tls1_2"],
    },
];

/// What a session read and wrote.
#[derive(Default)]
struct Bytes {
    read: Vec<u8>,
    written: Vec<u8>,
}

/// A stream that keeps a copy of every byte read and written, outside the connection, so the
/// copy outlives a handshake that fails.
struct Recorder {
    stream: TcpStream,
    bytes: Rc<RefCell<Bytes>>,
}

impl Read for Recorder {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.stream.read(buf)?;
        self.bytes.borrow_mut().read.extend_from_slice(&buf[..n]);
        Ok(n)
    }
}

impl Write for Recorder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.stream.write(buf)?;
        self.bytes.borrow_mut().written.extend_from_slice(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

/// Accepts exactly the leaf of the case: the recording tests the handshake, not the path.
struct Leaf {
    der: Vec<u8>,
}

impl ServerCertVerifier for Leaf {
    fn verify(
        &self,
        certificates: &[&[u8]],
        _host: &str,
        _now: Time,
    ) -> Result<LeafKey, CertificateRejection> {
        if certificates.first() != Some(&self.der.as_slice()) {
            return Err(CertificateRejection::NotTrusted(Check::NoPathToAnchor));
        }
        let cert = Certificate::from_der(&self.der)
            .map_err(|_| CertificateRejection::NotTrusted(Check::NoPathToAnchor))?;
        Ok(LeafKey::from_certificate(&cert))
    }
}

fn main() {
    if let Err(message) = run() {
        eprintln!("record-flights: {message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [out, work] = args.as_slice() else {
        return Err("usage: record-flights <out dir> <work dir>".to_string());
    };
    let (out, work) = (PathBuf::from(out), PathBuf::from(work));
    let certs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tls-test-certs/certs");
    fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    let version = openssl_version()?;
    let mut commands = String::new();
    let mut sums = String::new();
    for recording in RECORDINGS {
        let (bytes, command, outcome) = record(recording, &certs, &work)?;
        let file = format!("{}.bin", recording.name);
        fs::write(out.join(&file), &bytes).map_err(|e| format!("{file}: {e}"))?;
        let _ = writeln!(commands, "{}: {command}", recording.name);
        let _ = writeln!(
            sums,
            "{}  {file}",
            hex(crypto::hash(HashAlgorithm::Sha256, &bytes).as_ref())
        );
        println!(
            "record-flights: {} {} bytes, {outcome}",
            recording.name,
            bytes.len()
        );
    }
    let source = format!(
        "Recorded flights of an independent TLS 1.3 server, OpenSSL's s_server, against surf's\n\
         client, on 127.0.0.1 (story tls-handshake, docs/design/tls.md item 10.5). Recorded by\n\
         crates/tls/examples/record-flights.rs in the lane slow (task tls-flights-slow); the\n\
         tests of tests/tls/flights.rs replay them and start no process.\n\
         \n\
         implementation: {version}\n\
         recorded: {date}\n\
         recordings: {count}\n\
         \n\
         A file holds: SFLT, the format version 1, the number of draws, each draw (32 bytes),\n\
         the client's bytes and the server's bytes, each after a 4-byte length. The draws are\n\
         the client's random, session id, x25519 key and secp256r1 key from /dev/urandom:\n\
         secrets of a test session on loopback, used by nothing else. The server's private keys\n\
         are those of crates/tls-test-certs/certs/ and are not in these files.\n\
         \n\
         command of each recording (PORT a free port of 127.0.0.1, WORK the work directory):\n\
         \n\
         {commands}\n\
         sha256 of each file, path relative to this directory:\n\
         \n\
         {sums}",
        date = date_now(),
        count = RECORDINGS.len(),
    );
    fs::write(out.join("SOURCE"), source).map_err(|e| format!("SOURCE: {e}"))?;
    Ok(())
}

fn openssl_version() -> Result<String, String> {
    let output = Command::new("openssl")
        .arg("version")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("openssl version: {e}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The certificate files of `case` for `s_server`: the leaf PEM, the rest of the chain PEM, the
/// PKCS#8 DER key; and the leaf's DER.
fn case_files(
    certs: &Path,
    case: &str,
    work: &Path,
) -> Result<(PathBuf, PathBuf, PathBuf, Vec<u8>), String> {
    let chain_path = certs.join(case).join("chain.pem");
    let text =
        fs::read_to_string(&chain_path).map_err(|e| format!("{}: {e}", chain_path.display()))?;
    let end = "-----END CERTIFICATE-----";
    let at = text.find(end).ok_or("chain.pem holds no certificate")? + end.len();
    let (leaf_pem, rest) = text.split_at(at);
    let leaf = work.join(format!("{case}-leaf.pem"));
    let chain = work.join(format!("{case}-chain.pem"));
    fs::write(&leaf, format!("{}\n", leaf_pem.trim())).map_err(|e| e.to_string())?;
    fs::write(&chain, format!("{}\n", rest.trim())).map_err(|e| e.to_string())?;
    let blocks = pem::parse(text.as_bytes()).map_err(|e| format!("chain.pem: {e}"))?;
    let der = blocks
        .into_iter()
        .next()
        .ok_or("chain.pem holds no block")?
        .data;
    Ok((leaf, chain, certs.join(case).join("leaf.pk8"), der))
}

/// Starts the server and waits for its `ACCEPT` line.
fn start(arguments: &[String]) -> Result<Child, String> {
    let mut child = Command::new("openssl")
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("openssl s_server: {e}"))?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut lines = BufReader::new(stdout).lines();
    loop {
        match lines.next() {
            Some(Ok(line)) if line.trim() == "ACCEPT" => break,
            Some(Ok(_)) => {}
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("s_server ended before ACCEPT".to_string());
            }
        }
    }
    Ok(child)
}

fn record(
    recording: &Recording,
    certs: &Path,
    work: &Path,
) -> Result<(Vec<u8>, String, String), String> {
    let (leaf, chain, key, der) = case_files(certs, recording.case, work)?;
    let port = TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map_err(|e| format!("a free port: {e}"))?
        .port();
    let mut arguments: Vec<String> = [
        "s_server",
        "-accept",
        &format!("127.0.0.1:{port}"),
        "-naccept",
        "1",
        "-www",
        "-cert",
        &leaf.display().to_string(),
        "-cert_chain",
        &chain.display().to_string(),
        "-key",
        &key.display().to_string(),
        "-keyform",
        "DER",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    arguments.extend(recording.options.iter().map(|s| s.to_string()));
    let command = format!("openssl {}", arguments.join(" "))
        .replace(&format!("127.0.0.1:{port}"), "127.0.0.1:PORT")
        .replace(&work.display().to_string(), "WORK")
        .replace(&certs.display().to_string(), "crates/tls-test-certs/certs");
    let mut child = start(&arguments)?;
    let result = session(port, der);
    let _ = child.kill();
    let _ = child.wait();
    let (draws, client, server, outcome) = result?;
    let mut bytes = MAGIC.to_vec();
    bytes.push(VERSION);
    bytes.push(draws.len() as u8);
    for draw in &draws {
        bytes.extend_from_slice(draw);
    }
    for side in [&client, &server] {
        bytes.extend_from_slice(&(side.len() as u32).to_be_bytes());
        bytes.extend_from_slice(side);
    }
    Ok((bytes, command, outcome))
}

type Session = (Vec<[u8; 32]>, Vec<u8>, Vec<u8>, String);

/// One connection: the handshake, the request, the answer to its end; or the handshake's
/// error. Either way every byte both ways.
fn session(port: u16, leaf: Vec<u8>) -> Result<Session, String> {
    let mut urandom = fs::File::open("/dev/urandom").map_err(|e| format!("/dev/urandom: {e}"))?;
    let mut draws = vec![[0u8; 32]; 4];
    for draw in &mut draws {
        urandom.read_exact(draw).map_err(|e| e.to_string())?;
    }
    let slices: Vec<&[u8]> = draws.iter().map(|d| d.as_slice()).collect();
    let random = Random::fixed_sequence(&slices);
    let stream = TcpStream::connect(("127.0.0.1", port)).map_err(|e| format!("connect: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;
    let bytes = Rc::new(RefCell::new(Bytes::default()));
    let recorder = Recorder {
        stream,
        bytes: Rc::clone(&bytes),
    };
    let now = Time::from_utc(2026, 11, 22, 0, 0, 0).ok_or("the time")?;
    let verifier = Leaf { der: leaf };
    let outcome = match Connection::with_random(recorder, "localhost", &verifier, now, &random) {
        Ok(mut connection) => {
            connection
                .write_data(REQUEST)
                .map_err(|e| format!("write: {e}"))?;
            let mut answer = Vec::new();
            let end = loop {
                let mut buf = [0u8; 4096];
                match connection.read_data(&mut buf) {
                    Ok(0) => break "end of stream".to_string(),
                    Ok(n) => answer.extend_from_slice(&buf[..n]),
                    Err(e) => break format!("{e:?}"),
                }
            };
            connection.into_inner();
            let first = answer.split(|&b| b == b'\n').next().unwrap_or(&[]);
            format!(
                "complete, {} bytes of answer ({}), then {end}",
                answer.len(),
                String::from_utf8_lossy(first).trim()
            )
        }
        Err(error) => format!("{error:?}"),
    };
    drop(random);
    let bytes = Rc::try_unwrap(bytes)
        .map_err(|_| "the recorder is still held".to_string())?
        .into_inner();
    Ok((draws, bytes.written, bytes.read, outcome))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The date of today in UTC, `YYYY-MM-DD`.
fn date_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
