//! The title and the actionable elements of a document that `surf-html` parsed: the values
//! the rows of `spec/domains/document.yaml` hold, computed from the tree. The rules are those of
//! the WHATWG HTML Standard at the snapshot `docs/design/html-parser.md` names, and of WAI-ARIA
//! for `aria-hidden`; the decisions are items 5 and 6 of that note.
//!
//! # Rules taken
//!
//! - [`title`]: the `document.title` getter. The title element is the first HTML `title`
//!   element in tree order; the value is its child text content (the data of its text node
//!   children) with ASCII whitespace stripped and collapsed; the empty string when there is
//!   none. Template contents are not in the document and are not searched. The getter's branch
//!   for a document whose document element is an SVG `svg` element is not taken: the HTML
//!   parser always makes an `html` document element.
//! - [`actionable_elements`]: every HTML element under the root, in tree order, that is a
//!   hyperlink (`a` or `area` with an `href` attribute), a `button`, an `input` in any state but
//!   Hidden, a `select` or a `textarea`. The kind is the table of item 6. The `input` element's
//!   `type` is an enumerated attribute: its value is matched ASCII case-insensitively against
//!   the 22 keywords, and a missing or unknown value is the Text state.
//! - `target`: a link's `href` value as written.
//! - `value`: the default value. For the Text, Search, Telephone and Password states the `value`
//!   attribute (the empty string when absent) with newlines stripped; for URL and Email,
//!   newlines stripped and then leading and trailing ASCII whitespace; for Email with the
//!   `multiple` attribute, the value split on commas, each token stripped of leading and
//!   trailing ASCII whitespace, joined with `,`. File Upload: always the empty string (the value
//!   mode "filename" with no selected file). `textarea`: its child text content. `None` for the
//!   other kinds; a `select`'s options are not read.
//! - `checked`: the default checkedness of an `input` in the Checkbox or Radio Button state
//!   (the `checked` attribute is present).
//! - `disabled`: for `button`, `input`, `select` and `textarea`: the `disabled` attribute is
//!   present, or the element is a descendant of a `fieldset` with a `disabled` attribute and not
//!   a descendant of that fieldset's first `legend` element child. Always `false` for a link.
//! - `required`: the `required` attribute is present on an `input` in the Text, Search, URL,
//!   Telephone, Email, Password, Date, Month, Week, Time, Local Date and Time, Number, Checkbox,
//!   Radio Button or File Upload state, or on a `select` or a `textarea`.
//! - `hidden` and [`is_hidden`]: item 5 of the note. A node is hidden when it or an ancestor
//!   element has `aria-hidden` with the value `true`, or matches a rule of the HTML Standard's
//!   user-agent style sheet in 15.3.1 "Hidden elements": the elements `base, basefont, datalist,
//!   head, link, meta, noembed, noframes, param, rp, script, style, template, title`; the
//!   `hidden` attribute on any element but `embed`, whatever its value; an `input` whose `type`
//!   is `hidden` compared ASCII case-insensitively. The style sheet is in the HTML namespace, so
//!   those rules apply to HTML elements only; `aria-hidden` applies to any element. `area` is
//!   left out of the element rule: an `area` is drawn through its image's map, and the parser
//!   gives it no children. `noscript` is not hidden: surf parses with scripting disabled.
//!   `aria-hidden` on the document element or on the body element (the first `body` or
//!   `frameset` child of an HTML `html` document element) is not read: WAI-ARIA 1.3 says user
//!   agents must not expose the hidden state when it is specified on the root element or on the
//!   element that contains the contents of the primary document. The value `true` is compared
//!   exactly: neither WAI-ARIA nor the HTML Standard says the comparison ignores case. A node not
//!   in the document (template contents) is judged only by the ancestors it has.
//!
//! No function recurses over the depth of the tree: [`actionable_elements`] and [`title`] walk
//! with their own stack over [`Tree::children`], [`is_hidden`] walks up [`Tree::parent`].
//! [`actionable_elements`] computes the inherited flags (hidden, inside a disabling fieldset)
//! in that one walk, so its time is linear in the number of nodes.
//!
//! # Deviations
//!
//! - The values of the Number, Range, Color, Date, Month, Week, Time and Local Date and Time
//!   states are the `value` attribute as written (the empty string when absent). The standard's
//!   value sanitization algorithms for them (a value that is not a valid number, date or colour
//!   becomes the empty string, or the range's default, or `#000000`) are not run.
//! - Nothing that needs CSS is read: an author style sheet or `style` attribute that hides an
//!   element does not make it hidden.
//!
//! # Left to other stories
//!
//! - Each element's label, the accessible name (`story:html-accessible-name`).
//! - A control's form owner and the radio button group rule, by which a later checked radio
//!   button unchecks the others of its group: here every radio button with a `checked`
//!   attribute is `checked` (`story:agent-form-port`).
//! - Option selectedness of a `select` (`story:agent-form-port`).
//! - Resolving a link's target against the document base URL (`story:agent-follow-links`).
//! - Writing the rows (`story:html-document-port`).

