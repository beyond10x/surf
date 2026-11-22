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
//!   user-agent style sheet that needs no style computation. In 15.3.1 "Hidden elements": the
//!   elements `base, basefont, datalist, head, link, meta, noembed, noframes, param, rp, script,
//!   style, template, title`; the `hidden` attribute on any element but `embed`, whatever its
//!   value; an `input` whose `type` is `hidden` compared ASCII case-insensitively. In 15.3.3
//!   "Flow content": `dialog:not([open])`, a `dialog` without an `open` attribute. In 15.5.5
//!   "The details and summary elements": a `details` without an `open` attribute gives
//!   `content-visibility: hidden` to the slot that takes every child but its first `summary`
//!   element child, so each such child is hidden and that `summary` is not (a `details` with no
//!   `summary` child hides all its children; its default summary is not in the tree). The style
//!   sheet is in the HTML namespace, so those rules apply to HTML elements only; `aria-hidden`
//!   applies to any element. `area` is left out of the element rule: an `area` is drawn through
//!   its image's map, and the parser gives it no children. `noscript` is not hidden: surf parses
//!   with scripting disabled. The fallback content of `video`, `audio` and `object` is not
//!   hidden: surf plays no media and loads no object, so the fallback is what is shown.
//!   `aria-hidden` on the document element or on the body element (the first `body` or
//!   `frameset` child of an HTML `html` document element) is not read: WAI-ARIA 1.3 says user
//!   agents must not expose the hidden state when it is specified on the root element or on the
//!   element that contains the contents of the primary document. The value `true` is a keyword
//!   (WAI-ARIA 1.3 6.3.1; its appendix A maps true/false values in HTML to keyword and
//!   enumerated attributes) and is compared as HTML 2.3.3 says: an ASCII case-insensitive match,
//!   the value not stripped (`TRUE` hides, ` true` does not). A node not in the document
//!   (template contents) is judged only by the ancestors it has.
//! - `label`: the accessible name of Accessible Name and Description Computation 1.2 (4.3.2
//!   "Computation steps") with HTML-AAM 4.1 "Accessible Name Computations By HTML Element",
//!   computed over the tree. The sources of each element, in order, the first that gives a
//!   character other than ASCII whitespace wins: a link `a`: `aria-labelledby`, `aria-label`,
//!   its subtree, `title` (4.1.13); `area`: `aria-labelledby`, `aria-label`, `alt`, `title`
//!   (4.1.14); `button`: `aria-labelledby`, `aria-label`, its `label` elements, its subtree,
//!   `title` (4.1.4); an `input` in the Submit Button, Reset Button or Button state:
//!   `aria-labelledby`, `aria-label`, its `label` elements, `value`, `Submit` or `Reset` when
//!   `value` is absent (Submit Button and Reset Button), `title` (4.1.2); Image Button:
//!   `aria-labelledby`, `aria-label`, its `label` elements, `alt`, `title`, `Submit` (4.1.3);
//!   Text, Search, Telephone, URL, Email, Password, Number and `textarea`: `aria-labelledby`,
//!   `aria-label`, its `label` elements, `title`, `placeholder`, `aria-placeholder` (4.1.1);
//!   every other `input` state and `select`: `aria-labelledby`, `aria-label`, its `label`
//!   elements, `title` (4.1.7). A control's `label` elements are the HTML Standard's `labels`
//!   (4.10.4): every `label` whose labeled control it is, in tree order, their names joined by
//!   one space; a `label` with `for` labels the first element in tree order whose ID is the
//!   value if that element is labelable, and nothing else; a `label` without `for` labels its
//!   first labelable descendant. A control inside its label leaves itself out of that label's
//!   name (its value, `alt` or subtree, HTML-AAM 4.1.1 to 4.1.4). The IDREFs of
//!   `aria-labelledby` are its ASCII-whitespace-separated tokens, compared exactly with IDs; a
//!   token that names no element is skipped, and the results are joined by one space. Inside a
//!   subtree, a `label` or a referenced node, each node is read by the steps 2A to 2I: a hidden
//!   node gives nothing unless the node that started the traversal (the referenced node, the
//!   `label`) is hidden (2A); a descendant's own `aria-labelledby` is followed unless the walk is
//!   already inside an `aria-labelledby` traversal (2B); a control embedded in the name of
//!   another element gives its value (2C: a Text, Search, Telephone, URL, Email or Password
//!   `input` and a `textarea` their default value, a Number or Range `input` `aria-valuetext`,
//!   else `aria-valuenow`, else its `value` attribute); `aria-label` (2D); the element's own
//!   attributes (2E: an `img`'s `alt`, even blank; an `area`'s `alt`; an Image Button's `alt`,
//!   `title` or `Submit`; a Submit Button's, Reset Button's or Button's `value`, or `Submit` or
//!   `Reset` without one; a nested control's `label` elements are not followed); text nodes
//!   (2G); the children, joined without a space (2F, 2H); `title` (2I). Each node is read at
//!   most once per name (the note under 2F), so a repeated or cyclic reference adds nothing.
//!   The name is a flat string: ASCII whitespace stripped and collapsed. "Hidden" is the flag
//!   of `hidden`, recorded by the walk of [`actionable_elements`].
//!
//! - [`handles`]: surf's own numbering of the rows, not a standard's: `l<n>`, `b<n>`, `f<n>`
//!   (`docs/design/page-as-text.md` section 5). [`hidden_roots`]: the rule of [`is_hidden`] for a
//!   whole tree in one walk.
//!
//! No function recurses over the depth of the tree: [`actionable_elements`] and [`title`] walk
//! with their own stack over [`Tree::children`], [`is_hidden`] walks up [`Tree::parent`], and
//! each label is computed with its own stack. [`actionable_elements`] computes the inherited
//! flags (hidden, inside a disabling fieldset) in that one walk, and records there what the
//! labels need: a dense index of the nodes with the hidden flag of each, the first element per
//! ID, each `label` element with its labeled control, and, in one pass over the attributes of
//! each element, where the attributes a name reads are (`aria-labelledby`, `aria-label`,
//! `title`, `alt`, `value`, `type`, `aria-valuetext`, `aria-valuenow`, `placeholder`,
//! `aria-placeholder`, `multiple`). The labels call [`is_hidden`] for no node and scan no
//! attributes. The time of a call is linear in the number of nodes and of the attributes the
//! tree holds (the walk compares each attribute a bounded number of times, whatever the number
//! of names), plus the labels: at most [`LABEL_WORK_MAX`] units of work for all of them (a unit
//! is a node read or a byte read, each costing a hash lookup and a fixed number of steps), plus,
//! per row, the bytes of its own attribute values. The memory of the labels is at most [`LABEL_BYTES_MAX`] bytes per row, and in all
//! at most [`LABEL_WORK_MAX`] bytes plus the attribute bytes the tree stores; what the walk
//! records is linear in the number of nodes. [`is_hidden`] costs the depth of the node; it
//! reads the children of the root or of the document element only at an ancestor with
//! `aria-hidden` true that may be the document element or the body element, and the children
//! of a closed `details` only at an ancestor that is a `summary` child of it.
//!
//! # Steps not taken
//!
//! - 4.3.2 2F.ii "Name From Generated Content" (`::before`, `::after`, `::marker`): surf has no
//!   CSS.
//! - Hiding by author CSS (`display`, `visibility`, `content-visibility` from a style sheet or a
//!   `style` attribute): surf has no CSS; "hidden" is the rule of `hidden` above.
//! - 2F.iii "Determine Child Nodes" for shadow roots and slots: the tree has none, so the
//!   rendered child nodes are the children.
//! - The spaces accname may join with by CSS `display` (its note under 2F, AccName #225):
//!   children are joined without a space.
//! - An explicit `role` attribute: the role is the one the kind implies (item 6 of the note).
//! - The chosen option of a `select` in 2C: option selectedness is `story:agent-form-port`'s,
//!   so an embedded `select` gives the empty string.
//!
//! # Deviations
//!
//! - The values of the Number, Range, Color, Date, Month, Week, Time and Local Date and Time
//!   states are the `value` attribute as written (the empty string when absent). The standard's
//!   value sanitization algorithms for them (a value that is not a valid number, date or colour
//!   becomes the empty string, or the range's default, or `#000000`) are not run.
//! - Nothing that needs CSS is read: an author style sheet or `style` attribute that hides an
//!   element does not make it hidden.
//! - A hidden actionable element is named from its own subtree, hidden parts included, as a
//!   hidden node referenced by `aria-labelledby` is (step 2A would give it the empty name): its
//!   row is kept and flagged, and a hidden link is shown by its label.
//! - A label is cut at [`LABEL_BYTES_MAX`] bytes, and the work for the labels of one call is
//!   bounded by [`LABEL_WORK_MAX`]; past it a name comes from the element's own attributes. The
//!   standards bound neither.
//!
//! # Left to other stories
//!
//! - A control's form owner and the radio button group rule, by which a later checked radio
//!   button unchecks the others of its group: here every radio button with a `checked`
//!   attribute is `checked` (`story:agent-form-port`).
//! - Option selectedness of a `select` (`story:agent-form-port`).
//! - Resolving a link's target against the document base URL (`story:agent-follow-links`).
//! - Writing the rows (`story:html-document-port`).

