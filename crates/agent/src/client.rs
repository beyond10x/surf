//! The client: parse the words, read the token, send one request to `surfd` and print its
//! answer. `open` starts `surfd` when no listener is on the socket. The client holds no state.
//!
//! Exit statuses (design section 4): the status of the answer, passed through (0, or 1, 2 or 3
//! for a refusal, as the reason table of `docs/agent-answer.md` gives it); 2 a usage error of the
//! client; 3 `surfd` cannot be reached or started, or its answer cannot be read.
//!
//! Before it connects, every command checks the runtime directory, `surf/` in it and the socket
//! ([`wire::check_socket_path`]): a failed check is status 3 with the path and what is wrong, and
//! nothing is connected to or written. `open` starts `surfd` only when that check finds no
//! socket, or a socket nobody listens on, and checks again before it connects.
//!
//! Before that, `open --ca-file <path>` reads the file and checks it with `surf-tls`
//! ([`read_ca_file`]); a file it refuses is a usage error (status 2), with nothing connected,
//! started or written. A file it takes travels as the text of `arg anchors` in the `open`
//! request; `surfd` passes the request to the session process unchanged, over its socket.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use surf_tls::anchors::{AnchorError, Anchors, EXTRA_BYTES_MAX};

use crate::cli::{self, Environment, USAGE};
use crate::wire;

/// The times the client waits. [`Config::default`] holds the documented ones; a test shortens
/// them.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// How long to wait for the answer: [`wire::ANSWER_TIMEOUT`].
    pub answer_timeout: Duration,
    /// How long `open` waits for the line of a `surfd` it started: [`wire::START_TIMEOUT`].
    pub start_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            answer_timeout: wire::ANSWER_TIMEOUT,
            start_timeout: wire::START_TIMEOUT,
        }
    }
}

/// The exit status of a usage error of the client.
pub const USAGE_STATUS: u8 = 2;

/// The exit status when `surfd` cannot be reached or started.
pub const UNREACHABLE_STATUS: u8 = 3;

/// Run one call: `args` are the words after the program name. The answer lines go to `out`,
/// every message of the client and the answer's reason to `err`. Returns the exit status. Reads
/// no input: `fill <handle> -` reads an empty value; [`run_with_input`] gives it one.
pub fn run(
    args: &[OsString],
    env: &Environment,
    config: &Config,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    run_with_input(args, env, config, &mut io::empty(), out, err)
}

/// The most bytes of a value read from standard input: the most a request holds. One more is
/// read to tell a longer value, which is a usage error.
pub const STDIN_VALUE_MAX: usize = wire::REQUEST_MAX;

/// [`run`], with `input` read for the value word `-` of `fill` ([`cli::STDIN_VALUE`]): at most
/// [`STDIN_VALUE_MAX`] bytes, without one line feed at the end, as UTF-8. The value is written
/// to no message of the client.
pub fn run_with_input(
    args: &[OsString],
    env: &Environment,
    config: &Config,
    input: &mut dyn Read,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let status = match call(args, env, config, input, out, err) {
        Ok(status) => status,
        Err(Failure::Usage(message)) => {
            let _ = writeln!(err, "surf-agent: {message}");
            let _ = err.write_all(USAGE.as_bytes());
            USAGE_STATUS
        }
        Err(Failure::Unreachable(message)) => {
            let _ = writeln!(err, "surf-agent: {message}");
            UNREACHABLE_STATUS
        }
    };
    let _ = out.flush();
    let _ = err.flush();
    status
}

enum Failure {
    Usage(String),
    Unreachable(String),
}

fn unreachable(message: impl Into<String>) -> Failure {
    Failure::Unreachable(message.into())
}

