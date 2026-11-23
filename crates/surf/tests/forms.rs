//! Cases 5 to 20 of `story:agent-form-port`: the rows the parser step writes for forms, and the
//! submission step of `Browser`, which answers each started submission with `SubmitForm` or
//! `AbandonSubmission`. Each case loads one page from a transport in memory, fills the controls
//! through the `document` port (case 17 through the `agent` port), starts the submission and
//! reads how it ended and what the transport was asked for.
//!
//! The rules the expected values rest on, HTML Standard sections named in each case: 4.10.21.3
//! "Form submission algorithm" and 4.10.21.4 "Constructing the entry list"; the bytes are those
//! of `crates/url` `form_urlencoded::serialize`.

use std::cell::RefCell;
use std::rc::Rc;

use surf::{Answer, Browser, MemoryBrowser, RequestStarted, SystemEvent, Transport};
use surf_ports::{Caller, ComponentPorts};
use surf_types::agent::{
    ActionCommand, Go, GoOutcome, OpenSession, OpenSessionOutcome, Press, PressOutcome,
    RecordAction, RecordActionOutcome, SessionId,
};
use surf_types::document::{
    Check, CheckOutcome, DocumentId, Elements, SelectOption, SelectOptionOutcome, SetValue,
    SetValueOutcome, StartSubmission, StartSubmissionOutcome, SubmissionEndReason, SubmissionId,
};
use surf_types::net::Method;
use surf_types::primitives::Uuid;
use surf_types::shell::{OpenTab, OpenTabOutcome, OpenWindow, OpenWindowOutcome};
use surf_types::url::Url;

/// The address every page of these cases is loaded from.
const ADDRESS: &str = "http://127.0.0.1/shop/form.html";

/// What a submission's address answers.
const DONE: &str = "<!doctype html><title>Done</title><p>Done.";

type Requests = Rc<RefCell<Vec<RequestStarted>>>;

/// A transport in memory: `page` at [`ADDRESS`], [`DONE`] at every other address; it keeps
/// every request it is handed.
fn site(page: &'static str) -> (impl Transport, Requests) {
    let requests: Requests = Rc::default();
    let kept = Rc::clone(&requests);
    let transport = move |request: &RequestStarted| {
        kept.borrow_mut().push(request.clone());
        let body = if request.url.0 == ADDRESS { page } else { DONE };
        Answer::Response {
            url: request.url.0.clone(),
            method: request.method,
            status: 200,
            media_type: "text/html".to_owned(),
            body: body.as_bytes().to_vec(),
        }
    };
    (transport, requests)
}

/// How a submission ended, read from the log.
#[derive(Debug, PartialEq, Eq)]
enum Ended {
    Submitted {
        url: String,
        method: Method,
        body: Option<String>,
        body_type: Option<String>,
    },
    Abandoned(SubmissionEndReason),
    /// Neither `FormSubmitted` nor `SubmissionAbandoned` names it.
    Open,
}

fn post(url: &str, body: &str) -> Ended {
    Ended::Submitted {
        url: url.to_owned(),
        method: Method::Post,
        body: Some(body.to_owned()),
        body_type: Some("application/x-www-form-urlencoded".to_owned()),
    }
}

fn get(url: &str) -> Ended {
    Ended::Submitted {
        url: url.to_owned(),
        method: Method::Get,
        body: None,
        body_type: None,
    }
}

/// How the submission `id` ended, from the events on `browser`'s log.
fn ended<T>(browser: &MemoryBrowser<T>, id: &SubmissionId) -> Ended {
    for event in browser.system().published() {
        match event {
            SystemEvent::FormSubmitted(submitted) if &submitted.submission_id == id => {
                return Ended::Submitted {
                    url: submitted.url.0.clone(),
                    method: submitted.method,
                    body: submitted
                        .body
                        .as_ref()
                        .map(|body| String::from_utf8(body.clone()).unwrap()),
                    body_type: submitted.body_type.as_ref().map(|t| t.0.clone()),
                };
            }
            SystemEvent::SubmissionAbandoned(abandoned) if &abandoned.submission_id == id => {
                return Ended::Abandoned(abandoned.reason);
            }
            _ => {}
        }
    }
    Ended::Open
}

