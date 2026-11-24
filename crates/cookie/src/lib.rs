//! The cookie jar of one agent session: `Set-Cookie` in, `Cookie` out, in memory only, never
//! printed (`story:agent-cookie-jar`).
//!
//! surf follows draft-ietf-httpbis-rfc6265bis-22 of 2025-12-01, the revision of RFC 6265 in the
//! RFC Editor's queue; the section numbers below are that draft's. The rules, each with the
//! section it was checked against (`spec/domains/net.yaml` states the same rules R1 to R19):
//!
//! - **Parsing** (5.6): a `Set-Cookie` string with a control character other than HTAB is
//!   ignored (step 1); a string without `=` is a cookie with an empty name (step 3), sent as its
//!   value alone (5.8.3 step 6); an empty name whose value starts with `__Secure-` or `__Host-`,
//!   in any case, is ignored (5.7 step 22), and so is a cookie whose name and value are both
//!   empty (5.7 step 2).
//! - **Size**: a name and a value over 4096 octets together make the cookie ignored (5.6 step 5,
//!   5.7 step 4); an attribute value over 1024 octets makes that attribute ignored (5.6).
//! - **Lifetime**: dates by the cookie-date algorithm of 5.1.1 ([`parse_date`]); `Max-Age` wins
//!   over `Expires` (5.7 step 6); a `Max-Age` of 0 or less expires the cookie (5.6.2), which
//!   deletes a stored one of the same name, domain and path (5.7 step 23 and the eviction of
//!   expired cookies); a lifetime over 400 days is cut to 400 days (5.5, 5.6.1, 5.6.2); an
//!   `Expires` that does not parse is ignored and the cookie lasts as long as the jar (5.6.1).
//!   A cookie is expired once its expiry time is before the time given.
//! - **Domain**, host-only (surf's option B until the public suffix list is bundled): the
//!   attribute loses a leading `.` and is lower-cased (5.6.3). When the request host
//!   domain-matches it (5.1.3: equal, or a suffix after a `.`, and for an IP address host only
//!   when equal) the cookie is stored host-only for the request host; otherwise it is ignored
//!   (5.7 step 10). So a cookie that `a.example.test` sets with `Domain=example.test` is sent
//!   to `a.example.test` only.
//! - **Path**: a missing attribute, or one that does not start with `/`, gives the default-path
//!   of the request address (5.6.4, 5.1.4); a cookie is sent where its path path-matches the
//!   request's path (5.1.4).
//! - **Secure**: only the scheme `https:` counts as a "secure" connection (5.7 step 13, 5.8.3);
//!   `localhost` is not trusted for it. A `Secure` cookie from an `http:` response is ignored; it
//!   is sent to `https:` addresses only; a cookie without `Secure` from an `http:` response is
//!   ignored when a `Secure` cookie of the same name, a domain that domain-matches it either way
//!   and a path the new one's path path-matches is stored (5.7 step 16).
//! - **HttpOnly** is stored and sent: surf has no "non-HTTP" retrieval (5.6.6, 5.8.3).
//! - **SameSite** (5.6.7, 5.7 steps 17 to 19, 5.8.3): `Strict`, `Lax` and `None` in any case;
//!   any other value or none is "Default". A `None` cookie without `Secure` is ignored (5.7 step
//!   19). Every navigation of surf is top-level, so step 18 ignores no cookie a navigation
//!   receives. A [`Site::Same`] request gets every cookie that matches; a [`Site::Cross`] one the
//!   `None` cookies always, the `Lax` and "Default" ones only for [`Method::Get`], the `Strict`
//!   ones never. "Lax-allowing-unsafe" (5.6.7.2) is not applied.
//! - **Same site** ([`site`], 5.2): a request without an initiator is same-site; with one, it is
//!   same-site when the request's address has the initiator's scheme and host, ports ignored.
//!   Without the public suffix list "site" is the host, not the registrable domain.
//! - **Prefixes** (5.4, 5.7 steps 20 and 21), in any case: `__Secure-` needs `Secure`; `__Host-`
//!   needs `Secure`, a `Path` attribute of `/` and no `Domain` attribute (with every cookie
//!   host-only, the draft's host-only flag is read as "no `Domain` attribute was given").
//! - **Replacement** (5.7 step 23): a cookie of the same name, domain and path replaces the
//!   stored one and keeps its creation time.
//! - **The `Cookie` field** (5.8.3): longer paths first, then earlier creation; `name=value`
//!   pairs joined by `; `; none when no cookie matches. Its value is at most
//!   [`COOKIE_FIELD_BYTES_MAX`] bytes: cookies are dropped from the end of that order until it
//!   fits.
//! - **Bounds and eviction** (5.7, "remove excess cookies", 6.1): at most
//!   [`COOKIES_PER_DOMAIN_MAX`] cookies per domain and [`COOKIES_MAX`] per jar. Expired cookies
//!   are evicted whenever the jar is used. Over a bound, in the draft's order of priority:
//!   expired cookies; cookies without `Secure` of a domain that holds more than 50; any cookie of
//!   such a domain; any cookie. Within one priority the earliest last access goes first, then
//!   (surf's choice where the draft says nothing) the earliest creation. Creation and last access
//!   are ticks of the jar's own counter, one per stored cookie and one per retrieval, so the
//!   order is total.
//!
//! The time is an argument of every method (`now`): the session gives the system clock, a test a
//! time it sets.
//!
//! # Memory
//!
//! Names and values: at most 4,096 bytes per cookie and 3,000 cookies, 12,288,000 bytes per
//! jar. Besides them each cookie holds its domain, a host, at most as long as the address it
//! came from, and its path, a `Path` attribute of at most 1,024 bytes or the default-path of that
//! address.
//!
//! # Never printed
//!
//! The jar's [`Debug`](fmt::Debug) text is `Jar { cookies: <n> }`; no method returns a cookie's
//! name or value but [`Jar::cookie_field`], whose text the caller sends and prints nowhere. The
//! jar lives in memory only and is dropped with its owner: no file is written.
//!
//! # Not taken
//!
//! The public suffix list (a `Domain` attribute never widens a cookie to other hosts), cookie
//! partitioning, a cookie store shared between sessions or kept on disk, `document.cookie`, and
//! "Lax-allowing-unsafe". A `Set-Cookie` field that is not UTF-8 is not passed in (the caller
//! drops it).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use surf_url::{Host, Url};