#![forbid(unsafe_code)]

mod label;

use surf_html::{Attribute, Namespace, NodeData, NodeId, Tree};

pub use label::{LABEL_BYTES_MAX, LABEL_WORK_MAX};

thread_local! {
    /// The attributes this thread compared in this crate's functions.
    static ATTRIBUTE_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn count_attribute_visits(visits: usize) {
    ATTRIBUTE_VISITS.with(|v| v.set(v.get().saturating_add(visits)));
}

/// The number of attributes the functions of this crate have compared on this thread so far:
/// each attribute looked at while searching an element's attributes counts once. For tests that
/// bound the work of [`actionable_elements`] by a count that does not depend on the machine.
#[doc(hidden)]
pub fn attribute_visits() -> usize {
    ATTRIBUTE_VISITS.with(std::cell::Cell::get)
}

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
    /// The label: the accessible name, ASCII whitespace stripped and collapsed, at most
    /// [`LABEL_BYTES_MAX`] bytes; the empty string when no source gives one.
    pub label: String,
    /// The label was cut at [`LABEL_BYTES_MAX`] bytes, or taken from the element's own
    /// attributes only because [`LABEL_WORK_MAX`] was spent.
    pub label_cut: bool,
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
        /// The `label` elements without `for` around `node`.
        labels: label::Labels,
    }

    let primary = Primary::of(tree);
    let exempt = |node| primary.contains(node);
    let mut found = Vec::new();
    let mut facts = label::Facts::default();
    let mut pending = vec![Pending {
        node: tree.root(),
        hidden: false,
        in_disabled_fieldset: false,
        labels: None,
    }];
    while let Some(Pending {
        node,
        hidden,
        in_disabled_fieldset,
        labels,
    }) = pending.pop()
    {
        let this = element(tree, node);
        let hidden = hidden || this.as_ref().is_some_and(|e| hides(e, node, exempt));
        let labels = facts.record(node, hidden, this.as_ref(), labels);
        if let Some(this) = &this
            && let Some(actionable) = actionable(tree, node, this, hidden, in_disabled_fieldset)
        {
            found.push(actionable);
        }
        let children = tree.children(node);
        if children.is_empty() {
            continue;
        }
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
        let closed_details = this.as_ref().is_some_and(is_closed_details);
        let first_summary = if closed_details {
            first_summary(tree, node)
        } else {
            None
        };
        for &child in children.iter().rev() {
            pending.push(Pending {
                node: child,
                hidden: hidden || (closed_details && Some(child) != first_summary),
                in_disabled_fieldset: in_disabled_fieldset
                    || (disabling && Some(child) != first_legend),
                labels,
            });
        }
    }
    label::fill(tree, facts, &mut found);
    found
}

