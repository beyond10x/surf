//! The story's tests 1 to 4, through the crate's public surface only: the way another crate
//! (`headless-cli`) uses it.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::io;

use surf_ports::{
    AgentPorts, ComponentPorts, DocumentPorts, NavigatorPorts, NetPorts, RandomSource,
    RendererPorts, ShellPorts, shell_port,
};
use surf_types::agent::{
    ActionCommand, ActionData, ActionId, ActionSnapshot, ActionState, SessionId,
};
use surf_types::behaviour::{
    ActionStorage, DocumentStorage, ElementStorage, FormStorage, FrameStorage, ListOptionStorage,
    NavigationStorage, RequestStorage, SubmissionStorage, TabStorage, TryContext, WindowStorage,
};
use surf_types::document::obligations::{CheckBehavior, SelectOptionBehavior};
use surf_types::document::{
    Check, CheckOutcome, DocumentData, DocumentId, DocumentSnapshot, DocumentState, ElementData,
    ElementId, ElementKind, ElementSnapshot, ElementState, EncodingName, FormData, FormId,
    FormSnapshot, FormState, ListOptionData, ListOptionSnapshot, OptionId, OptionState, ParseLimit,
    SelectOption, SelectOptionOutcome, SubmissionData, SubmissionId, SubmissionSnapshot,
    SubmissionState,
};
use surf_types::navigation::{
    HistoryHandling, NavigationData, NavigationSnapshot, NavigationState,
};
use surf_types::net::{Method, RequestData, RequestId, RequestSnapshot, RequestState};
use surf_types::obligation::UnmetObligation;
use surf_types::primitives::Uuid;
use surf_types::render::{FrameData, FrameId, FrameSnapshot, FrameState};
use surf_types::shell::{
    OpenWindow, OpenWindowOutcome, TabData, TabId, TabSnapshot, TabState, WindowData, WindowId,
    WindowSnapshot, WindowState, Windows,
};
use surf_types::url::Url;

// ---- test 1: the eleven storages, and the two obligations of the document component ---------

/// One storage trait of one ports type, as plain functions.
struct Table<P, K, S> {
    get: fn(&P, &K) -> Option<S>,
    put: fn(&mut P, S),
    delete: fn(&mut P, &K),
    list: fn(&P) -> Vec<S>,
}

/// `put`, `get`, `list` in insertion order, `put` of a held identity replacing that row where it
/// stands, and `delete`. `replacement` carries the identity `keys[1]`.
fn exercise<P, K, S: Clone + PartialEq + Debug>(
    mut ports: P,
    table: Table<P, K, S>,
    keys: [K; 3],
    rows: [S; 3],
    replacement: S,
) {
    let Table {
        get,
        put,
        delete,
        list,
    } = table;
    let [first, second, third] = rows;

    assert_eq!(list(&ports), Vec::<S>::new());
    assert_eq!(get(&ports, &keys[0]), None);

    put(&mut ports, first.clone());
    put(&mut ports, second.clone());
    put(&mut ports, third.clone());
    assert_eq!(
        list(&ports),
        vec![first.clone(), second.clone(), third.clone()]
    );
    assert_eq!(get(&ports, &keys[0]), Some(first.clone()));
    assert_eq!(get(&ports, &keys[1]), Some(second.clone()));
    assert_eq!(get(&ports, &keys[2]), Some(third.clone()));

    put(&mut ports, replacement.clone());
    assert_eq!(
        list(&ports),
        vec![first.clone(), replacement.clone(), third.clone()]
    );
    assert_eq!(get(&ports, &keys[1]), Some(replacement.clone()));

    delete(&mut ports, &keys[0]);
    assert_eq!(get(&ports, &keys[0]), None);
    assert_eq!(list(&ports), vec![replacement.clone(), third.clone()]);

    delete(&mut ports, &keys[0]);
    assert_eq!(list(&ports), vec![replacement.clone(), third.clone()]);

    put(&mut ports, first.clone());
    assert_eq!(list(&ports), vec![replacement, third, first]);
}