/// The most cookies one domain holds (6.1 of the draft: at least 50).
pub const COOKIES_PER_DOMAIN_MAX: usize = 50;

/// The most cookies one jar holds (6.1 of the draft: at least 3,000).
pub const COOKIES_MAX: usize = 3000;

/// The most octets of a cookie's name and value together (5.6 step 5).
pub const NAME_VALUE_BYTES_MAX: usize = 4096;

/// The most octets of one attribute's value (5.6).
pub const ATTRIBUTE_VALUE_BYTES_MAX: usize = 1024;

/// The most bytes of the value of the `Cookie` field one request sends.
pub const COOKIE_FIELD_BYTES_MAX: usize = 8192;

/// The longest lifetime a cookie gets, 400 days in seconds (5.5).
const AGE_MAX_SECONDS: i64 = 400 * 24 * 60 * 60;

/// Whether a request is same-site for cookies ([`site`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    /// Same-site: every cookie that matches is sent.
    Same,
    /// Cross-site: `SameSite` decides which cookies are sent.
    Cross,
}

/// The method of one request of a redirect chain, the hop the `Cookie` field is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `GET`, a safe method.
    Get,
    /// `POST`, not a safe method.
    Post,
}

/// A cookie's `SameSite` enforcement (5.6.7).
#[derive(Clone, Copy, PartialEq, Eq)]
enum SameSite {
    Strict,
    Lax,
    None,
    Default,
}

