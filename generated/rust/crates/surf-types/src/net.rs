// generated from surf v1
// contract digest 8248095f602196a66470973ac0d75774fbdde611c3d1aa5b65ede54d807a7673
// do not edit: regenerate with `ess synthesize`

//! Network — `surf.net`.
//!
//! Fetching the resource an address names.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// FailureReason — `surf.net.FailureReason`: one of a closed set of names.
///
/// Why a request ended without a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureReason {
    /// `Unreachable`.
    Unreachable,
    /// `UnsupportedScheme`.
    UnsupportedScheme,
    /// `MalformedResponse`.
    MalformedResponse,
    /// `TooManyRedirects`.
    TooManyRedirects,
}

/// MediaType — `surf.net.MediaType`: a distinct wrapper around `String`.
///
/// What a response body is: the type and subtype of its Content-Type header in lower case, without parameters, such as text/html.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaType(pub String);

/// Method — `surf.net.Method`: one of a closed set of names.
///
/// The request method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `Get`.
    Get,
    /// `Post`.
    Post,
}

/// The states of `surf.net.Request`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Request<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestState {
    /// `Failed`.
    Failed,
    /// `Pending`.
    Pending,
    /// `Responded`.
    Responded,
}

/// RequestId — `surf.net.RequestId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(pub crate::primitives::Uuid);

/// Status — `surf.net.Status`: a distinct wrapper around `Integer`.
///
/// The status code of a response, such as 200 or 404.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status(pub i64);

/// What Request — `surf.net.Request` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Request<S>`], and at a boundary by [`RequestSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestData {
    /// The identity: `request_id` — `surf.net.RequestId`.
    pub request_id: RequestId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `method` — `surf.net.Method`.
    pub method: Method,
    /// `body_type` — `Optional<surf.net.MediaType>`.
    pub body_type: Option<MediaType>,
}

/// The states of `surf.net.Request`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](request_state::Marker), so [`Request<S>`](Request) can only ever rest in a real state.
pub mod request_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Failed {}
        impl Sealed for super::Pending {}
        impl Sealed for super::Responded {}
    }

    /// A declared state of `Request`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::RequestState;
    }

    /// `Failed`. Terminal: an instance may rest here forever.
    pub struct Failed;

    impl Marker for Failed {
        const STATE: super::RequestState = super::RequestState::Failed;
    }

    /// `Pending`. Where a new instance starts.
    pub struct Pending;

    impl Marker for Pending {
        const STATE: super::RequestState = super::RequestState::Pending;
    }

    /// `Responded`. Terminal: an instance may rest here forever.
    pub struct Responded;

    impl Marker for Responded {
        const STATE: super::RequestState = super::RequestState::Responded;
    }
}

/// Request — `surf.net.Request` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Pending`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`RequestSnapshot`]
/// and [`RequestSnapshot::refine`].
pub struct Request<S: request_state::Marker> {
    data: RequestData,
    state: core::marker::PhantomData<S>,
}

impl<S: request_state::Marker> Request<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> RequestState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &RequestData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> RequestData {
        self.data
    }
}

