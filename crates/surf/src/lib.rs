//! The `surf` system assembled: the six generated components behind their ports, the generated
//! bindings, and a transport for the `net` component.
//!
//! [`Browser`] holds the generated [`System`] and a [`Transport`]. The specification leaves one
//! step to the `net` component's own transport (`spec/domains/net.yaml`, the comment above
//! `surf.net.DeliverResponse`): after `surf.net.Fetch` made a request `Pending`, something issues
//! `surf.net.DeliverResponse` or `surf.net.FailRequest`. [`Browser::run`] is that step. It pumps the
//! system, reads every `surf.net.RequestStarted` published since its last pass, asks the transport
//! for each request that is still `Pending` in the `net` component's own `Requests` view, issues
//! the answer on the `net` port and on no other, and pumps again, until no new request started.
//!
//! A [`Browser`] is built from any six component ports that meet the generated bounds:
//! [`Browser::new`] takes a [`System`] the caller assembled, and [`Browser::from_ports`] the
//! in-memory ports of `surf-ports`. [`Browser::system`] reads the views, [`System::published`] and
//! [`System::invocations`]; the `*_mut` methods issue commands on one component's port.
//!
//! The run reads the log through a cursor of its own over [`System::published`]. No method hands
//! out the system mutably, and the one that takes from the log, [`Browser::forget_delivered`],
//! takes only what the system has delivered and moves the cursor back by as many events, so a
//! request is handed to the transport once.
//!
//! The system keeps every event it published and every binding invocation; a response body is
//! in five of them (`ResponseReceived`, `NavigationCommitted`, `ParsingStarted`, and the
//! invocations of `CommitNavigation` and `ParseDocument`), and a request body in three
//! (`NavigationStarted`, `RequestStarted` and the invocation of `Fetch`). A browser that lives
//! across many loads, as the agent's session does, calls [`Browser::forget_delivered`] once it has
//! read what it needs, so that it holds no body of a load it answered; [`Browser::held`] counts
//! what is still held.
//!
//! The transport is handed each `surf.net.RequestStarted` itself: the address, the method, and
//! the body with its media type. [`fetch_http`] is the transport step of the binaries: it sends a
//! `GET` without a body, and a `POST` with its body and media type, through `surf-fetch`, and
//! refuses a `GET` with a body or a media type before connecting.
//!
//! The `document` component leaves one step to its parser (`spec/domains/document.yaml`, the
//! comment above `surf.document.AddElement`; `docs/design/html-parser.md`, items 1 and 2): after
//! `surf.document.ParseDocument` made a document `Parsing`, something parses the body, issues
//! `surf.document.AddElement` once per actionable element in tree order and then
//! `surf.document.FinishParsing`. [`Browser::run`] is that step too. It hands each
//! `surf.document.ParsingStarted` whose document is still `Parsing` to the [`Parser`], a
//! parameter of [`Browser`] like the transport, and issues what it answers on the `document`
//! port and on no other. The default, [`HtmlParser`], decodes and parses the body with
//! `surf-html` at its default limits and computes the title, the actionable elements with
//! their labels, and the forms with each control's form owner, `name` and options with
//! `surf-page` ([`parse_page`]); the step issues `surf.document.AddForm` per form, the `name`
//! and `form_id` in each `AddElement`, and `surf.document.AddOption` per option of a select
//! after the select's `AddElement`, before `FinishParsing`. [`Browser`] keeps the tree of each
//! parsed document, keyed by the document's identity, with the node of each element row and its
//! forms: read only, through [`Browser::document_tree`] and [`Browser::element_nodes`], and for
//! the submission step. No component's port receives the tree. The tree is not changed after parsing (decision D2 of
//! `docs/plan/agent-plan-revision-1.md`), and it is kept for the life of the [`Browser`], as the
//! document's rows are.
//!
//! # The submission step
//!
//! The `document` component leaves the end of a submission to its submission step
//! (`spec/domains/document.yaml`, the comment above `surf.document.Submission`): after
//! `surf.document.StartSubmission` made a submission `Started`, something builds the request
//! from the tree and the control rows and issues `surf.document.SubmitForm` or
//! `surf.document.AbandonSubmission`. [`Browser::run`] is that step too: it hands each
//! `surf.document.SubmissionStarted` whose submission is still `Started` and whose submitter is
//! an element row of a document it parsed to the [`Submitter`], a parameter of [`Browser`] like
//! the parser step, with the document's address, tree, forms and the state of its control rows
//! ([`Submission`]), and issues what it answers on the `document` port. The default,
//! [`FormSubmitter`], is [`submit_form`]: the HTML Standard's form submission algorithm
//! (4.10.21.3) and "constructing the entry list" (4.10.21.4), from memory, in this order:
//!
//! 1. A submitter with no form owner ends `NoFormOwner` (the button's activation behaviour
//!    reads the form owner first); a Reset Button or a Button ends `NotSubmitButton`.
//! 2. Unless the form has `novalidate`, a control that suffers from being missing ends
//!    `InvalidControls`: a text, password, textarea or file control that is `required`, not
//!    disabled, not read-only and not in a `datalist`, with an empty value. The other
//!    constraints of constraint validation are not built.
//! 3. The entry list, of the form owner's controls in tree order: a control in a `datalist`, a
//!    disabled one, a button that is not the submitter, an unchecked checkbox or radio button
//!    and one without a `name` (or with an empty one) are skipped; an Image Button submitter
//!    gives `<name>.x` and `<name>.y` (`x` and `y` without a name) with the value `0`, the
//!    coordinate of an activation without a pointing device; a select gives each selected option
//!    that is not disabled; a checkbox or radio button its `value` attribute, else `on`; a
//!    Hidden `input` named `_charset_` (ASCII case-insensitive) `UTF-8`; another Hidden `input`
//!    and the submitter button their `value` attribute; a file control the empty string; the
//!    other controls the value their row holds. Every line break of a name or value becomes CR
//!    LF ("convert to a list of name-value pairs").
//! 4. The method `dialog` (ASCII case-insensitive) ends `DialogMethod`; `post` is `Post`;
//!    anything else, a missing method included, is `Get`.
//! 5. The action: the form's `action` attribute, the document's address when it is absent or
//!    empty, parsed with `surf-url` against the document base URL
//!    ([`surf_page::base_url::document_base_url`]); a failure ends `InvalidAction`.
//! 6. A scheme other than `http` and `https` ends `UnsupportedScheme`: `javascript:` (surf runs
//!    no script), `mailto:`, `data:`, `file:` and the others.
//! 7. `Get` ("mutate action URL"): the action with its query replaced by the entry list
//!    serialized as `application/x-www-form-urlencoded`, its fragment kept, whatever the
//!    enctype. `Post` ("submit as entity body"): the enctype `multipart/form-data` or
//!    `text/plain` (ASCII case-insensitive) ends `UnsupportedEnctype`; any other, a missing one
//!    included, sends the serialized entry list as the body with the media type
//!    `application/x-www-form-urlencoded`.
//!
//! Not taken: `formaction`, `formmethod`, `formenctype`, `formnovalidate` and `formtarget` on a
//! submitter; `accept-charset` and the document's encoding (every entry list is encoded as
//! UTF-8, the one encoding of `surf-url`'s serializer, and `_charset_` says so); `dirname`; the
//! `submit` event; a target other than the document's own tab; whether the document is still
//! the one its tab shows ("cannot navigate"): a submission of a document a later navigation
//! replaced still navigates the tab.
//!
//! [`Browser::load`] is what the `surf` binary does: `OpenWindow`, `OpenTab`, `Navigate`, run, and
//! the rows of the `Navigations`, `Requests`, `Documents` and `Elements` views with every
//! published `FetchFailed`, as a [`Report`]. [`exit_status`] turns its result into the binary's
//! exit status.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeMap;
use std::fmt;

