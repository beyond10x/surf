//! The one writer of the view's JSON, by the rules of `docs/view-stream.md` ("Writing"): no
//! whitespace outside strings, members in the document's order, absent values `null`, numbers in
//! decimal; in a string `"`, `\`, every character below U+0020, U+007F to U+009F, U+2028,
//! U+2029, `<`, `>` and `&` as `\u` and four lower-case hexadecimal digits, every other
//! character as itself in UTF-8. So no output holds a line end, a raw `<`, `>` or `&`.

use crate::model::{
    Action, Change, Changed, ChangedText, ControlChange, ControlState, ControlValue, Detail, Entry,
    EntryKind, Now, NowElement, PageLine, Path, PathAction, Reset, SessionList, SessionState,
    Submits, Summary,
};

/// A value the writer writes.
pub trait ToJson {
    /// Append the JSON of `self` to `out`.
    fn write_json(&self, out: &mut String);
}

/// The JSON of `value`, as bytes.
pub fn to_json<T: ToJson + ?Sized>(value: &T) -> Vec<u8> {
    let mut out = String::new();
    value.write_json(&mut out);
    out.into_bytes()
}

impl ToJson for str {
    fn write_json(&self, out: &mut String) {
        out.push('"');
        for character in self.chars() {
            let escaped = matches!(
                character,
                '"' | '\\'
                    | '\u{0}'..='\u{1f}'
                    | '\u{7f}'..='\u{9f}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '<'
                    | '>'
                    | '&'
            );
            if escaped {
                out.push_str(&format!("\\u{:04x}", u32::from(character)));
            } else {
                out.push(character);
            }
        }
        out.push('"');
    }
}

impl ToJson for String {
    fn write_json(&self, out: &mut String) {
        self.as_str().write_json(out);
    }
}

impl ToJson for u64 {
    fn write_json(&self, out: &mut String) {
        out.push_str(&self.to_string());
    }
}

impl ToJson for bool {
    fn write_json(&self, out: &mut String) {
        out.push_str(if *self { "true" } else { "false" });
    }
}

impl<T: ToJson> ToJson for Option<T> {
    fn write_json(&self, out: &mut String) {
        match self {
            Some(value) => value.write_json(out),
            None => out.push_str("null"),
        }
    }
}

impl<T: ToJson> ToJson for [T] {
    fn write_json(&self, out: &mut String) {
        out.push('[');
        for (at, value) in self.iter().enumerate() {
            if at > 0 {
                out.push(',');
            }
            value.write_json(out);
        }
        out.push(']');
    }
}

impl<T: ToJson> ToJson for Vec<T> {
    fn write_json(&self, out: &mut String) {
        self.as_slice().write_json(out);
    }
}

/// An object written member by member, in the order they are added.
struct Object<'a> {
    out: &'a mut String,
    first: bool,
}

impl<'a> Object<'a> {
    fn open(out: &'a mut String) -> Self {
        out.push('{');
        Object { out, first: true }
    }

    fn member(mut self, name: &str, value: &(impl ToJson + ?Sized)) -> Self {
        if !self.first {
            self.out.push(',');
        }
        self.first = false;
        name.write_json(self.out);
        self.out.push(':');
        value.write_json(self.out);
        self
    }

    fn close(self) {
        self.out.push('}');
    }
}

impl ToJson for PageLine {
    fn write_json(&self, out: &mut String) {
        match self {
            PageLine::None => Object::open(out).member("kind", "none").close(),
            PageLine::Loaded {
                status,
                media_type,
                address,
                title,
            } => Object::open(out)
                .member("kind", "loaded")
                .member("status", &u64::from(*status))
                .member("media_type", media_type)
                .member("address", address)
                .member("title", title)
                .close(),
            PageLine::Failed { reason, address } => Object::open(out)
                .member("kind", "failed")
                .member("reason", reason)
                .member("address", address)
                .close(),
        }
    }
}

impl ToJson for SessionState {
    fn write_json(&self, out: &mut String) {
        match self {
            SessionState::Open => "open",
            SessionState::Closed => "closed",
            SessionState::Died => "died",
        }
        .write_json(out);
    }
}

impl ToJson for Summary {
    fn write_json(&self, out: &mut String) {
        Object::open(out)
            .member("id", &self.id)
            .member("state", &self.state)
            .member("address", &self.address)
            .member("title", &self.title)
            .member("opened_at", &self.opened_at)
            .member("last_action_at", &self.last_action_at)
            .member("ended_at", &self.ended_at)
            .member("signal", &self.signal)
            .member("page", &self.page)
            .close();
    }
}

impl ToJson for SessionList {
    fn write_json(&self, out: &mut String) {
        Object::open(out)
            .member("newest", &self.newest)
            .member("sessions", &self.sessions)
            .close();
    }
}

impl ToJson for Detail {
    fn write_json(&self, out: &mut String) {
        Object::open(out)
            .member("session", &self.session)
            .member("dropped", &self.dropped)
            .member("log_dropped", &self.log_dropped)
            .member("unreadable", &self.unreadable)
            .member("entries", &self.entries)
            .close();
    }
}

