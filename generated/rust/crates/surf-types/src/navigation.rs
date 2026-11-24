// generated from surf v1
// contract digest 8a6a9f1bd1962c0655a4273e57621bd9c11144463771cf0d9887841440044e63
// do not edit: regenerate with `ess synthesize`

//! Navigation — `surf.navigation`.
//!
//! Taking a tab from one address to another, from the request to the committed page.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// HistoryHandling — `surf.navigation.HistoryHandling`: one of a closed set of names.
///
/// What a navigation's commit does to its tab's session history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryHandling {
    /// `push`.
    Push,
    /// `traverse`.
    Traverse,
}

/// The states of `surf.navigation.Navigation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Navigation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationState {
    /// `Cancelled`.
    Cancelled,
    /// `Committed`.
    Committed,
    /// `Failed`.
    Failed,
    /// `Started`.
    Started,
}

/// Standing — `surf.navigation.Standing`: one of a closed set of names.
///
/// Where a session history entry stands from its tab's current entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// `earlier`.
    Earlier,
    /// `back`.
    Back,
    /// `current`.
    Current,
    /// `forward`.
    Forward,
    /// `later`.
    Later,
    /// `dropped`.
    Dropped,
}

/// What Navigation — `surf.navigation.Navigation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Navigation<S>`], and at a boundary by [`NavigationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationData {
    /// The identity: `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `tab_id` — `surf.shell.TabId`.
    ///
    /// Carries `tab`: `surf.navigation.Navigation` references one `surf.shell.Tab`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `method` — `surf.net.Method`.
    pub method: crate::net::Method,
    /// `handling` — `surf.navigation.HistoryHandling`.
    pub handling: HistoryHandling,
    /// `standing` — `Optional<surf.navigation.Standing>`.
    pub standing: Option<Standing>,
    /// `next` — `Optional<surf.net.RequestId>`.
    ///
    /// Carries `next`: `surf.navigation.Navigation` references one `surf.navigation.Navigation`.
    pub next: Option<crate::net::RequestId>,
    /// `entry_id` — `Optional<surf.net.RequestId>`.
    ///
    /// Carries `entry`: `surf.navigation.Navigation` references one `surf.navigation.Navigation`.
    pub entry_id: Option<crate::net::RequestId>,
    /// `entry_next` — `Optional<surf.net.RequestId>`.
    ///
    /// Carries `entry_next`: `surf.navigation.Navigation` references one `surf.navigation.Navigation`.
    pub entry_next: Option<crate::net::RequestId>,
}

/// The states of `surf.navigation.Navigation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](navigation_state::Marker), so [`Navigation<S>`](Navigation) can only ever rest in a real state.
pub mod navigation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Cancelled {}
        impl Sealed for super::Committed {}
        impl Sealed for super::Failed {}
        impl Sealed for super::Started {}
    }

    /// A declared state of `Navigation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::NavigationState;
    }

    /// `Cancelled`. Terminal: an instance may rest here forever.
    pub struct Cancelled;

    impl Marker for Cancelled {
        const STATE: super::NavigationState = super::NavigationState::Cancelled;
    }

    /// `Committed`. Terminal: an instance may rest here forever.
    pub struct Committed;

    impl Marker for Committed {
        const STATE: super::NavigationState = super::NavigationState::Committed;
    }

    /// `Failed`. Terminal: an instance may rest here forever.
    pub struct Failed;

    impl Marker for Failed {
        const STATE: super::NavigationState = super::NavigationState::Failed;
    }

    /// `Started`. Where a new instance starts.
    pub struct Started;

    impl Marker for Started {
        const STATE: super::NavigationState = super::NavigationState::Started;
    }
}

/// Navigation — `surf.navigation.Navigation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Started`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`NavigationSnapshot`]
/// and [`NavigationSnapshot::refine`].
pub struct Navigation<S: navigation_state::Marker> {
    data: NavigationData,
    state: core::marker::PhantomData<S>,
}

