//! The parser of the `surf-agent` command line, written from `cli/surf-agent.yaml` (an
//! `ess-cli/2` document) with the standard library only. [`COMMANDS`] and [`GLOBALS`] are the
//! parser's table: [`parse`] reads the words through them and nothing else, and the test `drift`
//! holds them to `.out/cli/binding.json`, which `task cli` writes from the document.
//!
//! The differences from the document are its five `UNMAPPED:` markers, and only those:
//!
//! - (a) `ess-cli/2` has no environment source. The parser reads the session from `SURF_SESSION`
//!   when `--session` is absent and the token from `SURF_TOKEN` when `--token-file` is absent;
//!   an option given wins over the environment ([`Source::Environment`]).
//! - (b) `--token-stdin` and `--token-prompt` are declared because a `protected` source requires
//!   them. The parser refuses each with exit 2 and the usage, reading nothing from standard input
//!   or a terminal ([`Source::TokenStdin`], [`Source::TokenPrompt`]).
//! - (c) the state global `--session-dir`, which `ess-cli/2` requires. The parser refuses it with
//!   exit 2 and touches no path; the directory is `$XDG_RUNTIME_DIR/surf/` always ([`GLOBALS`]).
//! - (d) the value word `-` of `fill` ([`STDIN_VALUE`]). The parser keeps the word; the client
//!   reads the value from standard input in its place ([`crate::client::run_with_input`]).
//! - (e) `--` ends the options of every command: every word after it is a positional, one that
//!   starts with `-` included, and the value word `-` after it is one dash, not standard input
//!   ([`Call::stdin`]).
//!
//! The token is never an argument word: no option takes it, because other local users can read
//! a process's arguments. It is read from `SURF_TOKEN` or from a file that is a regular file, not
//! a symbolic link, owned by the caller and without a permission bit for group or others, and is
//! held only as a [`surf_secret::Token`]. No message of the parser repeats an argument word.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use surf_secret::Token;

use crate::wire::{self, Arg, Request};

/// Where an argument's value comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The `index`th word after the command, counted from 1.
    Positional {
        /// Its place among the positional words, from 1.
        index: usize,
        /// Whether the command needs it.
        required: bool,
    },
    /// `--<long> <value>` or `--<long>=<value>`.
    Option {
        /// The option name without its dashes.
        long: &'static str,
    },
    /// The `file` flag of a `protected` source: `--<long> <path>`, a token file.
    TokenFile {
        /// The option name without its dashes.
        long: &'static str,
    },
    /// The `stdin` flag of a `protected` source: declared by the document, refused (marker b).
    TokenStdin {
        /// The option name without its dashes.
        long: &'static str,
    },
    /// The `hidden_tty` flag of a `protected` source: declared by the document, refused
    /// (marker b).
    TokenPrompt {
        /// The option name without its dashes.
        long: &'static str,
    },
    /// An environment variable, read when no option of the argument was given (marker a).
    Environment {
        /// The variable's name.
        name: &'static str,
    },
}

impl Source {
    /// Whether the parser takes a value from this source; `false` for the two it refuses.
    pub fn accepted(&self) -> bool {
        !matches!(self, Source::TokenStdin { .. } | Source::TokenPrompt { .. })
    }

    /// The option name of a source that is an option word.
    pub fn long(&self) -> Option<&'static str> {
        match self {
            Source::Option { long }
            | Source::TokenFile { long }
            | Source::TokenStdin { long }
            | Source::TokenPrompt { long } => Some(long),
            Source::Positional { .. } | Source::Environment { .. } => None,
        }
    }
}

/// One field of a command's input and the sources of its value, in the order they are tried.
#[derive(Clone, Copy, Debug)]
pub struct Argument {
    /// The field name of the document (`url`, `session_id`, `token`).
    pub field: &'static str,
    /// Its sources.
    pub sources: &'static [Source],
}

/// One command of the command line.
#[derive(Clone, Copy, Debug)]
pub struct Command {
    /// The command word.
    pub name: &'static str,
    /// Its arguments.
    pub arguments: &'static [Argument],
}

/// An option before or after the command that belongs to no command.
#[derive(Clone, Copy, Debug)]
pub struct Global {
    /// The role the document gives it (`state`).
    pub role: &'static str,
    /// The option name without its dashes.
    pub long: &'static str,
    /// Whether the parser takes it; `false` for `--session-dir` (marker c).
    pub accepted: bool,
}