/// The handle of each actionable element, the one numbering the outline, the Markdown and the
/// `now` block print (`docs/design/page-as-text.md` section 5): `l<n>` for a link, `b<n>` for a
/// button, `f<n>` for the other seven kinds, each letter counted from 1 in document order over
/// [`actionable_elements`], hidden elements included. Written without the `@` the text puts
/// before it. The tree does not change after parsing, so the numbering is stable.
pub fn handles(tree: &Tree) -> Vec<(NodeId, String)> {
    let (mut links, mut buttons, mut fields) = (0u64, 0u64, 0u64);
    actionable_elements(tree)
        .into_iter()
        .map(|row| {
            let (letter, count) = match row.kind {
                Kind::Link => ('l', &mut links),
                Kind::Button => ('b', &mut buttons),
                _ => ('f', &mut fields),
            };
            *count += 1;
            (row.node, format!("{letter}{count}"))
        })
        .collect()
}

/// The nodes [`is_hidden`] calls hidden whose parent it does not: a node is hidden exactly when
/// it or an ancestor is in the set. One walk with its own stack that does not enter a hidden
/// subtree, so the time is linear in the number of nodes and of their attributes, where calling
/// [`is_hidden`] for every node costs the depth of each.
pub fn hidden_roots(tree: &Tree) -> std::collections::HashSet<NodeId> {
    let primary = Primary::of(tree);
    let exempt = |node| primary.contains(node);
    let mut roots = std::collections::HashSet::new();
    // Each node with whether it is slotted out of a closed `details` parent.
    let mut pending = vec![(tree.root(), false)];
    while let Some((node, slotted_out)) = pending.pop() {
        let this = element(tree, node);
        if slotted_out || this.as_ref().is_some_and(|e| hides(e, node, exempt)) {
            roots.insert(node);
            continue;
        }
        let first = match &this {
            Some(e) if is_closed_details(e) => Some(first_summary(tree, node)),
            _ => None,
        };
        for &child in tree.children(node).iter().rev() {
            pending.push((child, first.is_some_and(|summary| summary != Some(child))));
        }
    }
    roots
}

