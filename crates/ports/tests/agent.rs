//! Stories agent-session-spec and agent-page-path-spec: the `agent` component's ports, through
//! the crate's public surface.
//!
//! What the generation leaves to `AgentPorts`: the storage of `surf.agent.Session` and
//! `surf.agent.Action`, the identity of an action, the four commands whose outcome changes every
//! session a filter selects (`NoteNavigationStarted`, `NoteNavigationCommitted`,
//! `NoteNavigationFailed`, `NoteNavigationEnded`, `instances:`), and the caller a command is sent
//! as.

use std::io;

use surf_ports::{
    AgentPorts, Caller, ComponentPorts, DocumentPorts, NavigatorPorts, RandomSource, RendererPorts,
    ShellPorts, agent_port,
};
use surf_types::agent::{
    ActionCommand, ActionId, AnySession, CloseSession, CloseSessionOutcome, Go, GoOutcome,
    NoteNavigationCommitted, NoteNavigationCommittedOutcome, NoteNavigationEnded,
    NoteNavigationEndedOutcome, NoteNavigationFailed, NoteNavigationFailedOutcome,
    NoteNavigationStarted, NoteNavigationStartedOutcome, OpenSession, OpenSessionOutcome,
    RecordAction, RecordActionOutcome, Session, SessionData, SessionId, SessionLog, SessionState,
    Sessions,
};
use surf_types::behaviour::{SessionStorage, TryContext};
use surf_types::navigation::NavigationState;
use surf_types::net::{FailureReason, MediaType, RequestId, Status};
use surf_types::obligation::UnmetObligation;
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
fn with_session<R: RandomSource>(ports: AgentPorts<R>, id: u32) -> AgentPorts<R> {
    with_session_data(ports, session_data(id))
}

/// An Open session `id` over window `id + 1` and tab `id + 2` that has not navigated.
fn session_data(id: u32) -> SessionData {
    SessionData {
        session_id: session_id(id),
        window_id: WindowId(uuid(id + 1)),
        tab_id: tab(id + 2),
        holder: session_id(id),
        navigation_id: None,
        url: None,
        navigation_state: None,
        status: None,
        media_type: None,
        failure_reason: None,
    }
}

/// `ports` with `data` stored directly as an Open session.
fn with_session_data<R: RandomSource>(
    mut ports: AgentPorts<R>,
    data: SessionData,
) -> AgentPorts<R> {
    SessionStorage::put(&mut ports, AnySession::Open(Session::new(data)).snapshot());
    ports
}

fn ok() -> Status {
    Status(200)
}

