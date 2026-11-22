//! The recorded flights of an independent server (`story:tls-handshake`, docs/design/tls.md
//! item 10.5), replayed from `tests/data/flights/`: no server and no process starts here. With
//! `SURF_TLS_FLIGHTS` set to another directory (the lane `slow` sets it to its new recordings)
//! the same tests replay that one.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use surf_tls::connection::Connection;
use surf_tls::crypto::{self, HashAlgorithm, Random};
use surf_tls::error::{AlertDescription, Error};

use crate::alloc::peak_while;
use crate::handshake::{Double, HOST, crate_dir, hrr_random, now, records, to_hex};
use crate::handshake_hostile::{SEED, Seeded, clean_end, mutate};
use crate::rfc8448::Pipe;

/// The request the recorder sent after each handshake.
const REQUEST: &[u8] = b"GET / HTTP/1.0\r\n\r\n";

/// What each recording must replay to: the case of its certificate and whether the handshake
/// completes; else the error it ends with.
const EXPECTED: &[(&str, &str, Option<Error>)] = &[
    ("aes128-x25519-ecdsa", "ok-ecdsa-p256", None),
    ("aes256-x25519-ecdsa", "ok-ecdsa-p256", None),
    ("chacha20-x25519-ecdsa", "ok-ecdsa-p256", None),
    ("retry-secp256r1-ecdsa", "ok-ecdsa-p256", None),
    ("aes128-x25519-rsa", "ok-rsa2048", None),
    (
        "tls12-only",
        "ok-ecdsa-p256",
        Some(Error::AlertReceived {
            description: AlertDescription::PROTOCOL_VERSION,
            before_server_hello: true,
        }),
    ),
];

fn flights_dir() -> PathBuf {
    match std::env::var_os("SURF_TLS_FLIGHTS") {
        Some(dir) => PathBuf::from(dir),
        None => crate_dir().join("tests/data/flights"),
    }
}

/// One recording: the client's draws, the bytes it wrote, the bytes the server sent.
pub struct Flight {
    pub draws: Vec<Vec<u8>>,
    pub client: Vec<u8>,
    pub server: Vec<u8>,
}

/// Reads the format the recorder writes (its `SOURCE` states it).
pub fn parse(bytes: &[u8]) -> Flight {
    assert_eq!(&bytes[..5], b"SFLT\x01", "the magic and format version 1");
    let count = usize::from(bytes[5]);
    let mut at = 6;
    let mut draws = Vec::new();
    for _ in 0..count {
        draws.push(bytes[at..at + 32].to_vec());
        at += 32;
    }
    let mut sides = Vec::new();
    for _ in 0..2 {
        let len = u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        at += 4;
        sides.push(bytes[at..at + len as usize].to_vec());
        at += len as usize;
    }
    assert_eq!(at, bytes.len(), "nothing after the server's bytes");
    let server = sides.pop().unwrap_or_default();
    let client = sides.pop().unwrap_or_default();
    Flight {
        draws,
        client,
        server,
    }
}

/// The `.bin` files of the directory, by name.
fn recordings() -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(flights_dir()).expect("the flights directory") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if let Some(stem) = name.strip_suffix(".bin") {
            out.insert(stem.to_string(), fs::read(&path).expect("a recording"));
        }
    }
    out
}

/// What a replay ended with: the handshake's error, or the answer and how the reads ended.
#[derive(Debug, PartialEq)]
enum Replay {
    Failed(Error),
    Answered {
        answer: Vec<u8>,
        end: Result<(), Error>,
    },
}

/// Runs the client over the recorded server bytes with the recorded draws, as the recorder
/// did: the request after the handshake, then reads to the end; the connection given up without
/// `close_notify`. Returns the outcome and every byte the client wrote.
fn replay(flight: &Flight, server: &[u8], case: &str) -> (Replay, Vec<u8>) {
    let slices: Vec<&[u8]> = flight.draws.iter().map(|d| d.as_slice()).collect();
    let random = Random::fixed_sequence(&slices);
    let mut pipe = Pipe::with_input(server.to_vec());
    let verifier = Double::accepting(case);
    let outcome = match Connection::with_random(&mut pipe, HOST, &verifier, now(), &random) {
        Ok(mut connection) => match connection.write_data(REQUEST) {
            Err(e) => Replay::Answered {
                answer: Vec::new(),
                end: Err(e),
            },
            Ok(()) => {
                let mut answer = Vec::new();
                let end = loop {
                    let mut buf = [0u8; 4096];
                    match connection.read_data(&mut buf) {
                        Ok(0) => break Ok(()),
                        Ok(n) => answer.extend_from_slice(&buf[..n]),
                        Err(e) => break Err(e),
                    }
                };
                connection.into_inner();
                Replay::Answered { answer, end }
            }
        },
        Err(e) => Replay::Failed(e),
    };
    (outcome, pipe.output)
}