use agent::Agent;
use document::Document;
use navigator::Navigator;
use net::Net;
use renderer::Renderer;
use shell::Shell;
use surf_ports::{
    AgentPorts, ComponentPorts, DocumentPorts, NavigatorPorts, NetPorts, RendererPorts, ShellPorts,
};
use surf_types::behaviour::Generated;
use surf_types::document::{
    AbandonSubmission, AddElement, AddElementOutcome, AddForm, AddFormOutcome, AddOption,
    DocumentId, DocumentState, Documents, ElementId, ElementKind, Elements, EncodingName,
    FinishParsing, FinishParsingOutcome, FormId, ParseLimit, SubmissionState, SubmitForm,
};
pub use surf_types::document::{SubmissionEndReason, SubmissionStarted};
use surf_types::navigation::{Navigate, NavigateOutcome, NavigationState, Navigations};
use surf_types::net::{
    DeliverResponse, FailRequest, FailureReason, FetchFailed, MediaType, RequestId, RequestState,
    Requests, Status,
};
use surf_types::shell::{OpenTab, OpenTabOutcome, OpenWindow, OpenWindowOutcome};
use surf_types::url::Url;

pub use surf_html::{NOT_YET_PARSED, NodeId, Tree};
use surf_system::BindingInvocation;
pub use surf_system::{System, SystemEvent};
pub use surf_types::document::ParsingStarted;
pub use surf_types::net::{Method, RequestStarted};
pub use surf_types::obligation::UnmetObligation;

/// The exit status when the navigation is `Committed`.
pub const EXIT_COMMITTED: u8 = 0;
/// The exit status when the navigation is not `Committed`: `Failed`, or, only with a transport
/// that leaves a request unanswered, still `Started`.
pub const EXIT_NOT_COMMITTED: u8 = 1;
/// The exit status of a usage error.
pub const EXIT_USAGE: u8 = 2;
/// The exit status when the load stopped with an [`Error`], and the `surf` binary's exit status
/// when the rows could not be written to standard output.
pub const EXIT_STOPPED: u8 = 3;

/// What a [`Transport`] answers for one address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// A response: issued as `surf.net.DeliverResponse`.
    Response {
        /// The address the response came from: the address asked for when nothing redirected,
        /// else the address after the last redirect. Issued as `DeliverResponse.url`.
        url: String,
        /// The status code, such as 200 or 404.
        status: u16,
        /// The type and subtype of the response's media type in lower case, such as `text/html`.
        media_type: String,
        /// The body bytes.
        body: Vec<u8>,
    },
    /// No response, for this reason: issued as `surf.net.FailRequest`.
    Failure(FailureReason),
    /// No answer. Nothing is issued and the request stays `Pending`.
    Unanswered,
}

/// Fetches the resource a started request names, for the `net` component.
///
/// It is handed the `surf.net.RequestStarted` the `net` component published: the address, the
/// method, and the body with its media type, as the navigation asked for them. A closure
/// `FnMut(&RequestStarted) -> Answer` is a transport.
pub trait Transport {
    /// Fetches `request` and answers what came of it.
    fn fetch(&mut self, request: &RequestStarted) -> Answer;
}

impl<F: FnMut(&RequestStarted) -> Answer> Transport for F {
    fn fetch(&mut self, request: &RequestStarted) -> Answer {
        self(request)
    }
}

/// Fetches `request` with `surf-fetch` within `limits`, for a transport that goes to the network.
///
/// A request with [`Method::Get`], no body and no media type is sent as a `GET`; one with
/// [`Method::Post`] as a `POST` with its body and media type, through [`surf_fetch::send`],
/// whose checks, limits and redirect rule apply (a 301, 302 or 303 continues as a `GET` without
/// the body). A [`Method::Get`] with a body or a media type ends with
/// [`surf_fetch::Failure::UnsupportedScheme`], whose detail names the method, before any
/// connection is attempted.
pub fn fetch_http(
    request: &RequestStarted,
    limits: &surf_fetch::Limits,
) -> Result<surf_fetch::Response, surf_fetch::Failure> {
    let method = match (request.method, &request.body, &request.body_type) {
        (Method::Get, None, None) => surf_fetch::Method::Get,
        (Method::Get, _, _) => {
            return Err(surf_fetch::Failure::UnsupportedScheme(
                "a GET with a body or a media type is not sent".to_owned(),
            ));
        }
        (Method::Post, _, _) => surf_fetch::Method::Post,
    };
    surf_fetch::send(
        &surf_fetch::Request {
            address: &request.url.0,
            method,
            body: request.body.as_deref(),
            media_type: request
                .body_type
                .as_ref()
                .map(|media_type| media_type.0.as_str()),
        },
        limits,
    )
}

/// The [`Answer`] for what `surf-fetch` returned: a response, or the failure reason of the same
/// name.
pub fn answer(result: Result<surf_fetch::Response, surf_fetch::Failure>) -> Answer {
    match result {
        Ok(response) => Answer::Response {
            url: response.url,
            status: response.status,
            media_type: response.media_type,
            body: response.body,
        },
        Err(surf_fetch::Failure::Unreachable(_)) => Answer::Failure(FailureReason::Unreachable),
        Err(surf_fetch::Failure::UnsupportedScheme(_)) => {
            Answer::Failure(FailureReason::UnsupportedScheme)
        }
        Err(surf_fetch::Failure::MalformedResponse(_)) => {
            Answer::Failure(FailureReason::MalformedResponse)
        }
        Err(surf_fetch::Failure::TooManyRedirects(_)) => {
            Answer::Failure(FailureReason::TooManyRedirects)
        }
        Err(surf_fetch::Failure::ForbiddenDestination(_)) => {
            Answer::Failure(FailureReason::ForbiddenDestination)
        }
    }
}

/// One parsed page: its tree and the values the `document` component's rows take from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The document's node tree.
    pub tree: Tree,
    /// `document.title`: the empty string when the document has no title.
    pub title: String,
    /// The actionable elements in tree order, each with its node in [`Page::tree`]. Issued as one
    /// `surf.document.AddElement` each, with the position from 1.
    pub elements: Vec<surf_page::ActionableElement>,
    /// The encoding the text was decoded with, as the Encoding Standard writes its name.
    pub encoding: String,
    /// The encoding the encoding sniffing algorithm chose when `surf-html` has no decoder for it
    /// and the text was decoded as windows-1252.
    pub unsupported_encoding: Option<String>,
    /// The limit that stopped parsing, if one did.
    pub stopped_by: Option<surf_html::limits::Limit>,
    /// The forms and the controls with their form owner, `name` and options
    /// ([`surf_page::forms::forms`]). Issued as one `surf.document.AddForm` per form, the
    /// `name` and `form_id` of each element's `AddElement`, and one `surf.document.AddOption`
    /// per option of a select.
    pub forms: surf_page::forms::Forms,
}

/// What a [`Parser`] answers for one started parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parse {
    /// The page: issued as one `surf.document.AddElement` per element and then
    /// `surf.document.FinishParsing`.
    Parsed(Box<Page>),
    /// No answer. Nothing is issued and the document stays `Parsing`.
    Unanswered,
}

/// Parses the body a started parse carries, for the `document` component.
///
/// It is handed the `surf.document.ParsingStarted` the `document` component published: the
/// document's identity and the body. A closure `FnMut(&ParsingStarted) -> Parse` is a parser.
pub trait Parser {
    /// Parses `started`'s body and answers what came of it.
    fn parse(&mut self, started: &ParsingStarted) -> Parse;
}

impl<F: FnMut(&ParsingStarted) -> Parse> Parser for F {
    fn parse(&mut self, started: &ParsingStarted) -> Parse {
        self(started)
    }
}

/// The parser step of the binaries: [`parse_page_with`] over every body, with its options.
///
/// [`HtmlParser::default`] parses with `surf-html`'s default options (scripting disabled, the
/// default limits); [`HtmlParser::with_options`] takes others, such as small limits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HtmlParser {
    options: surf_html::ParseOptions,
}

impl HtmlParser {
    /// The parser step that parses with `options`.
    pub fn with_options(options: surf_html::ParseOptions) -> HtmlParser {
        HtmlParser { options }
    }

    /// The parser step that parses with the default options but `limits`.
    pub fn with_limits(limits: surf_html::limits::Limits) -> HtmlParser {
        HtmlParser::with_options(surf_html::ParseOptions {
            limits,
            ..surf_html::ParseOptions::default()
        })
    }

    /// The options it parses with.
    pub fn options(&self) -> &surf_html::ParseOptions {
        &self.options
    }
}

impl Parser for HtmlParser {
    fn parse(&mut self, started: &ParsingStarted) -> Parse {
        Parse::Parsed(Box::new(parse_page_with(&started.body, &self.options)))
    }
}

/// [`parse_page_with`] with `surf-html`'s default options: scripting disabled, the default
/// limits.
pub fn parse_page(body: &[u8]) -> Page {
    parse_page_with(body, &surf_html::ParseOptions::default())
}