const URL_REQUIRED: Argument = Argument {
    field: "url",
    sources: &[Source::Positional {
        index: 1,
        required: true,
    }],
};

const URL_OPTIONAL: Argument = Argument {
    field: "url",
    sources: &[Source::Positional {
        index: 1,
        required: false,
    }],
};

const HANDLE: Argument = Argument {
    field: "handle",
    sources: &[Source::Positional {
        index: 1,
        required: true,
    }],
};

const VALUE: Argument = Argument {
    field: "value",
    sources: &[Source::Positional {
        index: 2,
        required: true,
    }],
};

const OPTION: Argument = Argument {
    field: "option",
    sources: &[Source::Positional {
        index: 2,
        required: true,
    }],
};

const SESSION: Argument = Argument {
    field: "session_id",
    sources: &[
        Source::Option { long: "session" },
        Source::Environment {
            name: "SURF_SESSION",
        },
    ],
};

const TOKEN: Argument = Argument {
    field: "token",
    sources: &[
        Source::TokenFile { long: "token-file" },
        Source::TokenStdin {
            long: "token-stdin",
        },
        Source::TokenPrompt {
            long: "token-prompt",
        },
        Source::Environment { name: "SURF_TOKEN" },
    ],
};

const FORM: Argument = Argument {
    field: "form",
    sources: &[Source::Option { long: "as" }],
};

const FROM_LINE: Argument = Argument {
    field: "from_line",
    sources: &[Source::Option { long: "from-line" }],
};

/// The forms `--as` takes, the variants of `surf.agent.PageForm`.
pub const FORMS: &[&str] = &["tree", "markdown"];

/// The commands: `open [<url>]`, `go <url>`, `look`, `follow <handle>`, `fill <handle> <value>`,
/// `pick <handle> <option>`, `press <handle>`, `close`. `open` takes no session and no token.
/// `open`, `go` and `look` take `--as tree|markdown [--from-line <n>]`; `open` only with an
/// address. The value `-` of `fill` is read from standard input by the client
/// ([`STDIN_VALUE`]).
pub const COMMANDS: &[Command] = &[
    Command {
        name: "open",
        arguments: &[URL_OPTIONAL, FORM, FROM_LINE],
    },
    Command {
        name: "go",
        arguments: &[URL_REQUIRED, SESSION, TOKEN, FORM, FROM_LINE],
    },
    Command {
        name: "look",
        arguments: &[SESSION, TOKEN, FORM, FROM_LINE],
    },
    Command {
        name: "follow",
        arguments: &[HANDLE, SESSION, TOKEN],
    },
    Command {
        name: "fill",
        arguments: &[HANDLE, VALUE, SESSION, TOKEN],
    },
    Command {
        name: "pick",
        arguments: &[HANDLE, OPTION, SESSION, TOKEN],
    },
    Command {
        name: "press",
        arguments: &[HANDLE, SESSION, TOKEN],
    },
    Command {
        name: "close",
        arguments: &[SESSION, TOKEN],
    },
];

/// The globals: `--session-dir`, declared by the document and refused.
pub const GLOBALS: &[Global] = &[Global {
    role: "state",
    long: "session-dir",
    accepted: false,
}];

/// The usage, written to standard error after every usage error.
pub const USAGE: &str = "\
usage: surf-agent open [<url>] [--as tree|markdown [--from-line <n>]]
       surf-agent go <url> [--session <id>] [--token-file <path>] [--as tree|markdown [--from-line <n>]]
       surf-agent look [--session <id>] [--token-file <path>] [--as tree|markdown [--from-line <n>]]
       surf-agent follow <handle> [--session <id>] [--token-file <path>]
       surf-agent fill <handle> <value>|- [--session <id>] [--token-file <path>]
       surf-agent pick <handle> <option>|on|off [--session <id>] [--token-file <path>]
       surf-agent press <handle> [--session <id>] [--token-file <path>]
       surf-agent close [--session <id>] [--token-file <path>]