impl<S: navigation_state::Marker> Navigation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> NavigationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &NavigationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> NavigationData {
        self.data
    }
}

impl Navigation<navigation_state::Started> {
    /// A new instance, resting in `Started` — the only state the lifecycle starts one in.
    pub fn new(data: NavigationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Navigation<navigation_state::Started> {
    /// `commit` — `Started` → `Committed`. Taken by the `traversed` outcome of `surf.navigation.CommitNavigation`, the `committed` outcome of `surf.navigation.CommitNavigation`.
    pub fn commit(self) -> Navigation<navigation_state::Committed> {
        Navigation {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `fail` — `Started` → `Failed`. Taken by the `failed` outcome of `surf.navigation.FailNavigation`.
    pub fn fail(self) -> Navigation<navigation_state::Failed> {
        Navigation {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `cancel` — `Started` → `Cancelled`. Taken by the `cancelled` outcome of `surf.navigation.CancelNavigation`.
    pub fn cancel(self) -> Navigation<navigation_state::Cancelled> {
        Navigation {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.navigation.Navigation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`NavigationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: NavigationState,
    /// What it holds.
    pub data: NavigationData,
}

/// An `Navigation` in whichever declared state it was found.
pub enum AnyNavigation {
    /// Resting in `Cancelled`.
    Cancelled(Navigation<navigation_state::Cancelled>),
    /// Resting in `Committed`.
    Committed(Navigation<navigation_state::Committed>),
    /// Resting in `Failed`.
    Failed(Navigation<navigation_state::Failed>),
    /// Resting in `Started`.
    Started(Navigation<navigation_state::Started>),
}

impl NavigationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `NavigationState` cannot spell one.
    pub fn refine(self) -> AnyNavigation {
        match self.state {
            NavigationState::Cancelled => AnyNavigation::Cancelled(Navigation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            NavigationState::Committed => AnyNavigation::Committed(Navigation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            NavigationState::Failed => AnyNavigation::Failed(Navigation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            NavigationState::Started => AnyNavigation::Started(Navigation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyNavigation {
    /// The state, as the runtime value.
    pub fn state(&self) -> NavigationState {
        match self {
            Self::Cancelled(_) => NavigationState::Cancelled,
            Self::Committed(_) => NavigationState::Committed,
            Self::Failed(_) => NavigationState::Failed,
            Self::Started(_) => NavigationState::Started,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> NavigationSnapshot {
        match self {
            Self::Cancelled(instance) => NavigationSnapshot {
                state: NavigationState::Cancelled,
                data: instance.into_data(),
            },
            Self::Committed(instance) => NavigationSnapshot {
                state: NavigationState::Committed,
                data: instance.into_data(),
            },
            Self::Failed(instance) => NavigationSnapshot {
                state: NavigationState::Failed,
                data: instance.into_data(),
            },
            Self::Started(instance) => NavigationSnapshot {
                state: NavigationState::Started,
                data: instance.into_data(),
            },
        }
    }
}

/// Cancel a navigation — the input of `surf.navigation.CancelNavigation`.
///
/// Stop a navigation before a response arrives.
///
/// Everything it can result in is [`CancelNavigationOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelNavigation {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
}

/// Everything `surf.navigation.CancelNavigation` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelNavigationOutcome {
    /// `no-such-navigation` — for an identity no record carries.
    NoSuchNavigation {
        /// Why it was refused: `surf.navigation.NavigationNotFound`.
        error: NavigationNotFound,
    },
    /// `cancelled` — otherwise.
    ///
    /// The navigation is Cancelled.
    Cancelled {
        /// The `surf.navigation.NavigationCancelled` this outcome publishes.
        navigation_cancelled: NavigationCancelled,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The navigation had already ended.
    WrongState {
        /// Why it was refused: `surf.navigation.NavigationStateConflict`.
        error: NavigationStateConflict,
    },
}

/// Commit a navigation — the input of `surf.navigation.CommitNavigation`.
///
/// Record that a response arrived for the navigation, and hand it on.
///
/// Everything it can result in is [`CommitNavigationOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitNavigation {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `method` — `surf.net.Method`.
    pub method: crate::net::Method,
    /// `status` — `surf.net.Status`.
    pub status: crate::net::Status,
    /// `media_type` — `surf.net.MediaType`.
    pub media_type: crate::net::MediaType,
    /// `body` — `Bytes`.
    pub body: Vec<u8>,
}

/// Everything `surf.navigation.CommitNavigation` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitNavigationOutcome {
    /// `no-such-navigation` — for an identity no record carries.
    NoSuchNavigation {
        /// Why it was refused: `surf.navigation.NavigationNotFound`.
        error: NavigationNotFound,
    },
    /// `traversed` — when the existing subject's stored fields satisfy `handling == traverse`.
    ///
    /// The navigation is Committed and the entry it went to is current.
    Traversed {
        /// The `surf.navigation.NavigationCommitted` this outcome publishes.
        navigation_committed: NavigationCommitted,
    },
    /// `committed` — otherwise.
    ///
    /// The navigation is Committed and is its tab's current entry, after the one that was current; the entries after that one are dropped.
    Committed {
        /// The `surf.navigation.NavigationCommitted` this outcome publishes.
        navigation_committed: NavigationCommitted,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The navigation had already ended; a late response changes nothing.
    WrongState {
        /// Why it was refused: `surf.navigation.NavigationStateConflict`.
        error: NavigationStateConflict,
    },
}

/// Fail a navigation — the input of `surf.navigation.FailNavigation`.
///
/// Record that no response arrived for the navigation.
///
/// Everything it can result in is [`FailNavigationOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailNavigation {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `reason` — `surf.net.FailureReason`.
    pub reason: crate::net::FailureReason,
}

/// Everything `surf.navigation.FailNavigation` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailNavigationOutcome {
    /// `no-such-navigation` — for an identity no record carries.
    NoSuchNavigation {
        /// Why it was refused: `surf.navigation.NavigationNotFound`.
        error: NavigationNotFound,
    },
    /// `failed` — otherwise.
    ///
    /// The navigation is Failed.
    Failed {
        /// The `surf.navigation.NavigationFailed` this outcome publishes.
        navigation_failed: NavigationFailed,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The navigation had already ended; a late failure changes nothing.
    WrongState {
        /// Why it was refused: `surf.navigation.NavigationStateConflict`.
        error: NavigationStateConflict,
    },
}

/// Navigate — the input of `surf.navigation.Navigate`.
///
/// Start loading an address into a tab.
///
/// Everything it can result in is [`NavigateOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Navigate {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `method` — `surf.net.Method`.
    pub method: crate::net::Method,
    /// `body` — `Optional<Bytes>`.
    pub body: Option<Vec<u8>>,
    /// `body_type` — `Optional<surf.net.MediaType>`.
    pub body_type: Option<crate::net::MediaType>,
    /// `initiator` — `Optional<String>`.
    pub initiator: Option<String>,
}

/// Everything `surf.navigation.Navigate` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigateOutcome {
    /// `started` — otherwise.
    ///
    /// The navigation is Started and its address is to be fetched.
    Started {
        /// The `surf.navigation.NavigationStarted` this outcome publishes.
        navigation_started: NavigationStarted,
    },
}

/// Start a traversal — the input of `surf.navigation.StartTraversal`.
///
/// Start loading a session history entry's address into its tab.
///
/// Everything it can result in is [`StartTraversalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartTraversal {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `entry_id` — `surf.net.RequestId`.
    pub entry_id: crate::net::RequestId,
    /// `entry_next` — `Optional<surf.net.RequestId>`.
    pub entry_next: Option<crate::net::RequestId>,
}

/// Everything `surf.navigation.StartTraversal` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartTraversalOutcome {
    /// `started` — otherwise.
    ///
    /// The traversal's navigation is Started and the entry's address is to be fetched.
    Started {
        /// The `surf.navigation.NavigationStarted` this outcome publishes.
        navigation_started: NavigationStarted,
    },
}

/// Traverse the history — the input of `surf.navigation.TraverseHistory`.
///
/// Ask to load the entry before or after a tab's current one.
///
/// Everything it can result in is [`TraverseHistoryOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraverseHistory {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `step` — `surf.navigation.Standing`.
    pub step: Standing,
}

/// Everything `surf.navigation.TraverseHistory` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraverseHistoryOutcome {
    /// `unsupported` — when `(step != back and step != forward)`.
    ///
    /// Only one step back or forward is traversed.
    Unsupported {
        /// Why it was refused: `surf.navigation.TraversalUnsupported`.
        error: TraversalUnsupported,
    },
    /// `no-entry` — when, of the `surf.navigation.Navigation` rows satisfying `(tab_id == input.tab_id and defined(standing) and standing == input.step and method == Get)`, no row is selected.
    ///
    /// No entry made by GET is there.
    NoEntry {
        /// Why it was refused: `surf.navigation.NoHistoryEntry`.
        error: NoHistoryEntry,
    },
    /// `requested` — otherwise.
    ///
    /// The tab is to load the entry's address.
    Requested {
        /// The `surf.navigation.TraversalStarted` this outcome publishes.
        traversal_started: TraversalStarted,
    },
}

/// NavigationCancelled — the event `surf.navigation.NavigationCancelled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationCancelled {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
}