/// Decodes and parses `body` as an HTML document with `surf-html` under `options`
/// ([`surf_html::parse_document_bytes`]), and computes its title and its actionable elements
/// with their labels with `surf-page`. A parse a limit stopped keeps the tree built before it
/// (`surf-html`, "Limits"), and the title and the elements are read from that tree.
pub fn parse_page_with(body: &[u8], options: &surf_html::ParseOptions) -> Page {
    let parsed = surf_html::parse_document_bytes(body, options);
    // The sniffing algorithm always chooses an encoding; windows-1252 is its default.
    let chosen = parsed.encoding.unwrap_or("windows-1252");
    let decoded_with = surf_html::encoding::decode_with(b"", chosen)
        .map_or("windows-1252", |decoded| decoded.decoded_with);
    let unsupported_encoding = (decoded_with != chosen).then(|| chosen.to_owned());
    let title = surf_page::title(&parsed.tree);
    let elements = surf_page::actionable_elements(&parsed.tree);
    let forms = surf_page::forms::forms(&parsed.tree);
    Page {
        stopped_by: parsed.tree.stopped_by(),
        tree: parsed.tree,
        title,
        elements,
        encoding: decoded_with.to_owned(),
        unsupported_encoding,
        forms,
    }
}

/// One started submission as the submission step is handed it: the document's address, its
/// tree, its forms and controls, the submitter's node, and the state each control's rows hold
/// now.
#[derive(Debug, Clone, Copy)]
pub struct Submission<'a> {
    /// The `surf.document.SubmissionStarted` the `document` component published.
    pub started: &'a SubmissionStarted,
    /// The address of the submitter's document, its `Documents` row's `url`.
    pub address: &'a str,
    /// The document's tree.
    pub tree: &'a Tree,
    /// The document's forms and controls, as its parser step found them.
    pub forms: &'a surf_page::forms::Forms,
    /// The submitter's node in [`Submission::tree`].
    pub submitter: NodeId,
    /// The state of each control that has a row, by its node.
    pub filled: &'a BTreeMap<NodeId, Filled>,
}

/// The state of one control as its rows hold it when the submission starts: what `SetValue`,
/// `Check` and `SelectOption` left.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filled {
    /// The `Elements` row's `value`.
    pub value: Option<String>,
    /// The `Elements` row's `checked`.
    pub checked: bool,
    /// The `Elements` row's `required`.
    pub required: bool,
    /// For a select, the values of its `Options` rows that are selected and not disabled, in
    /// position order; empty for every other kind.
    pub selected: Vec<String>,
}

/// What a [`Submitter`] answers for one started submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submit {
    /// The request the submission navigates to: issued as `surf.document.SubmitForm` with the
    /// tab of the submitter's document.
    Navigate {
        /// The address.
        url: String,
        /// `Get` or `Post`.
        method: Method,
        /// A `Post`'s body.
        body: Option<Vec<u8>>,
        /// A `Post`'s media type.
        body_type: Option<String>,
    },
    /// No navigation, for this reason: issued as `surf.document.AbandonSubmission`.
    Abandon(SubmissionEndReason),
    /// No answer. Nothing is issued and the submission stays `Started`.
    Unanswered,
}

/// Ends a started submission, for the `document` component.
///
/// It is handed a [`Submission`]: the started submission with what its document holds. A
/// closure `FnMut(&Submission<'_>) -> Submit` is a submission step.
pub trait Submitter {
    /// Answers how `submission` ends.
    fn submit(&mut self, submission: &Submission<'_>) -> Submit;
}

impl<F: FnMut(&Submission<'_>) -> Submit> Submitter for F {
    fn submit(&mut self, submission: &Submission<'_>) -> Submit {
        self(submission)
    }
}

/// The submission step of the binaries and the session: [`submit_form`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FormSubmitter;

impl Submitter for FormSubmitter {
    fn submit(&mut self, submission: &Submission<'_>) -> Submit {
        submit_form(submission)
    }
}

/// The media type of the one encoding surf builds.
const URLENCODED: &str = "application/x-www-form-urlencoded";

/// The HTML Standard's form submission algorithm (4.10.21.3) for `submission`, as far as surf
/// takes it (see the crate documentation, "The submission step").
pub fn submit_form(submission: &Submission<'_>) -> Submit {
    use surf_page::forms::{ButtonType, Field};

    let forms = submission.forms;
    let Some(submitter) = forms.control(submission.submitter) else {
        return Submit::Unanswered;
    };
    let Some(owner) = submitter.owner else {
        return Submit::Abandon(SubmissionEndReason::NoFormOwner);
    };
    if !matches!(
        submitter.field,
        Field::Button(ButtonType::Submit | ButtonType::Image)
    ) {
        return Submit::Abandon(SubmissionEndReason::NotSubmitButton);
    }
    let form = &forms.forms[owner];
    let controls: Vec<&surf_page::forms::Control> = forms
        .controls
        .iter()
        .filter(|control| control.owner == Some(owner))
        .collect();
    let filled = |control: &surf_page::forms::Control| {
        submission
            .filled
            .get(&control.node)
            .cloned()
            .unwrap_or_default()
    };
    if !form.novalidate
        && controls
            .iter()
            .any(|control| suffers_from_being_missing(control, &filled(control)))
    {
        return Submit::Abandon(SubmissionEndReason::InvalidControls);
    }
    let pairs = entry_list(&controls, submission.submitter, &filled);
    let method = form.method.as_deref().unwrap_or("");
    let method = if method.eq_ignore_ascii_case("post") {
        Method::Post
    } else if method.eq_ignore_ascii_case("dialog") {
        return Submit::Abandon(SubmissionEndReason::DialogMethod);
    } else {
        Method::Get
    };
    let Ok(document) = surf_url::Url::parse(submission.address, None) else {
        return Submit::Abandon(SubmissionEndReason::InvalidAction);
    };
    let action = match form.action.as_deref() {
        None | Some("") => submission.address,
        Some(action) => action,
    };
    let base = surf_page::base_url::document_base_url(submission.tree, &document);
    let Ok(action) = surf_url::Url::parse(action, Some(&base)) else {
        return Submit::Abandon(SubmissionEndReason::InvalidAction);
    };
    if !matches!(action.scheme(), "http" | "https") {
        return Submit::Abandon(SubmissionEndReason::UnsupportedScheme);
    }
    let encoded = surf_url::form_urlencoded::serialize(&pairs);
    match method {
        Method::Get => Submit::Navigate {
            url: with_query(&action, &encoded),
            method,
            body: None,
            body_type: None,
        },
        Method::Post => {
            let enctype = form.enctype.as_deref().unwrap_or("");
            if enctype.eq_ignore_ascii_case("multipart/form-data")
                || enctype.eq_ignore_ascii_case("text/plain")
            {
                return Submit::Abandon(SubmissionEndReason::UnsupportedEnctype);
            }
            Submit::Navigate {
                url: action.href(),
                method,
                body: Some(encoded.into_bytes()),
                body_type: Some(URLENCODED.to_owned()),
            }
        }
    }
}

/// "Suffering from being missing", the one constraint built: a candidate for constraint
/// validation (not disabled, not in a `datalist`, not read-only) of a kind that holds a value,
/// `required`, with an empty value.
fn suffers_from_being_missing(control: &surf_page::forms::Control, filled: &Filled) -> bool {
    use surf_page::forms::Field;

    matches!(
        control.field,
        Field::Text | Field::Password | Field::Textarea | Field::File
    ) && !control.disabled
        && !control.in_datalist
        && !control.readonly
        && filled.required
        && filled.value.as_deref().unwrap_or("").is_empty()
}

/// 4.10.21.4 "Constructing the entry list" over the form's `controls` in tree order with the
/// submitter `submitter`, then "convert to a list of name-value pairs": every line break in a
/// name or value written as CR LF.
fn entry_list(
    controls: &[&surf_page::forms::Control],
    submitter: NodeId,
    filled: &dyn Fn(&surf_page::forms::Control) -> Filled,
) -> Vec<(String, String)> {
    use surf_page::forms::{ButtonType, Field};

    let mut entries = Vec::new();
    for control in controls {
        if control.in_datalist || control.disabled {
            continue;
        }
        if matches!(control.field, Field::Button(_)) && control.node != submitter {
            continue;
        }
        let state = filled(control);
        if matches!(control.field, Field::Checkbox | Field::Radio) && !state.checked {
            continue;
        }
        let name = control.name.as_deref().unwrap_or("");
        if control.field == Field::Button(ButtonType::Image) {
            // The selected coordinate of an activation without a pointing device is (0, 0).
            let (x, y) = if name.is_empty() {
                ("x".to_owned(), "y".to_owned())
            } else {
                (format!("{name}.x"), format!("{name}.y"))
            };
            entries.push((x, "0".to_owned()));
            entries.push((y, "0".to_owned()));
            continue;
        }
        if name.is_empty() {
            continue;
        }
        let attribute = || control.value.clone().unwrap_or_default();
        match control.field {
            Field::Select => {
                for value in state.selected {
                    entries.push((name.to_owned(), value));
                }
            }
            Field::Checkbox | Field::Radio => entries.push((
                name.to_owned(),
                control.value.clone().unwrap_or_else(|| "on".to_owned()),
            )),
            Field::Hidden if name.eq_ignore_ascii_case("_charset_") => {
                entries.push((name.to_owned(), "UTF-8".to_owned()));
            }
            Field::Hidden | Field::Button(_) => entries.push((name.to_owned(), attribute())),
            // A file control holds no file: the entry's file has the empty name, which is what
            // the urlencoded serializer writes for a file.
            Field::File => entries.push((name.to_owned(), String::new())),
            Field::Text | Field::Password | Field::Textarea => {
                entries.push((name.to_owned(), state.value.unwrap_or_default()));
            }
        }
    }
    entries
        .into_iter()
        .map(|(name, value)| (crlf(&name), crlf(&value)))
        .collect()
}

/// Every CR not followed by LF and every LF not preceded by CR replaced by CR LF.
fn crlf(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                out.push_str("\r\n");
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
            }
            '\n' => out.push_str("\r\n"),
            c => out.push(c),
        }
    }
    out
}

