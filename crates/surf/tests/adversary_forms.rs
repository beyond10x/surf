//! Adversary cases on `story:agent-form-port`: the submission step of `Browser` driven with
//! pages a story case does not cover. Each page is loaded at [`ADDRESS`] from a transport in
//! memory; a button is pressed through `StartSubmission`; how the submission ended is read from
//! the log, and a submission that navigated is compared with the one request the transport was
//! handed.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use surf::{
    Answer, Browser, Filled, MemoryBrowser, RequestStarted, Submission, SubmissionStarted,
    SystemEvent, Transport,
};
use surf_ports::ComponentPorts;
use surf_types::document::{
    DocumentId, ElementId, Elements, StartSubmission, StartSubmissionOutcome, SubmissionEndReason,
    SubmissionId,
};
use surf_types::net::Method;
use surf_types::primitives::Uuid;

const ADDRESS: &str = "http://127.0.0.1/shop/form.html";

const DONE: &str = "<!doctype html><title>Done</title><p>Done.";

type Requests = Rc<RefCell<Vec<RequestStarted>>>;

fn site(page: String) -> (impl Transport, Requests) {
    let requests: Requests = Rc::default();
    let kept = Rc::clone(&requests);
    let transport = move |request: &RequestStarted| {
        kept.borrow_mut().push(request.clone());
        let body = if request.url.0 == ADDRESS {
            &page
        } else {
            DONE
        };
        Answer::Response {
            url: request.url.0.clone(),
            status: 200,
            media_type: "text/html".to_owned(),
            body: body.as_bytes().to_vec(),
        }
    };
    (transport, requests)
}

#[derive(Debug, PartialEq, Eq)]
enum Ended {
    Submitted {
        url: String,
        method: Method,
        body: Option<String>,
        body_type: Option<String>,
    },
    Abandoned(SubmissionEndReason),
    Open,
}

fn get(url: &str) -> Ended {
    Ended::Submitted {
        url: url.to_owned(),
        method: Method::Get,
        body: None,
        body_type: None,
    }
}

fn post(url: &str, body: &str) -> Ended {
    Ended::Submitted {
        url: url.to_owned(),
        method: Method::Post,
        body: Some(body.to_owned()),
        body_type: Some("application/x-www-form-urlencoded".to_owned()),
    }
}

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

struct Loaded<T> {
    browser: MemoryBrowser<T>,
    document: DocumentId,
    requests: Requests,
}

fn load(page: String) -> Loaded<impl Transport> {
    let (transport, requests) = site(page);
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport);
    let report = browser.load(ADDRESS).unwrap();
    let [document] = report.documents.as_slice() else {
        panic!("{:?}", report.documents.len());
    };
    let document = document.document_id.clone();
    Loaded {
        browser,
        document,
        requests,
    }
}

impl<T: Transport> Loaded<T> {
    fn element(&self, label: &str) -> Elements {
        let rows = self
            .browser
            .system()
            .document
            .elements(self.document.clone())
            .unwrap();
        rows.into_iter()
            .find(|row| row.label == label)
            .unwrap_or_else(|| panic!("no element labelled {label:?}"))
    }

    /// Starts a submission with the button labelled `label`, runs, and reads how it ended.
    fn press(&mut self, label: &str) -> Ended {
        self.requests.borrow_mut().clear();
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
        if let Ended::Submitted { url, method, .. } = &end {
            let requests = self.requests.borrow();
            let [request] = requests.as_slice() else {
                panic!("one request, {} asked", requests.len());
            };
            assert_eq!(&request.url.0, url);
            assert_eq!(&request.method, method);
        }
        end
    }
}

/// The address a submission navigated to, or a panic naming how it ended.
fn address_of(end: &Ended) -> &str {
    match end {
        Ended::Submitted { url, .. } => url,
        other => panic!("not submitted: {other:?}"),
    }
}

