//! Cases 1 to 4 and 21 of `story:agent-form-port`: the forms of a document, the form owner and
//! `name` of each control, the options of a select with their selectedness, and the form
//! handles beside the handles of the rows.

use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};
use surf_page::forms::{ButtonType, Field, Forms, forms};
use surf_page::{form_handles, handles};

fn parse(html: &str) -> Tree {
    parse_document(html, &ParseOptions::default())
}

/// The value of the attribute `name` of the element `node`.
fn attribute(tree: &Tree, node: NodeId, name: &str) -> Option<String> {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes
            .iter()
            .find(|a| a.namespace.is_none() && a.local_name == name)
            .map(|a| a.value.clone()),
        _ => None,
    }
}

/// Each control as `name -> the id attribute of its form owner`, `-` for no owner.
fn owners(tree: &Tree, found: &Forms) -> Vec<String> {
    found
        .controls
        .iter()
        .map(|control| {
            let owner = control.owner.map_or("-".to_owned(), |at| {
                attribute(tree, found.forms[at].node, "id").unwrap_or_else(|| format!("#{at}"))
            });
            format!("{} -> {owner}", control.name.as_deref().unwrap_or("(none)"))
        })
        .collect()
}

/// HTML Standard, "form owner" (4.10.18.3, "Association of controls and forms"): a listed
/// element with a `form` attribute is owned by the first element in tree order whose ID is the
/// attribute's value when that element is a `form`, and by no form otherwise, its ancestors
/// not read.
#[test]
fn case_01_a_control_owned_through_the_form_attribute() {
    let html = "<!doctype html><form id=a action=/a></form>\
        <form id=b><input name=inside-b form=a></form>\
        <input name=after form=a>\
        <p id=c></p><input name=not-a-form form=c>\
        <form id=d><input name=unknown form=zzz><input name=empty form=\"\"></form>";
    let tree = parse(html);
    let found = forms(&tree);
    assert_eq!(found.forms.len(), 3);
    assert_eq!(
        owners(&tree, &found),
        [
            "inside-b -> a",
            "after -> a",
            "not-a-form -> -",
            "unknown -> -",
            "empty -> -"
        ]
    );
    assert_eq!(found.forms[0].action.as_deref(), Some("/a"));
    assert_eq!(found.forms[1].action, None);
}

/// "form owner": without a `form` attribute, the nearest ancestor `form` element.
#[test]
fn case_02_a_control_owned_through_nesting() {
    let html = "<!doctype html><form id=outer method=POST enctype=text/plain>\
        <div><fieldset><input name=deep><select name=s><option>x</select>\
        <textarea name=t></textarea><button name=b>Go</button>\
        <input type=hidden name=h value=1></fieldset></div></form>";
    let tree = parse(html);
    let found = forms(&tree);
    assert_eq!(
        owners(&tree, &found),
        [
            "deep -> outer",
            "s -> outer",
            "t -> outer",
            "b -> outer",
            "h -> outer"
        ]
    );
    let form = &found.forms[0];
    assert_eq!(form.method.as_deref(), Some("POST"));
    assert_eq!(form.enctype.as_deref(), Some("text/plain"));
    let fields: Vec<Field> = found.controls.iter().map(|c| c.field).collect();
    assert_eq!(
        fields,
        [
            Field::Text,
            Field::Select,
            Field::Textarea,
            Field::Button(ButtonType::Submit),
            Field::Hidden
        ]
    );
    assert_eq!(found.controls[4].value.as_deref(), Some("1"));
}

#[test]
fn case_03_a_control_in_no_form() {
    let html = "<!doctype html><input name=alone><button>Go</button>\
        <form id=f></form><select name=after><option>a</select>\
        <template><form id=t><input name=templated></form></template>";
    let tree = parse(html);
    let found = forms(&tree);
    // Template contents are not in the document: neither the form nor the input inside.
    assert_eq!(found.forms.len(), 1);
    assert_eq!(
        owners(&tree, &found),
        ["alone -> -", "(none) -> -", "after -> -"]
    );
}

/// HTML Standard, the `select` element (4.10.7): the selectedness setting algorithm for a
/// select without `multiple` whose display size is 1: of the options with `selected` the last
/// in the list of options stays selected; with none, the first option that is not disabled is.
/// An option's value is its `value` attribute, else its text; its label is its non-empty
/// `label` attribute, else its text; it is disabled by its own `disabled` or its `optgroup`'s.
#[test]
fn case_04_option_selectedness_of_a_select_without_multiple() {
    let html = "<!doctype html><form>\
        <select name=two><option selected>a</option><option value=bv selected>b</option></select>\
        <select name=none><option disabled>x</option>\
        <optgroup label=g disabled><option>y</option></optgroup>\
        <option label=Zed value=z>  z  text </option></select>\
        <select name=listbox size=3><option>p</option><option>q</option></select>\
        <select name=many multiple><option selected>m1</option><option selected>m2</option>\
        <option>m3</option></select></form>";
    let tree = parse(html);
    let found = forms(&tree);
    let selects: Vec<String> = found
        .controls
        .iter()
        .map(|control| {
            let options: Vec<String> = control
                .options
                .iter()
                .map(|o| {
                    format!(
                        "{}/{}{}{}",
                        o.value,
                        o.label,
                        if o.selected { " selected" } else { "" },
                        if o.disabled { " disabled" } else { "" }
                    )
                })
                .collect();
            format!(
                "{}: {}",
                control.name.as_deref().unwrap_or(""),
                options.join(", ")
            )
        })
        .collect();
    assert_eq!(
        selects,
        [
            "two: a/a, bv/b selected",
            "none: x/x disabled, y/y disabled, z/Zed selected",
            "listbox: p/p, q/q",
            "many: m1/m1 selected, m2/m2 selected, m3/m3",
        ]
    );
}

/// `form_handles` numbers the forms `F1`, `F2` in document order; `handles`, which the outline
/// and the Markdown print, gives what it gave before: no `F`.
#[test]
fn case_21_form_handles_beside_unchanged_handles() {
    let html = "<!doctype html><a href=/x>x</a>\
        <form id=one><input name=q><button>Go</button></form>\
        <form id=two></form>";
    let tree = parse(html);
    let named: Vec<(String, String)> = form_handles(&tree)
        .into_iter()
        .map(|(node, handle)| (attribute(&tree, node, "id").unwrap(), handle))
        .collect();
    assert_eq!(
        named,
        [
            ("one".to_owned(), "F1".to_owned()),
            ("two".to_owned(), "F2".to_owned())
        ]
    );
    let plain: Vec<String> = handles(&tree).into_iter().map(|(_, h)| h).collect();
    assert_eq!(plain, ["l1", "f1", "b1"]);
}