/// One stored cookie.
struct Cookie {
    name: String,
    value: String,
    /// The host it is sent to: every cookie is host-only.
    domain: String,
    path: String,
    /// Seconds since the epoch; `None` for a cookie that lasts as long as the jar.
    expiry: Option<i64>,
    secure: bool,
    same_site: SameSite,
    creation: u64,
    last_access: u64,
}

impl Cookie {
    fn expired(&self, now: i64) -> bool {
        self.expiry.is_some_and(|expiry| expiry < now)
    }
}

/// What the jar holds behind its lock.
#[derive(Default)]
struct Store {
    cookies: Vec<Cookie>,
    /// The jar's counter: one tick per stored cookie and per retrieval.
    ticks: u64,
}

impl Store {
    fn tick(&mut self) -> u64 {
        self.ticks = self.ticks.saturating_add(1);
        self.ticks
    }

    fn evict_expired(&mut self, now: i64) {
        self.cookies.retain(|cookie| !cookie.expired(now));
    }

    /// Remove excess cookies (5.7) after a cookie for `domain` was stored, the one domain that
    /// can have gone over its bound: first while it holds more than [`COOKIES_PER_DOMAIN_MAX`],
    /// its cookies without `Secure` before the others, then while the jar holds more than
    /// [`COOKIES_MAX`]; the earliest last access goes first, then the earliest creation.
    fn evict_excess(&mut self, domain: &str) {
        while self.cookies.iter().filter(|c| c.domain == domain).count() > COOKIES_PER_DOMAIN_MAX {
            let insecure = self.oldest(|c| c.domain == domain && !c.secure);
            let victim = insecure.or_else(|| self.oldest(|c| c.domain == domain));
            match victim {
                Some(at) => {
                    self.cookies.remove(at);
                }
                None => break,
            }
        }
        while self.cookies.len() > COOKIES_MAX {
            match self.oldest(|_| true) {
                Some(at) => {
                    self.cookies.remove(at);
                }
                None => break,
            }
        }
    }

    /// The index of the cookie that `chosen` selects with the earliest last access, then the
    /// earliest creation.
    fn oldest(&self, chosen: impl Fn(&Cookie) -> bool) -> Option<usize> {
        self.cookies
            .iter()
            .enumerate()
            .filter(|(_, cookie)| chosen(cookie))
            .min_by_key(|(_, cookie)| (cookie.last_access, cookie.creation))
            .map(|(at, _)| at)
    }
}

/// The cookies of one session, behind one lock, so that a `&Jar` is shared by the requests of
/// the session. Its [`Debug`](fmt::Debug) text is `Jar { cookies: <n> }`.
#[derive(Default)]
pub struct Jar {
    store: Mutex<Store>,
}

impl fmt::Debug for Jar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Jar {{ cookies: {} }}", self.lock().cookies.len())
    }
}

impl Jar {
    /// An empty jar.
    pub fn new() -> Jar {
        Jar::default()
    }

