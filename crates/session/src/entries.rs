//! The entries this session sends `surfd` for its log (`surfd::link`, "The entries";
//! `story:agent-session-events`): one line per action and per event, each value from outside a
//! field of [`surf_answer::quote`] (decision D26).
//!
//! [`event_line`] is the one function that maps a [`SystemEvent`] to its entry: the qualified
//! name `spec/` gives it ([`SystemEvent::name`]), then every field in the order the `ess/1`
//! document declares them, each value quoted, by the field's type:
//!
//! - an `Integer` (and `surf.net.Status`) in decimal;
//! - an enum by its variant name as `spec/` spells it: `open`, `go` and `close` for
//!   `surf.agent.ActionCommand`, the generated variant's name for `surf.navigation.Navigation.State`,
//!   `surf.net.FailureReason`, `surf.net.Method` and `surf.document.ParseLimit`, which `spec/`
//!   spells the same (`Committed`, `Unreachable`, `Post`, `Depth`), so that a variant a
//!   specification story adds is written without a change here;
//! - an identity in its text form, an address, a media type, a document's title and an encoding's
//!   name as their text;
//! - a field of the type `Bytes` (the `body` of `surf.net.ResponseReceived`, of
//!   `surf.navigation.NavigationCommitted` and of `surf.document.ParsingStarted`) and a present
//!   one of the type `Optional<Bytes>` (the request body of `surf.navigation.NavigationStarted`
//!   and of `surf.net.RequestStarted`) as its length, the text `<n> bytes`, never the bytes
//!   (decision D22);
//! - an absent `Optional` value as the field with nothing after its `=`.
//!
//! An event this function does not map is written with its name, the word `unmapped` and no
//! field, so nothing it carries leaks (decision D27). Every variant on `main` is mapped, so that
//! arm is not reached; a story that adds an event maps it here and in the mapping test.

use surf::SystemEvent;
use surf_types::agent::SessionLog;

use crate::command_word;

/// The action entry line of a row of the view `SessionLog`, without its line feed: the action
/// as the `path` line prints it (`docs/agent-answer.md`).
pub fn action_line(row: &SessionLog) -> String {
    let sequence = u64::try_from(row.sequence).unwrap_or_default();
    let command = command_word(row.command);
    match &row.url {
        Some(url) => format!("action {sequence} {command} {}", surf_answer::quote(&url.0)),
        None => format!("action {sequence} {command}"),
    }
}

/// An event line being written: `event <name>` and its fields so far.
struct Line(String);

impl Line {
    fn new(event: &SystemEvent) -> Line {
        Line(format!("event {}", event.name()))
    }

    /// The field `name` with `value`, quoted.
    fn text(mut self, name: &str, value: &str) -> Line {
        self.0.push(' ');
        self.0.push_str(name);
        self.0.push('=');
        self.0.push_str(&surf_answer::quote(value));
        self
    }

    /// An `Integer` in decimal.
    fn integer(self, name: &str, value: i64) -> Line {
        self.text(name, &value.to_string())
    }

    /// A field of the type `Bytes`: its length, never its bytes.
    fn bytes(self, name: &str, value: &[u8]) -> Line {
        self.text(name, &format!("{} bytes", value.len()))
    }

    /// An `Optional` value: the field with no value when it is absent.
    fn optional(mut self, name: &str, value: Option<&str>) -> Line {
        match value {
            Some(value) => self.text(name, value),
            None => {
                self.0.push(' ');
                self.0.push_str(name);
                self.0.push('=');
                self
            }
        }
    }

    /// A field of the type `Optional<Bytes>`: its length when it is present, never its bytes;
    /// the field with no value when it is absent.
    fn optional_bytes(self, name: &str, value: Option<&[u8]>) -> Line {
        match value {
            Some(value) => self.bytes(name, value),
            None => self.optional(name, None),
        }
    }
}

