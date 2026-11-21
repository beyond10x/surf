//! The client: parse the words, read the token, send one request to `surfd` and print its
//! answer. `open` starts `surfd` when no listener is on the socket. The client holds no state.
//!
//! Exit statuses (design section 4): the status of the answer, passed through (0, or 1, 2 or 3
//! for a refusal, as the reason table of `docs/agent-answer.md` gives it); 2 a usage error of the
//! client; 3 `surfd` cannot be reached or started, or its answer cannot be read.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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
/// every message of the client and the answer's reason to `err`. Returns the exit status.
pub fn run(
    args: &[OsString],
    env: &Environment,
    config: &Config,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let status = match call(args, env, config, out, err) {
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
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<u8, Failure> {
    let call = cli::parse(args, env).map_err(|error| Failure::Usage(error.0))?;
    let request = call
        .request()
        .encode()
        .map_err(|error| Failure::Usage(error.to_string()))?;

    let runtime_dir = env
        .runtime_dir
        .as_ref()
        .filter(|dir| !dir.is_empty())
        .ok_or_else(|| {
            unreachable(
                "XDG_RUNTIME_DIR is not set; surf-agent does not fall back to a shared directory",
            )
        })?;
    let socket = wire::socket_path(Path::new(runtime_dir))
        .map_err(|error| unreachable(error.to_string()))?;

    let mut stream = match UnixStream::connect(&socket) {
        Ok(stream) => stream,
        Err(error) if no_listener(&error) && call.command == "open" => {
            start_surfd(env, runtime_dir, config)?;
            UnixStream::connect(&socket).map_err(|error| {
                unreachable(format!(
                    "surfd reported ready and its socket does not answer: {error}"
                ))
            })?
        }
        Err(error) if no_listener(&error) => {
            return Err(unreachable(
                "surfd is not running; `surf-agent open` starts it",
            ));
        }
        Err(error) => return Err(unreachable(format!("cannot reach surfd: {error}"))),
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
