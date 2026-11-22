//! The forms of a document and its submittable elements (`story:agent-form-port`): each form
//! with its `action`, `method` and `enctype` as written, each control with its form owner and
//! its `name`, and the options of each select with their selectedness. The values the rows of
//! `surf.document.Form`, `surf.document.Element` (`name`, `form_id`) and `surf.document.Option`
//! hold, and what the submission step of `crates/surf` reads from the tree.
//!
//! # Rules taken (HTML Standard, from memory)
//!
//! - The forms: every HTML `form` element in tree order, template contents left out, as
//!   `document.forms` lists them.
//! - The controls: every HTML `button`, `input` (the Hidden state included), `select` and
//!   `textarea` in tree order, the submittable elements of 4.10.2 "Categories" but `object` and
//!   form-associated custom elements, which surf does not have.
//! - "form owner" (4.10.18.3, "Association of controls and forms"): a control with a `form`
//!   attribute is owned by the first element in tree order whose ID is the attribute's value
//!   when that element is a `form`, and by no form otherwise, its ancestors not read; an element
//!   with an empty `id` has no ID, so `form=""` names none. A control without a `form`
//!   attribute is owned by its nearest ancestor `form`.
//! - `disabled` as [`crate::actionable_elements`] computes it: the `disabled` attribute, or a
//!   `fieldset` with `disabled` above, not through its first `legend` child. `readonly` where the
//!   attribute applies: an `input` in a text-like, date-like, Number or Password state, and a
//!   `textarea`. `in_datalist`: a `datalist` element is an ancestor, which bars the control from
//!   the entry list and from constraint validation.
//! - The list of options of a select (4.10.7, "list of options"): its `option` descendants not
//!   inside an `option`, `select`, `hr`, `datalist` or nested `optgroup`, as `surf-html` takes
//!   them for `selectedcontent`. An option is disabled by its own `disabled` or by that of the
//!   `optgroup` above it. Its value is its `value` attribute, else its text; its label is its
//!   `label` attribute when that is not empty, else its text; the text is its `Text` descendants,
//!   `script` and SVG `script` subtrees left out, ASCII whitespace stripped and collapsed.
//! - Selectedness, as the selectedness setting algorithm leaves it after the parser inserted the
//!   last option: with `multiple`, each option with a `selected` attribute; without it, the last
//!   such option in the list, else, when the display size is 1, the first option that is not
//!   disabled.
//! - The radio button group rule at parse time (`story:agent-fill-pick`), which
//!   [`crate::actionable_elements`] applies to its rows: of the radio buttons of one group with
//!   `checked`, only the last in tree order stays checked (`uncheck_radio_groups`).
//!
//! # Deviations
//!
//! - The parser's form element pointer is not kept by `surf-html` (its tree records no form
//!   association), so a control the parser associated with a form it is not inside of (a `form`
//!   start tag inside a `table`, closed before the controls) has no form owner here.
//! - A control inside the copy a `selectedcontent` holds of the selected option is not listed,
//!   as [`crate::actionable_elements`] lists no row for it.
//!
//! One walk with its own stack over [`Tree::children`], then one walk of each select's options
//! and one of each option's text: the time is linear in the number of nodes and attributes.

use std::collections::{BTreeMap, HashMap};

use surf_html::{Namespace, NodeData, NodeId, Tree};

use crate::{Element, Mirror, State, element, strip_and_collapse};

/// The forms and the controls of one document.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Forms {
    /// The forms, in tree order. A control's owner is an index into this list.
    pub forms: Vec<Form>,
    /// The controls, in tree order.
    pub controls: Vec<Control>,
    /// The index in [`Forms::controls`] of each control's node.
    by_node: BTreeMap<NodeId, usize>,
}

impl Forms {
    /// The control whose element is `node`.
    pub fn control(&self, node: NodeId) -> Option<&Control> {
        self.by_node.get(&node).map(|&at| &self.controls[at])
    }
}

/// One `form` element, its attributes as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Form {
    /// The element in the tree.
    pub node: NodeId,
    /// The `action` attribute, `None` when absent.
    pub action: Option<String>,
    /// The `method` attribute, `None` when absent.
    pub method: Option<String>,
    /// The `enctype` attribute, `None` when absent.
    pub enctype: Option<String>,
    /// The `novalidate` attribute is present.
    pub novalidate: bool,
}

