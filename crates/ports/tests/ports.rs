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
    ActionStorage, DocumentStorage, FrameStorage, NavigationStorage, RequestStorage, TabStorage,
    TryContext, WindowStorage,
};
use surf_types::document::{DocumentData, DocumentId, DocumentSnapshot, DocumentState};
use surf_types::navigation::{NavigationData, NavigationSnapshot, NavigationState};
use surf_types::net::{RequestData, RequestId, RequestSnapshot, RequestState};
use surf_types::obligation::UnmetObligation;
use surf_types::primitives::Uuid;
use surf_types::render::{FrameData, FrameId, FrameSnapshot, FrameState};
use surf_types::shell::{
    OpenWindow, OpenWindowOutcome, TabData, TabId, TabSnapshot, TabState, WindowData, WindowId,
    WindowSnapshot, WindowState, Windows,
};
use surf_types::url::Url;

// ---- test 1: the six storages ----------------------------------------------------------------

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
        state: DocumentState::Parsed,
        data: DocumentData {
            document_id,
            url: url("a"),
        },
    });
    let mut replacement = rows[1].clone();
    replacement.data.url = url("b");
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
    assert_eq!(renderer.frames(), Ok(Vec::new()));
    assert_eq!(agent.sessions(), Ok(Vec::new()));
}