The session comes from --session or SURF_SESSION, the token from --token-file or SURF_TOKEN.
A token file is a regular file of yours that group and others cannot read (mode 0600).
--as prints the page as an outline of its tree or as Markdown, from line --from-line (from 1);
open takes it only with an address.
follow, fill, pick and press take a handle of the now block of the last answer, such as l3,
f1 or b1.
fill - reads the value from standard input; give a secret that way, since other users can
read the words of a command. A value of up to 16000 bytes from standard input always fits.
pick takes an option's label or value, or on or off for a box.
-- ends the options: a word after it is an argument even when it starts with -
(fill f1 -- -5, pick f2 -- \"-- choose --\"; fill f1 -- - fills one dash).
";

/// The value word of `fill` that the client replaces by standard input, without one line feed
/// at its end: the marker `UNMAPPED:` at `fill` in `cli/surf-agent.yaml`.
pub const STDIN_VALUE: &str = "-";

/// What the parser reads from the process: the environment and the program's directory. Its
/// `Debug` does not write the token.
#[derive(Clone, Default)]
pub struct Environment {
    /// `XDG_RUNTIME_DIR`.
    pub runtime_dir: Option<OsString>,
    /// `SURF_SESSION`.
    pub session: Option<OsString>,
    /// `SURF_TOKEN`.
    pub token: Option<OsString>,
    /// The directory of the running executable, where `surfd` is looked for.
    pub program_dir: Option<PathBuf>,
}

impl Environment {
    /// The environment of this process.
    pub fn from_process() -> Self {
        Environment {
            runtime_dir: std::env::var_os("XDG_RUNTIME_DIR"),
            session: std::env::var_os("SURF_SESSION"),
            token: std::env::var_os("SURF_TOKEN"),
            program_dir: std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf)),
        }
    }
}

impl fmt::Debug for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Environment")
            .field("runtime_dir", &self.runtime_dir)
            .field("session", &self.session)
            .field("token", &self.token.as_ref().map(|_| ".."))
            .field("program_dir", &self.program_dir)
            .finish()
    }
}

/// A call the parser read: the command and its values. Its `Debug` writes the token as
/// `Token(..)` and the value of `fill` as its length in bytes.
#[derive(Clone)]
pub struct Call {
    /// The command word.
    pub command: &'static str,
    /// The session identity in lower case; `None` for `open`.
    pub session: Option<String>,
    /// The token; `None` for `open`.
    pub token: Option<Token>,
    /// The address, for `go` and an `open` given one.
    pub url: Option<String>,
    /// The handle, for `follow`, `fill` and `pick`.
    pub handle: Option<String>,
    /// The value word of `fill`; [`STDIN_VALUE`] until the client reads standard input. The
    /// `Debug` of the call writes only its length.
    pub value: Option<String>,
    /// The option word of `pick`.
    pub option: Option<String>,
    /// The form of `--as`, one of [`FORMS`].
    pub form: Option<&'static str>,
    /// The line of `--from-line`, from 1; only beside [`Call::form`].
    pub from_line: Option<u64>,
    /// The value of `fill` is read from standard input: its word is [`STDIN_VALUE`], given before
    /// any `--` (after `--` the word `-` is a value of one dash).
    pub stdin: bool,
}

impl fmt::Debug for Call {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Call")
            .field("command", &self.command)
            .field("session", &self.session)
            .field("token", &self.token)
            .field("url", &self.url)
            .field("handle", &self.handle)
            .field(
                "value",
                &self
                    .value
                    .as_ref()
                    .map(|value| format!("{} bytes", value.len())),
            )
            .field("option", &self.option)
            .field("form", &self.form)
            .field("from_line", &self.from_line)
            .field("stdin", &self.stdin)
            .finish()
    }
}

impl Call {
    /// The request of the wire for this call: `arg url`, `arg handle`, `arg value`, `arg option`,
    /// `arg form` and `arg line`, each when the call has it.
    pub fn request(&self) -> Request {
        let arg = |name: &str, value: String| Arg {
            name: name.to_string(),
            value,
        };
        let args = [
            self.url.clone().map(|url| arg("url", url)),
            self.handle.clone().map(|handle| arg("handle", handle)),
            self.value.clone().map(|value| arg("value", value)),
            self.option.clone().map(|option| arg("option", option)),
            self.form.map(|form| arg("form", form.to_string())),
            self.from_line.map(|line| arg("line", line.to_string())),
        ];
        Request {
            command: self.command.to_string(),
            session: self.session.clone(),
            token: self.token.clone(),
            args: args.into_iter().flatten().collect(),
        }
    }
}

/// A usage error: exit 2 and the usage. The message repeats no argument word and no part of a
/// token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageError(pub String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UsageError {}

fn usage(message: impl Into<String>) -> UsageError {
    UsageError(message.into())
}

