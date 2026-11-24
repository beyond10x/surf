//! The fixture server: every task's site on one listener of 127.0.0.1 port 0, the site of task
//! `<task>` (the directory `sites/<task>/`) under `/<task>/`. So a path of `tasks.txt`, such as
//! `/article.html`, is `http://127.0.0.1:<port>/article/article.html` on the wire, and the
//! links and forms of a site are relative.
//!
//! `GET` only but for one route, one request per connection, answered with `Connection: close`.
//! A path component is of `[A-Za-z0-9._-]`, not `.` or `..`. `/<task>/results?q=<query>`, the
//! query of `[A-Za-z0-9]+`, answers `results-<query>.html` when it exists and
//! `results-none.html` otherwise. Everything else is a 404.
//!
//! The site of the task `login` has two routes of its own: `POST /login/login` with the body
//! `name=ada&password=lovelace` answers `303`, `Location: /login/account.html` and the
//! `Set-Cookie` of [`LOGIN_COOKIE`] (`Path=/; HttpOnly; SameSite=Lax`), any other body `303` to
//! `/login/login.html`; `GET /login/account.html` answers the account page only to a request
//! whose `Cookie` field holds [`LOGIN_COOKIE`], and `303` to `/login/login.html` otherwise. So
//! the task passes only when the agent's session keeps the cookie of its sign-in.
//!
//! [`Server::accepted`] counts the connections accepted, so that a test can tell how many a
//! task made.

use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

/// The most bytes of a request head the server reads.
const HEAD_MAX: usize = 16 * 1024;

/// How long the server waits for a request head.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// The cookie the site of the task `login` sets at a sign-in and asks for on its account page.
pub const LOGIN_COOKIE: &str = "sid=4f1c2a9e7b3d5f6081a2c3d4e5f60718";

/// The body of the sign-in of the task `login`.
const LOGIN_BODY: &[u8] = b"name=ada&password=lovelace";

/// The most bytes of a request body the server reads.
const BODY_MAX: usize = 16 * 1024;

/// A running fixture server. Its thread serves until the process ends.
pub struct Server {
    origin: String,
    accepted: Arc<AtomicUsize>,
}

impl Server {
    /// `http://127.0.0.1:<port>`.
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// The connections accepted so far.
    pub fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    /// The address of `path` (a path of `tasks.txt`, starting with `/`) on the site of `task`.
    pub fn address(&self, task: &str, path: &str) -> String {
        format!("{}/{task}{path}", self.origin)
    }
}

/// Bind 127.0.0.1 port 0 and serve `sites` from a thread, one more thread per connection.
pub fn serve(sites: PathBuf) -> io::Result<Server> {
    serve_sites(sites, None)
}

/// [`serve`], the sites of the tasks `only` alone when it is given: the path of any other
/// directory of `sites` is a 404.
pub fn serve_sites(sites: PathBuf, only: Option<Vec<String>>) -> io::Result<Server> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let origin = format!("http://{}", listener.local_addr()?);
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&accepted);
    let sites = Arc::new((sites, only));
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            counter.fetch_add(1, Ordering::SeqCst);
            let sites = Arc::clone(&sites);
            thread::spawn(move || {
                let _ = answer(stream, &sites.0, sites.1.as_deref());
            });
        }
    });
    Ok(Server { origin, accepted })
}