/// The element of `document` whose label is `label`.
fn element<T>(browser: &MemoryBrowser<T>, document: &DocumentId, label: &str) -> Elements {
    let rows = browser
        .system()
        .document
        .elements(document.clone())
        .unwrap();
    rows.iter()
        .find(|row| row.label == label)
        .cloned()
        .unwrap_or_else(|| panic!("no element labelled {label:?}: {rows:?}"))
}

/// A page loaded at [`ADDRESS`] into a browser of its own.
struct Loaded<T> {
    browser: MemoryBrowser<T>,
    document: DocumentId,
    requests: Requests,
}

fn load(page: &'static str) -> Loaded<impl Transport> {
    let (transport, requests) = site(page);
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport);
    let report = browser.load(ADDRESS).unwrap();
    let [document] = report.documents.as_slice() else {
        panic!("{:?}", report.documents);
    };
    let document = document.document_id.clone();
    requests.borrow_mut().clear();
    Loaded {
        browser,
        document,
        requests,
    }
}

impl<T: Transport> Loaded<T> {
    fn element(&self, label: &str) -> Elements {
        element(&self.browser, &self.document, label)
    }

    fn fill(&mut self, label: &str, value: &str) {
        let element_id = self.element(label).element_id;
        let outcome = self
            .browser
            .document_mut()
            .set_value(SetValue {
                element_id,
                value: value.to_owned(),
            })
            .unwrap();
        assert!(
            matches!(outcome, SetValueOutcome::Set { .. }),
            "{outcome:?}"
        );
        self.browser.run().unwrap();
    }

    fn check(&mut self, label: &str) {
        let element_id = self.element(label).element_id;
        let outcome = self
            .browser
            .document_mut()
            .check(Check { element_id })
            .unwrap();
        assert!(
            matches!(outcome, CheckOutcome::Checked { .. }),
            "{outcome:?}"
        );
        self.browser.run().unwrap();
    }

    /// Selects the option labelled `option` of the select labelled `select`.
    fn pick(&mut self, select: &str, option: &str) {
        let select_id = self.element(select).element_id;
        let options = self.browser.system().document.options().unwrap();
        let option_id = options
            .iter()
            .find(|row| row.select_id == select_id && row.label == option)
            .unwrap_or_else(|| panic!("no option {option:?} of {select:?}: {options:?}"))
            .option_id
            .clone();
        let outcome = self
            .browser
            .document_mut()
            .select_option(SelectOption { option_id })
            .unwrap();
        assert!(
            matches!(outcome, SelectOptionOutcome::Selected { .. }),
            "{outcome:?}"
        );
        self.browser.run().unwrap();
    }

    /// Starts a submission with the button labelled `label` as its submitter, runs, and reads
    /// how it ended. A submission that navigated asked the transport for its request.
    fn press(&mut self, label: &str) -> Ended {
        let element_id = self.element(label).element_id;
        let outcome = self
            .browser
            .document_mut()
            .start_submission(StartSubmission { element_id })
            .unwrap();
        let StartSubmissionOutcome::Started { submission_started } = outcome else {
            panic!("{outcome:?}");
        };
        self.browser.run().unwrap();
        let end = ended(&self.browser, &submission_started.submission_id);
        if let Ended::Submitted {
            url,
            method,
            body,
            body_type,
        } = &end
        {
            let requests = self.requests.borrow();
            let [request] = requests.as_slice() else {
                panic!("one request: {requests:?}");
            };
            assert_eq!(&request.url.0, url);
            assert_eq!(&request.method, method);
            assert_eq!(request.body.as_deref(), body.as_ref().map(String::as_bytes));
            assert_eq!(
                request.body_type.as_ref().map(|t| t.0.as_str()),
                body_type.as_deref()
            );
        } else {
            assert!(self.requests.borrow().is_empty(), "nothing navigates");
        }
        end
    }
}