fn call(
    args: &[OsString],
    env: &Environment,
    config: &Config,
    input: &mut dyn Read,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<u8, Failure> {
    let mut call = cli::parse(args, env).map_err(|error| Failure::Usage(error.0))?;
    let from_stdin = call.command == "fill" && call.stdin;
    if from_stdin {
        call.value = Some(read_value(input)?);
    }
    // The file of `--ca-file` is read and checked before anything is connected or started.
    if let Some(path) = &call.ca_file {
        call.anchors = Some(read_ca_file(path)?);
    }
    let request = call.request();
    let request = request.encode().map_err(|error| match error {
        wire::RequestError::TooLarge if from_stdin => {
            Failure::Usage(too_long_for_the_wire(&request))
        }
        other => Failure::Usage(other.to_string()),
    })?;

    let runtime_dir = env
        .runtime_dir
        .as_ref()
        .filter(|dir| !dir.is_empty())
        .ok_or_else(|| {
            unreachable(
                "XDG_RUNTIME_DIR is not set; surf-agent does not fall back to a shared directory",
            )
        })?;
    let runtime = Path::new(runtime_dir);
    let socket = wire::socket_path(runtime).map_err(|error| unreachable(error.to_string()))?;
    let caller = wire::caller_uid().map_err(|error| {
        unreachable(format!(
            "the caller's user id cannot be read from /proc/self: {error}"
        ))
    })?;

    // The directories and the socket are checked before every connection, so that no byte of
    // a request, and no token, goes to a socket another user could have put there.
    let present = checked(runtime, caller)?;
    let connected = present.then(|| UnixStream::connect(&socket));
    let mut stream = match connected {
        Some(Ok(stream)) => stream,
        Some(Err(error)) if !no_listener(&error) => {
            return Err(unreachable(format!("cannot reach surfd: {error}")));
        }
        _ if call.command == "open" => {
            start_surfd(env, runtime_dir, config)?;
            if !checked(runtime, caller)? {
                return Err(unreachable(format!(
                    "surfd reported ready and {} is not there",
                    socket.display()
                )));
            }
            UnixStream::connect(&socket).map_err(|error| {
                unreachable(format!(
                    "surfd reported ready and its socket does not answer: {error}"
                ))
            })?
        }
        _ => {
            return Err(unreachable(
                "surfd is not running; `surf-agent open` starts it",
            ));
        }
    };

    stream
        .set_write_timeout(Some(config.answer_timeout))
        .and_then(|()| stream.write_all(&request))
        .map_err(|error| unreachable(format!("cannot send the request to surfd: {error}")))?;
    let answer = wire::read_answer(&mut stream, config.answer_timeout).map_err(|error| {
        unreachable(match error {
            wire::AnswerError::Timeout => format!(
                "surfd did not answer within {} seconds",
                config.answer_timeout.as_secs_f64()
            ),
            other => format!("surfd's answer cannot be used: {other}"),
        })
    })?;
    let _ = out.write_all(answer.text.as_bytes());
    if let Some(reason) = &answer.reason {
        let _ = writeln!(err, "surf-agent: {}", printable(reason));
    }
    Ok(answer.status)
}

/// A value of `fill -` of up to this many bytes always fits in its request, beside a handle of
/// up to [`FILL_HANDLE_FITS`] bytes as quoted (a handle the session gives, such as `f12`, is a
/// few). [`wire::Request::encode`] quotes a byte outside printable ASCII, `"` and `\` as `\x`
/// and two digits, 4 bytes, so 16,000 bytes take at most 64,000; a `fill` request is 172 bytes
/// besides its value and its handle (`surf-wire 1`, `command fill`, the session's 36 characters,
/// the token's 64 digits, `arg handle ""`, `arg value ""`, `end`, each line with its line feed),
/// and 64,000 + 172 + 1,364 = 65,536, [`wire::REQUEST_MAX`].
pub const STDIN_VALUE_FITS: usize = 16_000;

/// The longest handle, as quoted, beside which a value of [`STDIN_VALUE_FITS`] bytes fits.
pub const FILL_HANDLE_FITS: usize = 1_364;

/// The message for a value from standard input whose request is over [`wire::REQUEST_MAX`]: the
/// value's size on the wire and the rest of the request, never a byte of the value.
fn too_long_for_the_wire(request: &wire::Request) -> String {
    let quoted = request
        .arg("value")
        .map_or(0, |value| surf_answer::quote(value).len());
    let whole = wire_length(request);
    format!(
        "the value read from standard input is longer than a request carries: {quoted} bytes \
         quoted on the wire beside {} bytes of the rest of the request, over {} bytes; a value of \
         up to {STDIN_VALUE_FITS} bytes always fits",
        whole - quoted,
        wire::REQUEST_MAX
    )
}

/// The bytes `request` takes on the wire, as [`wire::Request::encode`] writes it, counted
/// without the bound.
fn wire_length(request: &wire::Request) -> usize {
    let mut length = wire::VERSION_LINE.len() + 1 + "command ".len() + request.command.len() + 1;
    if let Some(session) = &request.session {
        length += "session ".len() + session.len() + 1;
    }
    if request.token.is_some() {
        length += "token ".len() + 64 + 1;
    }
    for arg in &request.args {
        length += "arg ".len() + arg.name.len() + 1 + surf_answer::quote(&arg.value).len() + 1;
    }
    length + "end\n".len()
}

/// The value of `fill -` from `input`: at most [`STDIN_VALUE_MAX`] bytes of UTF-8, without one
/// line feed at the end. No message repeats a byte of it.
fn read_value(input: &mut dyn Read) -> Result<String, Failure> {
    let mut bytes = Vec::new();
    input
        .take(STDIN_VALUE_MAX as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            Failure::Usage(format!(
                "the value cannot be read from standard input: {}",
                error.kind()
            ))
        })?;
    if bytes.len() > STDIN_VALUE_MAX {
        return Err(Failure::Usage(format!(
            "the value read from standard input is longer than {STDIN_VALUE_MAX} bytes"
        )));
    }
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    String::from_utf8(bytes)
        .map_err(|_| Failure::Usage("the value read from standard input is not UTF-8".into()))
}

