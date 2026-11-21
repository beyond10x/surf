//! Case 7 of story `headless-cli`, in the library: an unmet obligation reaches the caller and
//! exit status 3. Beside it, the transport step's own properties, through the public API only.

use std::cell::Cell;
use std::rc::Rc;

use surf::{
    Answer, Browser, Error, RequestStarted, SystemEvent, Transport, UnmetObligation, exit_status,
};
use surf_ports::{
    AgentPorts, Caller, ComponentPorts, DocumentPorts, NavigatorPorts, NetPorts, RandomSource,
    RendererPorts, ShellPorts, agent_port, document_port, navigator_port, net_port, renderer_port,
    shell_port,
};
use surf_system::System;
use surf_types::agent::{
    Go, GoOutcome, OpenSession, OpenSessionOutcome, SessionId, SessionState, SessionStateView,
};
use surf_types::navigation::NavigationState;
use surf_types::net::{FailureReason, RequestState};
use surf_types::primitives::Uuid;
use surf_types::shell::{OpenTab, OpenTabOutcome, OpenWindow, OpenWindowOutcome};
use surf_types::url::Url;

/// A random source that can never be read.
struct Failing;

impl RandomSource for Failing {
    fn fill(&mut self, _buffer: &mut [u8]) -> std::io::Result<()> {
        Err(std::io::Error::other("no random source"))
    }
}

/// A transport that counts its calls and gives one fixed answer.
struct Counting {
    calls: Rc<Cell<usize>>,
    answer: Answer,
}

impl Transport for Counting {
    fn fetch(&mut self, request: &RequestStarted) -> Answer {
        self.calls.set(self.calls.get() + 1);
        match self.answer.clone() {
            // A response comes from the address asked for: nothing redirects here.
            Answer::Response {
                status,
                media_type,
                body,
                ..
            } => Answer::Response {
                url: request.url.0.clone(),
                status,
                media_type,
                body,
            },
            other => other,
        }
    }
}

fn counting(answer: Answer) -> (Counting, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    (
        Counting {
            calls: Rc::clone(&calls),
            answer,
        },
        calls,
    )
}

/// A page; [`Counting`] gives it the address it was asked for as its `url`.
fn html() -> Answer {
    Answer::Response {
        url: String::new(),
        status: 200,
        media_type: "text/html".to_owned(),
        body: b"<p>surf</p>".to_vec(),
    }
}

#[test]
fn case_7_failing_random_source_returns_the_unmet_obligation_and_exit_status_3() {
    let (transport, calls) = counting(html());
    let system = System::new(
        agent_port(AgentPorts::new()),
        document_port(DocumentPorts::with_source(Failing)),
        navigator_port(NavigatorPorts::with_source(Failing)),
        net_port(NetPorts::new()),
        renderer_port(RendererPorts::with_source(Failing)),
        shell_port(ShellPorts::with_source(Failing)),
    );
    let mut browser = Browser::new(system, transport);
    let result = browser.load("http://127.0.0.1/");
    assert_eq!(
        result,
        Err(Error::Unmet(UnmetObligation {
            capability: "context answer",
            source: "surf.shell.WindowId",
        }))
    );
    assert_eq!(exit_status(&result), 3);
    assert_eq!(calls.get(), 0);
}

#[test]
fn case_7_unmet_obligation_inside_a_pump_is_returned_and_not_a_failed_navigation() {
    let (transport, calls) = counting(html());
    let system = System::new(
        agent_port(AgentPorts::new()),
        document_port(DocumentPorts::with_source(Failing)),
        navigator_port(NavigatorPorts::new()),
        net_port(NetPorts::new()),
        renderer_port(RendererPorts::new()),
        shell_port(ShellPorts::new()),
    );
    let mut browser = Browser::new(system, transport);
    let result = browser.load("http://127.0.0.1/");
    assert_eq!(
        result,
        Err(Error::Unmet(UnmetObligation {
            capability: "context answer",
            source: "surf.document.DocumentId",
        }))
    );
    assert_eq!(exit_status(&result), 3);
    assert_eq!(calls.get(), 1);
    let navigations = browser
        .system()
        .navigator
        .navigations()
        .expect("navigations");
    assert_eq!(navigations.len(), 1);
    assert_eq!(navigations[0].state, NavigationState::Committed);
}