fn html() -> MediaType {
    MediaType("text/html".to_owned())
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

/// A new navigation leaves nothing of the page before it: the status, the media type and the
/// failure reason are cleared.
#[test]
fn note_navigation_started_clears_the_status_media_type_and_failure_reason() {
    let shown = SessionData {
        navigation_id: Some(navigation(0x30)),
        url: Some(address()),
        navigation_state: Some(NavigationState::Committed),
        status: Some(ok()),
        media_type: Some(html()),
        failure_reason: Some(FailureReason::Unreachable),
        ..session_data(0x10)
    };
    let mut agent = agent_port(with_session_data(AgentPorts::new(), shown));

    agent
        .note_navigation_started(NoteNavigationStarted {
            tab_id: tab(0x12),
            navigation_id: navigation(0x31),
            url: address(),
        })
        .expect("the obligation is met");

    let started = row(&agent.sessions().expect("the view answers"), 0x10);
    assert_eq!(started.navigation_id, Some(navigation(0x31)));
    assert_eq!(started.navigation_state, Some(NavigationState::Started));
    assert_eq!(started.status, None);
    assert_eq!(started.media_type, None);
    assert_eq!(started.failure_reason, None);
}

#[test]
fn note_navigation_committed_sets_the_response_address_and_committed_on_the_session_showing_it() {
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

    let NoteNavigationCommittedOutcome::Noted {
        session_navigation_changed,
    } = agent
        .note_navigation_committed(NoteNavigationCommitted {
            navigation_id: navigation(0x40),
            url: Url("http://example.org/after-redirect".to_owned()),
            status: Status(404),
            media_type: html(),
        })
        .expect("the obligation is met");
    assert_eq!(session_navigation_changed.navigation_id, navigation(0x40));
    assert_eq!(session_navigation_changed.state, NavigationState::Committed);
    assert_eq!(session_navigation_changed.sessions, 1);

    let rows = agent.sessions().expect("the view answers");
    let committed = row(&rows, 0x20);
    // The session shows the address the response came from, not the one asked for.
    assert_eq!(
        committed.url,
        Some(Url("http://example.org/after-redirect".to_owned()))
    );
    assert_eq!(committed.navigation_state, Some(NavigationState::Committed));
    assert_eq!(committed.status, Some(Status(404)));
    assert_eq!(committed.media_type, Some(html()));
    assert_eq!(committed.failure_reason, None);
    let other = row(&rows, 0x10);
    assert_eq!(other.url, Some(address()));
    assert_eq!(other.navigation_state, Some(NavigationState::Started));
    assert_eq!(other.status, None);
    assert_eq!(other.media_type, None);

    let NoteNavigationCommittedOutcome::Noted {
        session_navigation_changed,
    } = agent
        .note_navigation_committed(NoteNavigationCommitted {
            navigation_id: navigation(0x99),
            url: Url("http://example.org/elsewhere".to_owned()),
            status: ok(),
            media_type: html(),
        })
        .expect("the obligation is met");
    assert_eq!(session_navigation_changed.sessions, 0);
    assert_eq!(agent.sessions().expect("the view answers"), rows);
}

#[test]
fn note_navigation_failed_sets_failed_and_the_reason_on_the_session_showing_it() {
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

    let NoteNavigationFailedOutcome::Noted {
        session_navigation_changed,
    } = agent
        .note_navigation_failed(NoteNavigationFailed {
            navigation_id: navigation(0x30),
            reason: FailureReason::UnsupportedScheme,
        })
        .expect("the obligation is met");
    assert_eq!(session_navigation_changed.navigation_id, navigation(0x30));
    assert_eq!(session_navigation_changed.state, NavigationState::Failed);
    assert_eq!(session_navigation_changed.sessions, 1);

    let rows = agent.sessions().expect("the view answers");
    let failed = row(&rows, 0x10);
    assert_eq!(failed.navigation_state, Some(NavigationState::Failed));
    assert_eq!(
        failed.failure_reason,
        Some(FailureReason::UnsupportedScheme)
    );
    assert_eq!(failed.status, None);
    assert_eq!(failed.media_type, None);
    let other = row(&rows, 0x20);
    assert_eq!(other.navigation_state, Some(NavigationState::Started));
    assert_eq!(other.failure_reason, None);

    let NoteNavigationFailedOutcome::Noted {
        session_navigation_changed,
    } = agent
        .note_navigation_failed(NoteNavigationFailed {
            navigation_id: navigation(0x99),
            reason: FailureReason::Unreachable,
        })
        .expect("the obligation is met");
    assert_eq!(session_navigation_changed.sessions, 0);
    assert_eq!(agent.sessions().expect("the view answers"), rows);
}

fn record(id: u32, sequence: i64, command: ActionCommand, url: Option<Url>) -> RecordAction {
    RecordAction {
        session_id: session_id(id),
        sequence,
        command,
        url,
    }
}

fn recorded_id(outcome: RecordActionOutcome) -> ActionId {
    match outcome {
        RecordActionOutcome::Recorded { action_recorded } => action_recorded.action_id,
        refused => panic!("not recorded: {refused:?}"),
    }
}

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

/// Each action gets an identity of its own, and a session's log holds its own actions only, in
/// sequence order whatever order they were recorded in.
#[test]
fn record_action_stores_each_action_and_session_log_reads_a_sessions_own_in_sequence_order() {
    let mut agent = agent_port(with_session(with_session(AgentPorts::new(), 0x10), 0x20));
    let go = recorded_id(
        agent
            .record_action(record(0x10, 2, ActionCommand::Go, Some(address())))
            .expect("the obligation is met"),
    );
    let other = recorded_id(
        agent
            .record_action(record(0x20, 1, ActionCommand::Open, None))
            .expect("the obligation is met"),
    );
    let open = recorded_id(
        agent
            .record_action(record(0x10, 1, ActionCommand::Open, None))
            .expect("the obligation is met"),
    );
    for ActionId(Uuid(text)) in [&go, &other, &open] {
        assert!(is_version_4(text), "not a version 4 UUID: {text}");
    }
    assert_ne!(go, open);
    assert_ne!(go, other);
    assert_ne!(open, other);

    assert_eq!(
        agent.session_log(session_id(0x10)),
        Ok(vec![
            SessionLog {
                action_id: open,
                session_id: session_id(0x10),
                sequence: 1,
                command: ActionCommand::Open,
                url: None,
            },
            SessionLog {
                action_id: go,
                session_id: session_id(0x10),
                sequence: 2,
                command: ActionCommand::Go,
                url: Some(address()),
            },
        ])
    );
    assert_eq!(
        agent.session_log(session_id(0x20)),
        Ok(vec![SessionLog {
            action_id: other,
            session_id: session_id(0x20),
            sequence: 1,
            command: ActionCommand::Open,
            url: None,
        }])
    );
}

#[test]
fn record_action_for_no_session_records_nothing() {
    let mut agent = agent_port(with_session(AgentPorts::new(), 0x10));
    let outcome = agent
        .record_action(record(0x20, 1, ActionCommand::Close, None))
        .expect("the obligation is met");
    assert!(
        matches!(outcome, RecordActionOutcome::NoSuchSession { ref error } if error.session_id == session_id(0x20)),
        "{outcome:?}"
    );
    assert_eq!(agent.session_log(session_id(0x20)), Ok(Vec::new()));
    assert_eq!(agent.session_log(session_id(0x10)), Ok(Vec::new()));
}

/// A source that can never be read.
struct Failing;

impl RandomSource for Failing {
    fn fill(&mut self, _buffer: &mut [u8]) -> io::Result<()> {
        Err(io::Error::other("no random source"))
    }
}

/// An action's identity comes from the ports' random source; one that cannot be read records
/// nothing and is answered as unmet, never with an identity of the ports' own making.
#[test]
fn a_failing_source_answers_the_action_identity_as_unmet_and_records_nothing() {
    let unmet = UnmetObligation {
        capability: "context answer",
        source: "surf.agent.ActionId",
    };
    let mut ports = AgentPorts::with_source(Failing);
    assert_eq!(
        ports.try_generate_surf_agent_action_id(),
        Err(unmet.clone())
    );

    let mut agent = agent_port(with_session(AgentPorts::with_source(Failing), 0x10));
    assert_eq!(
        agent.record_action(record(0x10, 1, ActionCommand::Open, None)),
        Err(unmet)
    );
    assert_eq!(agent.session_log(session_id(0x10)), Ok(Vec::new()));
    assert_eq!(agent.drain_outbox(), Vec::new());
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

/// The `agent` component's ports assign the identity of an action and no other component's.
#[test]
fn agent_ports_answer_the_caller_and_assign_only_an_actions_identity() {
    let caller = Caller::new();
    let mut ports = AgentPorts::with_caller(caller.clone());
    assert_eq!(ports.try_caller_session_id(), Ok(None));
    caller.set(Some(session_id(0x10)));
    assert_eq!(ports.try_caller_session_id(), Ok(Some(session_id(0x10))));
    assert_eq!(ports.caller().get(), Some(session_id(0x10)));

    let ActionId(Uuid(text)) = ports.try_generate_surf_agent_action_id().expect("assigned");
    assert!(is_version_4(&text), "not a version 4 UUID: {text}");
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

/// Only the `agent` component records actions; the other ports types assign no action identity,
/// whatever their random source could give.
#[test]
fn the_other_ports_answer_an_actions_identity_as_unmet() {
    let unmet = Err(UnmetObligation {
        capability: "context answer",
        source: "surf.agent.ActionId",
    });
    assert_eq!(ShellPorts::new().try_generate_surf_agent_action_id(), unmet);
    assert_eq!(
        NavigatorPorts::new().try_generate_surf_agent_action_id(),
        unmet
    );
    assert_eq!(
        DocumentPorts::new().try_generate_surf_agent_action_id(),
        unmet
    );
    assert_eq!(
        RendererPorts::new().try_generate_surf_agent_action_id(),
        unmet
    );
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