/// What a control is, as the entry list reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// An `input` in a text-like, date-like, Number, Range or Color state.
    Text,
    /// An `input` in the Password state.
    Password,
    /// A `textarea`.
    Textarea,
    /// An `input` in the Checkbox state.
    Checkbox,
    /// An `input` in the Radio Button state.
    Radio,
    /// An `input` in the File Upload state.
    File,
    /// A `select`.
    Select,
    /// An `input` in the Hidden state: it has no row.
    Hidden,
    /// A `button`, or an `input` in a button state.
    Button(ButtonType),
}

/// The state of a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonType {
    /// A `button` in the Submit Button state (its `type` missing or invalid too), or an `input`
    /// in the Submit Button state.
    Submit,
    /// An `input` in the Image Button state.
    Image,
    /// The Reset Button state.
    Reset,
    /// The Button state.
    Button,
}

/// One submittable element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Control {
    /// The element in the tree.
    pub node: NodeId,
    /// What it is.
    pub field: Field,
    /// Its form owner, an index into [`Forms::forms`]; `None` for no owner.
    pub owner: Option<usize>,
    /// The `name` attribute as written, `None` when absent.
    pub name: Option<String>,
    /// The `value` attribute as written, `None` when absent.
    pub value: Option<String>,
    /// The control is disabled.
    pub disabled: bool,
    /// The `readonly` attribute is present and applies to the control.
    pub readonly: bool,
    /// A `datalist` element is an ancestor.
    pub in_datalist: bool,
    /// A select's list of options, in tree order; empty for every other field.
    pub options: Vec<ListOption>,
}

/// One option of a select's list of options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListOption {
    /// The element in the tree.
    pub node: NodeId,
    /// Its value.
    pub value: String,
    /// Its label.
    pub label: String,
    /// Its selectedness after parsing.
    pub selected: bool,
    /// It is disabled, by itself or by its `optgroup`.
    pub disabled: bool,
}

/// The form handle of each form (`F<n>`, counted from 1 in document order, the order of the
/// `Forms` view's positions), without the `@` an answer puts before it. Apart from
/// [`crate::handles`], which the outline and the Markdown print: page text names no form. The
/// forms are those [`forms`] finds, by the same walk, which here lists no control and builds no
/// option or its text.
pub fn form_handles(tree: &Tree) -> Vec<(NodeId, String)> {
    walk(tree, false)
        .forms
        .into_iter()
        .zip(1u64..)
        .map(|(form, n)| (form.node, format!("F{n}")))
        .collect()
}

/// How a control's form owner is found once the walk knows every ID.
enum Owner {
    /// The first element with this ID, when it is a form.
    Id(String),
    /// The nearest ancestor form, if any.
    Ancestor(Option<usize>),
}

/// The radio button group rule at parse time (`story:agent-fill-pick`; HTML Standard 4.10.5.1.18
/// "Radio Button state"): setting a radio button's checkedness to true, as the parser does for
/// one with a `checked` attribute when it inserts it, sets the checkedness of every other radio
/// button of its group to false. So of the radio buttons of one group that carry `checked`, only
/// the last in tree order stays checked. The group: the radio buttons with the same form owner
/// and the same non-empty `name` (compared exactly), or with no form owner and the same name in
/// the same tree; one with no `name` or an empty one is in no group but its own. The form owner
/// is the one [`forms`] computes, after the whole tree is parsed. Rows of `found` that are not
/// radio buttons, or not checked, are left as they are; [`forms`] is computed only when two
/// radio buttons are checked.
pub(crate) fn uncheck_radio_groups(tree: &Tree, found: &mut [crate::ActionableElement]) {
    let checked = || {
        found
            .iter()
            .filter(|row| row.kind == crate::Kind::Radio && row.checked)
    };
    if checked().nth(1).is_none() {
        return;
    }
    let owners = forms(tree);
    let mut seen: std::collections::HashSet<(Option<usize>, String)> =
        std::collections::HashSet::new();
    for row in found.iter_mut().rev() {
        if row.kind != crate::Kind::Radio || !row.checked {
            continue;
        }
        let Some(control) = owners.control(row.node) else {
            continue;
        };
        let Some(name) = control.name.as_ref().filter(|name| !name.is_empty()) else {
            continue;
        };
        if !seen.insert((control.owner, name.clone())) {
            row.checked = false;
        }
    }
}

