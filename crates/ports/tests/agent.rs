//! Story agent-session-spec: the `agent` component's ports, through the crate's public surface.
//!
//! What the generation leaves to `AgentPorts`: the storage of `surf.agent.Session`, the two
//! commands whose outcome changes every session a filter selects (`NoteNavigationStarted`,
//! `NoteNavigationEnded`, `instances:`), and the caller a command is sent as.

use surf_ports::{
    AgentPorts, Caller, ComponentPorts, DocumentPorts, NavigatorPorts, RendererPorts, ShellPorts,
    agent_port,
};
use surf_types::agent::{
    AnySession, CloseSession, CloseSessionOutcome, Go, GoOutcome, NoteNavigationEnded,
    NoteNavigationEndedOutcome, NoteNavigationStarted, NoteNavigationStartedOutcome, OpenSession,
    OpenSessionOutcome, Session, SessionData, SessionId, SessionState, Sessions,
};
use surf_types::behaviour::{SessionStorage, TryContext};
use surf_types::navigation::NavigationState;
use surf_types::net::RequestId;
use surf_types::primitives::Uuid;
use surf_types::shell::{TabId, WindowId};
use surf_types::url::Url;

fn uuid(last: u32) -> Uuid {
    Uuid(format!("5a7c1e00-0000-4000-8000-{last:012x}"))
}

fn session_id(last: u32) -> SessionId {
    SessionId(uuid(last))
}

fn tab(last: u32) -> TabId {
    TabId(uuid(last))
}

fn navigation(last: u32) -> RequestId {
    RequestId(uuid(last))
}

fn address() -> Url {
    Url("http://example.org/".to_owned())
}

/// `ports` with an Open session `id` over window `id + 1` and tab `id + 2`, stored directly.
fn with_session(mut ports: AgentPorts, id: u32) -> AgentPorts {
    let data = SessionData {
        session_id: session_id(id),
        window_id: WindowId(uuid(id + 1)),
        tab_id: tab(id + 2),
        holder: session_id(id),
        navigation_id: None,
        url: None,
        navigation_state: None,
    };
    SessionStorage::put(&mut ports, AnySession::Open(Session::new(data)).snapshot());
    ports
}

fn row(rows: &[Sessions], id: u32) -> Sessions {
    rows.iter()
        .find(|row| row.session_id == session_id(id))
        .cloned()
        .unwrap_or_else(|| panic!("no row for session {id}: {rows:?}"))
}

#[test]
fn note_navigation_started_sets_the_navigation_on_the_session_holding_the_tab_and_counts_one() {
    let ports = with_session(with_session(AgentPorts::new(), 0x10), 0x20);
    let mut agent = agent_port(ports);

    let outcome = agent
        .note_navigation_started(NoteNavigationStarted {
            tab_id: tab(0x12),
            navigation_id: navigation(0x30),
            url: address(),
        })
        .expect("the obligation is met");
    let NoteNavigationStartedOutcome::Noted {
        session_navigation_changed,
    } = outcome;
    assert_eq!(session_navigation_changed.navigation_id, navigation(0x30));
    assert_eq!(session_navigation_changed.state, NavigationState::Started);
    assert_eq!(session_navigation_changed.sessions, 1);

    let rows = agent.sessions().expect("the view answers");
    let changed = row(&rows, 0x10);
    assert_eq!(changed.navigation_id, Some(navigation(0x30)));
    assert_eq!(changed.url, Some(address()));
    assert_eq!(changed.navigation_state, Some(NavigationState::Started));
    assert_eq!(changed.state, SessionState::Open);
    let other = row(&rows, 0x20);
    assert_eq!(other.navigation_id, None);
    assert_eq!(other.url, None);
    assert_eq!(other.navigation_state, None);
}

#[test]
fn note_navigation_started_in_a_tab_no_session_holds_changes_nothing_and_counts_zero() {
    let mut agent = agent_port(with_session(AgentPorts::new(), 0x10));
    let before = agent.sessions().expect("the view answers");

    let NoteNavigationStartedOutcome::Noted {
        session_navigation_changed,
    } = agent
        .note_navigation_started(NoteNavigationStarted {
            tab_id: tab(0x99),
            navigation_id: navigation(0x30),
            url: address(),
        })
        .expect("the obligation is met");
    assert_eq!(session_navigation_changed.sessions, 0);
    assert_eq!(agent.sessions().expect("the view answers"), before);
}

#[test]
fn note_navigation_ended_sets_the_state_on_the_session_showing_the_navigation() {
    let mut agent = agent_port(with_session(with_session(AgentPorts::new(), 0x10), 0x20));
    for (tab_id, navigation_id) in [(0x12, 0x30), (0x22, 0x40)] {
        agent
            .note_navigation_started(NoteNavigationStarted {
                tab_id: tab(tab_id),
                navigation_id: navigation(navigation_id),
                url: address(),
            })
            .expect("the obligation is met");
    }

    let NoteNavigationEndedOutcome::Noted {
        session_navigation_changed,
    } = agent
        .note_navigation_ended(NoteNavigationEnded {
            navigation_id: navigation(0x40),
            state: NavigationState::Committed,
        })
        .expect("the obligation is met");
    assert_eq!(session_navigation_changed.navigation_id, navigation(0x40));
    assert_eq!(session_navigation_changed.state, NavigationState::Committed);
    assert_eq!(session_navigation_changed.sessions, 1);

    let rows = agent.sessions().expect("the view answers");
    let ended = row(&rows, 0x20);
    assert_eq!(ended.navigation_id, Some(navigation(0x40)));
    assert_eq!(ended.url, Some(address()));
    assert_eq!(ended.navigation_state, Some(NavigationState::Committed));
    assert_eq!(
        row(&rows, 0x10).navigation_state,
        Some(NavigationState::Started)
    );
}