/// A `GET` whose entry list comes from the page alone (a hidden input of 70,000 bytes) builds
/// an address longer than `surf_fetch::MAX_ADDRESS_BYTES`, the 64 KiB every other address of
/// surf is held to. The step navigates there: nothing bounds the address a page makes.
#[test]
fn a_get_submission_makes_no_address_longer_than_64_kib() {
    let value = "v".repeat(70_000);
    let mut page = load(format!(
        "<!doctype html><form action=/r><input type=hidden name=h value={value}>\
         <button>Find</button></form>"
    ));
    let end = page.press("Find");
    if let Ended::Submitted { url, .. } = &end {
        assert!(
            url.len() <= surf_fetch::MAX_ADDRESS_BYTES,
            "the submission navigated to an address of {} bytes, more than {}",
            url.len(),
            surf_fetch::MAX_ADDRESS_BYTES
        );
    }
    assert_eq!(end, Ended::Abandoned(SubmissionEndReason::AddressTooLong));
}

/// A form's `action` of 70,000 bytes: the step navigates to it whole.
#[test]
fn a_submission_to_an_action_longer_than_64_kib_is_not_navigated() {
    let path = "a".repeat(70_000);
    let mut page = load(format!(
        "<!doctype html><form method=post action=/{path}><input name=q value=1>\
         <button>Send</button></form>"
    ));
    let end = page.press("Send");
    if let Ended::Submitted { url, .. } = &end {
        assert!(
            url.len() <= surf_fetch::MAX_ADDRESS_BYTES,
            "the submission navigated to an address of {} bytes, more than {}",
            url.len(),
            surf_fetch::MAX_ADDRESS_BYTES
        );
    }
    assert_eq!(end, Ended::Abandoned(SubmissionEndReason::AddressTooLong));
}

/// HTML Standard 4.10.7, the `select` element: a `required` select whose only selected option
/// is its placeholder label option (the first option, a child of the select, with the empty
/// value) suffers from being missing, so the form is not submitted. The story's Change: "only a
/// `required` control with an empty value is checked" — this select's value is empty.
#[test]
fn a_required_select_left_on_its_placeholder_is_not_submitted() {
    let mut page = load(
        "<!doctype html><form method=post action=/save>\
         <label>Size <select name=size required><option value=\"\">Choose\
         <option>Large</select></label><button>Save</button></form>"
            .to_owned(),
    );
    assert_eq!(
        page.press("Save"),
        Ended::Abandoned(SubmissionEndReason::InvalidControls)
    );
}

/// The password rule of round 2: a password control's value is in no `Debug` output. The
/// submission step's input, `surf::Submission` and its `surf::Filled` (both new in this story,
/// both `#[derive(Debug)]`), print the filled password.
#[test]
fn the_submission_steps_input_prints_no_password() {
    let page = surf::parse_page(
        b"<!doctype html><form method=post action=/login>\
          <input type=password name=pw><button>Go</button></form>",
    );
    let control = |field: surf_page::forms::Field| {
        page.forms
            .controls
            .iter()
            .find(|control| control.field == field)
            .unwrap()
            .node
    };
    let password = control(surf_page::forms::Field::Password);
    let button = control(surf_page::forms::Field::Button(
        surf_page::forms::ButtonType::Submit,
    ));
    let mut filled = BTreeMap::new();
    filled.insert(
        password,
        Filled {
            value: Some("s3cret-in-debug".to_owned()),
            ..Filled::default()
        },
    );
    let started = SubmissionStarted {
        submission_id: SubmissionId(Uuid("5a7c1e00-0000-4000-8000-0000000000a1".to_owned())),
        submitter: ElementId(Uuid("5a7c1e00-0000-4000-8000-0000000000a2".to_owned())),
    };
    let submission = Submission {
        started: &started,
        address: ADDRESS,
        tree: &page.tree,
        forms: &page.forms,
        submitter: button,
        filled: &filled,
    };
    let printed = format!("{submission:?}");
    assert!(
        !printed.contains("s3cret-in-debug"),
        "the Debug output of surf::Submission holds the filled password"
    );
}