/// NavigationCommitted — the event `surf.navigation.NavigationCommitted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationCommitted {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `status` — `surf.net.Status`.
    pub status: crate::net::Status,
    /// `media_type` — `surf.net.MediaType`.
    pub media_type: crate::net::MediaType,
    /// `body` — `Bytes`.
    pub body: Vec<u8>,
}

/// NavigationFailed — the event `surf.navigation.NavigationFailed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationFailed {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `reason` — `surf.net.FailureReason`.
    pub reason: crate::net::FailureReason,
}

/// NavigationStarted — the event `surf.navigation.NavigationStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationStarted {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `method` — `surf.net.Method`.
    pub method: crate::net::Method,
    /// `body` — `Optional<Bytes>`.
    pub body: Option<Vec<u8>>,
    /// `body_type` — `Optional<surf.net.MediaType>`.
    pub body_type: Option<crate::net::MediaType>,
    /// `initiator` — `Optional<String>`.
    pub initiator: Option<String>,
}

/// TraversalStarted — the event `surf.navigation.TraversalStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalStarted {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `entry_id` — `surf.net.RequestId`.
    pub entry_id: crate::net::RequestId,
    /// `entry_next` — `Optional<surf.net.RequestId>`.
    pub entry_next: Option<crate::net::RequestId>,
}

