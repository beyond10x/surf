//! `surf <url> [--as tree|markdown]`: load one address and print the rows it ends with; with
//! `--as`, then the page as the outline of its tree or as Markdown, every line behind `>`.
//!
//! The output format and the exit statuses are documented in `docs/surf-command.md`.

#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;

use surf::{
    Answer, EXIT_STOPPED, EXIT_USAGE, MemoryBrowser, Report, RequestStarted, Transport, answer,
    escape, exit_status, fetch_http,
};
use surf_answer::{Form, RenderError, render_text_whole};
use surf_fetch::Limits;
use surf_ports::ComponentPorts;
use surf_types::document::DocumentState;
use surf_types::navigation::NavigationState;

const USAGE: &str = "usage: surf <url> [--as tree|markdown]";

/// Write one line to standard error. A standard error that cannot be written is ignored: the
/// exit status still says what happened.
fn diagnose(line: fmt::Arguments<'_>) {
    let _ = writeln!(io::stderr().lock(), "{line}");
}

/// What the command line asks for: the address and the form of `--as`, if given.
struct Call {
    address: String,
    form: Option<Form>,
}

/// Read the words after the program name: one address and at most one `--as <form>` (or
/// `--as=<form>`), in any order. Otherwise a usage error, with a message for standard error
/// when the usage line alone does not say what is wrong; the message repeats no word.
fn parse(words: impl Iterator<Item = OsString>) -> Result<Call, Option<&'static str>> {
    let mut address = None;
    let mut form = None;
    let mut words = words;
    while let Some(word) = words.next() {
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
    Ok(Call {
        address: address.ok_or(None)?,
        form,
    })
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
    let address = call.address;
    let limits = Limits::default();
    // The binary navigates with `Get` and no body only, so `fetch_http` never refuses its request.
    let transport = move |request: &RequestStarted| -> Answer {
        let result = fetch_http(request, &limits);
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