/// What the library does with the addresses of the brief, pinned (green when the step follows
/// the HTML Standard's "mutate action URL" and the URL parser).
#[test]
fn adversary_forms_probe_addresses() {
    let mut page = load(
        "<!doctype html>\
         <form action=\"/r?x=1#top\"><input name=q value=v><button>Fragment</button></form>\
         <form action=/r><button>Nothing</button></form>\
         <form action=\"#\"><input name=q value=v><button>Hash</button></form>\
         <form action=\"http://u:p@127.0.0.1/r\"><button>Userinfo</button></form>\
         <form action=\" /sp \" method=PUT><input name=q value=v><button>Put</button></form>\
         <form method=post enctype=foo action=/e><input name=q value=v><button>Foo</button></form>\
         <form method=post enctype=TEXT/PLAIN action=/e><button>Plain</button></form>\
         <form method=DIALOG><button>Dialog</button></form>\
         <form action=\"javascript:alert(1)\"><button>Script</button></form>\
         <form action=\"data:text/html,x\"><button>Data</button></form>"
            .to_owned(),
    );
    assert_eq!(page.press("Fragment"), get("http://127.0.0.1/r?q=v#top"));
    assert_eq!(page.press("Nothing"), get("http://127.0.0.1/r?"));
    assert_eq!(
        page.press("Hash"),
        get("http://127.0.0.1/shop/form.html?q=v#")
    );
    assert_eq!(page.press("Userinfo"), get("http://u:p@127.0.0.1/r?"));
    assert_eq!(page.press("Put"), get("http://127.0.0.1/sp?q=v"));
    assert_eq!(page.press("Foo"), post("http://127.0.0.1/e", "q=v"));
    for (label, reason) in [
        ("Plain", SubmissionEndReason::UnsupportedEnctype),
        ("Dialog", SubmissionEndReason::DialogMethod),
        ("Script", SubmissionEndReason::UnsupportedScheme),
        ("Data", SubmissionEndReason::UnsupportedScheme),
    ] {
        assert_eq!(page.press(label), Ended::Abandoned(reason), "{label}");
    }
}

/// An empty `action` under a `base`: the document's address, not the base.
#[test]
fn adversary_forms_probe_empty_action_under_a_base() {
    let mut page = load(
        "<!doctype html><base href=\"http://127.0.0.1/elsewhere/\">\
         <form action=\"\" method=post><input name=a value=1><button>Here</button></form>"
            .to_owned(),
    );
    assert_eq!(page.press("Here"), post(ADDRESS, "a=1"));
}

/// The entry list against 4.10.21.4 for the controls the story's cases do not name. The
/// submitter is a button outside the form, tied by `form=`.
#[test]
fn adversary_forms_probe_entry_list() {
    let mut page = load(
        "<!doctype html><form id=f method=post action=/p>\
         <fieldset disabled><legend><input name=inlegend value=1></legend>\
         <legend><input name=second value=2></legend><input name=dis value=3></fieldset>\
         <input name=\"\" value=empty-name><input value=no-name>\
         <input type=image name=img alt=Img><input type=image alt=Anon>\
         <input type=checkbox name=c1>\
         <select name=one><option>  x \n y  </option></select>\
         <select name=many multiple><option>a<option>b</select>\
         <select name=dsel><option disabled>d1<option>d2</select>\
         <select name=og><optgroup disabled><option selected>g</optgroup></select>\
         <textarea name=ta>\nline1\nline2</textarea>\
         <input type=file name=up>\
         <button name=b1 value=v1>Not me</button>\
         <datalist><input name=dl value=z></datalist>\
         <template><input name=tp value=t></template>\
         <output name=out>o</output><object name=obj></object>\
         <input type=submit name=\"\" value=Unnamed>\
         <button name=plain value=p>Plain</button>\
         </form><button form=f name=outside value=o>Outside</button>"
            .to_owned(),
    );
    let standard = "inlegend=1&one=x+y&dsel=d2&ta=line1%0D%0Aline2&up=";
    assert_eq!(
        page.press("Outside"),
        post("http://127.0.0.1/p", &format!("{standard}&outside=o"))
    );
    assert_eq!(page.press("Unnamed"), post("http://127.0.0.1/p", standard));
    // An Image Button submitter's two entries stand where it stands in tree order.
    let rest = "one=x+y&dsel=d2&ta=line1%0D%0Aline2&up=";
    assert_eq!(
        page.press("Img"),
        post(
            "http://127.0.0.1/p",
            &format!("inlegend=1&img.x=0&img.y=0&{rest}")
        )
    );
    assert_eq!(
        page.press("Anon"),
        post("http://127.0.0.1/p", &format!("inlegend=1&x=0&y=0&{rest}"))
    );
}