#![forbid(unsafe_code)]

use surf_html::{Attribute, Namespace, NodeData, NodeId, Tree};

/// The `document.title` value of the document: the child text content of the first HTML
/// `title` element in tree order, ASCII whitespace stripped and collapsed; the empty string when
/// the document has none. Template contents are not searched.
pub fn title(tree: &Tree) -> String {
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if element(tree, node).is_some_and(|e| e.is_html("title")) {
            return strip_and_collapse(&child_text_content(tree, node));
        }
        pending.extend(tree.children(node).iter().rev());
    }
    String::new()
}

/// What an actionable element is: the nine kinds of `surf.document.ElementKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// An `a` or `area` element with an `href` attribute.
    Link,
    /// A `button` element, or an `input` in the Submit Button, Image Button, Reset Button or
    /// Button state.
    Button,
    /// An `input` in the Text, Search, Telephone, URL, Email, Number, Range, Color, Date,
    /// Month, Week, Time or Local Date and Time state.
    Text,
    /// An `input` in the Password state.
    Password,
    /// An `input` in the Checkbox state.
    Checkbox,
    /// An `input` in the Radio Button state.
    Radio,
    /// An `input` in the File Upload state.
    File,
    /// A `select` element.
    Select,
    /// A `textarea` element.
    Textarea,
}

impl Kind {
    /// The kind's name as the specification's `ElementKind` spells it: `link`, `button`,
    /// `text`, `password`, `checkbox`, `radio`, `file`, `select` or `textarea`.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Link => "link",
            Kind::Button => "button",
            Kind::Text => "text",
            Kind::Password => "password",
            Kind::Checkbox => "checkbox",
            Kind::Radio => "radio",
            Kind::File => "file",
            Kind::Select => "select",
            Kind::Textarea => "textarea",
        }
    }
}

/// One actionable element of a document, with the values its row holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionableElement {
    /// The element in the tree.
    pub node: NodeId,
    /// What it is.
    pub kind: Kind,
    /// A link's `href` value as written, `Some` also when empty; `None` for every other kind.
    pub target: Option<String>,
    /// The default value of a `text`, `password`, `file` or `textarea` element; `None` for the
    /// other kinds.
    pub value: Option<String>,
    /// The element or an ancestor is hidden ([`is_hidden`]).
    pub hidden: bool,
    /// The control is disabled; always `false` for a link.
    pub disabled: bool,
    /// The default checkedness of a checkbox or a radio button; `false` for every other kind.
    pub checked: bool,
    /// The `required` attribute is present and applies to the element.
    pub required: bool,
}

/// Every actionable HTML element under the root of `tree`, in tree order as
/// [`Tree::descendants`] gives it (template contents left out). An `input` in the Hidden state
/// is not listed. One walk with its own stack: the time is linear in the number of nodes.
pub fn actionable_elements(tree: &Tree) -> Vec<ActionableElement> {
    struct Pending {
        node: NodeId,
        /// An ancestor of `node` is hidden.
        hidden: bool,
        /// `node` is a descendant of a `fieldset` with a `disabled` attribute and not of that
        /// fieldset's first `legend` child.
        in_disabled_fieldset: bool,
    }

    let primary = Primary::of(tree);
    let mut found = Vec::new();
    let mut pending = vec![Pending {
        node: tree.root(),
        hidden: false,
        in_disabled_fieldset: false,
    }];
    while let Some(Pending {
        node,
        hidden,
        in_disabled_fieldset,
    }) = pending.pop()
    {
        let hidden = hidden || hides(tree, node, &primary);
        let this = element(tree, node);
        if let Some(this) = &this
            && let Some(actionable) = actionable(tree, node, this, hidden, in_disabled_fieldset)
        {
            found.push(actionable);
        }
        let children = tree.children(node);
        let disabling = this
            .as_ref()
            .is_some_and(|e| e.is_html("fieldset") && e.has("disabled"));
        let first_legend = if disabling {
            children
                .iter()
                .copied()
                .find(|&c| element(tree, c).is_some_and(|e| e.is_html("legend")))
        } else {
            None
        };
        for &child in children.iter().rev() {
            pending.push(Pending {
                node: child,
                hidden,
                in_disabled_fieldset: in_disabled_fieldset
                    || (disabling && Some(child) != first_legend),
            });
        }
    }
    found
}