/// The form of cases 7 and 9: a text field `user` = `ada` and a checked checkbox `remember`,
/// both by their attributes.
const LOGIN: &str = "<!doctype html><form method=post action=/session>\
    <label>User <input name=user value=ada></label>\
    <label>Remember <input type=checkbox name=remember checked></label>\
    <button name=go value=1>Go</button><button name=other value=2>Other</button></form>";

/// 4.10.21.4 "Constructing the entry list": a checkbox or radio button whose checkedness is
/// false is skipped; a checked one without a `value` attribute gives the value "on"; a select
/// gives each selected option that is not disabled. The values are the ones the controls hold
/// after `SetValue`, `Check` and `SelectOption`.
#[test]
fn case_05_the_entry_list_of_a_filled_form() {
    let mut page = load(
        "<!doctype html><form method=post action=/save>\
        <label>User <input name=user></label>\
        <label>Password <input type=password name=pw></label>\
        <label>Remember <input type=checkbox name=remember></label>\
        <label>News <input type=checkbox name=news value=yes></label>\
        <label>Small <input type=radio name=size value=s></label>\
        <label>Medium <input type=radio name=size value=m checked></label>\
        <label>Colour <select name=color><option>blue<option>red</select></label>\
        <button>Save</button></form>",
    );
    page.fill("User", "ada");
    page.fill("Password", "s3cret");
    page.check("Remember");
    page.pick("Colour", "red");
    assert_eq!(
        page.press("Save"),
        post(
            "http://127.0.0.1/save",
            "user=ada&pw=s3cret&remember=on&size=m&color=red"
        )
    );
}

/// 4.10.21.4 "Constructing the entry list": a disabled field is skipped, whether by its own
/// `disabled` or by a disabled `fieldset` around it.
#[test]
fn case_06_a_disabled_control_is_left_out() {
    let mut page = load(
        "<!doctype html><form method=post action=/save>\
        <input name=a value=1 disabled>\
        <fieldset disabled><input name=b value=2></fieldset>\
        <input name=c value=3><button>Save</button></form>",
    );
    assert_eq!(page.press("Save"), post("http://127.0.0.1/save", "c=3"));
}

/// 4.10.21.4 "Constructing the entry list": a button that is not the submitter is skipped; the
/// submitter gives its name and value.
#[test]
fn case_07_only_the_submitters_button_is_an_entry() {
    let mut page = load(LOGIN);
    assert_eq!(
        page.press("Go"),
        post("http://127.0.0.1/session", "user=ada&remember=on&go=1")
    );
}

/// 4.10.21.3 "Form submission algorithm", "mutate action URL": a `GET` sets the query of the
/// action URL to the serialized entry list, so the action's own query is replaced.
#[test]
fn case_08_get_replaces_the_query_of_the_action() {
    let mut page = load(
        "<!doctype html><form action=\"results?x=1\">\
        <label>Search <input name=q></label><button>Find</button></form>",
    );
    page.fill("Search", "lighthouse");
    assert_eq!(
        page.press("Find"),
        get("http://127.0.0.1/shop/results?q=lighthouse")
    );
}

/// 4.10.21.3 "Form submission algorithm", "submit as entity body": a `POST` over `http` sends
/// the entry list as its body, `application/x-www-form-urlencoded`.
#[test]
fn case_09_post_sends_the_body() {
    let mut page = load(
        "<!doctype html><form method=post action=/session>\
        <label>User <input name=user value=ada></label>\
        <label>Remember <input type=checkbox name=remember checked></label>\
        <button>Log in</button></form>",
    );
    assert_eq!(
        page.press("Log in"),
        post("http://127.0.0.1/session", "user=ada&remember=on")
    );
}