/// "Mutate action URL": `action` with its query set to `query`, its fragment kept.
fn with_query(action: &surf_url::Url, query: &str) -> String {
    let mut url = action.serialize(true);
    if let Some(old) = action.query() {
        url.truncate(url.len() - old.len() - 1);
    }
    url.push('?');
    url.push_str(query);
    if let Some(fragment) = action.fragment() {
        url.push('#');
        url.push_str(fragment);
    }
    url
}

fn element_kind(kind: surf_page::Kind) -> ElementKind {
    match kind {
        surf_page::Kind::Link => ElementKind::Link,
        surf_page::Kind::Button => ElementKind::Button,
        surf_page::Kind::Text => ElementKind::Text,
        surf_page::Kind::Password => ElementKind::Password,
        surf_page::Kind::Checkbox => ElementKind::Checkbox,
        surf_page::Kind::Radio => ElementKind::Radio,
        surf_page::Kind::File => ElementKind::File,
        surf_page::Kind::Select => ElementKind::Select,
        surf_page::Kind::Textarea => ElementKind::Textarea,
    }
}

fn parse_limit(limit: surf_html::limits::Limit) -> ParseLimit {
    match limit {
        surf_html::limits::Limit::InputBytes => ParseLimit::InputBytes,
        surf_html::limits::Limit::Depth => ParseLimit::Depth,
        surf_html::limits::Limit::Nodes => ParseLimit::Nodes,
        surf_html::limits::Limit::Attributes => ParseLimit::Attributes,
        surf_html::limits::Limit::StoredBytes => ParseLimit::StoredBytes,
        surf_html::limits::Limit::Steps => ParseLimit::Steps,
    }
}

/// Why a load stopped before its report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A command or a pump returned an unmet obligation of the generated code.
    Unmet(UnmetObligation),
    /// `surf.shell.OpenTab` refused the window `OpenWindow` had just opened. Ports that keep the
    /// window they were given never do this.
    Refused(String),
}

impl From<UnmetObligation> for Error {
    fn from(obligation: UnmetObligation) -> Self {
        Error::Unmet(obligation)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unmet(obligation) => obligation.fmt(f),
            Error::Refused(refusal) => f.write_str(refusal),
        }
    }
}

impl std::error::Error for Error {}

/// The rows a load ends with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The navigation the load started.
    pub navigation_id: RequestId,
    /// The rows of `surf.navigation.Navigations`.
    pub navigations: Vec<Navigations>,
    /// The rows of `surf.net.Requests`.
    pub requests: Vec<Requests>,
    /// The rows of `surf.document.Documents`.
    pub documents: Vec<Documents>,
    /// The rows of `surf.document.Elements` of each document, in the order of
    /// [`Report::documents`], each document's in position order.
    pub elements: Vec<Elements>,
    /// Every published `surf.net.FetchFailed`, in publication order.
    pub fetch_failures: Vec<FetchFailed>,
    /// What the parser does not yet parse as the HTML Standard says
    /// ([`Browser::parser_not_yet_parsed`]).
    pub not_yet_parsed: &'static [&'static str],
}

impl Report {
    /// The state of the navigation the load started, if its row is in the view.
    pub fn navigation_state(&self) -> Option<NavigationState> {
        self.navigations
            .iter()
            .find(|row| row.navigation_id == self.navigation_id)
            .map(|row| row.state)
    }

    /// The output lines, without line terminators: every navigation, then every request, every
    /// document, then for each `Parsed` document its title, its elements and the limit that
    /// stopped it, then every `FetchFailed`, each in the order its view or the log holds it, and
    /// last, when a document is `Parsed` and [`Report::not_yet_parsed`] is not empty, the
    /// parser's note.
    ///
    /// ```text
    /// navigation <navigation-id> <state> <address>
    /// request <request-id> <state> <address>
    /// document <document-id> <address>
    /// title <document-id> "<title>"
    /// element <document-id> <position> <kind> <flags> "<label>" "<target or value>"
    /// note limit <document-id> <limit>
    /// fetch-failed <request-id> <reason> <address>
    /// note parser: <parts>
    /// ```
    ///
    /// Fields are separated by one space. The address is the rest of the line, written by
    /// [`escape`]. A title, a label, a target and a value are quoted fields: written by
    /// [`escape`] with `"` written as `\x22`, between two `"`. `<flags>` is the flags of the row
    /// that are set, in the order `hidden`, `disabled`, `checked`, `required`, joined by `,`, or
    /// `-` when none is. The last field is a link's target or a control's value, or `-` when the
    /// row has neither; for a `password` it is `(set)` when the value is not empty and `(empty)`
    /// when it is, unquoted, never the value. `<kind>` is a name of `surf.document.ElementKind`, `<limit>` one of
    /// `surf.document.ParseLimit`, and `<parts>` the entries of [`Report::not_yet_parsed`]
    /// joined by `,`.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for row in &self.navigations {
            lines.push(format!(
                "navigation {} {} {}",
                row.navigation_id.0.0,
                navigation_state(row.state),
                escape(&row.url.0)
            ));
        }
        for row in &self.requests {
            lines.push(format!(
                "request {} {} {}",
                row.request_id.0.0,
                request_state(row.state),
                escape(&row.url.0)
            ));
        }
        for row in &self.documents {
            lines.push(format!(
                "document {} {}",
                row.document_id.0.0,
                escape(&row.url.0)
            ));
        }
        let parsed = || {
            self.documents
                .iter()
                .filter(|row| row.state == DocumentState::Parsed)
        };
        for row in parsed() {
            let id = &row.document_id.0.0;
            lines.push(format!(
                "title {id} {}",
                quoted(row.title.as_deref().unwrap_or(""))
            ));
            for element in self
                .elements
                .iter()
                .filter(|element| element.document_id == row.document_id)
            {
                lines.push(element_line(element));
            }
            if let Some(limit) = row.stopped_by {
                lines.push(format!("note limit {id} {}", limit_name(limit)));
            }
        }
        for event in &self.fetch_failures {
            lines.push(format!(
                "fetch-failed {} {} {}",
                event.request_id.0.0,
                failure_reason(event.reason),
                escape(&event.url.0)
            ));
        }
        if parsed().next().is_some() && !self.not_yet_parsed.is_empty() {
            lines.push(format!("note parser: {}", self.not_yet_parsed.join(",")));
        }
        lines
    }
}