impl Request<request_state::Pending> {
    /// A new instance, resting in `Pending` — the only state the lifecycle starts one in.
    pub fn new(data: RequestData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Request<request_state::Pending> {
    /// `respond` — `Pending` → `Responded`. Taken by the `responded` outcome of `surf.net.DeliverResponse`.
    pub fn respond(self) -> Request<request_state::Responded> {
        Request {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `fail` — `Pending` → `Failed`. Taken by the `failed` outcome of `surf.net.FailRequest`.
    pub fn fail(self) -> Request<request_state::Failed> {
        Request {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.net.Request` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`RequestSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: RequestState,
    /// What it holds.
    pub data: RequestData,
}

/// An `Request` in whichever declared state it was found.
pub enum AnyRequest {
    /// Resting in `Failed`.
    Failed(Request<request_state::Failed>),
    /// Resting in `Pending`.
    Pending(Request<request_state::Pending>),
    /// Resting in `Responded`.
    Responded(Request<request_state::Responded>),
}

impl RequestSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `RequestState` cannot spell one.
    pub fn refine(self) -> AnyRequest {
        match self.state {
            RequestState::Failed => AnyRequest::Failed(Request {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            RequestState::Pending => AnyRequest::Pending(Request {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            RequestState::Responded => AnyRequest::Responded(Request {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyRequest {
    /// The state, as the runtime value.
    pub fn state(&self) -> RequestState {
        match self {
            Self::Failed(_) => RequestState::Failed,
            Self::Pending(_) => RequestState::Pending,
            Self::Responded(_) => RequestState::Responded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> RequestSnapshot {
        match self {
            Self::Failed(instance) => RequestSnapshot {
                state: RequestState::Failed,
                data: instance.into_data(),
            },
            Self::Pending(instance) => RequestSnapshot {
                state: RequestState::Pending,
                data: instance.into_data(),
            },
            Self::Responded(instance) => RequestSnapshot {
                state: RequestState::Responded,
                data: instance.into_data(),
            },
        }
    }
}

/// Deliver a response — the input of `surf.net.DeliverResponse`.
///
/// Record the response to a request and publish it.
///
/// Everything it can result in is [`DeliverResponseOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliverResponse {
    /// `request_id` — `surf.net.RequestId`.
    pub request_id: RequestId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `status` — `surf.net.Status`.
    pub status: Status,
    /// `media_type` — `surf.net.MediaType`.
    pub media_type: MediaType,
    /// `body` — `Bytes`.
    pub body: Vec<u8>,
}

/// Everything `surf.net.DeliverResponse` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliverResponseOutcome {
    /// `no-such-request` — for an identity no record carries.
    NoSuchRequest {
        /// Why it was refused: `surf.net.RequestNotFound`.
        error: RequestNotFound,
    },
    /// `responded` — otherwise.
    ///
    /// The request is Responded.
    Responded {
        /// The `surf.net.ResponseReceived` this outcome publishes.
        response_received: ResponseReceived,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The request had already ended.
    WrongState {
        /// Why it was refused: `surf.net.RequestStateConflict`.
        error: RequestStateConflict,
    },
}

/// Fail a request — the input of `surf.net.FailRequest`.
///
/// Record that a request ended without a response and publish that.
///
/// Everything it can result in is [`FailRequestOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailRequest {
    /// `request_id` — `surf.net.RequestId`.
    pub request_id: RequestId,
    /// `reason` — `surf.net.FailureReason`.
    pub reason: FailureReason,
}

/// Everything `surf.net.FailRequest` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailRequestOutcome {
    /// `no-such-request` — for an identity no record carries.
    NoSuchRequest {
        /// Why it was refused: `surf.net.RequestNotFound`.
        error: RequestNotFound,
    },
    /// `failed` — otherwise.
    ///
    /// The request is Failed.
    Failed {
        /// The `surf.net.FetchFailed` this outcome publishes.
        fetch_failed: FetchFailed,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The request had already ended.
    WrongState {
        /// Why it was refused: `surf.net.RequestStateConflict`.
        error: RequestStateConflict,
    },
}

/// Fetch — the input of `surf.net.Fetch`.
///
/// Start fetching the resource at an address.
///
/// Everything it can result in is [`FetchOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetch {
    /// `request_id` — `surf.net.RequestId`.
    pub request_id: RequestId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `method` — `surf.net.Method`.
    pub method: Method,
    /// `body` — `Optional<Bytes>`.
    pub body: Option<Vec<u8>>,
    /// `body_type` — `Optional<surf.net.MediaType>`.
    pub body_type: Option<MediaType>,
}

/// Everything `surf.net.Fetch` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchOutcome {
    /// `started` — otherwise.
    ///
    /// The request is Pending.
    Started {
        /// The `surf.net.RequestStarted` this outcome publishes.
        request_started: RequestStarted,
    },
}

/// FetchFailed — the event `surf.net.FetchFailed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchFailed {
    /// `request_id` — `surf.net.RequestId`.
    pub request_id: RequestId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `reason` — `surf.net.FailureReason`.
    pub reason: FailureReason,
}

/// RequestStarted — the event `surf.net.RequestStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestStarted {
    /// `request_id` — `surf.net.RequestId`.
    pub request_id: RequestId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `method` — `surf.net.Method`.
    pub method: Method,
    /// `body` — `Optional<Bytes>`.
    pub body: Option<Vec<u8>>,
    /// `body_type` — `Optional<surf.net.MediaType>`.
    pub body_type: Option<MediaType>,
}

/// ResponseReceived — the event `surf.net.ResponseReceived`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseReceived {
    /// `request_id` — `surf.net.RequestId`.
    pub request_id: RequestId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `status` — `surf.net.Status`.
    pub status: Status,
    /// `media_type` — `surf.net.MediaType`.
    pub media_type: MediaType,
    /// `body` — `Bytes`.
    pub body: Vec<u8>,
}

/// The declared error `surf.net.RequestNotFound`.
///
/// No request has this identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestNotFound {
    /// `request_id` — `surf.net.RequestId`.
    pub request_id: RequestId,
}

/// The declared error `surf.net.RequestStateConflict`.
///
/// The request already ended, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestStateConflict {
    /// `state` — `surf.net.Request.State`.
    pub state: RequestState,
}

/// Requests — one row of the view `surf.net.Requests`.
///
/// Projects `surf.net.Request` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requests {
    /// `request_id` — `surf.net.RequestId`.
    pub request_id: RequestId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `method` — `surf.net.Method`.
    pub method: Method,
    /// `body_type` — `Optional<surf.net.MediaType>`.
    pub body_type: Option<MediaType>,
    /// `state` — `surf.net.Request.State`.
    pub state: RequestState,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `surf.net.DeliverResponse` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait DeliverResponseBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.net.DeliverResponse`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn deliver_response(&mut self, input: super::DeliverResponse) -> Result<super::DeliverResponseOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.net.FailRequest` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait FailRequestBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.net.FailRequest`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn fail_request(&mut self, input: super::FailRequest) -> Result<super::FailRequestOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.net.Fetch` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait FetchBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.net.Fetch`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn fetch(&mut self, input: super::Fetch) -> Result<super::FetchOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.net.Requests` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait RequestsQuery {
        /// Serves `surf.net.Requests` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn requests(&self) -> Result<Vec<super::Requests>, crate::obligation::UnmetObligation>;
    }

}