/// 4.10.21.3 "Form submission algorithm": an empty action, or none, is the form document's URL.
#[test]
fn case_10_an_empty_action_is_the_document_address() {
    let forms = "<!doctype html><form method=post action=\"\"><input name=a value=1>\
        <button>Empty</button></form>\
        <form method=post><input name=b value=2><button>Absent</button></form>";
    assert_eq!(load(forms).press("Empty"), post(ADDRESS, "a=1"));
    assert_eq!(load(forms).press("Absent"), post(ADDRESS, "b=2"));
}

/// 4.10.21.3 "Form submission algorithm": the action is parsed relative to the document, whose
/// base URL is its first `base` element with `href`.
#[test]
fn case_11_a_relative_action_under_a_base() {
    let mut page = load(
        "<!doctype html><base href=\"/app/\"><form method=post action=go>\
        <input name=a value=1><button>Send</button></form>",
    );
    assert_eq!(page.press("Send"), post("http://127.0.0.1/app/go", "a=1"));
}

/// 4.10.21.3 "Form submission algorithm": a `POST` over `http` is "submit as entity body",
/// which reads the enctype; surf encodes only `application/x-www-form-urlencoded`. A `GET` is
/// "mutate action URL", which reads no enctype.
#[test]
fn case_12_multipart_form_data_is_not_encoded() {
    let mut page = load(
        "<!doctype html><form method=post enctype=multipart/form-data action=/up>\
        <input name=a value=1><button>Multipart</button></form>\
        <form method=post enctype=text/plain action=/up><button>Plain</button></form>\
        <form enctype=multipart/form-data action=/find><input name=q value=x>\
        <button>Get</button></form>",
    );
    assert_eq!(
        page.press("Multipart"),
        Ended::Abandoned(SubmissionEndReason::UnsupportedEnctype)
    );
    assert_eq!(
        page.press("Plain"),
        Ended::Abandoned(SubmissionEndReason::UnsupportedEnctype)
    );
    assert_eq!(page.press("Get"), get("http://127.0.0.1/find?q=x"));
}

#[test]
fn case_13_a_javascript_action_is_not_navigated_to() {
    let mut page = load(
        "<!doctype html><form action=\"javascript:x\"><input name=a value=1>\
        <button>Run</button></form>",
    );
    assert_eq!(
        page.press("Run"),
        Ended::Abandoned(SubmissionEndReason::UnsupportedScheme)
    );
}

#[test]
fn case_14_a_file_action_is_not_navigated_to() {
    let mut page = load(
        "<!doctype html><form method=post action=\"file:///x\"><input name=a value=1>\
        <button>Open</button></form>",
    );
    assert_eq!(
        page.press("Open"),
        Ended::Abandoned(SubmissionEndReason::UnsupportedScheme)
    );
}

/// URL Standard, the `application/x-www-form-urlencoded` serializer (`crates/url`): `&` and `=`
/// in a name are percent-encoded, a space is `+`, and `é` its two UTF-8 bytes.
#[test]
fn case_15_names_and_values_are_percent_encoded() {
    let mut page = load(
        "<!doctype html><meta charset=utf-8><form method=post action=/save>\
        <label>Odd <input name=\"n&amp;a=me\"></label><button>Save</button></form>",
    );
    page.fill("Odd", "a b é");
    assert_eq!(
        page.press("Save"),
        post("http://127.0.0.1/save", "n%26a%3Dme=a+b+%C3%A9")
    );
}

/// 4.10.6 the `button` element, its activation behaviour: a button with no form owner does
/// nothing.
#[test]
fn case_16_a_submit_button_with_no_form_owner() {
    let mut page = load(
        "<!doctype html><form action=/a><input name=a value=1></form>\
        <button>Alone</button>",
    );
    assert_eq!(
        page.press("Alone"),
        Ended::Abandoned(SubmissionEndReason::NoFormOwner)
    );
}

