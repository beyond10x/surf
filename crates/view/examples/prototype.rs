//! The live view over example data, before `surfd` serves it (`story:view-page-prototype`).
//!
//! Serves the page's files ([`surf_view::page_files`]) and a [`Source`] over scripted sessions
//! kept in memory, in the shapes of `docs/view-stream.md` and nothing beside them. Prints one
//! line, `http://127.0.0.1:<port>/#<view token>`, and runs until it is killed. Open the line in
//! a browser on the same machine.
//!
//! The sessions: one `open` that ran `open` and `go http://127.0.0.1:41000/a.html`, and then
//! about every second one more `go` between `a.html` and `b.html`; one `closed` that ran `open`,
//! `go http://127.0.0.1:9/` (nothing listens there) and `close`; one `died` after `open` and a
//! `go` to a page whose title holds markup and a quotation mark. No page server runs: the
//! addresses are example data.

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::io::{self, Write};
use std::net::TcpListener;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use surf_secret::Token;
use surf_view::{Config, EventKind, Item, Limits, Next, Reader, Source, TooOld, page_files, serve};

/// The entries the example keeps; a reader from below them is answered `TooOld`.
const KEPT: usize = 10_000;
/// The live items (entries and session summaries) a reader can still catch up on.
const LIVE: usize = 4_096;
/// The actions an answer's `path` shows at most, the newest last.
const PATH_SHOWN: usize = 5;
/// The interval of the live session's `go`.
const EVERY: Duration = Duration::from_secs(1);

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let token = Token::draw()?;
    let address = format!("http://127.0.0.1:{port}/#{}", token.to_hex());
    let example = Example::new(now());
    let config = Config {
        token,
        host: format!("127.0.0.1:{port}"),
        files: page_files(),
        limits: Limits::default(),
    };
    let server = serve(listener, config, example.clone())?;
    {
        let mut out = io::stdout().lock();
        writeln!(out, "{address}")?;
        out.flush()?;
    }
    let mut next = Instant::now() + EVERY;
    loop {
        let left = next.saturating_duration_since(Instant::now());
        thread::sleep(left);
        next += EVERY;
        example.step(now());
        // The server lives as long as this loop, which ends only when the process is killed.
        let _ = server.address();
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}

// ---------------------------------------------------------------------------------------------
// The string writer of `docs/view-stream.md`, "Writing".

/// `value` as a JSON string: `"`, `\`, every character below U+0020, U+007F to U+009F, U+2028,
/// U+2029, `<`, `>` and `&` as `\u` and four lower-case hexadecimal digits; every other
/// character as itself.
fn string(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        let escaped = matches!(c, '"' | '\\' | '<' | '>' | '&' | '\u{2028}' | '\u{2029}')
            || c < ' '
            || ('\u{7f}'..='\u{9f}').contains(&c);
        if escaped {
            out.push_str(&format!("\\u{:04x}", c as u32));
        } else {
            out.push(c);
        }
    }
    out.push('"');
}

fn text_or_null(out: &mut String, value: Option<&str>) {
    match value {
        Some(value) => string(out, value),
        None => out.push_str("null"),
    }
}