/// Read `args` (the words after the program name) into a call. Every word is read before any
/// value is: a refused option is refused before the token is looked at. Then `--as` and
/// `--from-line` are read (`--from-line` only beside `--as`, `--as` on `open` only with an
/// address), then the token, then the session, then the address.
pub fn parse(args: &[OsString], env: &Environment) -> Result<Call, UsageError> {
    let mut command: Option<&'static Command> = None;
    // Each positional word, and whether it came after `--`.
    let mut positionals: Vec<(&OsStr, bool)> = Vec::new();
    let mut given: Vec<(&'static Source, OsString)> = Vec::new();
    let mut options_ended = false;
    let mut index = 0;
    while index < args.len() {
        let number = index + 1;
        let word = &args[index];
        index += 1;
        let bytes = word.as_encoded_bytes();
        if !options_ended && bytes == b"--" {
            options_ended = true;
        } else if let Some(rest) = bytes.strip_prefix(b"--").filter(|_| !options_ended) {
            let (name, inline) = match rest.iter().position(|&b| b == b'=') {
                Some(at) => (&rest[..at], Some(&rest[at + 1..])),
                None => (rest, None),
            };
            if let Some(global) = GLOBALS.iter().find(|g| g.long.as_bytes() == name) {
                return Err(usage(format!(
                    "--{} is not accepted: the session directory is always $XDG_RUNTIME_DIR/surf/",
                    global.long
                )));
            }
            let Some(current) = command else {
                return Err(usage(format!(
                    "word {number} is an option before the command"
                )));
            };
            let Some(source) = current
                .arguments
                .iter()
                .flat_map(|argument| argument.sources)
                .find(|source| source.long().is_some_and(|long| long.as_bytes() == name))
            else {
                return Err(usage(format!(
                    "word {number} is not an option of {}; no option takes the token; -- before it makes \
                     it an argument",
                    current.name
                )));
            };
            if !source.accepted() {
                return Err(usage(format!(
                    "--{} is not accepted: the token comes from --token-file or SURF_TOKEN",
                    source.long().unwrap_or_default()
                )));
            }
            let value = match inline {
                Some(value) => OsStr::new(
                    std::str::from_utf8(value)
                        .map_err(|_| usage(format!("the value of word {number} is not UTF-8")))?,
                )
                .to_os_string(),
                None => {
                    let Some(value) = args.get(index) else {
                        return Err(usage(format!(
                            "--{} needs a value",
                            source.long().unwrap_or_default()
                        )));
                    };
                    index += 1;
                    value.clone()
                }
            };
            if given.iter().any(|(earlier, _)| *earlier == source) {
                return Err(usage(format!(
                    "--{} is given twice",
                    source.long().unwrap_or_default()
                )));
            }
            given.push((source, value));
        } else if !options_ended && bytes.len() > 1 && bytes[0] == b'-' {
            return Err(usage(format!(
                "word {number} is not an option of surf-agent; -- before it makes it an argument"
            )));
        } else if command.is_none() {
            command = Some(
                COMMANDS
                    .iter()
                    .find(|c| c.name.as_bytes() == bytes)
                    .ok_or_else(|| {
                        usage(format!("word {number} is not a command of surf-agent"))
                    })?,
            );
        } else {
            positionals.push((word, options_ended));
        }
    }
    let Some(command) = command else {
        return Err(usage("no command"));
    };

    let mut call = Call {
        command: command.name,
        session: None,
        token: None,
        url: None,
        handle: None,
        value: None,
        option: None,
        form: None,
        from_line: None,
        stdin: false,
    };
    let positional_count = command
        .arguments
        .iter()
        .flat_map(|argument| argument.sources)
        .filter(|source| matches!(source, Source::Positional { .. }))
        .count();
    if positionals.len() > positional_count {
        return Err(usage(format!(
            "{} takes at most {positional_count} positional word",
            command.name
        )));
    }
    let option = |field: &str| -> Option<(&'static Source, &OsString)> {
        given.iter().find_map(|(source, value)| {
            let argument = command
                .arguments
                .iter()
                .find(|a| a.sources.contains(*source))?;
            (argument.field == field).then_some((*source, value))
        })
    };
    let declares = |field: &str| command.arguments.iter().any(|a| a.field == field);

    if let Some((_, value)) = option("form") {
        call.form = Some(
            FORMS
                .iter()
                .copied()
                .find(|form| OsStr::new(form) == value.as_os_str())
                .ok_or_else(|| usage("--as takes tree or markdown"))?,
        );
    }
    if let Some((_, value)) = option("from_line") {
        if call.form.is_none() {
            return Err(usage("--from-line is taken only beside --as"));
        }
        call.from_line = Some(
            value
                .to_str()
                .filter(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|digits| digits.parse::<u64>().ok())
                .filter(|line| *line >= 1)
                .ok_or_else(|| usage("--from-line takes a line number from 1"))?,
        );
    }
    if call.form.is_some() && command.name == "open" && positionals.is_empty() {
        return Err(usage("open takes --as only with an address"));
    }
    if declares("token") {
        call.token = Some(match option("token") {
            Some((_, path)) => read_token_file(Path::new(path))?,
            None => match env.token.as_deref().filter(|t| !t.is_empty()) {
                Some(text) => text
                    .to_str()
                    .and_then(Token::from_hex)
                    .ok_or_else(|| usage("SURF_TOKEN is not 64 lower-case hexadecimal digits"))?,
                None => {
                    return Err(usage(
                        "no token: give --token-file <path> or set SURF_TOKEN",
                    ));
                }
            },
        });
    }
    if declares("session_id") {
        let text = match option("session_id") {
            Some((_, value)) => value.clone(),
            None => match env.session.as_ref().filter(|s| !s.is_empty()) {
                Some(value) => value.clone(),
                None => return Err(usage("no session: give --session <id> or set SURF_SESSION")),
            },
        };
        call.session = Some(
            text.to_str()
                .and_then(wire::session_id)
                .ok_or_else(|| usage("the session is not a UUID in text form"))?,
        );
    }
    for argument in command.arguments {
        for source in argument.sources {
            if let Source::Positional { index, required } = source {
                match positionals.get(index - 1) {
                    Some(&(word, after_end)) => {
                        let text = word
                            .to_str()
                            .ok_or_else(|| usage(format!("the {} is not UTF-8", argument.field)))?;
                        match argument.field {
                            "handle" => call.handle = Some(text.to_string()),
                            "value" => {
                                call.stdin = !after_end && text == STDIN_VALUE;
                                call.value = Some(text.to_string());
                            }
                            "option" => call.option = Some(text.to_string()),
                            _ => call.url = Some(text.to_string()),
                        }
                    }
                    None if *required => {
                        let what = match argument.field {
                            "handle" => "a handle",
                            "value" => "a value, or - for standard input",
                            "option" => "an option, on or off",
                            _ => "an address",
                        };
                        return Err(usage(format!("{} needs {what}", command.name)));
                    }
                    None => {}
                }
            }
        }
    }
    Ok(call)
}

/// The most bytes read from a token file: 64 digits and a line feed, and one more to tell a
/// longer file.
const TOKEN_FILE_READ: u64 = 66;

/// The token in the file at `path`: a regular file, not a symbolic link, owned by the caller,
/// with no permission bit for group or others, holding 64 lower-case hexadecimal digits and at
/// most one line feed after them. The file opened is checked to be the file looked at.
fn read_token_file(path: &Path) -> Result<Token, UsageError> {
    let looked = fs::symlink_metadata(path)
        .map_err(|error| usage(format!("the token file cannot be read: {}", error.kind())))?;
    if looked.file_type().is_symlink() {
        return Err(usage("the token file is a symbolic link"));
    }
    if !looked.file_type().is_file() {
        return Err(usage("the token file is not a regular file"));
    }
    let file = File::open(path)
        .map_err(|error| usage(format!("the token file cannot be read: {}", error.kind())))?;
    let opened = file
        .metadata()
        .map_err(|error| usage(format!("the token file cannot be read: {}", error.kind())))?;
    if opened.dev() != looked.dev() || opened.ino() != looked.ino() || !opened.is_file() {
        return Err(usage("the token file changed while it was read"));
    }
    let caller = wire::caller_uid().map_err(|error| {
        usage(format!(
            "the caller's user id cannot be read: {}",
            error.kind()
        ))
    })?;
    if opened.uid() != caller {
        return Err(usage("the token file is not owned by you"));
    }
    if opened.mode() & 0o077 != 0 {
        return Err(usage(format!(
            "the token file has mode {:04o}; group and others must have no permission (0600)",
            opened.mode() & 0o7777
        )));
    }
    let mut digits = Vec::with_capacity(TOKEN_FILE_READ as usize);
    file.take(TOKEN_FILE_READ)
        .read_to_end(&mut digits)
        .map_err(|error| usage(format!("the token file cannot be read: {}", error.kind())))?;
    let text = digits.strip_suffix(b"\n").unwrap_or(&digits);
    std::str::from_utf8(text)
        .ok()
        .and_then(Token::from_hex)
        .ok_or_else(|| usage("the token file does not hold 64 lower-case hexadecimal digits"))
}