/// A session opened over its own window and tab, its `open` recorded, then `go` to
/// [`ADDRESS`]: the pattern of `opened` in `forget_delivered.rs`.
fn session(page: &'static str) -> (MemoryBrowser<impl Transport>, SessionId, Requests) {
    let (transport, requests) = site(page);
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
    let session_id = SessionId(Uuid("5a7c1e00-0000-4000-8000-000000000017".to_owned()));
    let outcome = browser
        .agent_mut()
        .open_session(OpenSession {
            session_id: session_id.clone(),
            window_id: window_opened.window_id,
            tab_id: tab_opened.tab_id,
        })
        .unwrap();
    assert!(matches!(outcome, OpenSessionOutcome::Opened { .. }));
    for (sequence, command, url) in [
        (1, ActionCommand::Open, None),
        (2, ActionCommand::Go, Some(ADDRESS)),
    ] {
        let outcome = browser
            .agent_mut()
            .record_action(RecordAction {
                session_id: session_id.clone(),
                sequence,
                command,
                url: url.map(|url| Url(url.to_owned())),
                handle: None,
                label: None,
            })
            .unwrap();
        assert!(matches!(outcome, RecordActionOutcome::Recorded { .. }));
    }
    browser.run().unwrap();
    caller.set(Some(session_id.clone()));
    let outcome = browser
        .agent_mut()
        .go(Go {
            session_id: session_id.clone(),
            url: Url(ADDRESS.to_owned()),
        })
        .unwrap();
    assert!(
        matches!(outcome, GoOutcome::Requested { .. }),
        "{outcome:?}"
    );
    browser.run().unwrap();
    requests.borrow_mut().clear();
    (browser, session_id, requests)
}

/// 4.10.6 the `button` element, its activation behaviour: a Reset Button resets the form and a
/// Button does nothing; neither submits. Pressed through the agent's `Press`, which the
/// binding `submit-on-press-requested` turns into `StartSubmission`.
#[test]
fn case_17_a_reset_and_a_plain_button_pressed_through_press() {
    let (mut browser, session_id, requests) = session(
        "<!doctype html><form method=post action=/save><input name=a value=1>\
        <button type=reset>Clear</button><input type=button value=Help></form>",
    );
    let documents = browser.system().document.documents().unwrap();
    let [document] = documents.as_slice() else {
        panic!("{documents:?}");
    };
    let document = document.document_id.clone();
    for (label, handle) in [("Clear", "b1"), ("Help", "b2")] {
        let element_id = element(&browser, &document, label).element_id;
        let outcome = browser
            .agent_mut()
            .press(Press {
                session_id: session_id.clone(),
                element_id: element_id.clone(),
                handle: handle.to_owned(),
            })
            .unwrap();
        assert!(
            matches!(outcome, PressOutcome::Requested { .. }),
            "{outcome:?}"
        );
        browser.run().unwrap();
        let submission = browser
            .system()
            .document
            .submissions()
            .unwrap()
            .into_iter()
            .find(|row| row.submitter == element_id)
            .unwrap_or_else(|| panic!("no submission of {label}"));
        assert_eq!(
            ended(&browser, &submission.submission_id),
            Ended::Abandoned(SubmissionEndReason::NotSubmitButton),
            "{label}"
        );
    }
    assert!(requests.borrow().is_empty(), "nothing navigates");
}

/// 4.10.21.3 "Form submission algorithm": unless the form has `novalidate`, a form whose
/// constraints are not met is not submitted. Built: a `required` control with an empty value
/// ("suffering from being missing").
#[test]
fn case_18_a_required_field_left_empty() {
    let forms = "<!doctype html><form method=post action=/save>\
        <label>Name <input name=name required></label><button>Save</button></form>\
        <form method=post action=/save novalidate>\
        <label>Other <input name=other required></label><button>Anyway</button></form>";
    let mut page = load(forms);
    assert_eq!(
        page.press("Save"),
        Ended::Abandoned(SubmissionEndReason::InvalidControls)
    );
    page.fill("Name", "Ada");
    assert_eq!(
        page.press("Save"),
        post("http://127.0.0.1/save", "name=Ada")
    );
    assert_eq!(
        load(forms).press("Anyway"),
        post("http://127.0.0.1/save", "other=")
    );
}