/// The `element` line of one row of `surf.document.Elements` ([`Report::lines`]).
fn element_line(element: &Elements) -> String {
    let flags: Vec<&str> = [
        (element.hidden, "hidden"),
        (element.disabled, "disabled"),
        (element.checked, "checked"),
        (element.required, "required"),
    ]
    .into_iter()
    .filter_map(|(set, name)| set.then_some(name))
    .collect();
    let flags = if flags.is_empty() {
        "-".to_owned()
    } else {
        flags.join(",")
    };
    // A password's value is never printed: `(set)` or `(empty)`, unquoted (design section 5.2).
    let last = if element.kind == ElementKind::Password {
        if element
            .value
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        {
            "(set)".to_owned()
        } else {
            "(empty)".to_owned()
        }
    } else {
        match element.target.as_deref().or(element.value.as_deref()) {
            Some(text) => quoted(text),
            None => "-".to_owned(),
        }
    };
    format!(
        "element {} {} {} {flags} {} {last}",
        element.document_id.0.0,
        element.position,
        kind_name(element.kind),
        quoted(&element.label),
    )
}

/// A value from a page as one quoted field: [`escape`], with `"` written as `\x22`, between two
/// `"`. The escape writes no `"` of its own, so the field ends at the first `"`.
fn quoted(text: &str) -> String {
    format!("\"{}\"", escape(text).replace('"', "\\x22"))
}

fn kind_name(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Link => "link",
        ElementKind::Button => "button",
        ElementKind::Text => "text",
        ElementKind::Password => "password",
        ElementKind::Checkbox => "checkbox",
        ElementKind::Radio => "radio",
        ElementKind::File => "file",
        ElementKind::Select => "select",
        ElementKind::Textarea => "textarea",
    }
}

fn limit_name(limit: ParseLimit) -> &'static str {
    match limit {
        ParseLimit::InputBytes => "InputBytes",
        ParseLimit::Depth => "Depth",
        ParseLimit::Nodes => "Nodes",
        ParseLimit::Attributes => "Attributes",
        ParseLimit::StoredBytes => "StoredBytes",
        ParseLimit::Steps => "Steps",
    }
}

/// An address as one output field: printable ASCII (U+0020 to U+007E) other than the backslash
/// is written as itself; the backslash and every other character is written as `\x` and two
/// lower-case hexadecimal digits per byte of its UTF-8 encoding. That covers the control
/// characters and line terminators, Unicode's line and paragraph separators (U+2028, U+2029),
/// and every format character, such as the bidirectional controls, so the field can neither
/// split a line nor reorder one. The standard library has no table of Unicode's general
/// categories, so no character outside ASCII is let through: `surf-fetch` refuses such an
/// address anyway.
pub fn escape(address: &str) -> String {
    let mut escaped = String::with_capacity(address.len());
    for character in address.chars() {
        if character == '\\' || !matches!(character, ' '..='~') {
            let mut bytes = [0u8; 4];
            for byte in character.encode_utf8(&mut bytes).bytes() {
                escaped.push_str(&format!("\\x{byte:02x}"));
            }
        } else {
            escaped.push(character);
        }
    }
    escaped
}

fn navigation_state(state: NavigationState) -> &'static str {
    match state {
        NavigationState::Cancelled => "Cancelled",
        NavigationState::Committed => "Committed",
        NavigationState::Failed => "Failed",
        NavigationState::Started => "Started",
    }
}

fn request_state(state: RequestState) -> &'static str {
    match state {
        RequestState::Failed => "Failed",
        RequestState::Pending => "Pending",
        RequestState::Responded => "Responded",
    }
}

fn failure_reason(reason: FailureReason) -> &'static str {
    match reason {
        FailureReason::Unreachable => "Unreachable",
        FailureReason::UnsupportedScheme => "UnsupportedScheme",
        FailureReason::MalformedResponse => "MalformedResponse",
        FailureReason::TooManyRedirects => "TooManyRedirects",
        FailureReason::ForbiddenDestination => "ForbiddenDestination",
    }
}

/// The exit status of a load: [`EXIT_COMMITTED`] when its navigation is `Committed`,
/// [`EXIT_NOT_COMMITTED`] when it is in any other state or has no row, and [`EXIT_STOPPED`] when
/// the load stopped with an [`Error`].
pub fn exit_status(result: &Result<Report, Error>) -> u8 {
    match result {
        Ok(report) => match report.navigation_state() {
            Some(NavigationState::Committed) => EXIT_COMMITTED,
            _ => EXIT_NOT_COMMITTED,
        },
        Err(_) => EXIT_STOPPED,
    }
}

/// The behaviours the `agent` component's port needs to be part of a [`System`].
pub trait AgentBehaviors:
    surf_types::agent::obligations::CloseSessionBehavior
    + surf_types::agent::obligations::FillBehavior
    + surf_types::agent::obligations::FollowBehavior
    + surf_types::agent::obligations::GoBehavior
    + surf_types::agent::obligations::NoteNavigationCommittedBehavior
    + surf_types::agent::obligations::NoteNavigationEndedBehavior
    + surf_types::agent::obligations::NoteNavigationFailedBehavior
    + surf_types::agent::obligations::NoteNavigationStartedBehavior
    + surf_types::agent::obligations::OpenSessionBehavior
    + surf_types::agent::obligations::PickBehavior
    + surf_types::agent::obligations::PressBehavior
    + surf_types::agent::obligations::RecordActionBehavior
    + surf_types::agent::obligations::SessionLogQuery
    + surf_types::agent::obligations::SessionStateViewQuery
    + surf_types::agent::obligations::SessionsQuery
{
}

impl<B> AgentBehaviors for B where
    B: surf_types::agent::obligations::CloseSessionBehavior
        + surf_types::agent::obligations::FillBehavior
        + surf_types::agent::obligations::FollowBehavior
        + surf_types::agent::obligations::GoBehavior
        + surf_types::agent::obligations::NoteNavigationCommittedBehavior
        + surf_types::agent::obligations::NoteNavigationEndedBehavior
        + surf_types::agent::obligations::NoteNavigationFailedBehavior
        + surf_types::agent::obligations::NoteNavigationStartedBehavior
        + surf_types::agent::obligations::OpenSessionBehavior
        + surf_types::agent::obligations::PickBehavior
        + surf_types::agent::obligations::PressBehavior
        + surf_types::agent::obligations::RecordActionBehavior
        + surf_types::agent::obligations::SessionLogQuery
        + surf_types::agent::obligations::SessionStateViewQuery
        + surf_types::agent::obligations::SessionsQuery
{
}

/// The behaviours the `document` component's port needs to be part of a [`System`].
pub trait DocumentBehaviors:
    surf_types::document::obligations::AbandonSubmissionBehavior
    + surf_types::document::obligations::AddElementBehavior
    + surf_types::document::obligations::AddFormBehavior
    + surf_types::document::obligations::AddOptionBehavior
    + surf_types::document::obligations::CheckBehavior
    + surf_types::document::obligations::FinishParsingBehavior
    + surf_types::document::obligations::ParseDocumentBehavior
    + surf_types::document::obligations::SelectOptionBehavior
    + surf_types::document::obligations::SetValueBehavior
    + surf_types::document::obligations::StartSubmissionBehavior
    + surf_types::document::obligations::SubmitFormBehavior
    + surf_types::document::obligations::ControlsQuery
    + surf_types::document::obligations::DocumentsQuery
    + surf_types::document::obligations::ElementsQuery
    + surf_types::document::obligations::FormsQuery
    + surf_types::document::obligations::OptionsQuery
    + surf_types::document::obligations::SubmissionsQuery
{
}

impl<B> DocumentBehaviors for B where
    B: surf_types::document::obligations::AbandonSubmissionBehavior
        + surf_types::document::obligations::AddElementBehavior
        + surf_types::document::obligations::AddFormBehavior
        + surf_types::document::obligations::AddOptionBehavior
        + surf_types::document::obligations::CheckBehavior
        + surf_types::document::obligations::FinishParsingBehavior
        + surf_types::document::obligations::ParseDocumentBehavior
        + surf_types::document::obligations::SelectOptionBehavior
        + surf_types::document::obligations::SetValueBehavior
        + surf_types::document::obligations::StartSubmissionBehavior
        + surf_types::document::obligations::SubmitFormBehavior
        + surf_types::document::obligations::ControlsQuery
        + surf_types::document::obligations::DocumentsQuery
        + surf_types::document::obligations::ElementsQuery
        + surf_types::document::obligations::FormsQuery
        + surf_types::document::obligations::OptionsQuery
        + surf_types::document::obligations::SubmissionsQuery
{
}