    /// The store; a lock another thread poisoned is taken as it is, since every change to it
    /// is complete before the lock is released.
    fn lock(&self) -> MutexGuard<'_, Store> {
        self.store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Store the cookie of one `Set-Cookie` field value, received in the response from `url` at
    /// `now` (5.6 and 5.7), or ignore it as those sections say.
    pub fn receive(&self, url: &Url, set_cookie: &str, now: SystemTime) {
        let now = seconds(now);
        let Some(host) = request_host(url) else {
            return;
        };
        let secure_request = url.scheme() == "https";
        let Some(parsed) = parse_set_cookie(set_cookie, now) else {
            return;
        };
        let mut store = self.lock();
        store.evict_expired(now);
        // 5.7 step 2: a cookie with neither name nor value is ignored.
        if parsed.name.is_empty() && parsed.value.is_empty() {
            return;
        }
        // Step 10, host-only: a Domain attribute the host domain-matches keeps the cookie on
        // the request host; any other ignores it.
        if let Some(domain) = &parsed.domain
            && !domain_matches(&host.text, host.is_ip, domain)
        {
            return;
        }
        // 5.6.4: a `Path` attribute that does not start with `/` stands for the default-path.
        let path = match &parsed.path {
            Some(Some(path)) => path.clone(),
            Some(None) | None => default_path(url),
        };
        // Step 13.
        if parsed.secure && !secure_request {
            return;
        }
        // Step 16.
        if !parsed.secure && !secure_request {
            let shadowed = store.cookies.iter().any(|old| {
                old.secure
                    && old.name == parsed.name
                    && (domain_matches(&host.text, host.is_ip, &old.domain)
                        || domain_matches(&old.domain, is_ip(&old.domain), &host.text))
                    && path_matches(&old.path, &path)
            });
            if shadowed {
                return;
            }
        }
        // Step 19.
        if parsed.same_site == SameSite::None && !parsed.secure {
            return;
        }
        // Steps 20 to 22.
        if starts_with_ignoring_case(&parsed.name, "__Secure-") && !parsed.secure {
            return;
        }
        if starts_with_ignoring_case(&parsed.name, "__Host-")
            && !(parsed.secure && parsed.domain.is_none() && parsed.path.is_some() && path == "/")
        {
            return;
        }
        if parsed.name.is_empty()
            && (starts_with_ignoring_case(&parsed.value, "__Secure-")
                || starts_with_ignoring_case(&parsed.value, "__Host-"))
        {
            return;
        }
        // Step 23: the same name, domain and path replace the stored cookie, whose creation
        // time the new one keeps.
        let tick = store.tick();
        let mut creation = tick;
        if let Some(at) = store
            .cookies
            .iter()
            .position(|old| old.name == parsed.name && old.domain == host.text && old.path == path)
        {
            creation = store.cookies.remove(at).creation;
        }
        let domain = host.text;
        store.cookies.push(Cookie {
            name: parsed.name,
            value: parsed.value,
            domain: domain.clone(),
            path,
            expiry: parsed.expiry,
            secure: parsed.secure,
            same_site: parsed.same_site,
            creation,
            last_access: tick,
        });
        // A cookie that came expired deletes the one it replaced and goes itself.
        store.evict_expired(now);
        store.evict_excess(&domain);
    }

    /// The value of the `Cookie` field of a request to `url` with `method`, `site` as [`site`]
    /// decided it, at `now` (5.8.3), or `None` when no cookie is sent. Updates the last access of
    /// each cookie sent.
    pub fn cookie_field(
        &self,
        url: &Url,
        site: Site,
        method: Method,
        now: SystemTime,
    ) -> Option<String> {
        let now = seconds(now);
        let host = request_host(url)?;
        let secure_request = url.scheme() == "https";
        let request_path = url.serialize_path();
        let request_path = if request_path.is_empty() {
            "/".to_owned()
        } else {
            request_path
        };
        let mut store = self.lock();
        store.evict_expired(now);
        let mut chosen: Vec<usize> = store
            .cookies
            .iter()
            .enumerate()
            .filter(|(_, cookie)| {
                cookie.domain == host.text
                    && path_matches(&request_path, &cookie.path)
                    && (!cookie.secure || secure_request)
                    && match (site, cookie.same_site) {
                        (Site::Same, _) | (Site::Cross, SameSite::None) => true,
                        (Site::Cross, SameSite::Lax | SameSite::Default) => method == Method::Get,
                        (Site::Cross, SameSite::Strict) => false,
                    }
            })
            .map(|(at, _)| at)
            .collect();
        chosen.sort_by(|&a, &b| {
            let (a, b) = (&store.cookies[a], &store.cookies[b]);
            b.path
                .len()
                .cmp(&a.path.len())
                .then(a.creation.cmp(&b.creation))
        });
        let mut field = String::new();
        let mut sent = Vec::new();
        for at in chosen {
            let cookie = &store.cookies[at];
            let pair_length = if cookie.name.is_empty() {
                cookie.value.len()
            } else {
                cookie.name.len() + 1 + cookie.value.len()
            };
            let separator = if field.is_empty() { 0 } else { 2 };
            if field.len() + separator + pair_length > COOKIE_FIELD_BYTES_MAX {
                break;
            }
            if !field.is_empty() {
                field.push_str("; ");
            }
            if !cookie.name.is_empty() {
                field.push_str(&cookie.name);
                field.push('=');
            }
            field.push_str(&cookie.value);
            sent.push(at);
        }
        if sent.is_empty() {
            return None;
        }
        let tick = store.tick();
        for at in sent {
            store.cookies[at].last_access = tick;
        }
        Some(field)
    }