fn uuid(n: u64) -> Uuid {
    Uuid(format!("00000000-0000-4000-8000-{n:012x}"))
}

fn url(path: &str) -> Url {
    Url(format!("https://example.org/{path}"))
}

#[test]
fn storage_window_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| WindowId(uuid(n)));
    let rows = keys.clone().map(|window_id| WindowSnapshot {
        state: WindowState::Open,
        data: WindowData { window_id },
    });
    // A window has one state and no field besides its identity, so the replacement equals the
    // row it replaces; the count and the order still tell a replacement from an append.
    let replacement = rows[1].clone();
    exercise(
        ShellPorts::new(),
        Table {
            get: <ShellPorts as WindowStorage>::get,
            put: <ShellPorts as WindowStorage>::put,
            delete: <ShellPorts as WindowStorage>::delete,
            list: <ShellPorts as WindowStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

#[test]
fn storage_tab_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| TabId(uuid(n)));
    let rows = keys.clone().map(|tab_id| TabSnapshot {
        state: TabState::Open,
        data: TabData {
            tab_id,
            window_id: WindowId(uuid(9)),
        },
    });
    let replacement = TabSnapshot {
        state: TabState::Closed,
        ..rows[1].clone()
    };
    exercise(
        ShellPorts::new(),
        Table {
            get: <ShellPorts as TabStorage>::get,
            put: <ShellPorts as TabStorage>::put,
            delete: <ShellPorts as TabStorage>::delete,
            list: <ShellPorts as TabStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

#[test]
fn storage_navigation_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| RequestId(uuid(n)));
    let rows = keys.clone().map(|navigation_id| NavigationSnapshot {
        state: NavigationState::Started,
        data: NavigationData {
            navigation_id,
            tab_id: TabId(uuid(9)),
            url: url("a"),
            method: Method::Get,
            handling: HistoryHandling::Push,
            standing: None,
            next: None,
            entry_id: None,
            entry_next: None,
        },
    });
    let replacement = NavigationSnapshot {
        state: NavigationState::Committed,
        ..rows[1].clone()
    };
    exercise(
        NavigatorPorts::new(),
        Table {
            get: <NavigatorPorts as NavigationStorage>::get,
            put: <NavigatorPorts as NavigationStorage>::put,
            delete: <NavigatorPorts as NavigationStorage>::delete,
            list: <NavigatorPorts as NavigationStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

#[test]
fn storage_request_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| RequestId(uuid(n)));
    let rows = keys.clone().map(|request_id| RequestSnapshot {
        state: RequestState::Pending,
        data: RequestData {
            request_id,
            url: url("a"),
            method: Method::Get,
            body_type: None,
        },
    });
    let replacement = RequestSnapshot {
        state: RequestState::Responded,
        ..rows[1].clone()
    };
    exercise(
        NetPorts::new(),
        Table {
            get: <NetPorts as RequestStorage>::get,
            put: <NetPorts as RequestStorage>::put,
            delete: <NetPorts as RequestStorage>::delete,
            list: <NetPorts as RequestStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

#[test]
fn storage_document_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| DocumentId(uuid(n)));
    let rows = keys.clone().map(|document_id| DocumentSnapshot {
        state: DocumentState::Parsing,
        data: DocumentData {
            document_id,
            url: url("a"),
            navigation_id: RequestId(uuid(9)),
            tab_id: TabId(uuid(8)),
            title: None,
            encoding: None,
            unsupported_encoding: None,
            stopped_by: None,
        },
    });
    let mut replacement = rows[1].clone();
    replacement.state = DocumentState::Parsed;
    replacement.data.url = url("b");
    replacement.data.title = Some(String::new());
    replacement.data.encoding = Some(EncodingName("windows-1252".to_owned()));
    replacement.data.unsupported_encoding = Some(EncodingName("EUC-KR".to_owned()));
    replacement.data.stopped_by = Some(ParseLimit::Depth);
    exercise(
        DocumentPorts::new(),
        Table {
            get: <DocumentPorts as DocumentStorage>::get,
            put: <DocumentPorts as DocumentStorage>::put,
            delete: <DocumentPorts as DocumentStorage>::delete,
            list: <DocumentPorts as DocumentStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

#[test]
fn storage_element_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| ElementId(uuid(n)));
    let rows = keys.clone().map(|element_id| ElementSnapshot {
        state: ElementState::Added,
        data: ElementData {
            element_id,
            document_id: DocumentId(uuid(9)),
            position: 1,
            kind: ElementKind::Link,
            label: "Home".to_owned(),
            target: Some("/".to_owned()),
            value: None,
            hidden: false,
            disabled: false,
            checked: false,
            required: false,
            readonly: false,
            name: None,
            form_id: None,
            radio_group: None,
        },
    });
    // An element has one state, so the replacement differs in its fields.
    let mut replacement = rows[1].clone();
    replacement.data.kind = ElementKind::Text;
    replacement.data.label = "Name".to_owned();
    replacement.data.target = None;
    replacement.data.value = Some("Ada".to_owned());
    replacement.data.required = true;
    exercise(
        DocumentPorts::new(),
        Table {
            get: <DocumentPorts as ElementStorage>::get,
            put: <DocumentPorts as ElementStorage>::put,
            delete: <DocumentPorts as ElementStorage>::delete,
            list: <DocumentPorts as ElementStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

#[test]
fn storage_form_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| FormId(uuid(n)));
    let rows = keys.clone().map(|form_id| FormSnapshot {
        state: FormState::Added,
        data: FormData {
            form_id,
            document_id: DocumentId(uuid(9)),
            position: 1,
            action: Some("/session".to_owned()),
            method: Some("post".to_owned()),
            enctype: None,
        },
    });
    // A form has one state, so the replacement differs in its fields.
    let mut replacement = rows[1].clone();
    replacement.data.position = 2;
    replacement.data.method = None;
    replacement.data.enctype = Some("text/plain".to_owned());
    exercise(
        DocumentPorts::new(),
        Table {
            get: <DocumentPorts as FormStorage>::get,
            put: <DocumentPorts as FormStorage>::put,
            delete: <DocumentPorts as FormStorage>::delete,
            list: <DocumentPorts as FormStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

#[test]
fn storage_option_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| OptionId(uuid(n)));
    let rows = keys.clone().map(|option_id| ListOptionSnapshot {
        state: OptionState::Added,
        data: ListOptionData {
            option_id,
            select_id: ElementId(uuid(9)),
            position: 1,
            value: "at".to_owned(),
            label: "Austria".to_owned(),
            selected: false,
            disabled: false,
            hidden: false,
        },
    });
    let mut replacement = rows[1].clone();
    replacement.data.selected = true;
    replacement.data.label = "Germany".to_owned();
    exercise(
        DocumentPorts::new(),
        Table {
            get: <DocumentPorts as ListOptionStorage>::get,
            put: <DocumentPorts as ListOptionStorage>::put,
            delete: <DocumentPorts as ListOptionStorage>::delete,
            list: <DocumentPorts as ListOptionStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

#[test]
fn storage_submission_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| SubmissionId(uuid(n)));
    let rows = keys.clone().map(|submission_id| SubmissionSnapshot {
        state: SubmissionState::Started,
        data: SubmissionData {
            submission_id,
            submitter: ElementId(uuid(9)),
        },
    });
    let mut replacement = rows[1].clone();
    replacement.state = SubmissionState::Submitted;
    exercise(
        DocumentPorts::new(),
        Table {
            get: <DocumentPorts as SubmissionStorage>::get,
            put: <DocumentPorts as SubmissionStorage>::put,
            delete: <DocumentPorts as SubmissionStorage>::delete,
            list: <DocumentPorts as SubmissionStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

/// A control of document 9 at `n`, unchecked, shown and enabled.
fn control(n: u8, kind: ElementKind, name: Option<&str>, form: Option<u8>) -> ElementSnapshot {
    ElementSnapshot {
        state: ElementState::Added,
        data: ElementData {
            element_id: ElementId(uuid(u64::from(n))),
            document_id: DocumentId(uuid(9)),
            position: i64::from(n),
            kind,
            label: format!("control {n}"),
            target: None,
            value: None,
            hidden: false,
            disabled: false,
            checked: false,
            required: false,
            readonly: false,
            name: name.map(str::to_owned),
            form_id: form.map(|form| FormId(uuid(u64::from(form)))),
            radio_group: (kind == ElementKind::Radio)
                .then_some(name)
                .flatten()
                .map(|name| match form {
                    Some(form) => format!("F{form} {name}"),
                    None => format!("- {name}"),
                }),
        },
    }
}

fn checked(ports: &DocumentPorts, n: u8) -> bool {
    ElementStorage::get(ports, &ElementId(uuid(u64::from(n))))
        .expect("a stored element")
        .data
        .checked
}

#[test]
fn check_on_a_radio_unchecks_its_group_only_and_toggles_a_checkbox() {
    let mut ports = DocumentPorts::new();
    let radios = [
        (1, Some("ship"), Some(7)),
        (2, Some("ship"), Some(7)),
        (3, Some("ship"), Some(8)),
        (4, Some("pay"), Some(7)),
        (5, None, Some(7)),
        (6, Some("ship"), None),
        (7, Some("ship"), None),
    ];
    for (n, name, form) in radios {
        let mut radio = control(n, ElementKind::Radio, name, form);
        radio.data.checked = true;
        ElementStorage::put(&mut ports, radio);
    }
    ElementStorage::put(&mut ports, control(10, ElementKind::Checkbox, None, None));
    let outcome = ports.check(Check {
        element_id: ElementId(uuid(2)),
    });
    assert!(
        matches!(outcome, Ok(CheckOutcome::RadioChecked { .. })),
        "{outcome:?}"
    );
    let after: Vec<bool> = [1, 2, 3, 4, 5, 6, 7]
        .into_iter()
        .map(|n| checked(&ports, n))
        .collect();
    // Only the radio of the same document, form owner and name is unchecked; a radio with no
    // name has no `radio_group` and is a group of its own.
    assert_eq!(after, [false, true, true, true, true, true, true]);
    // The radio buttons with no form owner and one name are one group.
    ports
        .check(Check {
            element_id: ElementId(uuid(6)),
        })
        .unwrap();
    assert_eq!([6, 7].map(|n| checked(&ports, n)), [true, false]);
    assert!(checked(&ports, 2) && checked(&ports, 3));
    for expected in [true, false] {
        let outcome = ports.check(Check {
            element_id: ElementId(uuid(10)),
        });
        assert!(outcome.is_ok(), "{outcome:?}");
        assert_eq!(checked(&ports, 10), expected);
    }
}

#[test]
fn check_refuses_a_link_a_disabled_and_a_hidden_control_and_changes_none() {
    let mut ports = DocumentPorts::new();
    ElementStorage::put(&mut ports, control(1, ElementKind::Link, None, None));
    let mut disabled = control(2, ElementKind::Checkbox, None, None);
    disabled.data.disabled = true;
    disabled.data.hidden = true;
    ElementStorage::put(&mut ports, disabled);
    let mut hidden = control(3, ElementKind::Checkbox, None, None);
    hidden.data.hidden = true;
    ElementStorage::put(&mut ports, hidden);
    let before = ElementStorage::list(&ports);
    let answers = [1, 2, 3, 4].map(|n| {
        ports.check(Check {
            element_id: ElementId(uuid(n)),
        })
    });
    assert!(
        matches!(answers[0], Ok(CheckOutcome::NotCheckable { .. })),
        "{answers:?}"
    );
    // Disabled is answered before hidden, in the order `spec/` declares them.
    assert!(
        matches!(answers[1], Ok(CheckOutcome::Disabled { .. })),
        "{answers:?}"
    );
    assert!(
        matches!(answers[2], Ok(CheckOutcome::Hidden { .. })),
        "{answers:?}"
    );
    assert!(
        matches!(answers[3], Ok(CheckOutcome::NoSuchElement { .. })),
        "{answers:?}"
    );
    assert_eq!(ElementStorage::list(&ports), before);
}

#[test]
fn select_option_selects_one_and_unselects_the_others_of_its_select_only() {
    let mut ports = DocumentPorts::new();
    for (n, select, selected) in [(1, 8, true), (2, 8, false), (3, 9, true)] {
        ListOptionStorage::put(
            &mut ports,
            ListOptionSnapshot {
                state: OptionState::Added,
                data: ListOptionData {
                    option_id: OptionId(uuid(n)),
                    select_id: ElementId(uuid(select)),
                    position: 1,
                    value: format!("v{n}"),
                    label: format!("Option {n}"),
                    selected,
                    disabled: false,
                    hidden: false,
                },
            },
        );
    }
    let outcome = ports.select_option(SelectOption {
        option_id: OptionId(uuid(2)),
    });
    assert!(
        matches!(outcome, Ok(SelectOptionOutcome::Selected { .. })),
        "{outcome:?}"
    );
    let selected: Vec<bool> = ListOptionStorage::list(&ports)
        .into_iter()
        .map(|option| option.data.selected)
        .collect();
    assert_eq!(selected, [false, true, true]);
}

#[test]
fn storage_document_and_element_tables_of_one_ports_value_are_apart() {
    let mut ports = DocumentPorts::new();
    DocumentStorage::put(
        &mut ports,
        DocumentSnapshot {
            state: DocumentState::Parsing,
            data: DocumentData {
                document_id: DocumentId(uuid(1)),
                url: url("a"),
                navigation_id: RequestId(uuid(9)),
                tab_id: TabId(uuid(8)),
                title: None,
                encoding: None,
                unsupported_encoding: None,
                stopped_by: None,
            },
        },
    );
    assert_eq!(ElementStorage::list(&ports), Vec::new());
    ElementStorage::put(
        &mut ports,
        ElementSnapshot {
            state: ElementState::Added,
            data: ElementData {
                element_id: ElementId(uuid(1)),
                document_id: DocumentId(uuid(1)),
                position: 1,
                kind: ElementKind::Select,
                label: "Country".to_owned(),
                target: None,
                value: None,
                hidden: false,
                disabled: false,
                checked: false,
                required: false,
                readonly: false,
                name: None,
                form_id: None,
                radio_group: None,
            },
        },
    );
    assert_eq!(DocumentStorage::list(&ports).len(), 1);
    assert_eq!(ElementStorage::list(&ports).len(), 1);
    ElementStorage::delete(&mut ports, &ElementId(uuid(1)));
    assert_eq!(DocumentStorage::list(&ports).len(), 1);
    assert_eq!(ElementStorage::list(&ports), Vec::new());
}

#[test]
fn storage_frame_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| FrameId(uuid(n)));
    let rows = keys.clone().map(|frame_id| FrameSnapshot {
        state: FrameState::Painted,
        data: FrameData {
            frame_id,
            document_id: DocumentId(uuid(9)),
            width: 800,
            height: 600,
        },
    });
    let mut replacement = rows[1].clone();
    replacement.data.width = 1024;
    exercise(
        RendererPorts::new(),
        Table {
            get: <RendererPorts as FrameStorage>::get,
            put: <RendererPorts as FrameStorage>::put,
            delete: <RendererPorts as FrameStorage>::delete,
            list: <RendererPorts as FrameStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

#[test]
fn storage_action_put_get_list_in_insertion_order_replace_in_place_delete() {
    let keys = [1, 2, 3].map(|n| ActionId(uuid(n)));
    let rows = keys.clone().map(|action_id| ActionSnapshot {
        state: ActionState::Recorded,
        data: ActionData {
            action_id,
            session_id: SessionId(uuid(9)),
            sequence: 1,
            command: ActionCommand::Open,
            url: None,
            handle: None,
            label: None,
        },
    });
    let mut replacement = rows[1].clone();
    replacement.data.command = ActionCommand::Go;
    replacement.data.url = Some(url("next"));
    exercise(
        AgentPorts::new(),
        Table {
            get: <AgentPorts as ActionStorage>::get,
            put: <AgentPorts as ActionStorage>::put,
            delete: <AgentPorts as ActionStorage>::delete,
            list: <AgentPorts as ActionStorage>::list,
        },
        keys,
        rows,
        replacement,
    );
}

// ---- test 2: identities ----------------------------------------------------------------------

/// Whether `text` is a version 4 UUID in canonical lower-case 8-4-4-4-12 form.
fn is_version_4(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(at, byte)| match at {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(byte),
        })
        && bytes[14] == b'4'
        && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
}

#[test]
fn identities_1000_assigned_are_distinct_with_version_4_and_variant_10_bits() {
    let mut ports = ShellPorts::new();
    let mut seen = BTreeSet::new();
    for _ in 0..1000 {
        let WindowId(Uuid(text)) = ports.try_generate_surf_shell_window_id().expect("assigned");
        assert!(is_version_4(&text), "not a version 4 UUID: {text}");
        assert!(seen.insert(text.clone()), "assigned twice: {text}");
    }
    assert_eq!(seen.len(), 1000);
}

/// The byte a [`Fixed`] source hands out at each position.
type ByteAt = fn(usize) -> u8;

/// A source that hands out fixed bytes and records how many it was asked for.
struct Fixed {
    bytes: ByteAt,
    asked: Vec<usize>,
}

impl RandomSource for Fixed {
    fn fill(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        self.asked.push(buffer.len());
        for (at, byte) in buffer.iter_mut().enumerate() {
            *byte = (self.bytes)(at);
        }
        Ok(())
    }
}

#[test]
fn identity_text_is_the_sources_16_bytes_with_version_and_variant_set_in_lower_case_8_4_4_4_12() {
    let cases: [(ByteAt, &str); 3] = [
        (|_| 0x00, "00000000-0000-4000-8000-000000000000"),
        (|_| 0xff, "ffffffff-ffff-4fff-bfff-ffffffffffff"),
        (|at| at as u8, "00010203-0405-4607-8809-0a0b0c0d0e0f"),
    ];
    for (bytes, expected) in cases {
        let mut ports = ShellPorts::with_source(Fixed {
            bytes,
            asked: Vec::new(),
        });
        let assigned = ports.try_generate_surf_shell_tab_id();
        assert_eq!(assigned, Ok(TabId(Uuid(expected.to_owned()))));
    }
    let mut source = Fixed {
        bytes: |_| 0,
        asked: Vec::new(),
    };
    let mut ports = DocumentPorts::with_source(&mut source);
    ports
        .try_generate_surf_document_document_id()
        .expect("assigned");
    drop(ports);
    assert_eq!(source.asked, vec![16]);

    let mut ports = DocumentPorts::with_source(Fixed {
        bytes: |at| at as u8,
        asked: Vec::new(),
    });
    assert_eq!(
        ports.try_generate_surf_document_element_id(),
        Ok(ElementId(Uuid(
            "00010203-0405-4607-8809-0a0b0c0d0e0f".to_owned()
        )))
    );
}

#[test]
fn element_identities_1000_assigned_are_distinct_version_4_and_unmet_on_the_agent_ports() {
    let mut ports = DocumentPorts::new();
    let mut seen = BTreeSet::new();
    for _ in 0..1000 {
        let ElementId(Uuid(text)) = ports
            .try_generate_surf_document_element_id()
            .expect("assigned");
        assert!(is_version_4(&text), "not a version 4 UUID: {text}");
        assert!(seen.insert(text.clone()), "assigned twice: {text}");
    }
    assert_eq!(seen.len(), 1000);
    assert_eq!(
        AgentPorts::new().try_generate_surf_document_element_id(),
        Err(UnmetObligation {
            capability: "context answer",
            source: "surf.document.ElementId",
        })
    );
}

// ---- test 3: a source that fails -------------------------------------------------------------

/// A source that can never be read.
struct Failing;

impl RandomSource for Failing {
    fn fill(&mut self, _buffer: &mut [u8]) -> io::Result<()> {
        Err(io::Error::other("no random source"))
    }
}

fn unmet(source: &'static str) -> UnmetObligation {
    UnmetObligation {
        capability: "context answer",
        source,
    }
}

#[test]
fn failing_source_answers_identity_with_unmet_obligation_and_open_window_returns_it() {
    let mut ports = ShellPorts::with_source(Failing);
    assert_eq!(
        ports.try_generate_surf_shell_window_id(),
        Err(unmet("surf.shell.WindowId"))
    );
    assert_eq!(
        ports.try_generate_surf_shell_tab_id(),
        Err(unmet("surf.shell.TabId"))
    );
    assert_eq!(
        ports.try_generate_surf_net_request_id(),
        Err(unmet("surf.net.RequestId"))
    );
    assert_eq!(
        ports.try_generate_surf_document_document_id(),
        Err(unmet("surf.document.DocumentId"))
    );
    assert_eq!(
        ports.try_generate_surf_render_frame_id(),
        Err(unmet("surf.render.FrameId"))
    );
    assert_eq!(
        NavigatorPorts::with_source(Failing).try_generate_surf_net_request_id(),
        Err(unmet("surf.net.RequestId"))
    );
    assert_eq!(
        DocumentPorts::with_source(Failing).try_generate_surf_document_document_id(),
        Err(unmet("surf.document.DocumentId"))
    );
    assert_eq!(
        DocumentPorts::with_source(Failing).try_generate_surf_document_element_id(),
        Err(unmet("surf.document.ElementId"))
    );
    assert_eq!(
        RendererPorts::with_source(Failing).try_generate_surf_render_frame_id(),
        Err(unmet("surf.render.FrameId"))
    );

    let mut shell = shell_port(ShellPorts::with_source(Failing));
    assert_eq!(
        shell.open_window(OpenWindow {}),
        Err(unmet("surf.shell.WindowId"))
    );
    assert_eq!(shell.windows(), Ok(Vec::new()));
    assert_eq!(shell.drain_outbox(), Vec::new());
}

// ---- test 4: the five component ports --------------------------------------------------------

#[test]
fn five_component_ports_build_and_open_window_answers_opened_then_one_windows_row() {
    let ComponentPorts {
        agent,
        mut shell,
        navigator,
        net,
        document,
        renderer,
    } = ComponentPorts::new();

    let OpenWindowOutcome::Opened { window_opened } =
        shell.open_window(OpenWindow {}).expect("opened");
    let WindowId(Uuid(text)) = &window_opened.window_id;
    assert!(is_version_4(text), "not a version 4 UUID: {text}");
    assert_eq!(
        shell.windows(),
        Ok(vec![Windows {
            window_id: window_opened.window_id.clone()
        }])
    );
    assert_eq!(
        shell.drain_outbox(),
        vec![shell::PublishedEvent::WindowOpened(window_opened)]
    );

    assert_eq!(navigator.navigations(), Ok(Vec::new()));
    assert_eq!(net.requests(), Ok(Vec::new()));
    assert_eq!(document.documents(), Ok(Vec::new()));
    assert_eq!(document.elements(DocumentId(uuid(1))), Ok(Vec::new()));
    assert_eq!(renderer.frames(), Ok(Vec::new()));
    assert_eq!(agent.sessions(), Ok(Vec::new()));
}