/// The behaviours the `navigator` component's port needs to be part of a [`System`].
pub trait NavigatorBehaviors:
    surf_types::navigation::obligations::CancelNavigationBehavior
    + surf_types::navigation::obligations::CommitNavigationBehavior
    + surf_types::navigation::obligations::FailNavigationBehavior
    + surf_types::navigation::obligations::NavigateBehavior
    + surf_types::navigation::obligations::NavigationsQuery
{
}

impl<B> NavigatorBehaviors for B where
    B: surf_types::navigation::obligations::CancelNavigationBehavior
        + surf_types::navigation::obligations::CommitNavigationBehavior
        + surf_types::navigation::obligations::FailNavigationBehavior
        + surf_types::navigation::obligations::NavigateBehavior
        + surf_types::navigation::obligations::NavigationsQuery
{
}

/// The behaviours the `net` component's port needs to be part of a [`System`].
pub trait NetBehaviors:
    surf_types::net::obligations::DeliverResponseBehavior
    + surf_types::net::obligations::FailRequestBehavior
    + surf_types::net::obligations::FetchBehavior
    + surf_types::net::obligations::RequestsQuery
{
}

impl<B> NetBehaviors for B where
    B: surf_types::net::obligations::DeliverResponseBehavior
        + surf_types::net::obligations::FailRequestBehavior
        + surf_types::net::obligations::FetchBehavior
        + surf_types::net::obligations::RequestsQuery
{
}

/// The behaviours the `renderer` component's port needs to be part of a [`System`].
pub trait RendererBehaviors:
    surf_types::render::obligations::PaintFrameBehavior + surf_types::render::obligations::FramesQuery
{
}

impl<B> RendererBehaviors for B where
    B: surf_types::render::obligations::PaintFrameBehavior
        + surf_types::render::obligations::FramesQuery
{
}

/// The behaviours the `shell` component's port needs to be part of a [`System`].
pub trait ShellBehaviors:
    surf_types::shell::obligations::CloseTabBehavior
    + surf_types::shell::obligations::OpenTabBehavior
    + surf_types::shell::obligations::OpenWindowBehavior
    + surf_types::shell::obligations::TabsQuery
    + surf_types::shell::obligations::WindowsQuery
{
}

impl<B> ShellBehaviors for B where
    B: surf_types::shell::obligations::CloseTabBehavior
        + surf_types::shell::obligations::OpenTabBehavior
        + surf_types::shell::obligations::OpenWindowBehavior
        + surf_types::shell::obligations::TabsQuery
        + surf_types::shell::obligations::WindowsQuery
{
}

/// The generated [`System`], the transport of its `net` component, and the parser step and the
/// submission step of its `document` component.
///
/// The type parameters are the behaviours behind each component's port, in the order
/// [`System::new`] takes the ports (`agent`, `document`, `navigator`, `net`, `renderer`,
/// `shell`), the transport, the parser step, [`HtmlParser`] unless [`Browser::with_parser`]
/// gave another, and the submission step, [`FormSubmitter`] unless [`Browser::with_submitter`]
/// gave another.
pub struct Browser<A, D, N, Ne, R, S, T, P = HtmlParser, F = FormSubmitter> {
    system: System<A, D, N, Ne, R, S>,
    transport: T,
    parser: P,
    submitter: F,
    /// How far into [`System::published`] the run has read.
    cursor: usize,
    /// The tree of each parsed document and the node of each of its element rows, by the
    /// document's identity.
    kept: BTreeMap<String, Kept>,
}

/// What a [`Browser`] keeps of one parsed document.
struct Kept {
    tree: Tree,
    /// The identity of each element row, in position order, with its node in `tree`.
    nodes: Vec<(ElementId, NodeId)>,
    /// The forms and controls of `tree`, for the submission step.
    forms: surf_page::forms::Forms,
}

/// A [`Browser`] over the in-memory ports of `surf-ports` and the operating system's random
/// source.
pub type MemoryBrowser<T> = Browser<
    Generated<AgentPorts>,
    Generated<DocumentPorts>,
    Generated<NavigatorPorts>,
    Generated<NetPorts>,
    Generated<RendererPorts>,
    Generated<ShellPorts>,
    T,
>;

impl<T> MemoryBrowser<T> {
    /// The system over `ports`, with `transport`.
    pub fn from_ports(ports: ComponentPorts, transport: T) -> Self {
        Browser::new(
            System::new(
                ports.agent,
                ports.document,
                ports.navigator,
                ports.net,
                ports.renderer,
                ports.shell,
            ),
            transport,
        )
    }
}

impl<A, D, N, Ne, R, S, T> Browser<A, D, N, Ne, R, S, T> {
    /// `system` with `transport`, the parser step [`HtmlParser`] and the submission step
    /// [`FormSubmitter`]. The first [`Browser::run`] reads `system`'s log from its start.
    pub fn new(system: System<A, D, N, Ne, R, S>, transport: T) -> Self {
        Self {
            system,
            transport,
            parser: HtmlParser::default(),
            submitter: FormSubmitter,
            cursor: 0,
            kept: BTreeMap::new(),
        }
    }
}

impl<A, D, N, Ne, R, S, T, P, F> Browser<A, D, N, Ne, R, S, T, P, F> {
    /// The same browser with the parser step `parser` in place of its own. What it ran and kept
    /// stays.
    pub fn with_parser<Q>(self, parser: Q) -> Browser<A, D, N, Ne, R, S, T, Q, F> {
        Browser {
            system: self.system,
            transport: self.transport,
            parser,
            submitter: self.submitter,
            cursor: self.cursor,
            kept: self.kept,
        }
    }

    /// The same browser with the submission step `submitter` in place of its own. What it ran
    /// and kept stays.
    pub fn with_submitter<G>(self, submitter: G) -> Browser<A, D, N, Ne, R, S, T, P, G> {
        Browser {
            system: self.system,
            transport: self.transport,
            parser: self.parser,
            submitter,
            cursor: self.cursor,
            kept: self.kept,
        }
    }

    /// The system: its components' views, [`System::published`] and [`System::invocations`].
    pub fn system(&self) -> &System<A, D, N, Ne, R, S> {
        &self.system
    }

    /// The node tree of the document `document_id`, kept since its parser step answered; `None`
    /// for a document that was not parsed here. Read only: nothing changes a kept tree.
    pub fn document_tree(&self, document_id: &DocumentId) -> Option<&Tree> {
        self.kept.get(&document_id.0.0).map(|kept| &kept.tree)
    }

    /// The identity of each element row of the document `document_id`, in position order, with
    /// the element's node in [`Browser::document_tree`]; `None` for a document that was not
    /// parsed here. An element row keeps its identity and its node for the life of the browser.
    pub fn element_nodes(&self, document_id: &DocumentId) -> Option<&[(ElementId, NodeId)]> {
        self.kept
            .get(&document_id.0.0)
            .map(|kept| kept.nodes.as_slice())
    }

    /// What the parser does not yet parse as the HTML Standard says:
    /// [`surf_html::NOT_YET_PARSED`].
    pub fn parser_not_yet_parsed(&self) -> &'static [&'static str] {
        NOT_YET_PARSED
    }

    /// The `agent` component's port. A command issued on it is published by the next run, and
    /// is sent as whoever the caller its ports answer with names.
    pub fn agent_mut(&mut self) -> &mut Agent<A> {
        &mut self.system.agent
    }

    /// The `document` component's port. A command issued on it is published by the next run.
    pub fn document_mut(&mut self) -> &mut Document<D> {
        &mut self.system.document
    }

    /// The `navigator` component's port. A command issued on it is published by the next run.
    pub fn navigator_mut(&mut self) -> &mut Navigator<N> {
        &mut self.system.navigator
    }

    /// The `net` component's port. A command issued on it is published by the next run.
    pub fn net_mut(&mut self) -> &mut Net<Ne> {
        &mut self.system.net
    }

    /// The `renderer` component's port. A command issued on it is published by the next run.
    pub fn renderer_mut(&mut self) -> &mut Renderer<R> {
        &mut self.system.renderer
    }

    /// The `shell` component's port. A command issued on it is published by the next run.
    pub fn shell_mut(&mut self) -> &mut Shell<S> {
        &mut self.system.shell
    }

    /// Drops every event the system has delivered ([`System::take_published`]) and every binding
    /// invocation it recorded ([`System::take_invocations`]), so that a browser that lives across
    /// many loads does not keep each response body for its life. The views are not touched: they
    /// read the components' rows, not the log. Events published and not delivered yet stay, and
    /// the run's cursor moves back by what was taken, so the next [`Browser::run`] still hands
    /// each started request to the transport once.
    pub fn forget_delivered(&mut self) {
        let taken = self.system.take_published().len();
        self.cursor = self.cursor.saturating_sub(taken);
        self.system.take_invocations();
    }

    /// How much of its record the system holds: the events on its log, the binding invocations
    /// it recorded, and the bytes of the request and response bodies among them. Reads only.
    pub fn held(&self) -> Held {
        let published = self.system.published();
        let invocations = self.system.invocations();
        let body_bytes = published.iter().map(event_body).sum::<usize>()
            + invocations.iter().map(invocation_body).sum::<usize>();
        Held {
            events: published.len(),
            invocations: invocations.len(),
            body_bytes,
        }
    }
}