/// Read one request head and answer it.
fn answer(mut stream: TcpStream, sites: &Path, only: Option<&[String]>) -> io::Result<()> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 2048];
    let head_end = loop {
        if let Some(at) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break at + 4;
        }
        if bytes.len() > HEAD_MAX {
            return respond(&mut stream, "431 Request Header Fields Too Large", None);
        }
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Ok(());
        }
        bytes.extend_from_slice(&buffer[..read]);
    };
    let head = String::from_utf8_lossy(&bytes[..head_end]).into_owned();
    let request_line = head.split("\r\n").next().unwrap_or("");
    let mut words = request_line.split(' ');
    let (Some(method), Some(target)) = (words.next(), words.next()) else {
        return respond(&mut stream, "400 Bad Request", None);
    };
    let login = only.is_none_or(|only| only.iter().any(|name| name == "login"));
    match (method, target) {
        ("POST", "/login/login") if login => {
            let length = field(&head, "content-length")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            if length > BODY_MAX {
                return respond(&mut stream, "413 Content Too Large", None);
            }
            while bytes.len() < head_end + length {
                let read = stream.read(&mut buffer)?;
                if read == 0 {
                    return Ok(());
                }
                bytes.extend_from_slice(&buffer[..read]);
            }
            let body = &bytes[head_end..head_end + length];
            return if body == LOGIN_BODY {
                let cookie = format!("{LOGIN_COOKIE}; Path=/; HttpOnly; SameSite=Lax");
                see_other(&mut stream, "/login/account.html", Some(&cookie))
            } else {
                see_other(&mut stream, "/login/login.html", None)
            };
        }
        ("GET", "/login/account.html") if login && !signed_in(&head) => {
            return see_other(&mut stream, "/login/login.html", None);
        }
        _ => {}
    }
    if method != "GET" {
        return respond(&mut stream, "405 Method Not Allowed", None);
    }
    match route(sites, only, target).and_then(|file| Some((fs::read(&file).ok()?, file))) {
        Some((body, file)) => {
            let html = file.extension().is_some_and(|ext| ext == "html");
            let media = if html {
                "text/html; charset=utf-8"
            } else {
                "text/plain; charset=utf-8"
            };
            respond(&mut stream, "200 OK", Some((media, &body)))
        }
        None => respond(&mut stream, "404 Not Found", None),
    }
}

/// The value of the first field of `head` named `name` (lower case), in any case.
fn field<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.split("\r\n").skip(1).find_map(|line| {
        let (field, value) = line.split_once(':')?;
        field
            .eq_ignore_ascii_case(name)
            .then_some(value.trim_matches([' ', '\t']))
    })
}

/// Whether the `Cookie` field of `head` holds [`LOGIN_COOKIE`].
fn signed_in(head: &str) -> bool {
    field(head, "cookie").is_some_and(|cookies| cookies.split("; ").any(|c| c == LOGIN_COOKIE))
}

/// A `303` to `location`, with `Set-Cookie: <cookie>` when given, and no body.
fn see_other(stream: &mut TcpStream, location: &str, cookie: Option<&str>) -> io::Result<()> {
    let set_cookie = cookie.map_or(String::new(), |cookie| format!("Set-Cookie: {cookie}\r\n"));
    let head = format!(
        "HTTP/1.1 303 See Other\r\nLocation: {location}\r\n{set_cookie}Content-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(head.as_bytes())?;
    stream.flush()
}

fn respond(stream: &mut TcpStream, status: &str, body: Option<(&str, &[u8])>) -> io::Result<()> {
    let (media, body) = body.unwrap_or(("text/plain; charset=utf-8", b"not found\n"));
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {media}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

/// The file for a request target `/<task>/<path>[?<query>]`, `<task>` one of `only` when given.
fn route(sites: &Path, only: Option<&[String]>, target: &str) -> Option<PathBuf> {
    let rest = target.strip_prefix('/')?;
    let (task, path) = rest.split_once('/')?;
    if task.is_empty() || !task.bytes().all(|b| b.is_ascii_lowercase()) {
        return None;
    }
    if only.is_some_and(|only| !only.iter().any(|name| name == task)) {
        return None;
    }
    file_for(&sites.join(task), &format!("/{path}"))
}

/// Whether `component` may name a file or directory of a site.
fn is_component(component: &str) -> bool {
    !component.is_empty()
        && component != "."
        && component != ".."
        && component
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// The file the server answers for `target`, a path of `tasks.txt` (`/<path>[?<query>]`), on
/// the site in the directory `site`; `None` when it answers 404.
pub fn file_for(site: &Path, target: &str) -> Option<PathBuf> {
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (target, None),
    };
    let path = path.strip_prefix('/')?;
    if path == "results" {
        let query = query?.strip_prefix("q=")?;
        if query.is_empty() || !query.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return None;
        }
        let found = site.join(format!("results-{query}.html"));
        let file = if found.is_file() {
            found
        } else {
            site.join("results-none.html")
        };
        return file.is_file().then_some(file);
    }
    if query.is_some() || !path.split('/').all(is_component) {
        return None;
    }
    let file = site.join(path);
    file.is_file().then_some(file)
}