/// The forms and the controls of `tree`, template contents left out.
pub fn forms(tree: &Tree) -> Forms {
    walk(tree, true)
}

/// The walk of [`forms`]; with `with_controls` false only the forms, so no control, option or
/// option text is built.
fn walk(tree: &Tree, with_controls: bool) -> Forms {
    #[derive(Clone, Copy)]
    struct Above {
        form: Option<usize>,
        in_disabled_fieldset: bool,
        in_datalist: bool,
        mirror: Mirror,
        copy: bool,
    }

    let mut found = Forms::default();
    let mut form_of_node: HashMap<NodeId, usize> = HashMap::new();
    let mut ids: HashMap<&str, NodeId> = HashMap::new();
    let mut owners: Vec<Owner> = Vec::new();
    let start = Above {
        form: None,
        in_disabled_fieldset: false,
        in_datalist: false,
        mirror: Mirror::default(),
        copy: false,
    };
    let mut pending = vec![(tree.root(), start)];
    while let Some((node, above)) = pending.pop() {
        let this = element(tree, node);
        let (mirror, mirrored) = above.mirror.below(this.as_ref());
        let copy = above.copy || mirrored;
        let mut below = Above {
            mirror,
            copy,
            ..above
        };
        if let Some(this) = &this
            && !copy
        {
            if let Some(id) = this.attribute("id").filter(|id| !id.is_empty()) {
                ids.entry(id).or_insert(node);
            }
            if this.is_html("form") {
                form_of_node.insert(node, found.forms.len());
                below.form = Some(found.forms.len());
                found.forms.push(Form {
                    node,
                    action: this.attribute("action").map(str::to_owned),
                    method: this.attribute("method").map(str::to_owned),
                    enctype: this.attribute("enctype").map(str::to_owned),
                    novalidate: this.has("novalidate"),
                });
            } else if with_controls && let Some(field) = field(this) {
                owners.push(match this.attribute("form") {
                    Some(id) => Owner::Id(id.to_owned()),
                    None => Owner::Ancestor(above.form),
                });
                let options = if field == Field::Select {
                    list_of_options(tree, node, this)
                } else {
                    Vec::new()
                };
                found.by_node.insert(node, found.controls.len());
                found.controls.push(Control {
                    node,
                    field,
                    owner: None,
                    name: this.attribute("name").map(str::to_owned),
                    value: this.attribute("value").map(str::to_owned),
                    disabled: this.has("disabled") || above.in_disabled_fieldset,
                    readonly: this.has("readonly") && readonly_applies(this),
                    in_datalist: above.in_datalist,
                    options,
                });
            }
            if this.is_html("datalist") {
                below.in_datalist = true;
            }
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
            pending.push((
                child,
                Above {
                    in_disabled_fieldset: below.in_disabled_fieldset
                        || (disabling && Some(child) != first_legend),
                    ..below
                },
            ));
        }
    }
    for (control, owner) in found.controls.iter_mut().zip(owners) {
        control.owner = match owner {
            Owner::Id(id) => ids
                .get(id.as_str())
                .and_then(|node| form_of_node.get(node))
                .copied(),
            Owner::Ancestor(form) => form,
        };
    }
    found
}

/// The field an HTML element is, if it is a submittable element surf lists.
fn field(this: &Element<'_>) -> Option<Field> {
    if this.namespace != Namespace::Html {
        return None;
    }
    Some(match this.local_name {
        "button" => Field::Button(match this.attribute("type") {
            Some(t) if t.eq_ignore_ascii_case("reset") => ButtonType::Reset,
            Some(t) if t.eq_ignore_ascii_case("button") => ButtonType::Button,
            _ => ButtonType::Submit,
        }),
        "select" => Field::Select,
        "textarea" => Field::Textarea,
        "input" => match State::of(this.attribute("type")) {
            State::Hidden => Field::Hidden,
            State::Password => Field::Password,
            State::Checkbox => Field::Checkbox,
            State::RadioButton => Field::Radio,
            State::FileUpload => Field::File,
            State::SubmitButton => Field::Button(ButtonType::Submit),
            State::ImageButton => Field::Button(ButtonType::Image),
            State::ResetButton => Field::Button(ButtonType::Reset),
            State::Button => Field::Button(ButtonType::Button),
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
            | State::Color => Field::Text,
        },
        _ => return None,
    })
}