/// Whether `node` is hidden: it or an ancestor element has `aria-hidden="true"` (not read on
/// the document element and the body element), or matches a rule of the HTML Standard's
/// user-agent style sheet that needs no style computation (15.3.1 "Hidden elements", a closed
/// `dialog`, the content of a closed `details` but its first `summary`; see the crate
/// documentation). A walk up [`Tree::parent`]: without `aria-hidden="true"` on the path a call
/// costs the depth of `node`.
///
/// # Panics
///
/// When `node` is not of `tree`.
pub fn is_hidden(tree: &Tree, node: NodeId) -> bool {
    let exempt = |node| is_primary(tree, node);
    let mut current = Some(node);
    while let Some(next) = current {
        let parent = tree.parent(next);
        if element(tree, next).is_some_and(|e| hides(&e, next, exempt))
            || parent.is_some_and(|p| slotted_out(tree, p, next))
        {
            return true;
        }
        current = parent;
    }
    false
}

/// The elements of 15.3.1 "Hidden elements" with `display: none`, `area` left out.
const HIDDEN_ELEMENTS: [&str; 14] = [
    "base", "basefont", "datalist", "head", "link", "meta", "noembed", "noframes", "param", "rp",
    "script", "style", "template", "title",
];

/// The document element and the body element, on which `aria-hidden` is not read, worked out
/// once for a walk over the whole tree.
struct Primary {
    document_element: Option<NodeId>,
    body: Option<NodeId>,
}

impl Primary {
    fn of(tree: &Tree) -> Primary {
        let document_element = document_element(tree);
        Primary {
            document_element,
            body: document_element.and_then(|d| body(tree, d)),
        }
    }

    fn contains(&self, node: NodeId) -> bool {
        self.document_element == Some(node) || self.body == Some(node)
    }
}