/// Acceptance 3: each recorded TLS 1.3 flight completes, the client writes byte for byte what
/// it wrote to the server (its ClientHello first), and the server's HTTP answer decrypts; the
/// recording against `s_server -tls1_2` ends with `AlertReceived { protocol_version,
/// before_server_hello: true }`. Every recording in the directory is one of these.
#[test]
fn recorded_flights_replay() {
    let recordings = recordings();
    let names: Vec<&str> = recordings.keys().map(String::as_str).collect();
    let mut expected: Vec<&str> = EXPECTED.iter().map(|(n, _, _)| *n).collect();
    expected.sort_unstable();
    assert_eq!(
        names,
        expected,
        "the recordings in {}",
        flights_dir().display()
    );
    for (name, case, error) in EXPECTED {
        let flight = parse(&recordings[*name]);
        let (outcome, written) = replay(&flight, &flight.server, case);
        if *name == "retry-secp256r1-ecdsa" {
            let first = &records(&flight.server)[0].1;
            assert_eq!(
                &first[6..38],
                &hrr_random(),
                "{name}: a HelloRetryRequest first"
            );
        }
        let hello = &records(&flight.client)[0];
        assert_eq!(
            to_hex(&records(&written)[0].1),
            to_hex(&hello.1),
            "{name}: our ClientHello equals the recorded one"
        );
        assert_eq!(
            to_hex(&written),
            to_hex(&flight.client),
            "{name}: every byte written"
        );
        match (outcome, error) {
            (Replay::Failed(got), Some(want)) => assert_eq!(&got, want, "{name}"),
            (Replay::Answered { answer, end }, None) => {
                assert!(
                    answer.starts_with(b"HTTP/1.0 200 ok\r\n"),
                    "{name}: {:?}",
                    String::from_utf8_lossy(&answer[..answer.len().min(40)])
                );
                assert_eq!(end, Ok(()), "{name}: the server ends with close_notify");
            }
            (other, want) => panic!("{name}: {other:?}, expected {want:?}"),
        }
    }
}

/// The sha256 of each `.bin` against the lines of `SOURCE`: `Ok(count)`, or the name of the
/// first file that is missing from `SOURCE`, listed without its file, or differs.
fn check_pins(files: &BTreeMap<String, Vec<u8>>, source: &str) -> Result<usize, String> {
    let mut pinned = BTreeMap::new();
    for line in source.lines() {
        let Some((sum, file)) = line.split_once("  ") else {
            continue;
        };
        if sum.len() == 64 && sum.bytes().all(|b| b.is_ascii_hexdigit()) {
            pinned.insert(file.to_string(), sum.to_string());
        }
    }
    for (name, bytes) in files {
        let file = format!("{name}.bin");
        let sum = to_hex(crypto::hash(HashAlgorithm::Sha256, bytes).as_ref());
        match pinned.remove(&file) {
            Some(pin) if pin == sum => {}
            Some(_) => return Err(format!("{file} differs from its sha256 in SOURCE")),
            None => return Err(format!("{file} is not in SOURCE")),
        }
    }
    match pinned.keys().next() {
        Some(file) => Err(format!("{file} is in SOURCE but missing")),
        None => Ok(files.len()),
    }
}

/// Acceptance 5, the part a test can show without a process: every recording matches its
/// sha256 in `SOURCE`, and a copy with one byte changed fails naming the file. `DIGEST` holds
/// the sha256 of the recorder's source.
#[test]
fn source_pins_each_recording() {
    let files = recordings();
    let dir = flights_dir();
    let source = fs::read_to_string(dir.join("SOURCE")).expect("SOURCE");
    assert_eq!(check_pins(&files, &source), Ok(EXPECTED.len()));
    for line in [
        "implementation: OpenSSL ",
        "recorded: ",
        "openssl s_server ",
    ] {
        assert!(source.contains(line), "SOURCE names {line:?}");
    }
    let mut changed = files.clone();
    let bytes = changed.get_mut("aes128-x25519-rsa").expect("a recording");
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    assert_eq!(
        check_pins(&changed, &source),
        Err("aes128-x25519-rsa.bin differs from its sha256 in SOURCE".to_string())
    );

    let digest = fs::read_to_string(dir.join("DIGEST")).expect("DIGEST");
    let recorder = fs::read(crate_dir().join("examples/record-flights.rs")).expect("recorder");
    let sum = to_hex(crypto::hash(HashAlgorithm::Sha256, &recorder).as_ref());
    assert_eq!(
        digest,
        format!("{sum}  crates/tls/examples/record-flights.rs\n"),
        "the recorder changed since the flights were recorded: run task tls-flights-slow"
    );
}

/// 400 runs from seed [`SEED`] (the seed of `handshake_hostile`): the server bytes of a recorded
/// flight changed at one to three places, half of the runs inside its first record (the
/// ServerHello or HelloRetryRequest, in the clear), half anywhere. No panic; each run ends with
/// an alert, a clean close or a complete handshake; the peak of live heap stays under 512 KiB.
#[test]
fn mutated_flights_end_cleanly() {
    let recordings = recordings();
    let flights: Vec<(Flight, &str)> = EXPECTED
        .iter()
        .map(|(name, case, _)| (parse(&recordings[*name]), *case))
        .collect();
    let mut random = Seeded(SEED);
    let mut ends: BTreeMap<String, usize> = BTreeMap::new();
    let mut max_peak = 0;
    for run in 0..400 {
        let (flight, case) = &flights[random.below(flights.len())];
        let mut server = flight.server.clone();
        if run % 2 == 0 && server.len() > 5 {
            let first =
                (5 + usize::from(u16::from_be_bytes([server[3], server[4]]))).min(server.len());
            let mut head = server[..first].to_vec();
            mutate(&mut head, &mut random);
            head.extend_from_slice(&server[first..]);
            server = head;
        } else {
            mutate(&mut server, &mut random);
        }
        let ((outcome, _), peak) = peak_while(|| replay(flight, &server, case));
        max_peak = max_peak.max(peak);
        let key = match outcome {
            Replay::Failed(e) | Replay::Answered { end: Err(e), .. } => {
                assert!(clean_end(&e), "run {run}: {e:?}");
                format!("{e:?}")
                    .split([' ', '{', '('])
                    .next()
                    .unwrap_or("")
                    .to_string()
            }
            Replay::Answered { end: Ok(()), .. } => "complete".to_string(),
        };
        *ends.entry(key).or_insert(0) += 1;
    }
    println!("mutated flights, seed {SEED:#x}, 400 runs: {ends:?}, peak {max_peak} bytes");
    assert!(max_peak < 512 * 1024);
}