/// Whether `readonly` applies to the control `this`: a `textarea`, or an `input` in the Text,
/// Search, URL, Telephone, Email, Password, Date, Month, Week, Time, Local Date and Time or
/// Number state.
fn readonly_applies(this: &Element<'_>) -> bool {
    match this.local_name {
        "textarea" => true,
        "input" => matches!(
            State::of(this.attribute("type")),
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
        ),
        _ => false,
    }
}

/// The list of options of the select `select`, each with its value, label, disabledness and
/// selectedness after parsing.
fn list_of_options(tree: &Tree, select: NodeId, this: &Element<'_>) -> Vec<ListOption> {
    let mut options = Vec::new();
    // Each node with the `optgroup` above it: `None`, or whether that one has `disabled`.
    let mut pending: Vec<(NodeId, Option<bool>)> = tree
        .children(select)
        .iter()
        .rev()
        .map(|&child| (child, None))
        .collect();
    while let Some((node, optgroup)) = pending.pop() {
        let Some(e) = element(tree, node) else {
            continue;
        };
        if e.is_html("option") {
            let text = option_text(tree, node);
            options.push(ListOption {
                node,
                value: e
                    .attribute("value")
                    .map_or_else(|| text.clone(), str::to_owned),
                label: e
                    .attribute("label")
                    .filter(|label| !label.is_empty())
                    .map_or_else(|| text.clone(), str::to_owned),
                selected: e.has("selected"),
                disabled: e.has("disabled") || optgroup == Some(true),
            });
            continue;
        }
        let is_optgroup = e.is_html("optgroup");
        if ["select", "hr", "datalist"]
            .iter()
            .any(|name| e.is_html(name))
            || (is_optgroup && optgroup.is_some())
        {
            continue;
        }
        let inside = if is_optgroup {
            Some(e.has("disabled"))
        } else {
            optgroup
        };
        pending.extend(
            tree.children(node)
                .iter()
                .rev()
                .map(|&child| (child, inside)),
        );
    }
    if this.has("multiple") {
        return options;
    }
    let last = options.iter().rposition(|option| option.selected);
    let chosen = match last {
        Some(at) => Some(at),
        None if display_size(this) == 1 => options.iter().position(|option| !option.disabled),
        None => None,
    };
    for (at, option) in options.iter_mut().enumerate() {
        option.selected = Some(at) == chosen;
    }
    options
}

/// An option's text: the data of its `Text` descendants in tree order, leaving out those under
/// a `script` or SVG `script` element, ASCII whitespace stripped and collapsed.
fn option_text(tree: &Tree, option: NodeId) -> String {
    let mut text = String::new();
    let mut pending: Vec<NodeId> = tree.children(option).iter().rev().copied().collect();
    while let Some(node) = pending.pop() {
        match tree.data(node) {
            NodeData::Text(data) => text.push_str(data),
            NodeData::Element {
                namespace,
                local_name,
                ..
            } if local_name == "script"
                && matches!(namespace, Namespace::Html | Namespace::Svg) => {}
            _ => pending.extend(tree.children(node).iter().rev()),
        }
    }
    strip_and_collapse(&text)
}

/// The display size of a select without `multiple`: its `size` attribute by the rules for
/// parsing non-negative integers, else 1; the reading of `surf-html`'s `selectedcontent`.
fn display_size(this: &Element<'_>) -> u64 {
    let Some(size) = this.attribute("size") else {
        return 1;
    };
    let value = size.trim_start_matches(['\t', '\n', '\x0C', '\r', ' ']);
    let (negative, rest) = match value.as_bytes().first() {
        Some(b'-') => (true, &value[1..]),
        Some(b'+') => (false, &value[1..]),
        _ => (false, value),
    };
    let digits = &rest[..rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len())];
    if digits.is_empty() {
        return 1;
    }
    let zero = digits.bytes().all(|b| b == b'0');
    if negative && !zero {
        return 1;
    }
    digits.parse().unwrap_or(u64::MAX)
}