fn number_or_null(out: &mut String, value: Option<u64>) {
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

/// A JSON object written member by member, in the order they are added.
struct Object(String);

impl Object {
    fn new() -> Self {
        Self(String::from("{"))
    }

    fn name(&mut self, name: &str) -> &mut String {
        if self.0.len() > 1 {
            self.0.push(',');
        }
        string(&mut self.0, name);
        self.0.push(':');
        &mut self.0
    }

    fn text(mut self, name: &str, value: &str) -> Self {
        string(self.name(name), value);
        self
    }

    fn maybe_text(mut self, name: &str, value: Option<&str>) -> Self {
        text_or_null(self.name(name), value);
        self
    }

    fn number(mut self, name: &str, value: u64) -> Self {
        self.name(name).push_str(&value.to_string());
        self
    }

    fn maybe_number(mut self, name: &str, value: Option<u64>) -> Self {
        number_or_null(self.name(name), value);
        self
    }

    fn raw(mut self, name: &str, json: &str) -> Self {
        self.name(name).push_str(json);
        self
    }

    fn end(mut self) -> String {
        self.0.push('}');
        self.0
    }
}

// ---------------------------------------------------------------------------------------------
// The example data.

/// The page line of an answer, `docs/view-stream.md` "The page line".
#[derive(Clone)]
enum PageLine {
    None,
    Loaded {
        status: u16,
        media_type: &'static str,
        address: String,
        title: Option<String>,
    },
    Failed {
        reason: &'static str,
        address: String,
    },
}

impl PageLine {
    fn json(&self) -> String {
        match self {
            Self::None => Object::new().text("kind", "none").end(),
            Self::Loaded {
                status,
                media_type,
                address,
                title,
            } => Object::new()
                .text("kind", "loaded")
                .number("status", u64::from(*status))
                .text("media_type", media_type)
                .text("address", address)
                .maybe_text("title", title.as_deref())
                .end(),
            Self::Failed { reason, address } => Object::new()
                .text("kind", "failed")
                .text("reason", reason)
                .text("address", address)
                .end(),
        }
    }
}

/// A page of the example's page server, which does not run.
struct Page {
    address: &'static str,
    bytes: u64,
    title: Option<&'static str>,
}

const PAGE_A: Page = Page {
    address: "http://127.0.0.1:41000/a.html",
    bytes: 5000,
    title: Some("Page a"),
};
const PAGE_B: Page = Page {
    address: "http://127.0.0.1:41000/b.html",
    bytes: 3120,
    title: None,
};
const PAGE_MARKUP: Page = Page {
    address: "http://127.0.0.1:41000/markup.html",
    bytes: 812,
    title: Some("Say \"hi\" <script>alert(1)</script> & bye"),
};
const NOWHERE: &str = "http://127.0.0.1:9/";

struct Session {
    id: String,
    state: &'static str,
    address: Option<String>,
    title: Option<String>,
    opened_at: u64,
    last_action_at: Option<u64>,
    ended_at: Option<u64>,
    page: Option<PageLine>,
    window_id: String,
    tab_id: String,
    /// Entries of this session so far; the next `index` is one more.
    entries: u64,
    /// The actions so far: sequence, command, argument.
    actions: Vec<(u64, &'static str, Option<String>)>,
    /// Entries of this session dropped by the bound, the oldest first.
    dropped: u64,
}

impl Session {
    fn summary(&self) -> String {
        Object::new()
            .text("id", &self.id)
            .text("state", self.state)
            .maybe_text("address", self.address.as_deref())
            .maybe_text("title", self.title.as_deref())
            .number("opened_at", self.opened_at)
            .maybe_number("last_action_at", self.last_action_at)
            .maybe_number("ended_at", self.ended_at)
            .raw(
                "page",
                &self
                    .page
                    .as_ref()
                    .map_or_else(|| "null".to_string(), PageLine::json),
            )
            .end()
    }
}

struct Kept {
    number: u64,
    session: usize,
    data: String,
}

struct Data {
    sessions: Vec<Session>,
    kept: VecDeque<Kept>,
    newest: u64,
    /// Live items, the first of them the live item `live_start`.
    live: VecDeque<Item>,
    live_start: u64,
    /// The identity counter for windows, tabs, navigations and documents.
    seed: u64,
    /// Which page the live session goes to next.
    flip: bool,
}

/// The example's [`Source`]: data in memory behind one lock, and a condition readers wait on.
#[derive(Clone)]
struct Example(Arc<(Mutex<Data>, Condvar)>);

impl Example {
    fn new(start: u64) -> Self {
        let mut data = Data {
            sessions: Vec::new(),
            kept: VecDeque::new(),
            newest: 0,
            live: VecDeque::new(),
            live_start: 0,
            seed: 1,
            flip: true,
        };
        let open = data.session("5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69", start - 60_000);
        let mut at = start - 60_000 + 10;
        data.open(open, &mut at);
        at += 1_500;
        data.go(open, &PAGE_A, &mut at);

        let closed = data.session("0b7d4e21-9c3a-4f58-b6e2-7a1d9c8e5f30", start - 45_000);
        let mut at = start - 45_000 + 10;
        data.open(closed, &mut at);
        at += 1_000;
        data.go_failed(closed, NOWHERE, "Unreachable", &mut at);
        at += 1_000;
        data.close(closed, &mut at);
        data.sessions[closed].state = "closed";
        data.sessions[closed].ended_at = Some(at + 8);

        let died = data.session("9d3e7f10-6a2b-4c8d-9e1f-3a5b7c9d1e2f", start - 30_000);
        let mut at = start - 30_000 + 10;
        data.open(died, &mut at);
        at += 2_000;
        data.go(died, &PAGE_MARKUP, &mut at);
        data.sessions[died].state = "died";
        data.sessions[died].ended_at = Some(at + 4_000);

        Self(Arc::new((Mutex::new(data), Condvar::new())))
    }

    fn lock(&self) -> MutexGuard<'_, Data> {
        self.0
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// One more `go` of the open session, every entry of it at once, then its `session` item.
    fn step(&self, now: u64) {
        let mut data = self.lock();
        let page = if data.flip { &PAGE_B } else { &PAGE_A };
        data.flip = !data.flip;
        let mut at = now;
        let first = data.newest + 1;
        data.go(0, page, &mut at);
        let last = data.newest;
        let items: Vec<Item> = data
            .kept
            .iter()
            .filter(|kept| kept.number >= first && kept.number <= last)
            .map(|kept| Item {
                number: kept.number,
                kind: EventKind::Entry,
                data: kept.data.clone().into_bytes(),
            })
            .collect();
        for item in items {
            data.push_live(item);
        }
        let summary = data.sessions[0].summary();
        data.push_live(Item {
            number: last,
            kind: EventKind::Session,
            data: summary.into_bytes(),
        });
        drop(data);
        self.0.1.notify_all();
    }
}

impl Data {
    fn session(&mut self, id: &str, opened_at: u64) -> usize {
        let window_id = self.identity();
        let tab_id = self.identity();
        self.sessions.push(Session {
            id: id.to_string(),
            state: "open",
            address: None,
            title: None,
            opened_at,
            last_action_at: None,
            ended_at: None,
            page: None,
            window_id,
            tab_id,
            entries: 0,
            actions: Vec::new(),
            dropped: 0,
        });
        self.sessions.len() - 1
    }

    /// A version 4 identity in its text form, drawn from a counter so that the example is the
    /// same at every start.
    fn identity(&mut self) -> String {
        let mut next = || {
            self.seed = self.seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.seed;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        let (high, low) = (next(), next());
        format!(
            "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
            high >> 32,
            (high >> 16) & 0xffff,
            high & 0x0fff,
            0x8000 | ((low >> 48) & 0x3fff),
            low & 0xffff_ffff_ffff
        )
    }

    /// Keeps one entry: its five first members, then `rest` (members without the braces).
    fn keep(&mut self, session: usize, at: &mut u64, kind: &str, rest: &str) {
        self.newest += 1;
        let number = self.newest;
        let record = &mut self.sessions[session];
        record.entries += 1;
        let mut data = Object::new()
            .number("number", number)
            .text("session", &record.id)
            .number("index", record.entries)
            .number("kept_at", *at)
            .text("kind", kind)
            .end();
        data.pop();
        data.push(',');
        data.push_str(rest);
        data.push('}');
        if kind == "action" {
            record.last_action_at = Some(*at);
        }
        *at += 1;
        self.kept.push_back(Kept {
            number,
            session,
            data,
        });
        while self.kept.len() > KEPT {
            if let Some(dropped) = self.kept.pop_front() {
                self.sessions[dropped.session].dropped += 1;
            }
        }
    }

    fn members(object: Object) -> String {
        let mut text = object.end();
        text.remove(0);
        text.pop();
        text
    }

    fn action(
        &mut self,
        session: usize,
        at: &mut u64,
        command: &'static str,
        argument: Option<&str>,
    ) {
        let record = &mut self.sessions[session];
        let sequence = record.actions.len() as u64 + 1;
        record
            .actions
            .push((sequence, command, argument.map(str::to_string)));
        let rest = Self::members(
            Object::new()
                .number("sequence", sequence)
                .text("command", command)
                .maybe_text("argument", argument),
        );
        self.keep(session, at, "action", &rest);
    }

    fn event(&mut self, session: usize, at: &mut u64, name: &str, fields: &[(&str, &str)]) {
        let fields: Vec<(&str, Option<&str>)> = fields
            .iter()
            .map(|(field, value)| (*field, Some(*value)))
            .collect();
        self.event_or_null(session, at, name, &fields);
    }

    /// An event whose fields may be absent, written `null`.
    fn event_or_null(
        &mut self,
        session: usize,
        at: &mut u64,
        name: &str,
        fields: &[(&str, Option<&str>)],
    ) {
        let mut object = Object::new();
        for (field, value) in fields {
            object = object.maybe_text(field, *value);
        }
        let rest = Self::members(
            Object::new()
                .text("name", name)
                .raw("unmapped", "false")
                .raw("fields", &object.end()),
        );
        self.keep(session, at, "event", &rest);
    }

    fn answer(&mut self, session: usize, at: &mut u64, page: PageLine) {
        let record = &mut self.sessions[session];
        let total = record.actions.len();
        let shown = total.min(PATH_SHOWN);
        let mut actions = String::from("[");
        for (i, (sequence, command, argument)) in record.actions[total - shown..].iter().enumerate()
        {
            if i > 0 {
                actions.push(',');
            }
            actions.push_str(
                &Object::new()
                    .number("sequence", *sequence)
                    .text("command", command)
                    .raw("handle", "null")
                    .maybe_text("argument", argument.as_deref())
                    .raw("argument_cut", "false")
                    .end(),
            );
        }
        actions.push(']');
        let path = Object::new()
            .number("shown", shown as u64)
            .number("total", total as u64)
            .raw("actions", &actions)
            .end();
        record.page = Some(page.clone());
        let rest = Self::members(
            Object::new()
                .raw("page", &page.json())
                .raw("path", &path)
                .raw("now", "null"),
        );
        self.keep(session, at, "answer", &rest);
    }

    /// The `surf.agent.ActionRecorded` of the newest action of `session`.
    fn recorded(&mut self, session: usize, at: &mut u64, url: Option<&str>) {
        let action_id = self.identity();
        let record = &self.sessions[session];
        let id = record.id.clone();
        let (sequence, command, _) = record.actions.last().cloned().unwrap_or((0, "", None));
        let sequence = sequence.to_string();
        self.event_or_null(
            session,
            at,
            "surf.agent.ActionRecorded",
            &[
                ("action_id", Some(&action_id)),
                ("session_id", Some(&id)),
                ("sequence", Some(&sequence)),
                ("command", Some(command)),
                ("url", url),
                ("handle", None),
                ("label", None),
            ],
        );
    }

    /// The four events of a real `open`, in the order of the example of `docs/view-stream.md`
    /// (entries 2 to 5).
    fn open(&mut self, session: usize, at: &mut u64) {
        let window_id = self.sessions[session].window_id.clone();
        let tab_id = self.sessions[session].tab_id.clone();
        let id = self.sessions[session].id.clone();
        self.action(session, at, "open", None);
        self.event(
            session,
            at,
            "surf.shell.WindowOpened",
            &[("window_id", &window_id)],
        );
        self.event(
            session,
            at,
            "surf.shell.TabOpened",
            &[("tab_id", &tab_id), ("window_id", &window_id)],
        );
        self.event(
            session,
            at,
            "surf.agent.SessionOpened",
            &[
                ("session_id", &id),
                ("window_id", &window_id),
                ("tab_id", &tab_id),
            ],
        );
        self.recorded(session, at, None);
        self.answer(session, at, PageLine::None);
    }

    /// The nine events of a real `go`, in the order of the example of `docs/view-stream.md`
    /// (entries 8 to 16), ending with `surf.document.ParsingStarted`.
    fn go(&mut self, session: usize, page: &Page, at: &mut u64) {
        let tab_id = self.sessions[session].tab_id.clone();
        let id = self.sessions[session].id.clone();
        let navigation_id = self.identity();
        let document_id = self.identity();
        let url = page.address;
        let body = format!("{} bytes", page.bytes);
        self.action(session, at, "go", Some(url));
        self.event_or_null(
            session,
            at,
            "surf.agent.NavigationRequested",
            &[
                ("session_id", Some(&id)),
                ("tab_id", Some(&tab_id)),
                ("url", Some(url)),
                ("handle", None),
                ("label", None),
            ],
        );
        self.recorded(session, at, Some(url));
        self.event_or_null(
            session,
            at,
            "surf.navigation.NavigationStarted",
            &[
                ("navigation_id", Some(&navigation_id)),
                ("tab_id", Some(&tab_id)),
                ("url", Some(url)),
                ("method", Some("Get")),
                ("body", None),
                ("body_type", None),
            ],
        );
        self.event(
            session,
            at,
            "surf.agent.SessionNavigationChanged",
            &[
                ("navigation_id", &navigation_id),
                ("state", "Started"),
                ("sessions", "1"),
            ],
        );
        self.event_or_null(
            session,
            at,
            "surf.net.RequestStarted",
            &[
                ("request_id", Some(&navigation_id)),
                ("url", Some(url)),
                ("method", Some("Get")),
                ("body", None),
                ("body_type", None),
            ],
        );
        self.event(
            session,
            at,
            "surf.net.ResponseReceived",
            &[
                ("request_id", &navigation_id),
                ("url", url),
                ("method", "Get"),
                ("status", "200"),
                ("media_type", "text/html"),
                ("body", &body),
            ],
        );
        self.event(
            session,
            at,
            "surf.navigation.NavigationCommitted",
            &[
                ("navigation_id", &navigation_id),
                ("tab_id", &tab_id),
                ("url", url),
                ("status", "200"),
                ("media_type", "text/html"),
                ("body", &body),
            ],
        );
        self.event(
            session,
            at,
            "surf.agent.SessionNavigationChanged",
            &[
                ("navigation_id", &navigation_id),
                ("state", "Committed"),
                ("sessions", "1"),
            ],
        );
        self.event(
            session,
            at,
            "surf.document.ParsingStarted",
            &[("document_id", &document_id), ("body", &body)],
        );
        let line = PageLine::Loaded {
            status: 200,
            media_type: "text/html",
            address: url.to_string(),
            title: page.title.map(str::to_string),
        };
        self.answer(session, at, line);
        let record = &mut self.sessions[session];
        record.address = Some(url.to_string());
        record.title = page.title.map(str::to_string);
    }

    fn go_failed(&mut self, session: usize, url: &str, reason: &'static str, at: &mut u64) {
        let tab_id = self.sessions[session].tab_id.clone();
        let navigation_id = self.identity();
        self.action(session, at, "go", Some(url));
        self.event_or_null(
            session,
            at,
            "surf.navigation.NavigationStarted",
            &[
                ("navigation_id", Some(&navigation_id)),
                ("tab_id", Some(&tab_id)),
                ("url", Some(url)),
                ("method", Some("Get")),
                ("body", None),
                ("body_type", None),
            ],
        );
        self.event_or_null(
            session,
            at,
            "surf.net.RequestStarted",
            &[
                ("request_id", Some(&navigation_id)),
                ("url", Some(url)),
                ("method", Some("Get")),
                ("body", None),
                ("body_type", None),
            ],
        );
        self.event(
            session,
            at,
            "surf.net.FetchFailed",
            &[
                ("request_id", &navigation_id),
                ("url", url),
                ("reason", reason),
            ],
        );
        self.event(
            session,
            at,
            "surf.navigation.NavigationFailed",
            &[
                ("navigation_id", &navigation_id),
                ("tab_id", &tab_id),
                ("url", url),
            ],
        );
        let line = PageLine::Failed {
            reason,
            address: url.to_string(),
        };
        self.answer(session, at, line);
        let record = &mut self.sessions[session];
        record.address = Some(url.to_string());
        record.title = None;
    }

    fn close(&mut self, session: usize, at: &mut u64) {
        let window_id = self.sessions[session].window_id.clone();
        let tab_id = self.sessions[session].tab_id.clone();
        let page = self.sessions[session]
            .page
            .clone()
            .unwrap_or(PageLine::None);
        self.action(session, at, "close", None);
        self.event(
            session,
            at,
            "surf.shell.TabClosed",
            &[("tab_id", &tab_id), ("window_id", &window_id)],
        );
        self.answer(session, at, page);
    }

    fn push_live(&mut self, item: Item) {
        self.live.push_back(item);
        while self.live.len() > LIVE {
            self.live.pop_front();
            self.live_start += 1;
        }
    }

    fn live_end(&self) -> u64 {
        self.live_start + self.live.len() as u64
    }
}

impl Source for Example {
    fn sessions(&self) -> Vec<u8> {
        let data = self.lock();
        let mut list = String::from("[");
        for (at, session) in data.sessions.iter().enumerate() {
            if at > 0 {
                list.push(',');
            }
            list.push_str(&session.summary());
        }
        list.push(']');
        Object::new()
            .number("newest", data.newest)
            .raw("sessions", &list)
            .end()
            .into_bytes()
    }

    fn session(&self, id: &str) -> Option<Vec<u8>> {
        let data = self.lock();
        let at = data.sessions.iter().position(|session| session.id == id)?;
        let session = &data.sessions[at];
        let mut entries = String::from("[");
        for kept in data.kept.iter().filter(|kept| kept.session == at) {
            if entries.len() > 1 {
                entries.push(',');
            }
            entries.push_str(&kept.data);
        }
        entries.push(']');
        Some(
            Object::new()
                .raw("session", &session.summary())
                .number("dropped", session.dropped)
                .raw("log_dropped", "false")
                .number("unreadable", 0)
                .raw("entries", &entries)
                .end()
                .into_bytes(),
        )
    }

    fn newest(&self) -> u64 {
        self.lock().newest
    }

    fn read_from(&self, number: u64) -> Result<Box<dyn Reader>, TooOld> {
        let data = self.lock();
        let oldest = data
            .kept
            .front()
            .map_or(data.newest + 1, |kept| kept.number);
        if number + 1 < oldest {
            return Err(TooOld);
        }
        let replay = data
            .kept
            .iter()
            .filter(|kept| kept.number > number)
            .map(|kept| Item {
                number: kept.number,
                kind: EventKind::Entry,
                data: kept.data.clone().into_bytes(),
            })
            .collect();
        let cursor = data.live_end();
        drop(data);
        Ok(Box::new(ExampleReader {
            example: self.clone(),
            replay,
            cursor,
        }))
    }
}

/// A reader from a number: the kept entries above it, then the live items from the moment it
/// was made.
struct ExampleReader {
    example: Example,
    replay: VecDeque<Item>,
    cursor: u64,
}

impl Reader for ExampleReader {
    fn next(&mut self, wait: Duration) -> Next {
        if let Some(item) = self.replay.pop_front() {
            return Next::Item(item);
        }
        let deadline = Instant::now() + wait;
        let mut data = self.example.lock();
        loop {
            if self.cursor < data.live_start {
                // Fell behind what is kept live: the server ends the stream, the page resumes
                // with Last-Event-ID.
                return Next::End;
            }
            if self.cursor < data.live_end() {
                let at = (self.cursor - data.live_start) as usize;
                self.cursor += 1;
                return Next::Item(data.live[at].clone());
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Next::Idle;
            }
            data = self
                .example
                .0
                .1
                .wait_timeout(data, left)
                .map_or_else(|poisoned| poisoned.into_inner().0, |(data, _)| data);
        }
    }
}
