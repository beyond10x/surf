//! `Browser::forget_delivered` and `Browser::held`: what a long-lived browser drops of its
//! record once the system delivered it, and that every view answers as before.

#![forbid(unsafe_code)]

use std::cell::Cell;
use std::rc::Rc;

use surf::{Answer, Browser, Held, MemoryBrowser, Transport};
use surf_ports::{Caller, ComponentPorts};
use surf_types::agent::{
    ActionCommand, Go, GoOutcome, OpenSession, OpenSessionOutcome, RecordAction,
    RecordActionOutcome, SessionId,
};
use surf_types::navigation::NavigationState;
use surf_types::primitives::Uuid;
use surf_types::shell::{OpenTab, OpenTabOutcome, OpenWindow, OpenWindowOutcome};
use surf_types::url::Url;

/// The bytes of every body the transport answers with.
const BODY: usize = 1000;

/// A transport that answers every address with status 200, `text/html` and a body of `BODY`
/// bytes, and counts the addresses it was asked for.
fn pages() -> (impl Transport, Rc<Cell<usize>>) {
    let asked = Rc::new(Cell::new(0));
    let counter = Rc::clone(&asked);
    let transport = move |_: &str| {
        counter.set(counter.get() + 1);
        Answer::Response {
            status: 200,
            media_type: "text/html".to_owned(),
            body: vec![b'x'; BODY],
        }
    };
    (transport, asked)
}

/// A session opened over its own window and tab, its `open` recorded.
fn opened<T: Transport>(transport: T) -> (MemoryBrowser<T>, Caller, SessionId) {
    let caller = Caller::new();
    let mut browser = Browser::from_ports(ComponentPorts::with_caller(caller.clone()), transport);
    let OpenWindowOutcome::Opened { window_opened } =
        browser.shell_mut().open_window(OpenWindow {}).unwrap();
    browser.run().unwrap();
    let OpenTabOutcome::Opened { tab_opened } = browser
        .shell_mut()
        .open_tab(OpenTab {
            window_id: window_opened.window_id.clone(),
        })
        .unwrap()
    else {
        panic!("the tab was refused");
    };
    browser.run().unwrap();
    let session_id = SessionId(Uuid("5a7c1e00-0000-4000-8000-000000000002".to_owned()));
    let outcome = browser
        .agent_mut()
        .open_session(OpenSession {
            session_id: session_id.clone(),
            window_id: window_opened.window_id,
            tab_id: tab_opened.tab_id,
        })
        .unwrap();
    assert!(matches!(outcome, OpenSessionOutcome::Opened { .. }));
    record(&mut browser, &session_id, 1, ActionCommand::Open, None);
    browser.run().unwrap();
    caller.set(Some(session_id.clone()));
    (browser, caller, session_id)
}

fn record<T: Transport>(
    browser: &mut MemoryBrowser<T>,
    session_id: &SessionId,
    sequence: i64,
    command: ActionCommand,
    url: Option<&str>,
) {
    let outcome = browser
        .agent_mut()
        .record_action(RecordAction {
            session_id: session_id.clone(),
            sequence,
            command,
            url: url.map(|url| Url(url.to_owned())),
        })
        .unwrap();
    assert!(matches!(outcome, RecordActionOutcome::Recorded { .. }));
}

fn go<T: Transport>(
    browser: &mut MemoryBrowser<T>,
    session_id: &SessionId,
    sequence: i64,
    url: &str,
) {
    let outcome = browser
        .agent_mut()
        .go(Go {
            session_id: session_id.clone(),
            url: Url(url.to_owned()),
        })
        .unwrap();
    assert!(
        matches!(outcome, GoOutcome::Requested { .. }),
        "{outcome:?}"
    );
    record(browser, session_id, sequence, ActionCommand::Go, Some(url));
    browser.run().unwrap();
}

/// Every view the answers and the conformance target read, as `Debug` text.
fn views<T: Transport>(browser: &MemoryBrowser<T>, session_id: &SessionId) -> String {
    let system = browser.system();
    format!(
        "{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}",
        system.agent.session_state_view(session_id.clone()).unwrap(),
        system.agent.session_log(session_id.clone()).unwrap(),
        system.agent.sessions().unwrap(),
        system.navigator.navigations().unwrap(),
        system.document.documents().unwrap(),
        system.net.requests().unwrap(),
    )
}

#[test]
fn forget_delivered_drops_the_bodies_and_keeps_every_view() {
    let (transport, asked) = pages();
    let (mut browser, _caller, session_id) = opened(transport);
    go(&mut browser, &session_id, 2, "http://127.0.0.1/a");

    // One committed HTML page holds its body four times: in the events `ResponseReceived` and
    // `NavigationCommitted`, and in the invocations of `CommitNavigation` and `ParseDocument`.
    let held = browser.held();
    assert_eq!(held.body_bytes, 4 * 1000, "{held:?}");
    assert!(held.events > 0 && held.invocations > 0, "{held:?}");
    let before = views(&browser, &session_id);
    assert!(before.contains("Committed"), "{before}");

    browser.forget_delivered();
    assert_eq!(
        browser.held(),
        Held {
            events: 0,
            invocations: 0,
            body_bytes: 0
        }
    );
    assert_eq!(
        views(&browser, &session_id),
        before,
        "the views after forgetting"
    );

    // The next command runs as before: its request is handed to the transport once, and only
    // its own body is held until the next forget.
    go(&mut browser, &session_id, 3, "http://127.0.0.1/b");
    assert_eq!(
        asked.get(),
        2,
        "each request is handed to the transport once"
    );
    let row = browser
        .system()
        .agent
        .session_state_view(session_id.clone())
        .unwrap()
        .remove(0);
    assert_eq!(row.url, Some(Url("http://127.0.0.1/b".to_owned())));
    assert_eq!(row.navigation_state, Some(NavigationState::Committed));
    assert_eq!(browser.system().navigator.navigations().unwrap().len(), 2);
    assert_eq!(browser.system().document.documents().unwrap().len(), 2);
    assert_eq!(
        browser
            .system()
            .agent
            .session_log(session_id)
            .unwrap()
            .len(),
        3
    );
    assert_eq!(browser.held().body_bytes, 4 * 1000);
}

#[test]
fn forget_delivered_keeps_what_is_not_delivered() {
    let (transport, asked) = pages();
    let (mut browser, _caller, session_id) = opened(transport);
    browser.forget_delivered();
    // A command issued and not run yet: its event is in the port's outbox, not on the log.
    let outcome = browser
        .agent_mut()
        .go(Go {
            session_id: session_id.clone(),
            url: Url("http://127.0.0.1/c".to_owned()),
        })
        .unwrap();
    assert!(matches!(outcome, GoOutcome::Requested { .. }));
    browser.forget_delivered();
    browser.run().unwrap();
    assert_eq!(
        asked.get(),
        1,
        "the request after forgetting is still handed over"
    );
    let row = browser
        .system()
        .agent
        .session_state_view(session_id)
        .unwrap()
        .remove(0);
    assert_eq!(row.navigation_state, Some(NavigationState::Committed));
}