/// Names and values with the bytes of the brief, from hidden inputs (no row, the attribute as
/// written): `&`, `=`, `+`, `%`, `*`, `~`, a space, a line feed (CR LF, `%0D%0A`), and NUL and a
/// lone surrogate, which the parser writes as U+FFFD.
#[test]
fn adversary_forms_probe_encoding() {
    let mut page = load(
        "<!doctype html><meta charset=utf-8><form method=post action=/e>\
         <input type=hidden name=\"&amp;=+%*~ \" value=\"&amp;=+%*~ \">\
         <input type=hidden name=nl value=\"a\nb\">\
         <input type=hidden name=nul value=\"&#0;\">\
         <input type=hidden name=sur value=\"&#xD800;\">\
         <button>Send</button></form>"
            .to_owned(),
    );
    assert_eq!(
        page.press("Send"),
        post(
            "http://127.0.0.1/e",
            "%26%3D%2B%25*%7E+=%26%3D%2B%25*%7E+&nl=a%0D%0Ab&nul=%EF%BF%BD&sur=%EF%BF%BD"
        )
    );
}

/// A form with a 1 MiB value posts it whole.
#[test]
fn adversary_forms_probe_one_mib_value() {
    let value = "w".repeat(1 << 20);
    let mut page = load(format!(
        "<!doctype html><form method=post action=/big><input type=hidden name=h value={value}>\
         <button>Big</button></form>"
    ));
    match page.press("Big") {
        Ended::Submitted { body, .. } => assert_eq!(body.map(|b| b.len()), Some((1 << 20) + 2)),
        other => panic!("{other:?}"),
    }
}

/// The size of the two probes below: at 100,000 they ran 7 s to 14 s; at this size each runs
/// under 3 s alone. They assert what is submitted, never wall time.
const PROBE: usize = 20_000;

/// Loads `html`, presses `Last` and checks the address it submits to.
fn submits_to(html: String, expected: &str) {
    let mut page = load(html);
    assert_eq!(address_of(&page.press("Last")), expected);
}

/// `PROBE` forms through the parser step: the button of the last form submits the last form.
#[test]
fn adversary_forms_probe_many_forms() {
    let mut html = String::from("<!doctype html>");
    for n in 0..PROBE {
        html.push_str(&format!("<form action=/f{n}></form>"));
    }
    html.push_str("<form action=/last><input name=q value=1><button>Last</button></form>");
    submits_to(html, "http://127.0.0.1/last?q=1");
}

/// One select of `PROBE` options through the parser step: the first option is submitted.
#[test]
fn adversary_forms_probe_many_options() {
    let mut html = String::from("<!doctype html><form action=/last><select name=s>");
    for n in 0..PROBE {
        html.push_str(&format!("<option>o{n}"));
    }
    html.push_str("</select><button>Last</button></form>");
    submits_to(html, "http://127.0.0.1/last?s=o0");
}

/// The handle `F<n>` names the `Forms` row at position `n`, and a button tied by `form=` to
/// the form of `F2` submits to that form's action.
#[test]
fn adversary_forms_probe_form_handles_name_the_submitted_form() {
    let mut page = load(
        "<!doctype html><button form=b>Early</button>\
         <form id=a action=/a><form id=x action=/x></form><button>A</button></form>\
         <template><form action=/t></form></template>\
         <form id=b action=/b></form><form id=c action=/c></form>"
            .to_owned(),
    );
    let tree = page.browser.document_tree(&page.document).unwrap();
    let by_handle: Vec<(String, Option<String>)> = surf_page::form_handles(tree)
        .into_iter()
        .map(|(node, handle)| {
            let action = match tree.data(node) {
                surf_html::NodeData::Element { attributes, .. } => attributes
                    .iter()
                    .find(|a| a.local_name == "action")
                    .map(|a| a.value.clone()),
                _ => None,
            };
            (handle, action)
        })
        .collect();
    let mut rows = page.browser.system().document.forms().unwrap();
    rows.sort_by_key(|row| row.position);
    let by_row: Vec<(String, Option<String>)> = rows
        .into_iter()
        .map(|row| (format!("F{}", row.position), row.action))
        .collect();
    assert_eq!(by_handle, by_row);
    assert_eq!(by_handle[1], ("F2".to_owned(), Some("/b".to_owned())));
    assert_eq!(page.press("Early"), get("http://127.0.0.1/b?"));
}