/// The text of the file of `--ca-file` (`docs/design/tls.md` item 8), read before any
/// connection: a regular file, read up to one byte past [`EXTRA_BYTES_MAX`] so that a larger one
/// is refused as too large without being read whole, and taken only when
/// [`Anchors::with_extra_pem`] of `surf-tls` takes it (at most 64 blocks, every block a
/// `CERTIFICATE`, every certificate a CA). A file it refuses is a usage error naming the kind of
/// [`AnchorError`] and the block at fault ([`refusal`]); no message repeats the path or a byte
/// of the file.
fn read_ca_file(path: &Path) -> Result<String, Failure> {
    let unreadable = |error: io::Error| {
        Failure::Usage(format!(
            "the --ca-file file cannot be read: {}",
            error.kind()
        ))
    };
    let not_regular = || Failure::Usage("the --ca-file file is not a regular file".to_string());
    // The path is looked at before it is opened: opening a FIFO with no writer for reading
    // waits for a writer without end. The file opened must then be the file looked at.
    let looked = fs::metadata(path).map_err(unreadable)?;
    if !looked.is_file() {
        return Err(not_regular());
    }
    let file = File::open(path).map_err(unreadable)?;
    let opened = file.metadata().map_err(unreadable)?;
    if opened.dev() != looked.dev() || opened.ino() != looked.ino() || !opened.is_file() {
        return Err(Failure::Usage(
            "the --ca-file file changed while it was read".to_string(),
        ));
    }
    let mut pem = Vec::new();
    file.take(EXTRA_BYTES_MAX as u64 + 1)
        .read_to_end(&mut pem)
        .map_err(unreadable)?;
    Anchors::bundled().with_extra_pem(&pem).map_err(|error| {
        Failure::Usage(format!(
            "the --ca-file file is refused: {}",
            refusal(&error)
        ))
    })?;
    String::from_utf8(pem)
        .map_err(|_| Failure::Usage("the --ca-file file is not UTF-8 text".to_string()))
}