/// Whether `node` is hidden: it or an ancestor element has `aria-hidden="true"` (not read on
/// the document element and the body element) or matches a rule of the HTML Standard's 15.3.1
/// "Hidden elements" (see the crate documentation). A walk up [`Tree::parent`].
///
/// # Panics
///
/// When `node` is not of `tree`.
pub fn is_hidden(tree: &Tree, node: NodeId) -> bool {
    let primary = Primary::of(tree);
    let mut current = Some(node);
    while let Some(next) = current {
        if hides(tree, next, &primary) {
            return true;
        }
        current = tree.parent(next);
    }
    false
}

/// The elements of 15.3.1 "Hidden elements" with `display: none`, `area` left out.
const HIDDEN_ELEMENTS: [&str; 14] = [
    "base", "basefont", "datalist", "head", "link", "meta", "noembed", "noframes", "param", "rp",
    "script", "style", "template", "title",
];

/// The document element and the body element, on which `aria-hidden` is not read.
struct Primary {
    document_element: Option<NodeId>,
    body: Option<NodeId>,
}

impl Primary {
    fn of(tree: &Tree) -> Primary {
        let root = tree.root();
        if !matches!(tree.data(root), NodeData::Document) {
            return Primary {
                document_element: None,
                body: None,
            };
        }
        let document_element = tree
            .children(root)
            .iter()
            .copied()
            .find(|&c| element(tree, c).is_some());
        let body = document_element
            .filter(|&d| element(tree, d).is_some_and(|e| e.is_html("html")))
            .and_then(|d| {
                tree.children(d).iter().copied().find(|&c| {
                    element(tree, c).is_some_and(|e| e.is_html("body") || e.is_html("frameset"))
                })
            });
        Primary {
            document_element,
            body,
        }
    }

    fn contains(&self, node: NodeId) -> bool {
        self.document_element == Some(node) || self.body == Some(node)
    }
}

/// Whether `node` itself is hidden by `aria-hidden` or a rule of 15.3.1, ancestors not read.
fn hides(tree: &Tree, node: NodeId, primary: &Primary) -> bool {
    let Some(this) = element(tree, node) else {
        return false;
    };
    if this.attribute("aria-hidden") == Some("true") && !primary.contains(node) {
        return true;
    }
    if this.namespace != Namespace::Html {
        return false;
    }
    HIDDEN_ELEMENTS.contains(&this.local_name)
        || (this.local_name != "embed" && this.has("hidden"))
        || (this.local_name == "input"
            && this
                .attribute("type")
                .is_some_and(|t| t.eq_ignore_ascii_case("hidden")))
}

/// The actionable element `node` is, if it is one.
fn actionable(
    tree: &Tree,
    node: NodeId,
    this: &Element<'_>,
    hidden: bool,
    in_disabled_fieldset: bool,
) -> Option<ActionableElement> {
    if this.namespace != Namespace::Html {
        return None;
    }
    let mut found = ActionableElement {
        node,
        kind: Kind::Link,
        target: None,
        value: None,
        hidden,
        disabled: false,
        checked: false,
        required: false,
    };
    match this.local_name {
        "a" | "area" => {
            found.target = Some(this.attribute("href")?.to_string());
            return Some(found);
        }
        "button" => found.kind = Kind::Button,
        "select" => {
            found.kind = Kind::Select;
            found.required = this.has("required");
        }
        "textarea" => {
            found.kind = Kind::Textarea;
            found.value = Some(child_text_content(tree, node));
            found.required = this.has("required");
        }
        "input" => {
            let state = State::of(this.attribute("type"));
            found.kind = state.kind()?;
            found.value = state.value(this);
            found.checked = state.has_checkedness() && this.has("checked");
            found.required = state.applies_required() && this.has("required");
        }
        _ => return None,
    }
    found.disabled = this.has("disabled") || in_disabled_fieldset;
    Some(found)
}

/// The states of the `input` element's `type` attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Hidden,
    Text,
    Search,
    Telephone,
    Url,
    Email,
    Password,
    Date,
    Month,
    Week,
    Time,
    LocalDateAndTime,
    Number,
    Range,
    Color,
    Checkbox,
    RadioButton,
    FileUpload,
    SubmitButton,
    ImageButton,
    ResetButton,
    Button,
}

/// The keywords of the `type` attribute and their states.
const KEYWORDS: [(&str, State); 22] = [
    ("hidden", State::Hidden),
    ("text", State::Text),
    ("search", State::Search),
    ("tel", State::Telephone),
    ("url", State::Url),
    ("email", State::Email),
    ("password", State::Password),
    ("date", State::Date),
    ("month", State::Month),
    ("week", State::Week),
    ("time", State::Time),
    ("datetime-local", State::LocalDateAndTime),
    ("number", State::Number),
    ("range", State::Range),
    ("color", State::Color),
    ("checkbox", State::Checkbox),
    ("radio", State::RadioButton),
    ("file", State::FileUpload),
    ("submit", State::SubmitButton),
    ("image", State::ImageButton),
    ("reset", State::ResetButton),
    ("button", State::Button),
];