    /// The number of cookies the jar holds at `now`, expired ones evicted first.
    pub fn count(&self, now: SystemTime) -> usize {
        let mut store = self.lock();
        store.evict_expired(seconds(now));
        store.cookies.len()
    }
}

/// Whether a request to `url` whose navigation `initiator` started is same-site (5.2, surf's
/// reading without the public suffix list): with no initiator, a navigation the person at the
/// keyboard started, it is; otherwise it is when `url` has the scheme and the host of
/// `initiator`, a serialised origin such as `http://127.0.0.1:8080`, ports ignored. An
/// initiator that does not parse as a URL with a host makes the request cross-site.
pub fn site(initiator: Option<&str>, url: &Url) -> Site {
    let Some(initiator) = initiator else {
        return Site::Same;
    };
    let Ok(origin) = Url::parse(initiator, None) else {
        return Site::Cross;
    };
    let same = origin.scheme() == url.scheme()
        && origin.host().is_some()
        && origin.host().map(Host::serialize) == url.host().map(Host::serialize);
    if same { Site::Same } else { Site::Cross }
}

/// A date of `Expires` by the cookie-date algorithm (5.1.1), as seconds since the epoch (UTC),
/// or `None` when it does not parse: a field missing, a day of month outside 1 to 31, a year
/// below 1601, an hour over 23, a minute or second over 59, or a date that does not exist.
pub fn parse_date(text: &str) -> Option<i64> {
    let mut time: Option<(i64, i64, i64)> = None;
    let mut day: Option<i64> = None;
    let mut month: Option<i64> = None;
    let mut year: Option<i64> = None;
    for token in text.as_bytes().split(|&b| is_date_delimiter(b)) {
        if token.is_empty() {
            continue;
        }
        if time.is_none()
            && let Some(found) = date_time(token)
        {
            time = Some(found);
            continue;
        }
        if day.is_none()
            && let Some(found) = digits_then_rest(token, 1, 2)
        {
            day = Some(found);
            continue;
        }
        if month.is_none()
            && let Some(found) = date_month(token)
        {
            month = Some(found);
            continue;
        }
        if year.is_none()
            && let Some(found) = digits_then_rest(token, 2, 4)
        {
            year = Some(found);
        }
    }
    let (hour, minute, second) = time?;
    let (day, month, mut year) = (day?, month?, year?);
    if (70..=99).contains(&year) {
        year += 1900;
    } else if (0..=69).contains(&year) {
        year += 2000;
    }
    if !(1..=31).contains(&day) || year < 1601 || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    if day > days_in_month(year, month) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + minute * 60 + second)
}

/// A delimiter of the cookie-date grammar: %x09 / %x20-2F / %x3B-40 / %x5B-60 / %x7B-7E.
fn is_date_delimiter(b: u8) -> bool {
    matches!(b, 0x09 | 0x20..=0x2f | 0x3b..=0x40 | 0x5b..=0x60 | 0x7b..=0x7e)
}

/// `min` to `max` digits, then nothing or a non-digit and anything: their value.
fn digits_then_rest(token: &[u8], min: usize, max: usize) -> Option<i64> {
    let count = token.iter().take_while(|b| b.is_ascii_digit()).count();
    if count < min || count > max {
        return None;
    }
    token[..count]
        .iter()
        .try_fold(0i64, |sum, b| Some(sum * 10 + i64::from(b - b'0')))
}

