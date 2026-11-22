//! `Browser::held` after one HTML page, and after a form's submission: the body is in five
//! places of the record until the system has delivered them and the browser forgot them.

#![forbid(unsafe_code)]

use surf::{Answer, Browser, Held, RequestStarted, SystemEvent};
use surf_ports::{ComponentPorts, DocumentPorts, document_port};
use surf_types::behaviour::SubmissionStorage;
use surf_types::document::{
    DocumentState, ElementId, SubmissionData, SubmissionId, SubmissionSnapshot, SubmissionState,
    SubmitForm, SubmitFormOutcome,
};
use surf_types::net::{MediaType, Method};
use surf_types::primitives::Uuid;
use surf_types::shell::TabId;
use surf_types::url::Url;

/// The bytes of the one body the transport answers with.
const BODY: usize = 777;

#[test]
fn a_parsed_body_is_held_five_times_until_delivered() {
    let body = vec![b'x'; BODY];
    let answered = body.clone();
    let transport = move |request: &RequestStarted| Answer::Response {
        url: request.url.0.clone(),
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

    // The parser step (`story:html-document-port`) parsed the page, which has no title; the
    // events it adds carry no body.
    let documents = browser.system().document.documents().unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].state, DocumentState::Parsed);
    assert_eq!(documents[0].title.as_deref(), Some(""));

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

/// The fix round of round 1: a form's body is held as a request body is, in each of the five
/// places that carry it, until delivered: `FormSubmitted`, `NavigationStarted` and
/// `RequestStarted` on the log, and the invocations of `Navigate` and `Fetch`.
#[test]
fn a_submitted_form_body_is_held_in_every_place_that_carries_it() {
    let id = |n: u8| Uuid(format!("3f1d5b7e-0000-4000-8000-0000000000{n:02x}"));
    let mut document = DocumentPorts::new();
    SubmissionStorage::put(
        &mut document,
        SubmissionSnapshot {
            state: SubmissionState::Started,
            data: SubmissionData {
                submission_id: SubmissionId(id(1)),
                submitter: ElementId(id(2)),
            },
        },
    );
    let mut ports = ComponentPorts::new();
    ports.document = document_port(document);
    let mut browser = Browser::from_ports(ports, |_: &RequestStarted| Answer::Unanswered);
    let outcome = browser
        .document_mut()
        .submit_form(SubmitForm {
            submission_id: SubmissionId(id(1)),
            tab_id: TabId(id(3)),
            url: Url("http://127.0.0.1/session".to_owned()),
            method: Method::Post,
            body: Some(vec![b'f'; BODY]),
            body_type: Some(MediaType("application/x-www-form-urlencoded".to_owned())),
        })
        .unwrap();
    assert!(
        matches!(outcome, SubmitFormOutcome::Submitted { .. }),
        "{outcome:?}"
    );
    browser.run().unwrap();
    let held = browser.held();
    assert_eq!(held.body_bytes, 5 * BODY, "{held:?}");
    browser.forget_delivered();
    assert_eq!(browser.held(), Held::default());
}