/// A navigation that ends after a newer one started in the same tab changes nothing.
#[test]
fn note_navigation_ended_for_a_navigation_no_session_shows_changes_nothing_and_counts_zero() {
    let mut agent = agent_port(with_session(AgentPorts::new(), 0x10));
    for navigation_id in [0x30, 0x31] {
        agent
            .note_navigation_started(NoteNavigationStarted {
                tab_id: tab(0x12),
                navigation_id: navigation(navigation_id),
                url: address(),
            })
            .expect("the obligation is met");
    }
    let before = agent.sessions().expect("the view answers");

    let NoteNavigationEndedOutcome::Noted {
        session_navigation_changed,
    } = agent
        .note_navigation_ended(NoteNavigationEnded {
            navigation_id: navigation(0x30),
            state: NavigationState::Cancelled,
        })
        .expect("the obligation is met");
    assert_eq!(session_navigation_changed.sessions, 0);
    assert_eq!(agent.sessions().expect("the view answers"), before);
    assert_eq!(
        row(&before, 0x10).navigation_state,
        Some(NavigationState::Started)
    );
}

/// The generated holder check reads the caller from the ports: a command is sent as whoever the
/// shared [`Caller`] names when it runs.
#[test]
fn a_command_is_sent_as_the_session_the_shared_caller_names() {
    let caller = Caller::new();
    let mut agent = agent_port(AgentPorts::with_caller(caller.clone()));
    let opened = agent
        .open_session(OpenSession {
            session_id: session_id(0x10),
            window_id: WindowId(uuid(0x11)),
            tab_id: tab(0x12),
        })
        .expect("the obligation is met");
    assert!(
        matches!(opened, OpenSessionOutcome::Opened { .. }),
        "{opened:?}"
    );
    let go = || Go {
        session_id: session_id(0x10),
        url: address(),
    };

    // A caller carrying no `session_id` is no `SessionHolder`: the model declares no outcome for
    // it, and the generated check answers that as an unmet obligation.
    assert!(agent.go(go()).is_err());
    caller.set(Some(session_id(0x20)));
    assert!(matches!(agent.go(go()), Ok(GoOutcome::NotHolder { .. })));
    caller.set(Some(session_id(0x10)));
    assert!(matches!(agent.go(go()), Ok(GoOutcome::Requested { .. })));
    assert_eq!(caller.get(), Some(session_id(0x10)));

    let closed = agent.close_session(CloseSession {
        session_id: session_id(0x10),
    });
    assert!(
        matches!(closed, Ok(CloseSessionOutcome::Closed { .. })),
        "{closed:?}"
    );
    caller.set(None);
    assert_eq!(caller.get(), None);
}

#[test]
fn agent_ports_answer_the_caller_and_assign_no_identity() {
    let caller = Caller::new();
    let mut ports = AgentPorts::with_caller(caller.clone());
    assert_eq!(ports.try_caller_session_id(), Ok(None));
    caller.set(Some(session_id(0x10)));
    assert_eq!(ports.try_caller_session_id(), Ok(Some(session_id(0x10))));
    assert_eq!(ports.caller().get(), Some(session_id(0x10)));

    assert!(ports.try_generate_surf_document_document_id().is_err());
    assert!(ports.try_generate_surf_net_request_id().is_err());
    assert!(ports.try_generate_surf_render_frame_id().is_err());
    assert!(ports.try_generate_surf_shell_tab_id().is_err());
    assert!(ports.try_generate_surf_shell_window_id().is_err());
}

/// Only the `agent` component's commands read a caller; the other ports types know none and do
/// not make one up.
#[test]
fn the_other_ports_answer_the_caller_as_unmet() {
    assert!(ShellPorts::new().try_caller_session_id().is_err());
    assert!(NavigatorPorts::new().try_caller_session_id().is_err());
    assert!(DocumentPorts::new().try_caller_session_id().is_err());
    assert!(RendererPorts::new().try_caller_session_id().is_err());
}

#[test]
fn component_ports_hold_the_agent_port_and_its_caller() {
    let caller = Caller::new();
    let ComponentPorts { mut agent, .. } = ComponentPorts::with_caller(caller.clone());
    caller.set(Some(session_id(0x10)));
    agent
        .open_session(OpenSession {
            session_id: session_id(0x10),
            window_id: WindowId(uuid(0x11)),
            tab_id: tab(0x12),
        })
        .expect("the obligation is met");
    assert!(matches!(
        agent.go(Go {
            session_id: session_id(0x10),
            url: address(),
        }),
        Ok(GoOutcome::Requested { .. })
    ));
    assert_eq!(agent.drain_outbox().len(), 2);
    assert_eq!(
        ComponentPorts::new()
            .agent
            .sessions()
            .expect("the view answers"),
        Vec::new()
    );
}
