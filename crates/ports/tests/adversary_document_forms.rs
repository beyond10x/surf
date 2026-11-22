//! Adversary pass on `story:agent-forms-spec`: the two obligations `DocumentPorts` meets,
//! `Check` and `SelectOption`, held to `spec/domains/document.yaml` on the inputs the story's
//! own tests leave out: the outcome a checkbox is answered with, a radio button's group read by
//! kind and document, and the refusals of an option.

use surf_ports::DocumentPorts;
use surf_types::behaviour::{ElementStorage, ListOptionStorage};
use surf_types::document::obligations::{CheckBehavior, SelectOptionBehavior};
use surf_types::document::{
    Check, CheckOutcome, CheckednessSet, ControlDisabled, ControlHidden, DocumentId, ElementData,
    ElementId, ElementKind, ElementSnapshot, ElementState, FormId, ListOptionData,
    ListOptionSnapshot, OptionId, OptionState, SelectOption, SelectOptionOutcome,
};
use surf_types::primitives::Uuid;

fn uuid(n: u64) -> Uuid {
    Uuid(format!("00000000-0000-4000-8000-{n:012x}"))
}

/// A shown, enabled control `n` of `document` in form 7 named `group`.
fn control(n: u64, document: u64, kind: ElementKind, checked: bool) -> ElementSnapshot {
    ElementSnapshot {
        state: ElementState::Added,
        data: ElementData {
            element_id: ElementId(uuid(n)),
            document_id: DocumentId(uuid(document)),
            position: 1,
            kind,
            label: format!("control {n}"),
            target: None,
            value: None,
            hidden: false,
            disabled: false,
            checked,
            required: false,
            name: Some("group".to_owned()),
            form_id: Some(FormId(uuid(7))),
        },
    }
}

fn checked(ports: &DocumentPorts, n: u64) -> bool {
    ElementStorage::get(ports, &ElementId(uuid(n)))
        .expect("a stored element")
        .data
        .checked
}

/// `spec/`: a checked checkbox is answered `unchecked` with `CheckednessSet { checked: false }`,
/// an unchecked one `checked` with `checked: true`.
#[test]
fn adversary_document_a_checkbox_is_answered_with_the_outcome_of_its_new_state() {
    let mut ports = DocumentPorts::new();
    ElementStorage::put(&mut ports, control(1, 9, ElementKind::Checkbox, false));
    let check = || Check {
        element_id: ElementId(uuid(1)),
    };
    let first = ports.check(check()).unwrap();
    assert_eq!(
        first,
        CheckOutcome::Checked {
            checkedness_set: CheckednessSet {
                element_id: ElementId(uuid(1)),
                checked: true,
            },
        }
    );
    let second = ports.check(check()).unwrap();
    assert_eq!(
        second,
        CheckOutcome::Unchecked {
            checkedness_set: CheckednessSet {
                element_id: ElementId(uuid(1)),
                checked: false,
            },
        }
    );
}

/// `spec/`: a radio button's group is the radio buttons of its document with its form owner and
/// name. A checkbox of the same name and form, and a radio button of another document that
/// names the same form and name, stay checked; a checked radio button checked again stays so.
#[test]
fn adversary_document_a_radio_group_holds_radio_buttons_of_its_document_only() {
    let mut ports = DocumentPorts::new();
    ElementStorage::put(&mut ports, control(1, 9, ElementKind::Radio, false));
    ElementStorage::put(&mut ports, control(2, 9, ElementKind::Radio, true));
    ElementStorage::put(&mut ports, control(3, 9, ElementKind::Checkbox, true));
    ElementStorage::put(&mut ports, control(4, 8, ElementKind::Radio, true));
    let outcome = ports
        .check(Check {
            element_id: ElementId(uuid(1)),
        })
        .unwrap();
    assert_eq!(
        outcome,
        CheckOutcome::RadioChecked {
            checkedness_set: CheckednessSet {
                element_id: ElementId(uuid(1)),
                checked: true,
            },
        }
    );
    let after: Vec<bool> = [1, 2, 3, 4].map(|n| checked(&ports, n)).to_vec();
    assert_eq!(after, [true, false, true, true]);
    ports
        .check(Check {
            element_id: ElementId(uuid(1)),
        })
        .unwrap();
    assert!(checked(&ports, 1));
}

/// `spec/` (the marker at `surf.document.Check`): a filter that reads an absent `form_id`
/// selects nothing, so a radio button with no form owner unchecks no other, not even one with
/// no form owner and the same name.
#[test]
fn adversary_document_a_radio_button_with_no_form_owner_unchecks_no_other() {
    let mut ports = DocumentPorts::new();
    for (n, checked) in [(1, false), (2, true)] {
        let mut radio = control(n, 9, ElementKind::Radio, checked);
        radio.data.form_id = None;
        ElementStorage::put(&mut ports, radio);
    }
    ports
        .check(Check {
            element_id: ElementId(uuid(1)),
        })
        .unwrap();
    assert_eq!([1, 2].map(|n| checked(&ports, n)), [true, true]);
}

fn option(n: u64, selected: bool, disabled: bool, hidden: bool) -> ListOptionSnapshot {
    ListOptionSnapshot {
        state: OptionState::Added,
        data: ListOptionData {
            option_id: OptionId(uuid(n)),
            select_id: ElementId(uuid(50)),
            position: i64::try_from(n).unwrap(),
            value: format!("v{n}"),
            label: format!("Option {n}"),
            selected,
            disabled,
            hidden,
        },
    }
}

/// `spec/`: a disabled option is refused before a hidden one, each naming the select, and
/// neither changes a row; an enabled, shown one leaves one option of the select selected.
#[test]
fn adversary_document_a_disabled_or_hidden_option_is_refused_naming_its_select() {
    let mut ports = DocumentPorts::new();
    ListOptionStorage::put(&mut ports, option(1, true, false, false));
    ListOptionStorage::put(&mut ports, option(2, false, true, true));
    ListOptionStorage::put(&mut ports, option(3, false, false, true));
    ListOptionStorage::put(&mut ports, option(4, false, false, false));
    let before = ListOptionStorage::list(&ports);
    let pick = |n| SelectOption {
        option_id: OptionId(uuid(n)),
    };
    assert_eq!(
        ports.select_option(pick(2)).unwrap(),
        SelectOptionOutcome::Disabled {
            error: ControlDisabled {
                element_id: ElementId(uuid(50)),
            },
        }
    );
    assert_eq!(
        ports.select_option(pick(3)).unwrap(),
        SelectOptionOutcome::Hidden {
            error: ControlHidden {
                element_id: ElementId(uuid(50)),
            },
        }
    );
    assert_eq!(ListOptionStorage::list(&ports), before);
    assert!(matches!(
        ports.select_option(pick(4)).unwrap(),
        SelectOptionOutcome::Selected { .. }
    ));
    let selected: Vec<bool> = ListOptionStorage::list(&ports)
        .into_iter()
        .map(|option| option.data.selected)
        .collect();
    assert_eq!(selected, [false, false, false, true]);
}