/// `hms-time [ non-digit *OCTET ]`: three fields of one or two digits separated by `:`.
fn date_time(token: &[u8]) -> Option<(i64, i64, i64)> {
    let mut fields = [0i64; 3];
    let mut rest = token;
    for (at, field) in fields.iter_mut().enumerate() {
        let count = rest.iter().take_while(|b| b.is_ascii_digit()).count();
        if !(1..=2).contains(&count) {
            return None;
        }
        *field = rest[..count]
            .iter()
            .fold(0, |sum, b| sum * 10 + i64::from(b - b'0'));
        rest = &rest[count..];
        if at < 2 {
            rest = rest.strip_prefix(b":")?;
        }
    }
    // After the time, nothing or a non-digit (and anything after it).
    if rest.first().is_some_and(u8::is_ascii_digit) {
        return None;
    }
    Some((fields[0], fields[1], fields[2]))
}

/// The month of a token whose first three characters name one, in any case.
fn date_month(token: &[u8]) -> Option<i64> {
    const MONTHS: [&[u8]; 12] = [
        b"jan", b"feb", b"mar", b"apr", b"may", b"jun", b"jul", b"aug", b"sep", b"oct", b"nov",
        b"dec",
    ];
    let head = token.get(..3)?;
    MONTHS
        .iter()
        .position(|month| head.eq_ignore_ascii_case(month))
        .and_then(|at| i64::try_from(at + 1).ok())
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 of a date of the proleptic Gregorian calendar.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// `now` as whole seconds since the epoch; a time before it is negative.
fn seconds(now: SystemTime) -> i64 {
    match now.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_secs()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_secs()).map_or(i64::MIN, |s| -s),
    }
}

/// The canonicalized host of a request, and whether it is an IP address.
struct RequestHost {
    text: String,
    is_ip: bool,
}

fn request_host(url: &Url) -> Option<RequestHost> {
    match url.host()? {
        Host::Domain(name) => Some(RequestHost {
            text: name.clone(),
            is_ip: false,
        }),
        host @ (Host::Ipv4(_) | Host::Ipv6(_)) => Some(RequestHost {
            text: host.serialize(),
            is_ip: true,
        }),
        Host::Opaque(_) | Host::Empty => None,
    }
}

/// Whether a stored domain is an IP address, as [`request_host`] wrote it.
fn is_ip(domain: &str) -> bool {
    domain.starts_with('[') || domain.parse::<std::net::Ipv4Addr>().is_ok()
}

/// Domain-match (5.1.3): `host` equals `domain`, or `host` is a name that ends with `.` and
/// `domain`.
fn domain_matches(host: &str, host_is_ip: bool, domain: &str) -> bool {
    if host == domain {
        return true;
    }
    !host_is_ip
        && host.len() > domain.len()
        && host.ends_with(domain)
        && host.as_bytes()[host.len() - domain.len() - 1] == b'.'
}

/// Path-match (5.1.4): `cookie_path` equals `request_path`, or is a prefix of it that ends with
/// `/` or is followed by `/`.
fn path_matches(request_path: &str, cookie_path: &str) -> bool {
    if request_path == cookie_path {
        return true;
    }
    request_path.starts_with(cookie_path)
        && (cookie_path.ends_with('/')
            || request_path.as_bytes().get(cookie_path.len()) == Some(&b'/'))
}

/// The default-path of `url` (5.1.4): its path up to, not including, the last `/`, or `/` when
/// that leaves nothing.
fn default_path(url: &Url) -> String {
    let path = url.serialize_path();
    if !path.starts_with('/') {
        return "/".to_owned();
    }
    match path.rfind('/') {
        Some(0) | None => "/".to_owned(),
        Some(last) => path[..last].to_owned(),
    }
}