/// The variant name of an enum whose generated variants are spelled as `spec/` spells them.
fn variant(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

/// The event entry line of `event`, without its line feed.
pub fn event_line(event: &SystemEvent) -> String {
    let line = Line::new(event);
    let line = match event {
        SystemEvent::ActionRecorded(e) => line
            .text("action_id", &e.action_id.0.0)
            .text("session_id", &e.session_id.0.0)
            .integer("sequence", e.sequence)
            .text("command", command_word(e.command))
            .optional("url", e.url.as_ref().map(|url| url.0.as_str())),
        SystemEvent::NavigationRequested(e) => line
            .text("session_id", &e.session_id.0.0)
            .text("tab_id", &e.tab_id.0.0)
            .text("url", &e.url.0),
        SystemEvent::SessionClosed(e) => line
            .text("session_id", &e.session_id.0.0)
            .text("window_id", &e.window_id.0.0)
            .text("tab_id", &e.tab_id.0.0),
        SystemEvent::SessionNavigationChanged(e) => line
            .text("navigation_id", &e.navigation_id.0.0)
            .text("state", &variant(e.state))
            .integer("sessions", e.sessions),
        SystemEvent::SessionOpened(e) => line
            .text("session_id", &e.session_id.0.0)
            .text("window_id", &e.window_id.0.0)
            .text("tab_id", &e.tab_id.0.0),
        SystemEvent::DocumentParsed(e) => line
            .text("document_id", &e.document_id.0.0)
            .text("url", &e.url.0)
            .text("navigation_id", &e.navigation_id.0.0)
            .text("title", &e.title)
            .text("encoding", &e.encoding.0)
            .optional(
                "unsupported_encoding",
                e.unsupported_encoding.as_ref().map(|name| name.0.as_str()),
            )
            .optional("stopped_by", e.stopped_by.map(variant).as_deref()),
        SystemEvent::ElementAdded(e) => line
            .text("element_id", &e.element_id.0.0)
            .text("document_id", &e.document_id.0.0)
            .integer("position", e.position),
        SystemEvent::ParsingStarted(e) => line
            .text("document_id", &e.document_id.0.0)
            .bytes("body", &e.body),
        SystemEvent::NavigationCancelled(e) => line
            .text("navigation_id", &e.navigation_id.0.0)
            .text("tab_id", &e.tab_id.0.0),
        SystemEvent::NavigationCommitted(e) => line
            .text("navigation_id", &e.navigation_id.0.0)
            .text("tab_id", &e.tab_id.0.0)
            .text("url", &e.url.0)
            .integer("status", e.status.0)
            .text("media_type", &e.media_type.0)
            .bytes("body", &e.body),
        SystemEvent::NavigationFailed(e) => line
            .text("navigation_id", &e.navigation_id.0.0)
            .text("tab_id", &e.tab_id.0.0)
            .text("url", &e.url.0)
            .text("reason", &variant(e.reason)),
        SystemEvent::NavigationStarted(e) => line
            .text("navigation_id", &e.navigation_id.0.0)
            .text("tab_id", &e.tab_id.0.0)
            .text("url", &e.url.0)
            .text("method", &variant(e.method))
            .optional_bytes("body", e.body.as_deref())
            .optional(
                "body_type",
                e.body_type.as_ref().map(|media_type| media_type.0.as_str()),
            ),
        SystemEvent::FetchFailed(e) => line
            .text("request_id", &e.request_id.0.0)
            .text("url", &e.url.0)
            .text("reason", &variant(e.reason)),
        SystemEvent::RequestStarted(e) => line
            .text("request_id", &e.request_id.0.0)
            .text("url", &e.url.0)
            .text("method", &variant(e.method))
            .optional_bytes("body", e.body.as_deref())
            .optional(
                "body_type",
                e.body_type.as_ref().map(|media_type| media_type.0.as_str()),
            ),
        SystemEvent::ResponseReceived(e) => line
            .text("request_id", &e.request_id.0.0)
            .text("url", &e.url.0)
            .integer("status", e.status.0)
            .text("media_type", &e.media_type.0)
            .bytes("body", &e.body),
        SystemEvent::FramePainted(e) => line
            .text("frame_id", &e.frame_id.0.0)
            .text("document_id", &e.document_id.0.0)
            .integer("width", e.width)
            .integer("height", e.height),
        SystemEvent::TabClosed(e) => line
            .text("tab_id", &e.tab_id.0.0)
            .text("window_id", &e.window_id.0.0),
        SystemEvent::TabOpened(e) => line
            .text("tab_id", &e.tab_id.0.0)
            .text("window_id", &e.window_id.0.0),
        SystemEvent::WindowOpened(e) => line.text("window_id", &e.window_id.0.0),
        // `SystemEvent` is not `#[non_exhaustive]`: while every variant is mapped this arm is
        // not reached, and an event a specification story adds lands here until it is mapped.
        #[allow(unreachable_patterns)]
        _ => return format!("event {} unmapped", event.name()),
    };
    line.0
}