/// 4.10.21.3 "Form submission algorithm": an action that does not parse ends the submission.
/// `http://[x`: the host state of `crates/url` reaches the end with the buffer `[x`, and the
/// host parser finds no closing `]` (`Ipv6Unclosed`).
#[test]
fn case_19_an_action_that_does_not_parse() {
    let mut page = load(
        "<!doctype html><form method=post action=\"http://[x\"><input name=a value=1>\
        <button>Send</button></form>",
    );
    assert_eq!(
        page.press("Send"),
        Ended::Abandoned(SubmissionEndReason::InvalidAction)
    );
}

/// 4.10.21.3 "Form submission algorithm": the method `dialog` closes a dialog and navigates
/// nowhere.
#[test]
fn case_20_the_dialog_method() {
    let mut page = load(
        "<!doctype html><form method=dialog><input name=a value=1>\
        <button>Close</button></form>",
    );
    assert_eq!(
        page.press("Close"),
        Ended::Abandoned(SubmissionEndReason::DialogMethod)
    );
}

/// 4.10.21.4 "Constructing the entry list": an `input` in the Hidden state has no row, and is
/// an entry with its `value`; one named `_charset_` gives the encoding's name.
#[test]
fn a_hidden_input_is_an_entry() {
    let mut page = load(
        "<!doctype html><form method=post action=/save>\
        <input type=hidden name=token value=t0k><input type=hidden name=_charset_>\
        <input name=a value=1><button>Save</button></form>",
    );
    assert_eq!(
        page.press("Save"),
        post("http://127.0.0.1/save", "token=t0k&_charset_=UTF-8&a=1")
    );
}

/// The parser step writes the rows of the forms, the `name` and form owner of each control,
/// and the options of each select.
#[test]
fn the_parse_step_writes_the_form_rows() {
    let page = load(
        "<!doctype html><form action=/a method=post enctype=text/plain>\
        <label>One <input name=one></label></form>\
        <label>Out <input name=out form=second></label>\
        <form id=second><label>Pick <select name=pick><option value=1>A\
        <option selected disabled>B</select></label></form>\
        <a href=/x>Link</a>",
    );
    let system = page.browser.system();
    let mut forms = system.document.forms().unwrap();
    forms.sort_by_key(|row| row.position);
    let written: Vec<String> = forms
        .iter()
        .map(|row| {
            format!(
                "{} {:?} {:?} {:?}",
                row.position, row.action, row.method, row.enctype
            )
        })
        .collect();
    assert_eq!(
        written,
        [
            "1 Some(\"/a\") Some(\"post\") Some(\"text/plain\")",
            "2 None None None"
        ]
    );
    assert!(forms.iter().all(|row| row.document_id == page.document));
    let owner = |label: &str| {
        let row = page.element(label);
        let at = forms
            .iter()
            .find(|form| Some(&form.form_id) == row.form_id.as_ref())
            .map(|form| form.position);
        (row.name, at)
    };
    assert_eq!(owner("One"), (Some("one".to_owned()), Some(1)));
    assert_eq!(owner("Out"), (Some("out".to_owned()), Some(2)));
    assert_eq!(owner("Pick"), (Some("pick".to_owned()), Some(2)));
    assert_eq!(owner("Link"), (None, None));
    let select_id = page.element("Pick").element_id;
    let mut options: Vec<_> = system
        .document
        .options()
        .unwrap()
        .into_iter()
        .filter(|row| row.select_id == select_id)
        .collect();
    options.sort_by_key(|row| row.position);
    let written: Vec<String> = options
        .iter()
        .map(|row| {
            format!(
                "{} {} {} selected={} disabled={} hidden={}",
                row.position, row.value, row.label, row.selected, row.disabled, row.hidden
            )
        })
        .collect();
    assert_eq!(
        written,
        [
            "1 1 A selected=false disabled=false hidden=false",
            "2 B B selected=true disabled=true hidden=false"
        ]
    );
}

