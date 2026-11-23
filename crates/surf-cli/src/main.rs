//! `surf <url> [--as tree|markdown] [--ca-file <path>]`: load one address and print the rows it
//! ends with; with `--as`, then the page as the outline of its tree or as Markdown, every line
//! behind `>`. An `https:` server is authenticated against the bundled trust anchors and those
//! of `--ca-file`. `surf --anchors [--ca-file <path>]` prints the trust anchors instead.
//!
//! The output format and the exit statuses are documented in `docs/surf-command.md`.

#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use surf::{
    Answer, EXIT_STOPPED, EXIT_USAGE, MemoryBrowser, Report, RequestStarted, Transport, answer,
    escape, exit_status, fetch_http,
};
use surf_answer::{Form, RenderError, render_text_whole};
use surf_fetch::{Anchors, Limits, anchors::EXTRA_BYTES_MAX};
use surf_ports::ComponentPorts;
use surf_types::document::DocumentState;
use surf_types::navigation::NavigationState;

const USAGE: &str = "usage: surf <url> [--as tree|markdown]";

/// Write one line to standard error. A standard error that cannot be written is ignored: the
/// exit status still says what happened.
fn diagnose(line: fmt::Arguments<'_>) {
    let _ = writeln!(io::stderr().lock(), "{line}");
}

/// What the command line asks for: the address (none with `--anchors`), the form of `--as`,
/// the file of `--ca-file`, and whether `--anchors` was given.
struct Call {
    address: Option<String>,
    form: Option<Form>,
    ca_file: Option<OsString>,
    anchors: bool,
}

/// Read the words after the program name: one address, at most one `--as <form>` (or
/// `--as=<form>`) and at most one `--ca-file <path>` (or `--ca-file=<path>`), in any order; or
/// `--anchors` with at most one `--ca-file` and nothing else. Otherwise a usage error, with a
/// message for standard error when the usage line alone does not say what is wrong; the message
/// repeats no word.
fn parse(words: impl Iterator<Item = OsString>) -> Result<Call, Option<&'static str>> {
    let mut address = None;
    let mut form = None;
    let mut ca_file = None;
    let mut anchors = false;
    let mut words = words;
    while let Some(word) = words.next() {
        if word == "--anchors" {
            if anchors {
                return Err(Some("--anchors is given twice"));
            }
            anchors = true;
            continue;
        }
        let file = if word == "--ca-file" {
            Some(words.next().ok_or(Some("--ca-file needs a path"))?)
        } else {
            word.to_str()
                .and_then(|text| text.strip_prefix("--ca-file="))
                .map(OsString::from)
        };
        if let Some(file) = file {
            if ca_file.is_some() {
                return Err(Some("--ca-file is given twice"));
            }
            if file.is_empty() {
                return Err(Some("--ca-file needs a path"));
            }
            ca_file = Some(file);
            continue;
        }
        let value = if word == "--as" {
            Some(words.next().ok_or(Some("--as needs tree or markdown"))?)
        } else {
            word.to_str()
                .and_then(|text| text.strip_prefix("--as="))
                .map(OsString::from)
        };
        match value {
            Some(value) => {
                if form.is_some() {
                    return Err(Some("--as is given twice"));
                }
                form = Some(match value.to_str() {
                    Some("tree") => Form::Tree,
                    Some("markdown") => Form::Markdown,
                    _ => return Err(Some("--as takes tree or markdown")),
                });
            }
            None if address.is_some() => return Err(None),
            None => {
                address = Some(
                    word.into_string()
                        .map_err(|_| Some("the address is not valid UTF-8"))?,
                );
            }
        }
    }
    if anchors {
        if address.is_some() || form.is_some() {
            return Err(Some("--anchors takes no address and no --as"));
        }
    } else if address.is_none() {
        return Err(None);
    }
    Ok(Call {
        address,
        form,
        ca_file,
        anchors,
    })
}

/// The bundled trust anchors with those of the file `path` (`--ca-file`), and how many the file
/// added; or the line for standard error naming the path and why the file is refused. The file
/// is read up to one byte past the most the reader takes, so a larger one is refused as too
/// large without being read whole.
fn read_anchors(path: &OsString) -> Result<(Anchors, usize), String> {
    let shown = escape(&path.to_string_lossy());
    let refused = |why: String| format!("surf: --ca-file {shown}: {}", escape(&why));
    let mut pem = Vec::new();
    File::open(path)
        .and_then(|file| file.take(EXTRA_BYTES_MAX as u64 + 1).read_to_end(&mut pem))
        .map_err(|error| refused(error.to_string()))?;
    let mut anchors = Anchors::bundled();
    let added = anchors
        .with_extra_pem(&pem)
        .map_err(|error| refused(error.to_string()))?;
    Ok((anchors, added))
}