/// What a [`Browser`]'s system holds of its record ([`Browser::held`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Held {
    /// The events on the log, [`System::published`].
    pub events: usize,
    /// The binding invocations recorded, [`System::invocations`].
    pub invocations: usize,
    /// The bytes of the request, form and response bodies those events and invocations carry.
    pub body_bytes: usize,
}

/// The bytes of the body `event` carries, a request's, a form's or a response's, 0 for an event
/// that carries none. Every event is named, so an event a later story adds is sorted here.
fn event_body(event: &SystemEvent) -> usize {
    match event {
        SystemEvent::NavigationStarted(started) => started.body.as_ref().map_or(0, Vec::len),
        SystemEvent::RequestStarted(started) => started.body.as_ref().map_or(0, Vec::len),
        SystemEvent::FormSubmitted(submitted) => submitted.body.as_ref().map_or(0, Vec::len),
        SystemEvent::ResponseReceived(received) => received.body.len(),
        SystemEvent::NavigationCommitted(committed) => committed.body.len(),
        SystemEvent::ParsingStarted(started) => started.body.len(),
        SystemEvent::ActionRecorded(_)
        | SystemEvent::FillRequested(_)
        | SystemEvent::NavigationRequested(_)
        | SystemEvent::PickRequested(_)
        | SystemEvent::PressRequested(_)
        | SystemEvent::SessionClosed(_)
        | SystemEvent::SessionNavigationChanged(_)
        | SystemEvent::SessionOpened(_)
        | SystemEvent::CheckednessSet(_)
        | SystemEvent::DocumentParsed(_)
        | SystemEvent::ElementAdded(_)
        | SystemEvent::FormAdded(_)
        | SystemEvent::OptionAdded(_)
        | SystemEvent::OptionSelected(_)
        | SystemEvent::SubmissionAbandoned(_)
        | SystemEvent::SubmissionStarted(_)
        | SystemEvent::ValueSet(_)
        | SystemEvent::NavigationCancelled(_)
        | SystemEvent::NavigationFailed(_)
        | SystemEvent::FetchFailed(_)
        | SystemEvent::FramePainted(_)
        | SystemEvent::TabClosed(_)
        | SystemEvent::TabOpened(_)
        | SystemEvent::WindowOpened(_) => 0,
    }
}

/// The bytes of the body `invocation` passed, a request's or a response's, 0 for one that
/// passed none. Every binding is named, so a binding a later story adds is sorted here.
fn invocation_body(invocation: &BindingInvocation) -> usize {
    match invocation {
        BindingInvocation::FetchOnNavigationStarted(fetch) => {
            fetch.body.as_ref().map_or(0, Vec::len)
        }
        BindingInvocation::NavigateOnFormSubmitted(navigate)
        | BindingInvocation::NavigateOnNavigationRequested(navigate) => {
            navigate.body.as_ref().map_or(0, Vec::len)
        }
        BindingInvocation::CommitOnResponse(commit) => commit.body.len(),
        BindingInvocation::ParseOnNavigationCommitted(parse) => parse.body.len(),
        BindingInvocation::CheckOnPickRequested(_)
        | BindingInvocation::CloseTabOnSessionClosed(_)
        | BindingInvocation::FailOnFetchFailed(_)
        | BindingInvocation::NoteOnNavigationCancelled(_)
        | BindingInvocation::NoteOnNavigationCommitted(_)
        | BindingInvocation::NoteOnNavigationFailed(_)
        | BindingInvocation::NoteOnNavigationStarted(_)
        | BindingInvocation::SelectOnPickRequested(_)
        | BindingInvocation::SetValueOnFillRequested(_)
        | BindingInvocation::SubmitOnPressRequested(_) => 0,
    }
}

/// A step [`Browser::run`] hands on: a started request to the transport, a started parse to the
/// parser step, a started submission to the submission step.
enum Step {
    Request(RequestStarted),
    Parse(ParsingStarted),
    Submit(SubmissionStarted),
}