/// The document element: the first element child of the root, when the root is a document.
fn document_element(tree: &Tree) -> Option<NodeId> {
    let root = tree.root();
    if !matches!(tree.data(root), NodeData::Document) {
        return None;
    }
    tree.children(root)
        .iter()
        .copied()
        .find(|&c| element(tree, c).is_some())
}

/// The body element of a document whose document element is `document_element`: its first
/// `body` or `frameset` child, when it is an HTML `html` element.
fn body(tree: &Tree, document_element: NodeId) -> Option<NodeId> {
    if !element(tree, document_element).is_some_and(|e| e.is_html("html")) {
        return None;
    }
    tree.children(document_element)
        .iter()
        .copied()
        .find(|&c| element(tree, c).is_some_and(|e| e.is_html("body") || e.is_html("frameset")))
}

/// Whether `node` is the document element or the body element, worked out for one node: the
/// children of the root or of the document element are read only when `node` is a child of the
/// root, or an HTML `body` or `frameset` element whose parent is a child of the root.
fn is_primary(tree: &Tree, node: NodeId) -> bool {
    let root = tree.root();
    let Some(parent) = tree.parent(node) else {
        return false;
    };
    if parent == root {
        return document_element(tree) == Some(node);
    }
    tree.parent(parent) == Some(root)
        && element(tree, node).is_some_and(|e| e.is_html("body") || e.is_html("frameset"))
        && document_element(tree) == Some(parent)
        && body(tree, parent) == Some(node)
}

/// Whether the element `this`, the node `node`, is itself hidden by `aria-hidden` or by a rule
/// of the style sheet on the element itself (15.3.1, a closed `dialog`), ancestors not read.
/// `exempt` says whether `node` is the document element or the body element.
fn hides(this: &Element<'_>, node: NodeId, exempt: impl Fn(NodeId) -> bool) -> bool {
    if this
        .attribute("aria-hidden")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
        && !exempt(node)
    {
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
        || (this.local_name == "dialog" && !this.has("open"))
}

/// An HTML `details` element without an `open` attribute.
fn is_closed_details(this: &Element<'_>) -> bool {
    this.is_html("details") && !this.has("open")
}

/// The first HTML `summary` element child of `node`.
fn first_summary(tree: &Tree, node: NodeId) -> Option<NodeId> {
    tree.children(node)
        .iter()
        .copied()
        .find(|&c| element(tree, c).is_some_and(|e| e.is_html("summary")))
}

/// Whether `child` of `parent` is in the second slot of a closed `details` (15.5.5): `parent`
/// is an HTML `details` without `open` and `child` is not its first `summary` element child.
/// The children of `parent` are read only when `child` is an HTML `summary`.
fn slotted_out(tree: &Tree, parent: NodeId, child: NodeId) -> bool {
    element(tree, parent).is_some_and(|d| is_closed_details(&d))
        && (!element(tree, child).is_some_and(|e| e.is_html("summary"))
            || first_summary(tree, parent) != Some(child))
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
        label: String::new(),
        label_cut: false,
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
        self.value_of(input.attribute("value"), input.has("multiple"))
    }

    /// The default value of an `input` in this state with the `value` attribute `value` and the
    /// `multiple` attribute present or not.
    fn value_of(self, value: Option<&str>, multiple: bool) -> Option<String> {
        let raw = value.unwrap_or("");
        Some(match self {
            State::Text | State::Search | State::Telephone | State::Password => strip_newlines(raw),
            State::Email if multiple => split_on_commas(raw).join(","),
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

impl<'a> Element<'a> {
    fn is_html(&self, local_name: &str) -> bool {
        self.namespace == Namespace::Html && self.local_name == local_name
    }

    /// The value of the attribute in no namespace named `name`.
    fn attribute(&self, name: &str) -> Option<&'a str> {
        let at = self
            .attributes
            .iter()
            .position(|a| a.namespace.is_none() && a.local_name == name);
        count_attribute_visits(at.map_or(self.attributes.len(), |at| at + 1));
        at.map(|at| self.attributes[at].value.as_str())
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
