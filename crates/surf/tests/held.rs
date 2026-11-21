//! `Browser::held` after one HTML page: the body is in five places of the record until the system
//! has delivered them and the browser forgot them.

#![forbid(unsafe_code)]

use surf::{Answer, Browser, Held, SystemEvent};
use surf_ports::ComponentPorts;
use surf_types::document::DocumentState;

/// The bytes of the one body the transport answers with.
const BODY: usize = 777;

#[test]
fn a_parsed_body_is_held_five_times_until_delivered() {
    let body = vec![b'x'; BODY];
    let answered = body.clone();
    let transport = move |_: &str| Answer::Response {
        status: 200,
        media_type: "text/html".to_owned(),
        body: answered.clone(),
    };
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport);
    browser.load("http://127.0.0.1/page").unwrap();

    // `ResponseReceived`, `NavigationCommitted` and `ParsingStarted` on the log, and the
    // invocations of `CommitNavigation` and `ParseDocument`.
    let held = browser.held();
    assert_eq!(held.body_bytes, 5 * BODY, "{held:?}");
    let parsing_started: Vec<_> = browser
        .system()
        .published()
        .iter()
        .filter_map(|event| match event {
            SystemEvent::ParsingStarted(started) => Some(started.body.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(parsing_started, vec![body]);

    // No parser step runs yet (`story:html-document-port`): the document stays `Parsing`.
    let documents = browser.system().document.documents().unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].state, DocumentState::Parsing);
    assert_eq!(documents[0].title, None);

    browser.forget_delivered();
    assert_eq!(
        browser.held(),
        Held {
            events: 0,
            invocations: 0,
            body_bytes: 0
        }
    );
    assert_eq!(browser.system().document.documents().unwrap(), documents);
}