#[test]
fn each_started_request_is_handed_to_the_transport_once() {
    let (transport, calls) = counting(Answer::Failure(FailureReason::Unreachable));
    let mut browser = Browser::from_ports(surf_ports::ComponentPorts::new(), transport);
    let report = browser.load("http://127.0.0.1/").expect("load");
    assert_eq!(report.navigation_state(), Some(NavigationState::Failed));
    assert_eq!(exit_status(&Ok(report)), 1);
    browser.run().expect("second run");
    browser.run().expect("third run");
    assert_eq!(calls.get(), 1);
    // Six events of the page load, and since story agent-session-spec the two the `agent`
    // component publishes for the navigation's start and end (`note-on-navigation-started`,
    // `note-on-navigation-failed`), each changing no session.
    let published = browser.system().published();
    assert_eq!(published.len(), 8);
    let noted: Vec<_> = published
        .iter()
        .filter_map(|event| match event {
            SystemEvent::SessionNavigationChanged(changed) => {
                Some((changed.state, changed.sessions))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        noted,
        vec![(NavigationState::Started, 0), (NavigationState::Failed, 0)]
    );
}

#[test]
fn an_unanswered_request_stays_pending_and_the_run_ends() {
    let (transport, calls) = counting(Answer::Unanswered);
    let mut browser = Browser::from_ports(surf_ports::ComponentPorts::new(), transport);
    let report = browser.load("http://127.0.0.1/").expect("load");
    assert_eq!(calls.get(), 1);
    assert_eq!(report.navigation_state(), Some(NavigationState::Started));
    assert_eq!(report.requests.len(), 1);
    assert_eq!(report.requests[0].state, RequestState::Pending);
    assert_eq!(exit_status(&Ok(report)), 1);
    browser.run().expect("second run");
    assert_eq!(calls.get(), 1);
}

#[test]
fn a_closure_is_a_transport() {
    let mut browser = Browser::from_ports(
        surf_ports::ComponentPorts::new(),
        |request: &RequestStarted| Answer::Response {
            url: request.url.0.clone(),
            status: 404,
            media_type: "text/plain".to_owned(),
            body: Vec::new(),
        },
    );
    let report = browser.load("http://127.0.0.1/").expect("load");
    assert_eq!(report.navigation_state(), Some(NavigationState::Committed));
    assert_eq!(exit_status(&Ok(report)), 0);
}

/// Story agent-session-spec: an agent's session, driven through the `agent` port the browser
/// hands out, reads back the navigation `Go` started and the state the transport ended it in.
#[test]
fn an_agent_session_goes_through_the_browser_and_reads_its_navigation_back() {
    let caller = Caller::new();
    let mut browser = Browser::from_ports(
        ComponentPorts::with_caller(caller.clone()),
        |_: &RequestStarted| Answer::Failure(FailureReason::Unreachable),
    );
    let OpenWindowOutcome::Opened { window_opened } = browser
        .shell_mut()
        .open_window(OpenWindow {})
        .expect("window");
    let window_id = window_opened.window_id;
    let OpenTabOutcome::Opened { tab_opened } = browser
        .shell_mut()
        .open_tab(OpenTab {
            window_id: window_id.clone(),
        })
        .expect("tab")
    else {
        panic!("the tab was refused");
    };
    let session_id = SessionId(Uuid("5a7c1e00-0000-4000-8000-000000000001".to_owned()));
    let opened = browser
        .agent_mut()
        .open_session(OpenSession {
            session_id: session_id.clone(),
            window_id,
            tab_id: tab_opened.tab_id.clone(),
        })
        .expect("session");
    assert!(
        matches!(opened, OpenSessionOutcome::Opened { .. }),
        "{opened:?}"
    );
    browser.run().expect("run");

    caller.set(Some(session_id.clone()));
    let went = browser
        .agent_mut()
        .go(Go {
            session_id: session_id.clone(),
            url: Url("http://127.0.0.1/".to_owned()),
        })
        .expect("go");
    assert!(matches!(went, GoOutcome::Requested { .. }), "{went:?}");
    browser.run().expect("run");

    let rows = browser
        .system()
        .agent
        .session_state_view(session_id.clone())
        .expect("view");
    assert_eq!(
        rows,
        vec![SessionStateView {
            session_id,
            state: SessionState::Open,
            url: Some(Url("http://127.0.0.1/".to_owned())),
            navigation_state: Some(NavigationState::Failed),
            status: None,
            media_type: None,
            failure_reason: Some(FailureReason::Unreachable),
        }]
    );
    let navigations = browser.system().navigator.navigations().expect("view");
    assert_eq!(navigations.len(), 1);
    assert_eq!(navigations[0].tab_id, tab_opened.tab_id);
    assert_eq!(navigations[0].state, NavigationState::Failed);
}

#[test]
fn an_address_with_control_characters_stays_on_one_line() {
    let mut browser =
        Browser::from_ports(surf_ports::ComponentPorts::new(), |_: &RequestStarted| {
            Answer::Failure(FailureReason::Unreachable)
        });
    let report = browser.load("http://a b/\n\\x").expect("load");
    let lines = report.lines();
    assert_eq!(lines.len(), 3);
    for line in &lines {
        assert!(line.ends_with(" http://a b/\\x0a\\x5cx"), "{line:?}");
    }
}