impl Action {
    fn members<'a>(&self, object: Object<'a>) -> Object<'a> {
        object
            .member("sequence", &self.sequence)
            .member("command", &self.command)
            .member("argument", &self.argument)
    }
}

impl ToJson for Action {
    fn write_json(&self, out: &mut String) {
        self.members(Object::open(out)).close();
    }
}

impl ToJson for PathAction {
    fn write_json(&self, out: &mut String) {
        Object::open(out)
            .member("sequence", &self.sequence)
            .member("command", &self.command)
            .member("handle", &self.handle)
            .member("argument", &self.argument)
            .member("argument_cut", &self.argument_cut)
            .close();
    }
}

impl ToJson for Path {
    fn write_json(&self, out: &mut String) {
        Object::open(out)
            .member("shown", &self.shown)
            .member("total", &self.total)
            .member("actions", &self.actions)
            .close();
    }
}

impl ToJson for NowElement {
    fn write_json(&self, out: &mut String) {
        Object::open(out)
            .member("handle", &self.handle)
            .member("kind", &self.kind)
            .member("visibility", &self.visibility)
            .member("label", &self.label)
            .member("label_cut", &self.label_cut)
            .member("target", &self.target)
            .member("target_cut", &self.target_cut)
            .member("unresolved", &self.unresolved)
            .member("text", &self.text)
            .member("state", &self.state)
            .close();
    }
}

impl ToJson for Now {
    fn write_json(&self, out: &mut String) {
        Object::open(out)
            .member("shown", &self.shown)
            .member("total", &self.total)
            .member("elements", &self.elements)
            .member("visibility_note", &self.visibility_note)
            .close();
    }
}

/// The `fields` of an `event` entry: an object of strings, `null` for an absent value.
struct Fields<'a>(&'a [(String, Option<String>)]);

impl ToJson for Fields<'_> {
    fn write_json(&self, out: &mut String) {
        let mut object = Object::open(out);
        for (name, value) in self.0 {
            object = object.member(name, value);
        }
        object.close();
    }
}

impl ToJson for Entry {
    fn write_json(&self, out: &mut String) {
        let object = Object::open(out)
            .member("number", &self.number)
            .member("session", &self.session)
            .member("index", &self.index)
            .member("kept_at", &self.kept_at);
        match &self.kind {
            EntryKind::Action(action) => action.members(object.member("kind", "action")).close(),
            EntryKind::Event {
                name,
                unmapped,
                fields,
            } => object
                .member("kind", "event")
                .member("name", name)
                .member("unmapped", unmapped)
                .member("fields", &Fields(if *unmapped { &[] } else { fields }))
                .close(),
            EntryKind::Answer {
                page,
                path,
                now,
                changed,
            } => object
                .member("kind", "answer")
                .member("page", page)
                .member("path", path)
                .member("now", now)
                .member("changed", changed)
                .close(),
        }
    }
}

impl ToJson for ControlValue {
    fn write_json(&self, out: &mut String) {
        let (kind, text, cut) = match self {
            ControlValue::Text { text, cut } => ("text", Some(text.as_str()), Some(*cut)),
            ControlValue::Set => ("set", None, None),
            ControlValue::Empty => ("empty", None, None),
        };
        Object::open(out)
            .member("kind", kind)
            .member("text", &text.map(str::to_string))
            .member("cut", &cut)
            .close();
    }
}

impl ToJson for Submits {
    fn write_json(&self, out: &mut String) {
        Object::open(out)
            .member("form", &self.form)
            .member("method", &self.method)
            .member("action", &self.action)
            .member("action_cut", &self.action_cut)
            .close();
    }
}

impl ToJson for ControlState {
    fn write_json(&self, out: &mut String) {
        Object::open(out)
            .member("value", &self.value)
            .member("disabled", &self.disabled)
            .member("checked", &self.checked)
            .member("required", &self.required)
            .member("readonly", &self.readonly)
            .member("submits", &self.submits)
            .close();
    }
}

impl ToJson for ControlChange {
    fn write_json(&self, out: &mut String) {
        let object = Object::open(out).member("handle", &self.handle);
        match &self.change {
            Change::Value { before, after } => object
                .member("what", "value")
                .member("before", before)
                .member("after", after),
            Change::Checked { before, after } => object
                .member("what", "checked")
                .member("before", before)
                .member("after", after),
            Change::Selected { before, after } => object
                .member("what", "selected")
                .member("before", before)
                .member("after", after),
        }
        .close();
    }
}

impl ToJson for ChangedText {
    fn write_json(&self, out: &mut String) {
        Object::open(out)
            .member("text", &self.text)
            .member("cut", &self.cut)
            .close();
    }
}

impl ToJson for Changed {
    fn write_json(&self, out: &mut String) {
        match self {
            Changed::Controls(lines) => Object::open(out)
                .member("kind", "controls")
                .member("lines", lines)
                .close(),
            Changed::Page {
                from,
                to,
                texts,
                more,
            } => Object::open(out)
                .member("kind", "page")
                .member("from", from)
                .member("to", to)
                .member("texts", texts)
                .member("more", more)
                .close(),
        }
    }
}

impl ToJson for Reset {
    fn write_json(&self, out: &mut String) {
        Object::open(out).member("newest", &self.newest).close();
    }
}