/// The lines `--as` adds after the rows: `note text: no-css`, `note text: deep <n>` when the
/// Markdown's nesting passed the prefix bound, and the whole `text` block of the document the
/// load committed ([`surf_answer::render_text_whole`]), or `note text: no-document` when there
/// is none (a failed load, a response that is not HTML).
fn page_text<T: Transport>(
    browser: &MemoryBrowser<T>,
    report: &Report,
    form: Form,
) -> Result<String, RenderError> {
    let committed = report.navigation_state() == Some(NavigationState::Committed);
    let tree = report
        .documents
        .iter()
        .find(|row| {
            committed
                && row.navigation_id == report.navigation_id
                && row.state == DocumentState::Parsed
        })
        .and_then(|row| browser.document_tree(&row.document_id));
    let Some(tree) = tree else {
        return Ok("note text: no-document\n".to_string());
    };
    let (block, deep) = match form {
        Form::Tree => (render_text_whole(form, surf_page_text::outline(tree))?, 0),
        Form::Markdown => {
            let markdown = surf_page_text::markdown::markdown(tree);
            let deep = markdown.deep();
            (render_text_whole(form, markdown)?, deep)
        }
    };
    let deep = if deep > 0 {
        format!("note text: deep {deep}\n")
    } else {
        String::new()
    };
    Ok(format!("note text: no-css\n{deep}{block}"))
}

fn main() -> ExitCode {
    let call = match parse(env::args_os().skip(1)) {
        Ok(call) => call,
        Err(message) => {
            if let Some(message) = message {
                diagnose(format_args!("surf: {message}"));
            }
            diagnose(format_args!("{USAGE}"));
            return ExitCode::from(EXIT_USAGE);
        }
    };
    // The file of `--ca-file` is read and checked before anything is fetched: a refused file
    // makes no connection.
    let extra = match &call.ca_file {
        None => None,
        Some(path) => match read_anchors(path) {
            Ok(read) => Some((path, read)),
            Err(line) => {
                diagnose(format_args!("{line}"));
                return ExitCode::from(EXIT_USAGE);
            }
        },
    };
    if call.anchors {
        let mut lines = format!("{}\n", Anchors::bundled_line());
        if let Some((path, (_, added))) = &extra {
            lines.push_str(&format!(
                "anchors {added} file {}\n",
                escape(&path.to_string_lossy())
            ));
        }
        let mut out = io::stdout().lock();
        if let Err(error) = out.write_all(lines.as_bytes()).and_then(|()| out.flush()) {
            diagnose(format_args!(
                "surf: standard output: {}",
                escape(&error.to_string())
            ));
            return ExitCode::from(EXIT_STOPPED);
        }
        return ExitCode::SUCCESS;
    }
    let anchors = extra.map(|(_, (anchors, _))| anchors);
    let Some(address) = call.address else {
        diagnose(format_args!("{USAGE}"));
        return ExitCode::from(EXIT_USAGE);
    };
    let limits = Limits::default();
    // The binary navigates with `Get` and no body only, so `fetch_http` never refuses its request.
    let transport = move |request: &RequestStarted| -> Answer {
        let result = fetch_http(request, &limits, anchors.as_ref());
        if let Err(failure) = &result {
            let address = &request.url.0;
            // The failure's detail can quote the address and the operating system's text, so
            // the whole line is escaped, not only the address.
            diagnose(format_args!(
                "surf: {}",
                escape(&format!("{address}: {failure}"))
            ));
        }
        answer(result)
    };
    let mut browser = MemoryBrowser::from_ports(ComponentPorts::new(), transport);
    let result = browser.load(&address);
    match &result {
        Ok(report) => {
            let text = match call.form.map(|form| page_text(&browser, report, form)) {
                None => None,
                Some(Ok(text)) => Some(text),
                // A line the renderer refuses is the renderer's defect; nothing is written.
                Some(Err(error)) => {
                    diagnose(format_args!("surf: the page text: {error:?}"));
                    return ExitCode::from(EXIT_STOPPED);
                }
            };
            let mut out = io::stdout().lock();
            let written = report
                .lines()
                .iter()
                .try_for_each(|line| writeln!(out, "{line}"))
                .and_then(|()| match &text {
                    Some(text) => out.write_all(text.as_bytes()),
                    None => Ok(()),
                })
                .and_then(|()| out.flush());
            if let Err(error) = written {
                diagnose(format_args!(
                    "surf: standard output: {}",
                    escape(&error.to_string())
                ));
                return ExitCode::from(EXIT_STOPPED);
            }
        }
        // An `Error` displays static text of the generated code and identities this process
        // minted; nothing in it came from the address or a server.
        Err(error) => diagnose(format_args!("surf: {error}")),
    }
    ExitCode::from(exit_status(&result))
}