/// The declared error `surf.navigation.NavigationNotFound`.
///
/// No navigation has this identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationNotFound {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
}

/// The declared error `surf.navigation.NavigationStateConflict`.
///
/// The navigation already ended, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationStateConflict {
    /// `state` — `surf.navigation.Navigation.State`.
    pub state: NavigationState,
}

/// The declared error `surf.navigation.NoHistoryEntry`.
///
/// The tab has no entry at that step from its current one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoHistoryEntry {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `step` — `surf.navigation.Standing`.
    pub step: Standing,
}

/// The declared error `surf.navigation.TraversalUnsupported`.
///
/// Only one step back or one step forward is traversed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalUnsupported {
    /// `step` — `surf.navigation.Standing`.
    pub step: Standing,
}

/// Navigations — one row of the view `surf.navigation.Navigations`.
///
/// Projects `surf.navigation.Navigation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Navigations {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `state` — `surf.navigation.Navigation.State`.
    pub state: NavigationState,
    /// `method` — `surf.net.Method`.
    pub method: crate::net::Method,
    /// `handling` — `surf.navigation.HistoryHandling`.
    pub handling: HistoryHandling,
    /// `standing` — `Optional<surf.navigation.Standing>`.
    pub standing: Option<Standing>,
    /// `next` — `Optional<surf.net.RequestId>`.
    pub next: Option<crate::net::RequestId>,
    /// `entry_id` — `Optional<surf.net.RequestId>`.
    pub entry_id: Option<crate::net::RequestId>,
    /// `entry_next` — `Optional<surf.net.RequestId>`.
    pub entry_next: Option<crate::net::RequestId>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `surf.navigation.CancelNavigation` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait CancelNavigationBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.navigation.CancelNavigation`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn cancel_navigation(&mut self, input: super::CancelNavigation) -> Result<super::CancelNavigationOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.navigation.CommitNavigation` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by effects on filtered rows (`affects:`), in `traversed`.
    ///
    /// Contract: given `surf.navigation.CommitNavigation` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `no-such-navigation` for an identity no record carries, error `surf.navigation.NavigationNotFound`; `traversed` when the existing subject's stored fields satisfy `handling == traverse`, takes `commit` of `surf.navigation.Navigation`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and standing == back)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and standing == forward)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and standing == current)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and next == subject.entry_id)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and navigation_id == subject.entry_next)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and navigation_id == subject.entry_id)`, setting `standing`, emits `surf.navigation.NavigationCommitted`; `committed` otherwise, takes `commit` of `surf.navigation.Navigation`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and (standing == forward or standing == later))`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and standing == back)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and standing == current)`, setting `standing`, `next`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and handling == traverse and state == Started)`, moving it along `cancel` (a row resting outside its `from` states is left as it is), emits `surf.navigation.NavigationCommitted`; `wrong-state` from a state no declared move starts in, error `surf.navigation.NavigationStateConflict`.
    pub trait CommitNavigationBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.navigation.CommitNavigation`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn commit_navigation(&mut self, input: super::CommitNavigation) -> Result<super::CommitNavigationOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.navigation.FailNavigation` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait FailNavigationBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.navigation.FailNavigation`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn fail_navigation(&mut self, input: super::FailNavigation) -> Result<super::FailNavigationOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.navigation.Navigate` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait NavigateBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.navigation.Navigate`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn navigate(&mut self, input: super::Navigate) -> Result<super::NavigateOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.navigation.StartTraversal` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait StartTraversalBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.navigation.StartTraversal`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn start_traversal(&mut self, input: super::StartTraversal) -> Result<super::StartTraversalOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.navigation.TraverseHistory` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a guard over the rows a selector selects (`when_related: {entity, where, exists | count | forall}`), in `no-entry`.
    ///
    /// Contract: given `surf.navigation.TraverseHistory` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `unsupported` when `(step != back and step != forward)`, error `surf.navigation.TraversalUnsupported`; `no-entry` when, of the `surf.navigation.Navigation` rows satisfying `(tab_id == input.tab_id and defined(standing) and standing == input.step and method == Get)`, no row is selected, error `surf.navigation.NoHistoryEntry`; `requested` otherwise, emits `surf.navigation.TraversalStarted`.
    pub trait TraverseHistoryBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.navigation.TraverseHistory`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn traverse_history(&mut self, input: super::TraverseHistory) -> Result<super::TraverseHistoryOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.navigation.Navigations` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait NavigationsQuery {
        /// Serves `surf.navigation.Navigations` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn navigations(&self) -> Result<Vec<super::Navigations>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl CommitNavigationBehavior for Unimplemented {
        fn commit_navigation(&mut self, _input: super::CommitNavigation) -> Result<super::CommitNavigationOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "surf.navigation.CommitNavigation" })
        }
    }

    impl TraverseHistoryBehavior for Unimplemented {
        fn traverse_history(&mut self, _input: super::TraverseHistory) -> Result<super::TraverseHistoryOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "surf.navigation.TraverseHistory" })
        }
    }
}