/// Case 21 of `story:agent-fill-pick`: of two radio buttons of one group that carry `checked`,
/// the parser step leaves the last checked (HTML Standard 4.10.5.1.18, the radio button group:
/// setting a radio button's checkedness to true unchecks the others of its group, and the parser
/// inserts them in tree order), so a submission gives one entry of the group.
#[test]
fn fill_pick_case_21_radio_group_two_checked_submits_the_last() {
    let mut page = load(
        "<!doctype html><form action=\"/save\" method=\"post\">\
        <label><input type=radio name=size value=s checked> Small</label>\
        <label><input type=radio name=size value=m checked> Medium</label>\
        <button>Save</button></form>",
    );
    assert!(!page.element("Small").checked);
    assert!(page.element("Medium").checked);
    assert_eq!(page.press("Save"), post("http://127.0.0.1/save", "size=m"));
}

/// The parser step writes `readonly` where it applies (`story:agent-fill-pick`, case 19), and
/// `SetValue` refuses such a control.
#[test]
fn the_parse_step_writes_readonly_and_set_value_refuses_it() {
    let mut page = load(
        "<!doctype html><form><label>Code <input name=code value=X1 readonly></label>\
        <label>Box <input type=checkbox name=box readonly></label>\
        <label>Note <textarea name=note readonly>n</textarea></label></form>",
    );
    assert!(page.element("Code").readonly);
    assert!(
        !page.element("Box").readonly,
        "readonly does not apply to a checkbox"
    );
    assert!(page.element("Note").readonly);
    let element_id = page.element("Code").element_id;
    let outcome = page
        .browser
        .document_mut()
        .set_value(SetValue {
            element_id,
            value: "X2".to_owned(),
        })
        .unwrap();
    assert!(
        matches!(outcome, SetValueOutcome::Readonly { .. }),
        "{outcome:?}"
    );
    assert_eq!(page.element("Code").value.as_deref(), Some("X1"));
}

/// The adversary's pass on this story, decision 1: a `GET` address of exactly
/// `surf_fetch::MAX_ADDRESS_BYTES` bytes is navigated to, one byte more ends `AddressTooLong`.
#[test]
fn a_get_address_at_the_bound_is_navigated_and_one_byte_more_is_not() {
    let start = "http://127.0.0.1/r?h=";
    for (extra, navigated) in [(0, true), (1, false)] {
        let value = "v".repeat(surf_fetch::MAX_ADDRESS_BYTES - start.len() + extra);
        let html: &'static str = Box::leak(
            format!(
                "<!doctype html><form action=/r><input type=hidden name=h value={value}>\
                 <button>Find</button></form>"
            )
            .into_boxed_str(),
        );
        let mut page = load(html);
        let end = page.press("Find");
        if navigated {
            assert_eq!(end, get(&format!("{start}{value}")));
        } else {
            assert_eq!(end, Ended::Abandoned(SubmissionEndReason::AddressTooLong));
        }
    }
}

/// Decision 3: the `Debug` of [`surf::Filled`] and of [`surf::Submit::Navigate`] writes a
/// value's and a body's length, never their bytes, and the address whole.
#[test]
fn filled_and_submit_print_lengths_not_values() {
    let filled = surf::Filled {
        value: Some("s3cret-value".to_owned()),
        selected: vec!["s3cret-option".to_owned()],
        ..surf::Filled::default()
    };
    let printed = format!("{filled:?}");
    assert!(!printed.contains("s3cret"), "{printed}");
    assert!(
        printed.contains("12 bytes") && printed.contains("13 bytes"),
        "{printed}"
    );
    let submit = surf::Submit::Navigate {
        url: "http://127.0.0.1/r?q=visible".to_owned(),
        method: Method::Post,
        body: Some(b"pw=s3cret".to_vec()),
        body_type: Some("application/x-www-form-urlencoded".to_owned()),
    };
    let printed = format!("{submit:?}");
    assert!(!printed.contains("s3cret"), "{printed}");
    assert!(printed.contains("9 bytes"), "{printed}");
    assert!(
        printed.contains("http://127.0.0.1/r?q=visible"),
        "{printed}"
    );
}