impl<A, D, N, Ne, R, S, T, P, F> Browser<A, D, N, Ne, R, S, T, P, F>
where
    A: AgentBehaviors,
    D: DocumentBehaviors,
    N: NavigatorBehaviors,
    Ne: NetBehaviors,
    R: RendererBehaviors,
    S: ShellBehaviors,
    T: Transport,
    P: Parser,
    F: Submitter,
{
    /// Runs the system until no event is undelivered and no request, parse or submission
    /// started since the last run is left unasked: pump, hand every `surf.net.RequestStarted`
    /// published since the last pass whose request is `Pending` to the transport once and issue
    /// its answer on the `net` port, hand every `surf.document.ParsingStarted` published since
    /// the last pass whose document is `Parsing` to the parser step once, and every
    /// `surf.document.SubmissionStarted` whose submission is `Started` to the submission step
    /// once, and issue their answers on the `document` port, in the order they were published,
    /// and pump again.
    ///
    /// `Err` is the first unmet obligation of a pump or of a `net` or `document` command,
    /// returned at once.
    pub fn run(&mut self) -> Result<(), UnmetObligation> {
        loop {
            self.system.pump()?;
            let published = self.system.published();
            let steps: Vec<Step> = published[self.cursor..]
                .iter()
                .filter_map(|event| match event {
                    SystemEvent::RequestStarted(started) => Some(Step::Request(started.clone())),
                    SystemEvent::ParsingStarted(started) => Some(Step::Parse(started.clone())),
                    SystemEvent::SubmissionStarted(started) => Some(Step::Submit(started.clone())),
                    _ => None,
                })
                .collect();
            self.cursor = published.len();
            if steps.is_empty() {
                return Ok(());
            }
            for step in steps {
                match step {
                    Step::Request(request) => self.answer(request)?,
                    Step::Parse(started) => self.parse(started)?,
                    Step::Submit(started) => self.submit(started)?,
                }
            }
        }
    }

    /// Asks the parser step for one started parse, if its document is still `Parsing`, and
    /// issues the answer on the `document` port: one `AddForm` per form in tree order, then one
    /// `AddElement` per element in tree order with its `name` and its form owner's identity,
    /// each select's followed by one `AddOption` per option of its list of options, positions
    /// from 1, then `FinishParsing`. The tree is kept with the node of each element whose row was
    /// added once `FinishParsing` moved the document to `Parsed`. A declared refusal of any of
    /// these commands is the `document` component's outcome and is left as it is: a form whose
    /// row was refused owns no element row.
    fn parse(&mut self, started: ParsingStarted) -> Result<(), UnmetObligation> {
        let parsing = self.system.document.documents()?.iter().any(|row| {
            row.document_id == started.document_id && row.state == DocumentState::Parsing
        });
        if !parsing {
            return Ok(());
        }
        let Parse::Parsed(page) = self.parser.parse(&started) else {
            return Ok(());
        };
        let Page {
            tree,
            title,
            elements,
            encoding,
            unsupported_encoding,
            stopped_by,
            forms,
        } = *page;
        let mut form_ids: Vec<Option<FormId>> = Vec::with_capacity(forms.forms.len());
        for (position, form) in (1..).zip(&forms.forms) {
            let outcome = self.system.document.add_form(AddForm {
                document_id: started.document_id.clone(),
                position,
                action: form.action.clone(),
                method: form.method.clone(),
                enctype: form.enctype.clone(),
            })?;
            form_ids.push(match outcome {
                AddFormOutcome::Added { form_added } => Some(form_added.form_id),
                _ => None,
            });
        }
        let mut nodes = Vec::with_capacity(elements.len());
        for (position, element) in (1..).zip(elements) {
            let control = forms.control(element.node);
            let outcome = self.system.document.add_element(AddElement {
                document_id: started.document_id.clone(),
                position,
                kind: element_kind(element.kind),
                label: element.label,
                target: element.target,
                value: element.value,
                hidden: element.hidden,
                disabled: element.disabled,
                checked: element.checked,
                required: element.required,
                readonly: control.is_some_and(|control| control.readonly),
                name: control.and_then(|control| control.name.clone()),
                form_id: control
                    .and_then(|control| control.owner)
                    .and_then(|owner| form_ids[owner].clone()),
            })?;
            let AddElementOutcome::Added { element_added } = outcome else {
                continue;
            };
            for (position, option) in (1..).zip(control.map_or(&[][..], |c| &c.options[..])) {
                self.system.document.add_option(AddOption {
                    select_id: element_added.element_id.clone(),
                    position,
                    value: option.value.clone(),
                    label: option.label.clone(),
                    selected: option.selected,
                    disabled: option.disabled || element.disabled,
                    hidden: element.hidden,
                })?;
            }
            nodes.push((element_added.element_id, element.node));
        }
        let outcome = self.system.document.finish_parsing(FinishParsing {
            document_id: started.document_id.clone(),
            title,
            encoding: EncodingName(encoding),
            unsupported_encoding: unsupported_encoding.map(EncodingName),
            stopped_by: stopped_by.map(parse_limit),
        })?;
        if let FinishParsingOutcome::Parsed { .. } = outcome {
            self.kept
                .insert(started.document_id.0.0, Kept { tree, nodes, forms });
        }
        Ok(())
    }

    /// Hands one started submission to the submission step, if it is still `Started` and its
    /// submitter is an element row of a document parsed here, with that document's address,
    /// tree, forms and the state of its control rows, and issues the answer on the `document`
    /// port: `SubmitForm` with the tab of the submitter's document, or `AbandonSubmission`. A
    /// submission whose submitter is not of a kept document is left `Started`. A declared refusal
    /// of either command is the `document` component's outcome and is left as it is.
    fn submit(&mut self, started: SubmissionStarted) -> Result<(), UnmetObligation> {
        let open = self.system.document.submissions()?.iter().any(|row| {
            row.submission_id == started.submission_id && row.state == SubmissionState::Started
        });
        if !open {
            return Ok(());
        }
        let Some((key, submitter)) = self.kept.iter().find_map(|(key, kept)| {
            kept.nodes
                .iter()
                .find(|(element_id, _)| *element_id == started.submitter)
                .map(|&(_, node)| (key.clone(), node))
        }) else {
            return Ok(());
        };
        let Some(document) = self
            .system
            .document
            .documents()?
            .into_iter()
            .find(|row| row.document_id.0.0 == key)
        else {
            return Ok(());
        };
        let rows = self
            .system
            .document
            .elements(document.document_id.clone())?;
        let mut selected: BTreeMap<String, Vec<(i64, String)>> = BTreeMap::new();
        for option in self.system.document.options()? {
            if option.selected && !option.disabled {
                selected
                    .entry(option.select_id.0.0)
                    .or_default()
                    .push((option.position, option.value));
            }
        }
        let kept = &self.kept[&key];
        let node_of: BTreeMap<&str, NodeId> = kept
            .nodes
            .iter()
            .map(|(element_id, node)| (element_id.0.0.as_str(), *node))
            .collect();
        let mut filled = BTreeMap::new();
        for row in rows {
            let Some(&node) = node_of.get(row.element_id.0.0.as_str()) else {
                continue;
            };
            let mut values = selected.remove(&row.element_id.0.0).unwrap_or_default();
            values.sort();
            filled.insert(
                node,
                Filled {
                    value: row.value,
                    checked: row.checked,
                    required: row.required,
                    selected: values.into_iter().map(|(_, value)| value).collect(),
                },
            );
        }
        let answer = self.submitter.submit(&Submission {
            started: &started,
            address: &document.url.0,
            tree: &kept.tree,
            forms: &kept.forms,
            submitter,
            filled: &filled,
        });
        match answer {
            Submit::Navigate {
                url,
                method,
                body,
                body_type,
            } => {
                self.system.document.submit_form(SubmitForm {
                    submission_id: started.submission_id,
                    tab_id: document.tab_id,
                    url: Url(url),
                    method,
                    body,
                    body_type: body_type.map(MediaType),
                })?;
            }
            Submit::Abandon(reason) => {
                self.system.document.abandon_submission(AbandonSubmission {
                    submission_id: started.submission_id,
                    reason,
                })?;
            }
            Submit::Unanswered => {}
        }
        Ok(())
    }

    /// Asks the transport for one started request, if it is still `Pending`, and issues the
    /// answer on the `net` port. A declared refusal of that command is the `net` component's
    /// outcome and is left as it is.
    fn answer(&mut self, started: RequestStarted) -> Result<(), UnmetObligation> {
        let pending =
            self.system.net.requests()?.iter().any(|row| {
                row.request_id == started.request_id && row.state == RequestState::Pending
            });
        if !pending {
            return Ok(());
        }
        match self.transport.fetch(&started) {
            Answer::Response {
                url,
                status,
                media_type,
                body,
            } => {
                self.system.net.deliver_response(DeliverResponse {
                    request_id: started.request_id,
                    url: Url(url),
                    status: Status(i64::from(status)),
                    media_type: MediaType(media_type),
                    body,
                })?;
            }
            Answer::Failure(reason) => {
                self.system.net.fail_request(FailRequest {
                    request_id: started.request_id,
                    reason,
                })?;
            }
            Answer::Unanswered => {}
        }
        Ok(())
    }

    /// Issues `OpenWindow`, `OpenTab` in that window and `Navigate` that tab to `address` with
    /// [`Method::Get`] and no body, pumping after the first two, and returns the navigation's
    /// identity. The navigation is published by the next [`Browser::run`].
    pub fn start(&mut self, address: &str) -> Result<RequestId, Error> {
        let window_id = match self.system.shell.open_window(OpenWindow {})? {
            OpenWindowOutcome::Opened { window_opened } => window_opened.window_id,
        };
        self.system.pump()?;
        let tab_id = match self.system.shell.open_tab(OpenTab { window_id })? {
            OpenTabOutcome::Opened { tab_opened } => tab_opened.tab_id,
            OpenTabOutcome::NoSuchWindow { error } => {
                return Err(Error::Refused(format!(
                    "surf.shell.OpenTab refused: no window {}",
                    error.window_id.0.0
                )));
            }
        };
        self.system.pump()?;
        let navigation_id = match self.system.navigator.navigate(Navigate {
            tab_id,
            url: Url(address.to_owned()),
            method: Method::Get,
            body: None,
            body_type: None,
        })? {
            NavigateOutcome::Started { navigation_started } => navigation_started.navigation_id,
        };
        Ok(navigation_id)
    }

    /// The rows of the `Navigations`, `Requests` and `Documents` views, the `Elements` of each
    /// document and every published `FetchFailed`, for the navigation `navigation_id`.
    pub fn report(&self, navigation_id: RequestId) -> Result<Report, UnmetObligation> {
        let documents = self.system.document.documents()?;
        let mut elements = Vec::new();
        for document in &documents {
            elements.extend(
                self.system
                    .document
                    .elements(document.document_id.clone())?,
            );
        }
        Ok(Report {
            navigation_id,
            navigations: self.system.navigator.navigations()?,
            requests: self.system.net.requests()?,
            documents,
            elements,
            not_yet_parsed: self.parser_not_yet_parsed(),
            fetch_failures: self
                .system
                .published()
                .iter()
                .filter_map(|event| match event {
                    SystemEvent::FetchFailed(failed) => Some(failed.clone()),
                    _ => None,
                })
                .collect(),
        })
    }

    /// [`Browser::start`] at `address`, [`Browser::run`], and the [`Browser::report`] of the
    /// navigation it started.
    pub fn load(&mut self, address: &str) -> Result<Report, Error> {
        let navigation_id = self.start(address)?;
        self.run()?;
        Ok(self.report(navigation_id)?)
    }
}
