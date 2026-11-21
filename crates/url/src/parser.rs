//! URL Standard, "URL parsing": the basic URL parser's state machine, without the `url` and
//! `state override` arguments and with UTF-8 as its only encoding.

use crate::host::{parse_host, Host};
use crate::percent::{self, EncodeSet};
use crate::{
    default_port, is_special_scheme, ParseError, Path, Url, ValidationError, MAX_INPUT_LENGTH,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    SchemeStart,
    Scheme,
    NoScheme,
    SpecialRelativeOrAuthority,
    PathOrAuthority,
    Relative,
    RelativeSlash,
    SpecialAuthoritySlashes,
    SpecialAuthorityIgnoreSlashes,
    Authority,
    Host,
    Port,
    File,
    FileSlash,
    FileHost,
    PathStart,
    Path,
    OpaquePath,
    Query,
    Fragment,
}

fn invalid(error: ValidationError) -> ParseError {
    ParseError::Invalid(error)
}

/// A Windows drive letter: an ASCII alpha and `:` or `|`.
fn is_windows_drive_letter(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && matches!(bytes[1], b':' | b'|')
}

/// A normalized Windows drive letter: an ASCII alpha and `:`.
fn is_normalized_windows_drive_letter(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Whether the code points `rest` start with a Windows drive letter.
fn starts_with_windows_drive_letter(rest: &[char]) -> bool {
    match rest {
        [a, b] => a.is_ascii_alphabetic() && matches!(b, ':' | '|'),
        [a, b, c, ..] => {
            a.is_ascii_alphabetic() && matches!(b, ':' | '|') && matches!(c, '/' | '\\' | '?' | '#')
        }
        _ => false,
    }
}

fn is_single_dot(segment: &str) -> bool {
    segment == "." || segment.eq_ignore_ascii_case("%2e")
}

fn is_double_dot(segment: &str) -> bool {
    segment == ".."
        || segment.eq_ignore_ascii_case(".%2e")
        || segment.eq_ignore_ascii_case("%2e.")
        || segment.eq_ignore_ascii_case("%2e%2e")
}

impl Url {
    fn new_record() -> Url {
        Url {
            scheme: String::new(),
            username: String::new(),
            password: String::new(),
            host: None,
            port: None,
            path: Path::Segments(Vec::new()),
            query: None,
            fragment: None,
        }
    }

    /// Shorten the URL's path.
    fn shorten_path(&mut self) {
        let is_file = self.scheme == "file";
        if let Path::Segments(path) = &mut self.path {
            if is_file && path.len() == 1 && is_normalized_windows_drive_letter(&path[0]) {
                return;
            }
            path.pop();
        }
    }

    fn push_segment(&mut self, segment: String) {
        if let Path::Segments(path) = &mut self.path {
            path.push(segment);
        }
    }

    fn path_is_empty(&self) -> bool {
        matches!(&self.path, Path::Segments(path) if path.is_empty())
    }
}

/// The host parser, with the opaque-host parser's empty result as the empty host.
fn host_of(buffer: &str, special: bool) -> Result<Host, ParseError> {
    match parse_host(buffer, !special)? {
        Host::Opaque(text) if text.is_empty() => Ok(Host::Empty),
        host => Ok(host),
    }
}

pub(crate) fn parse(input: &str, base: Option<&Url>) -> Result<Url, ParseError> {
    if input.len() > MAX_INPUT_LENGTH {
        return Err(ParseError::TooLong);
    }
    // Remove leading and trailing C0 control or space, then every ASCII tab or newline.
    let input: Vec<char> = input
        .trim_matches(|c: char| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let length = input.len() as isize;
    let at = |pointer: isize| -> Option<char> {
        usize::try_from(pointer)
            .ok()
            .and_then(|i| input.get(i).copied())
    };
    // The code points after the one the pointer points to.
    let remaining = |pointer: isize| -> &[char] {
        usize::try_from(pointer + 1)
            .ok()
            .and_then(|i| input.get(i..))
            .unwrap_or(&[])
    };
    // The code points from the pointer to the end.
    let from = |pointer: isize| -> &[char] {
        usize::try_from(pointer)
            .ok()
            .and_then(|i| input.get(i..))
            .unwrap_or(&[])
    };

    let mut url = Url::new_record();
    let mut state = State::SchemeStart;
    let mut buffer = String::new();
    let mut at_sign_seen = false;
    let mut inside_brackets = false;
    let mut password_token_seen = false;
    let mut pointer: isize = 0;

    loop {
        let c = at(pointer);
        let special = is_special_scheme(&url.scheme);
        match state {
            State::SchemeStart => {
                if let Some(ch) = c.filter(char::is_ascii_alphabetic) {
                    buffer.push(ch.to_ascii_lowercase());
                    state = State::Scheme;
                } else {
                    state = State::NoScheme;
                    pointer -= 1;
                }
            }
            State::Scheme => match c {
                Some(ch) if ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.') => {
                    buffer.push(ch.to_ascii_lowercase());
                }
                Some(':') => {
                    url.scheme = std::mem::take(&mut buffer);
                    let special = is_special_scheme(&url.scheme);
                    if url.scheme == "file" {
                        state = State::File;
                    } else if special && base.is_some_and(|base| base.scheme == url.scheme) {
                        state = State::SpecialRelativeOrAuthority;
                    } else if special {
                        state = State::SpecialAuthoritySlashes;
                    } else if remaining(pointer).first() == Some(&'/') {
                        state = State::PathOrAuthority;
                        pointer += 1;
                    } else {
                        url.path = Path::Opaque(String::new());
                        state = State::OpaquePath;
                    }
                }
                _ => {
                    buffer.clear();
                    state = State::NoScheme;
                    pointer = -1;
                }
            },
            State::NoScheme => {
                let Some(base) = base else {
                    return Err(invalid(ValidationError::MissingSchemeNonRelativeUrl));
                };
                let base_opaque = matches!(base.path, Path::Opaque(_));
                if base_opaque && c != Some('#') {
                    return Err(invalid(ValidationError::MissingSchemeNonRelativeUrl));
                } else if base_opaque {
                    url.scheme = base.scheme.clone();
                    url.path = base.path.clone();
                    url.query = base.query.clone();
                    url.fragment = Some(String::new());
                    state = State::Fragment;
                } else if base.scheme != "file" {
                    state = State::Relative;
                    pointer -= 1;
                } else {
                    state = State::File;
                    pointer -= 1;
                }
            }
            State::SpecialRelativeOrAuthority => {
                if c == Some('/') && remaining(pointer).first() == Some(&'/') {
                    state = State::SpecialAuthorityIgnoreSlashes;
                    pointer += 1;
                } else {
                    state = State::Relative;
                    pointer -= 1;
                }
            }
            State::PathOrAuthority => {
                if c == Some('/') {
                    state = State::Authority;
                } else {
                    state = State::Path;
                    pointer -= 1;
                }
            }
            State::Relative => {
                let Some(base) = base else {
                    return Err(invalid(ValidationError::MissingSchemeNonRelativeUrl));
                };
                url.scheme = base.scheme.clone();
                let special = is_special_scheme(&url.scheme);
                if c == Some('/') || (special && c == Some('\\')) {
                    state = State::RelativeSlash;
                } else {
                    url.username = base.username.clone();
                    url.password = base.password.clone();
                    url.host = base.host.clone();
                    url.port = base.port;
                    url.path = base.path.clone();
                    url.query = base.query.clone();
                    match c {
                        Some('?') => {
                            url.query = Some(String::new());
                            state = State::Query;
                        }
                        Some('#') => {
                            url.fragment = Some(String::new());
                            state = State::Fragment;
                        }
                        Some(_) => {
                            url.query = None;
                            url.shorten_path();
                            state = State::Path;
                            pointer -= 1;
                        }
                        None => {}
                    }
                }
            }
            State::RelativeSlash => {
                if special && matches!(c, Some('/' | '\\')) {
                    state = State::SpecialAuthorityIgnoreSlashes;
                } else if c == Some('/') {
                    state = State::Authority;
                } else {
                    let Some(base) = base else {
                        return Err(invalid(ValidationError::MissingSchemeNonRelativeUrl));
                    };
                    url.username = base.username.clone();
                    url.password = base.password.clone();
                    url.host = base.host.clone();
                    url.port = base.port;
                    state = State::Path;
                    pointer -= 1;
                }
            }
            State::SpecialAuthoritySlashes => {
                if c == Some('/') && remaining(pointer).first() == Some(&'/') {
                    pointer += 1;
                } else {
                    pointer -= 1;
                }
                state = State::SpecialAuthorityIgnoreSlashes;
            }
            State::SpecialAuthorityIgnoreSlashes => {
                if !matches!(c, Some('/' | '\\')) {
                    state = State::Authority;
                    pointer -= 1;
                }
            }
            State::Authority => match c {
                Some('@') => {
                    if at_sign_seen {
                        buffer.insert_str(0, "%40");
                    }
                    at_sign_seen = true;
                    for code_point in buffer.chars() {
                        if code_point == ':' && !password_token_seen {
                            password_token_seen = true;
                            continue;
                        }
                        let target = if password_token_seen {
                            &mut url.password
                        } else {
                            &mut url.username
                        };
                        percent::utf8_percent_encode_char(target, code_point, EncodeSet::Userinfo);
                    }
                    buffer.clear();
                }
                None | Some('/' | '?' | '#') => {
                    if at_sign_seen && buffer.is_empty() {
                        return Err(invalid(ValidationError::HostMissing));
                    }
                    pointer -= buffer.chars().count() as isize + 1;
                    buffer.clear();
                    state = State::Host;
                }
                Some('\\') if special => {
                    if at_sign_seen && buffer.is_empty() {
                        return Err(invalid(ValidationError::HostMissing));
                    }
                    pointer -= buffer.chars().count() as isize + 1;
                    buffer.clear();
                    state = State::Host;
                }
                Some(ch) => buffer.push(ch),
            },
            State::Host => {
                if c == Some(':') && !inside_brackets {
                    if buffer.is_empty() {
                        return Err(invalid(ValidationError::HostMissing));
                    }
                    url.host = Some(host_of(&buffer, special)?);
                    buffer.clear();
                    state = State::Port;
                } else if matches!(c, None | Some('/' | '?' | '#')) || (special && c == Some('\\'))
                {
                    pointer -= 1;
                    if special && buffer.is_empty() {
                        return Err(invalid(ValidationError::HostMissing));
                    }
                    url.host = Some(host_of(&buffer, special)?);
                    buffer.clear();
                    state = State::PathStart;
                } else if let Some(ch) = c {
                    if ch == '[' {
                        inside_brackets = true;
                    }
                    if ch == ']' {
                        inside_brackets = false;
                    }
                    buffer.push(ch);
                }
            }
            State::Port => {
                if let Some(ch) = c.filter(char::is_ascii_digit) {
                    buffer.push(ch);
                } else if matches!(c, None | Some('/' | '?' | '#')) || (special && c == Some('\\'))
                {
                    if !buffer.is_empty() {
                        let port = buffer.bytes().fold(0u32, |n, digit| {
                            n.saturating_mul(10).saturating_add(u32::from(digit - b'0'))
                        });
                        let Ok(port) = u16::try_from(port) else {
                            return Err(invalid(ValidationError::PortOutOfRange));
                        };
                        url.port = if default_port(&url.scheme) == Some(port) {
                            None
                        } else {
                            Some(port)
                        };
                        buffer.clear();
                    }
                    state = State::PathStart;
                    pointer -= 1;
                } else {
                    return Err(invalid(ValidationError::PortInvalid));
                }
            }
            State::File => {
                url.scheme = "file".to_string();
                url.host = Some(Host::Empty);
                if matches!(c, Some('/' | '\\')) {
                    state = State::FileSlash;
                } else if let Some(base) = base.filter(|base| base.scheme == "file") {
                    url.host = base.host.clone();
                    url.path = base.path.clone();
                    url.query = base.query.clone();
                    match c {
                        Some('?') => {
                            url.query = Some(String::new());
                            state = State::Query;
                        }
                        Some('#') => {
                            url.fragment = Some(String::new());
                            state = State::Fragment;
                        }
                        Some(_) => {
                            url.query = None;
                            if starts_with_windows_drive_letter(from(pointer)) {
                                url.path = Path::Segments(Vec::new());
                            } else {
                                url.shorten_path();
                            }
                            state = State::Path;
                            pointer -= 1;
                        }
                        None => {}
                    }
                } else {
                    state = State::Path;
                    pointer -= 1;
                }
            }
            State::FileSlash => {
                if matches!(c, Some('/' | '\\')) {
                    state = State::FileHost;
                } else {
                    if let Some(base) = base.filter(|base| base.scheme == "file") {
                        url.host = base.host.clone();
                        if !starts_with_windows_drive_letter(from(pointer)) {
                            if let Path::Segments(base_path) = &base.path {
                                if let Some(first) = base_path.first() {
                                    if is_normalized_windows_drive_letter(first) {
                                        url.push_segment(first.clone());
                                    }
                                }
                            }
                        }
                    }
                    state = State::Path;
                    pointer -= 1;
                }
            }
            State::FileHost => {
                if matches!(c, None | Some('/' | '\\' | '?' | '#')) {
                    pointer -= 1;
                    if is_windows_drive_letter(&buffer) {
                        state = State::Path;
                    } else if buffer.is_empty() {
                        url.host = Some(Host::Empty);
                        state = State::PathStart;
                    } else {
                        let mut host = host_of(&buffer, special)?;
                        if host == Host::Domain("localhost".to_string()) {
                            host = Host::Empty;
                        }
                        url.host = Some(host);
                        buffer.clear();
                        state = State::PathStart;
                    }
                } else if let Some(ch) = c {
                    buffer.push(ch);
                }
            }
            State::PathStart => {
                if special {
                    state = State::Path;
                    if !matches!(c, Some('/' | '\\')) {
                        pointer -= 1;
                    }
                } else if c == Some('?') {
                    url.query = Some(String::new());
                    state = State::Query;
                } else if c == Some('#') {
                    url.fragment = Some(String::new());
                    state = State::Fragment;
                } else if let Some(ch) = c {
                    state = State::Path;
                    if ch != '/' {
                        pointer -= 1;
                    }
                }
            }
            State::Path => {
                let slash = c == Some('/') || (special && c == Some('\\'));
                if c.is_none() || slash || matches!(c, Some('?' | '#')) {
                    if is_double_dot(&buffer) {
                        url.shorten_path();
                        if !slash {
                            url.push_segment(String::new());
                        }
                    } else if is_single_dot(&buffer) {
                        if !slash {
                            url.push_segment(String::new());
                        }
                    } else {
                        if url.scheme == "file"
                            && url.path_is_empty()
                            && is_windows_drive_letter(&buffer)
                        {
                            buffer.replace_range(1..2, ":");
                        }
                        url.push_segment(std::mem::take(&mut buffer));
                    }
                    buffer.clear();
                    if c == Some('?') {
                        url.query = Some(String::new());
                        state = State::Query;
                    }
                    if c == Some('#') {
                        url.fragment = Some(String::new());
                        state = State::Fragment;
                    }
                } else if let Some(ch) = c {
                    percent::utf8_percent_encode_char(&mut buffer, ch, EncodeSet::Path);
                }
            }
            State::OpaquePath => match c {
                Some('?') => {
                    url.query = Some(String::new());
                    state = State::Query;
                }
                Some('#') => {
                    url.fragment = Some(String::new());
                    state = State::Fragment;
                }
                Some(ch) => {
                    if let Path::Opaque(path) = &mut url.path {
                        if ch == ' ' {
                            if matches!(remaining(pointer).first(), Some('?' | '#')) {
                                path.push_str("%20");
                            } else {
                                path.push(' ');
                            }
                        } else {
                            percent::utf8_percent_encode_char(path, ch, EncodeSet::C0Control);
                        }
                    }
                }
                None => {}
            },
            State::Query => {
                if matches!(c, None | Some('#')) {
                    let set = if special {
                        EncodeSet::SpecialQuery
                    } else {
                        EncodeSet::Query
                    };
                    let query = url.query.get_or_insert_with(String::new);
                    percent::utf8_percent_encode_into(query, &buffer, set);
                    buffer.clear();
                    if c == Some('#') {
                        url.fragment = Some(String::new());
                        state = State::Fragment;
                    }
                } else if let Some(ch) = c {
                    buffer.push(ch);
                }
            }
            State::Fragment => {
                if let Some(ch) = c {
                    let fragment = url.fragment.get_or_insert_with(String::new);
                    percent::utf8_percent_encode_char(fragment, ch, EncodeSet::Fragment);
                }
            }
        }
        if pointer >= length {
            break;
        }
        pointer += 1;
    }
    Ok(url)
}