impl State {
    /// The state of a `type` attribute value: an ASCII case-insensitive match of a keyword,
    /// else the Text state (the missing value default and the invalid value default).
    fn of(value: Option<&str>) -> State {
        value
            .and_then(|v| {
                KEYWORDS
                    .iter()
                    .find(|(keyword, _)| v.eq_ignore_ascii_case(keyword))
            })
            .map_or(State::Text, |&(_, state)| state)
    }

    /// The kind of an `input` in this state; `None` for Hidden, which is not listed.
    fn kind(self) -> Option<Kind> {
        Some(match self {
            State::Hidden => return None,
            State::Text
            | State::Search
            | State::Telephone
            | State::Url
            | State::Email
            | State::Date
            | State::Month
            | State::Week
            | State::Time
            | State::LocalDateAndTime
            | State::Number
            | State::Range
            | State::Color => Kind::Text,
            State::Password => Kind::Password,
            State::Checkbox => Kind::Checkbox,
            State::RadioButton => Kind::Radio,
            State::FileUpload => Kind::File,
            State::SubmitButton | State::ImageButton | State::ResetButton | State::Button => {
                Kind::Button
            }
        })
    }

    /// The default value of an `input` in this state, for the kinds that hold one.
    fn value(self, input: &Element<'_>) -> Option<String> {
        let raw = input.attribute("value").unwrap_or("");
        Some(match self {
            State::Text | State::Search | State::Telephone | State::Password => strip_newlines(raw),
            State::Email if input.has("multiple") => split_on_commas(raw).join(","),
            State::Url | State::Email => strip_newlines(raw).trim_ascii().to_string(),
            State::Date
            | State::Month
            | State::Week
            | State::Time
            | State::LocalDateAndTime
            | State::Number
            | State::Range
            | State::Color => raw.to_string(),
            State::FileUpload => String::new(),
            State::Hidden
            | State::Checkbox
            | State::RadioButton
            | State::SubmitButton
            | State::ImageButton
            | State::ResetButton
            | State::Button => return None,
        })
    }

    fn has_checkedness(self) -> bool {
        matches!(self, State::Checkbox | State::RadioButton)
    }

    /// Whether the `required` attribute applies to an `input` in this state.
    fn applies_required(self) -> bool {
        matches!(
            self,
            State::Text
                | State::Search
                | State::Url
                | State::Telephone
                | State::Email
                | State::Password
                | State::Date
                | State::Month
                | State::Week
                | State::Time
                | State::LocalDateAndTime
                | State::Number
                | State::Checkbox
                | State::RadioButton
                | State::FileUpload
        )
    }
}

/// An element's namespace, local name and attributes.
struct Element<'a> {
    namespace: Namespace,
    local_name: &'a str,
    attributes: &'a [Attribute],
}

impl Element<'_> {
    fn is_html(&self, local_name: &str) -> bool {
        self.namespace == Namespace::Html && self.local_name == local_name
    }

    /// The value of the attribute in no namespace named `name`.
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.namespace.is_none() && a.local_name == name)
            .map(|a| a.value.as_str())
    }

    fn has(&self, name: &str) -> bool {
        self.attribute(name).is_some()
    }
}

fn element(tree: &Tree, node: NodeId) -> Option<Element<'_>> {
    match tree.data(node) {
        NodeData::Element {
            namespace,
            local_name,
            attributes,
            ..
        } => Some(Element {
            namespace: *namespace,
            local_name,
            attributes,
        }),
        _ => None,
    }
}

/// The data of the text node children of `node`, in tree order (DOM, "child text content").
fn child_text_content(tree: &Tree, node: NodeId) -> String {
    tree.children(node)
        .iter()
        .filter_map(|&c| match tree.data(c) {
            NodeData::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// Infra, "strip and collapse ASCII whitespace".
fn strip_and_collapse(text: &str) -> String {
    text.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
}

/// Infra, "strip newlines": every LF and CR removed.
fn strip_newlines(text: &str) -> String {
    text.chars().filter(|&c| c != '\n' && c != '\r').collect()
}

/// Infra, "split on commas": the tokens between commas, each stripped of leading and trailing
/// ASCII whitespace; a comma at the end starts no token.
fn split_on_commas(text: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let (token, after) = rest.split_once(',').unwrap_or((rest, ""));
        tokens.push(token.trim_ascii());
        rest = after;
    }
    tokens
}