fn starts_with_ignoring_case(text: &str, prefix: &str) -> bool {
    text.as_bytes()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

/// A `Set-Cookie` field value parsed (5.6), before the storage model.
struct Parsed {
    name: String,
    value: String,
    /// The last `Domain` attribute, without its leading `.` and lower-cased.
    domain: Option<String>,
    /// The last `Path` attribute: its value when it starts with `/`, `Some(None)` for one that
    /// stands for the default-path (5.6.4).
    path: Option<Option<String>>,
    expiry: Option<i64>,
    secure: bool,
    same_site: SameSite,
}

/// Whether `b` is a control character other than HTAB (5.6 step 1).
fn is_control(b: u8) -> bool {
    matches!(b, 0x00..=0x08 | 0x0a..=0x1f | 0x7f)
}

fn trim_wsp(text: &str) -> &str {
    text.trim_matches([' ', '\t'])
}

/// The parsing algorithm of 5.6, with the lifetime of 5.6.1, 5.6.2 and 5.7 step 6 computed at
/// `now`. `None` when the whole string is ignored.
fn parse_set_cookie(set_cookie: &str, now: i64) -> Option<Parsed> {
    if set_cookie.bytes().any(is_control) {
        return None;
    }
    let (pair, attributes) = match set_cookie.split_once(';') {
        Some((pair, attributes)) => (pair, Some(attributes)),
        None => (set_cookie, None),
    };
    let (name, value) = match pair.split_once('=') {
        Some((name, value)) => (trim_wsp(name), trim_wsp(value)),
        None => ("", trim_wsp(pair)),
    };
    if name.len() + value.len() > NAME_VALUE_BYTES_MAX {
        return None;
    }
    let mut parsed = Parsed {
        name: name.to_owned(),
        value: value.to_owned(),
        domain: None,
        path: None,
        expiry: None,
        secure: false,
        same_site: SameSite::Default,
    };
    let mut max_age: Option<i64> = None;
    let mut expires: Option<i64> = None;
    let latest = now.saturating_add(AGE_MAX_SECONDS);
    for attribute in attributes.into_iter().flat_map(|rest| rest.split(';')) {
        let (key, attribute_value) = match attribute.split_once('=') {
            Some((key, attribute_value)) => (trim_wsp(key), trim_wsp(attribute_value)),
            None => (trim_wsp(attribute), ""),
        };
        if attribute_value.len() > ATTRIBUTE_VALUE_BYTES_MAX {
            continue;
        }
        if key.eq_ignore_ascii_case("Expires") {
            if let Some(date) = parse_date(attribute_value) {
                expires = Some(date.min(latest));
            }
        } else if key.eq_ignore_ascii_case("Max-Age") {
            let digits = attribute_value.strip_prefix('-').unwrap_or(attribute_value);
            let well_formed = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
            if well_formed {
                let negative = attribute_value.starts_with('-');
                // A number too large for i64 is past the 400 days anyway.
                let seconds = digits.parse::<i64>().unwrap_or(i64::MAX);
                max_age = Some(if negative || seconds == 0 {
                    i64::MIN
                } else {
                    now.saturating_add(seconds.min(AGE_MAX_SECONDS))
                });
            }
        } else if key.eq_ignore_ascii_case("Domain") {
            if !attribute_value.is_empty() {
                let domain = attribute_value.strip_prefix('.').unwrap_or(attribute_value);
                parsed.domain = Some(domain.to_ascii_lowercase());
            }
        } else if key.eq_ignore_ascii_case("Path") {
            parsed.path = Some(
                attribute_value
                    .starts_with('/')
                    .then(|| attribute_value.to_owned()),
            );
        } else if key.eq_ignore_ascii_case("Secure") {
            parsed.secure = true;
        } else if key.eq_ignore_ascii_case("SameSite") {
            parsed.same_site = if attribute_value.eq_ignore_ascii_case("None") {
                SameSite::None
            } else if attribute_value.eq_ignore_ascii_case("Strict") {
                SameSite::Strict
            } else if attribute_value.eq_ignore_ascii_case("Lax") {
                SameSite::Lax
            } else {
                SameSite::Default
            };
        }
        // HttpOnly is stored and sent as any cookie: surf has no non-HTTP retrieval.
    }
    parsed.expiry = max_age.or(expires);
    Some(parsed)
}