/// `story:agent-press-submit`, cases 20 and 21 in the library: a submit button's `formaction`
/// and `formmethod` replace the form's `action` and `method`; a button without them takes the
/// form's. 4.10.21.3 "Form submission algorithm", the submitter's action and method.
#[test]
fn press_submit_formaction_and_formmethod_of_the_submitter() {
    let forms = "<!doctype html><form action=/session method=post>\
        <label>User <input name=user value=ada></label><button>Save</button>\
        <button formaction=/draft formmethod=get>Draft</button>\
        <button formaction=\"mailto:a@example.org\">Mail</button>\
        <button formmethod=dialog>Close</button></form>";
    assert_eq!(
        load(forms).press("Save"),
        post("http://127.0.0.1/session", "user=ada")
    );
    assert_eq!(
        load(forms).press("Draft"),
        get("http://127.0.0.1/draft?user=ada")
    );
    assert_eq!(
        load(forms).press("Mail"),
        Ended::Abandoned(SubmissionEndReason::UnsupportedScheme)
    );
    assert_eq!(
        load(forms).press("Close"),
        Ended::Abandoned(SubmissionEndReason::DialogMethod)
    );
}

/// The two values the story left to the implementor, by the HTML Standard: an empty
/// `formaction` is the document's address and not the form's `action` (the element's action is
/// the document's URL when it is the empty string); a `formmethod` of an unknown value is `GET`
/// and not the form's `method` (its invalid value default is the GET state). A `type=button`
/// does not read either.
#[test]
fn press_submit_an_empty_formaction_and_an_unknown_formmethod() {
    let forms = "<!doctype html><form action=/session method=post>\
        <input name=a value=1><button formaction=\"\">Empty</button>\
        <button formmethod=bogus>Unknown</button>\
        <button type=button formaction=/x formmethod=get>Plain</button></form>";
    assert_eq!(load(forms).press("Empty"), post(ADDRESS, "a=1"));
    assert_eq!(
        load(forms).press("Unknown"),
        get("http://127.0.0.1/session?a=1")
    );
    let page = load(forms);
    let plain = page.element("Plain").element_id;
    let plan = page.browser.plan_submission(&plain).unwrap().unwrap();
    let submits = plan.submits.expect("a form owner");
    assert_eq!(submits.method, surf::FormMethod::Post);
    assert_eq!(
        submits.action.map(|action| action.href()),
        Ok("http://127.0.0.1/session".to_owned())
    );
}

/// What the session checks is what is requested: [`surf::Plan::address`] of a `GET` is the
/// address the submission navigates to, the query written; of a `POST` its action; none for an
/// action that does not parse.
#[test]
fn press_submit_the_planned_address_is_the_requested_one() {
    let forms = "<!doctype html><form action=\"results?x=1#top\">\
        <label>Search <input name=q></label><button>Find</button></form>\
        <form method=post action=/session><button>Post</button></form>\
        <form action=\"http://[\"><button>Broken</button></form>";
    let mut page = load(forms);
    page.fill("Search", "light house");
    for (label, address) in [
        (
            "Find",
            Some("http://127.0.0.1/shop/results?q=light+house#top"),
        ),
        ("Post", Some("http://127.0.0.1/session")),
        ("Broken", None),
    ] {
        let id = page.element(label).element_id;
        let plan = page.browser.plan_submission(&id).unwrap().unwrap();
        assert_eq!(plan.address.as_deref(), address, "{label}");
    }
    assert_eq!(
        page.press("Find"),
        get("http://127.0.0.1/shop/results?q=light+house#top")
    );
}