/// The refusal of a `--ca-file` file as the client prints it: the kind of the [`AnchorError`]
/// and the index of the block at fault, and no byte of the file. The `Display` of `surf-tls`
/// shows the label of a block that is not a certificate, and the certificate reader's text can
/// show a byte of the certificate; here they are the kind's name alone.
fn refusal(error: &AnchorError) -> String {
    let name = error.name();
    match error {
        AnchorError::NotACertificate { index, .. } => {
            format!("{name}: block {index} is not a CERTIFICATE")
        }
        AnchorError::BadCertificate { index, error } => {
            format!("{name}: certificate {index} is refused: {}", error.name())
        }
        other => other.to_string(),
    }
}

/// [`wire::check_socket_path`]: whether the socket is there, or the refusal naming the path.
fn checked(runtime: &Path, caller: u32) -> Result<bool, Failure> {
    match wire::check_socket_path(runtime, caller) {
        Ok(found) => Ok(found == wire::SocketCheck::Present),
        Err(reason) => Err(unreachable(format!(
            "refusing to connect to surfd: {reason}"
        ))),
    }
}

/// No socket, or a socket nobody listens on: the cases in which `open` starts `surfd`.
fn no_listener(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
    )
}

/// Start `surfd` from the directory of this executable and wait for its one line. Does not wait
/// for `surfd` to end; a `surfd` that writes nothing in time is killed.
fn start_surfd(env: &Environment, runtime_dir: &OsString, config: &Config) -> Result<(), Failure> {
    let program: PathBuf = env
        .program_dir
        .as_ref()
        .ok_or_else(|| {
            unreachable("the directory of surf-agent is not known, so surfd cannot be started")
        })?
        .join("surfd");
    let mut child = Command::new(&program)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .env_remove("SURF_TOKEN")
        .env_remove("SURF_SESSION")
        .process_group(0)
        .spawn()
        .map_err(|error| unreachable(format!("cannot start surfd: {error}")))?;
    let stdout = child.stdout.take().expect("a piped standard output");
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut line = Vec::new();
        let result = BufReader::new(stdout)
            .take(wire::START_LINE_MAX as u64)
            .read_until(b'\n', &mut line);
        let _ = sender.send((result, line));
    });
    let (result, line) = match receiver.recv_timeout(config.start_timeout) {
        Ok(received) => received,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(unreachable(format!(
                "surfd did not report ready within {} seconds",
                config.start_timeout.as_secs_f64()
            )));
        }
    };
    result.map_err(|error| unreachable(format!("cannot read surfd's start: {error}")))?;
    let Some(line) = line.strip_suffix(b"\n") else {
        return Err(unreachable("surfd ended without reporting ready"));
    };
    if line == wire::READY.as_bytes() {
        return Ok(());
    }
    if let Some(reason) = line
        .strip_prefix(wire::REFUSED.as_bytes())
        .and_then(|rest| rest.strip_prefix(b" "))
    {
        return Err(unreachable(format!(
            "surfd refused to start: {}",
            printable(&String::from_utf8_lossy(reason))
        )));
    }
    Err(unreachable(
        "surfd wrote a line that is neither ready nor refused",
    ))
}

/// `text` with every character outside printable ASCII written as `\x` and two lower-case
/// hexadecimal digits per UTF-8 byte, so that a reason from `surfd` cannot steer a terminal.
fn printable(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, ' '..='~') {
            out.push(character);
        } else {
            let mut bytes = [0u8; 4];
            for byte in character.encode_utf8(&mut bytes).bytes() {
                let _ = write!(out, "\\x{byte:02x}");
            }
        }
    }
    out
}
