//! In-memory ports for the generated `surf` components, one store per component.
//!
//! The generated crate `surf-types` implements every command and view of the specification over
//! ports the implementor supplies: one storage trait per entity, and `TryContext` for the
//! identities the model leaves to the implementation. This crate supplies them, in memory:
//!
//! | ports type | component | holds | context |
//! |---|---|---|---|
//! | [`ShellPorts`] | `shell` | `Window`, `Tab` | yes |
//! | [`NavigatorPorts`] | `navigator` | `Navigation` | yes |
//! | [`NetPorts`] | `net` | `Request` | no |
//! | [`DocumentPorts`] | `document` | `Document`, `Element`, `Form`, `Option`, `Submission` | yes |
//! | [`RendererPorts`] | `renderer` | `Frame` | yes |
//! | [`AgentPorts`] | `agent` | `Session`, `Action` | the caller and an action's identity |
//!
//! Each ports type keeps its tables private: a component's rows are reachable only through the
//! storage traits of its own ports type, so no component's behaviours read another's.
//!
//! An assigned identity is a version 4 UUID (RFC 9562) in its canonical lower-case text form,
//! drawn from the [`RandomSource`] the ports type was built with; [`OsRandom`] by default. A
//! source that cannot be read is answered as an [`UnmetObligation`] through `TryContext`, never
//! with a value of the ports type's own making.
//!
//! The `agent` component's commands read their caller's `session_id`. [`AgentPorts`] answers it
//! from a [`Caller`], a handle whoever authenticates a call sets before issuing the command; the
//! other ports types know no caller and answer that question as unmet. Only [`AgentPorts`]
//! assigns an action's identity; the other ports types answer that as unmet too. [`AgentPorts`]
//! also meets the four obligations the generation leaves to the component:
//! `NoteNavigationStarted`, `NoteNavigationCommitted`, `NoteNavigationFailed` and
//! `NoteNavigationEnded`, which change every session their filter selects. [`DocumentPorts`]
//! meets the two the generation leaves to the `document` component, `Check` and `SelectOption`,
//! which change a control and the others of its group or select. [`NavigatorPorts`] meets the
//! two the generation leaves to the `navigator` component, `CommitNavigation` and
//! `TraverseHistory`, which keep a tab's session history entries; [`commit_navigation`] and
//! [`traverse_history`] meet them over any [`NavigationStorage`], for a ports type that wraps one.
//!
//! [`shell_port`], [`navigator_port`], [`net_port`], [`document_port`], [`renderer_port`] and
//! [`agent_port`] build each component's generated port over a ports type;
//! [`ComponentPorts::new`] builds all six with the default source.
//!
//! The navigator's port, built from its own ports type:
//!
//! ```
//! use surf_ports::{NavigatorPorts, navigator_port};
//! use surf_types::navigation::{Navigate, NavigateOutcome};
//! use surf_types::net::Method;
//! use surf_types::primitives::Uuid;
//! use surf_types::shell::TabId;
//! use surf_types::url::Url;
//!
//! let mut navigator = navigator_port(NavigatorPorts::new());
//! let outcome = navigator.navigate(Navigate {
//!     tab_id: TabId(Uuid("6f1c2d3e-4b5a-4c6d-8e7f-001122334455".to_owned())),
//!     url: Url("https://example.org/".to_owned()),
//!     method: Method::Get,
//!     body: None,
//!     body_type: None,
//!     initiator: None,
//! });
//! assert!(matches!(outcome, Ok(NavigateOutcome::Started { .. })));
//! ```
//!
//! The same port built from the shell's ports type does not compile:
//!
//! ```compile_fail,E0277
//! use surf_ports::{ShellPorts, navigator_port};
//! use surf_types::navigation::{Navigate, NavigateOutcome};
//! use surf_types::net::Method;
//! use surf_types::primitives::Uuid;
//! use surf_types::shell::TabId;
//! use surf_types::url::Url;
//!
//! // `ShellPorts` holds windows and tabs only. It does not implement `NavigationStorage`, the
//! // storage trait `navigator_port` and the generated `Navigate` behaviour require.
//! let mut navigator = navigator_port(ShellPorts::new());
//! let outcome = navigator.navigate(Navigate {
//!     tab_id: TabId(Uuid("6f1c2d3e-4b5a-4c6d-8e7f-001122334455".to_owned())),
//!     url: Url("https://example.org/".to_owned()),
//!     method: Method::Get,
//!     body: None,
//!     body_type: None,
//!     initiator: None,
//! });
//! assert!(matches!(outcome, Ok(NavigateOutcome::Started { .. })));
//! ```
//!
//! [`UnmetObligation`]: surf_types::obligation::UnmetObligation

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod ports;
mod random;
mod store;

pub use ports::{
    AgentPorts, Caller, DocumentPorts, NavigatorPorts, NetPorts, RendererPorts, ShellPorts,
    commit_navigation, traverse_history,
};
pub use random::{OsRandom, RandomSource};

use surf_types::agent::obligations::{
    NoteNavigationCommittedBehavior, NoteNavigationEndedBehavior, NoteNavigationFailedBehavior,
    NoteNavigationStartedBehavior,
};
use surf_types::behaviour::{
    ActionStorage, DocumentStorage, ElementStorage, FormStorage, FrameStorage, Generated,
    ListOptionStorage, NavigationStorage, RequestStorage, SessionStorage, SubmissionStorage,
    TabStorage, TryContext, WindowStorage,
};
use surf_types::document::obligations::{CheckBehavior, SelectOptionBehavior};
use surf_types::navigation::obligations::{CommitNavigationBehavior, TraverseHistoryBehavior};

/// The `shell` component's generated port over [`ShellPorts`].
pub type ShellPort<R = OsRandom> = shell::Shell<Generated<ShellPorts<R>>>;
/// The `navigator` component's generated port over [`NavigatorPorts`].
pub type NavigatorPort<R = OsRandom> = navigator::Navigator<Generated<NavigatorPorts<R>>>;
/// The `net` component's generated port over [`NetPorts`].
pub type NetPort = net::Net<Generated<NetPorts>>;
/// The `document` component's generated port over [`DocumentPorts`].
pub type DocumentPort<R = OsRandom> = document::Document<Generated<DocumentPorts<R>>>;
/// The `renderer` component's generated port over [`RendererPorts`].
pub type RendererPort<R = OsRandom> = renderer::Renderer<Generated<RendererPorts<R>>>;
/// The `agent` component's generated port over [`AgentPorts`].
pub type AgentPort<R = OsRandom> = agent::Agent<Generated<AgentPorts<R>>>;

/// The `shell` component's port, its behaviours generated over `ports`.
pub fn shell_port<P>(ports: P) -> shell::Shell<Generated<P>>
where
    P: TryContext + WindowStorage + TabStorage,
{
    shell::Shell::new(Generated::new(ports))
}

/// The `navigator` component's port, its behaviours generated over `ports`, and the two its
/// generation leaves owed taken from `ports`.
pub fn navigator_port<P>(ports: P) -> navigator::Navigator<Generated<P>>
where
    P: TryContext + NavigationStorage + CommitNavigationBehavior + TraverseHistoryBehavior,
{
    navigator::Navigator::new(Generated::new(ports))
}

/// The `net` component's port, its behaviours generated over `ports`.
pub fn net_port<P>(ports: P) -> net::Net<Generated<P>>
where
    P: RequestStorage,
{
    net::Net::new(Generated::new(ports))
}

/// The `document` component's port, its behaviours generated over `ports`, and the two its
/// generation leaves owed taken from `ports`.
pub fn document_port<P>(ports: P) -> document::Document<Generated<P>>
where
    P: TryContext
        + DocumentStorage
        + ElementStorage
        + FormStorage
        + ListOptionStorage
        + SubmissionStorage
        + CheckBehavior
        + SelectOptionBehavior,
{
    document::Document::new(Generated::new(ports))
}

/// The `renderer` component's port, its behaviours generated over `ports`.
pub fn renderer_port<P>(ports: P) -> renderer::Renderer<Generated<P>>
where
    P: TryContext + FrameStorage,
{
    renderer::Renderer::new(Generated::new(ports))
}

/// The `agent` component's port, its behaviours generated over `ports`, and the four its
/// generation leaves owed taken from `ports`.
pub fn agent_port<P>(ports: P) -> agent::Agent<Generated<P>>
where
    P: TryContext
        + SessionStorage
        + ActionStorage
        + NoteNavigationStartedBehavior
        + NoteNavigationCommittedBehavior
        + NoteNavigationFailedBehavior
        + NoteNavigationEndedBehavior,
{
    agent::Agent::new(Generated::new(ports))
}

/// All six component ports, each over empty tables of its own and the operating system's random
/// source, the `agent` component's sent as one [`Caller`].
pub struct ComponentPorts {
    /// The `agent` component's port.
    pub agent: AgentPort,
    /// The `shell` component's port.
    pub shell: ShellPort,
    /// The `navigator` component's port.
    pub navigator: NavigatorPort,
    /// The `net` component's port.
    pub net: NetPort,
    /// The `document` component's port.
    pub document: DocumentPort,
    /// The `renderer` component's port.
    pub renderer: RendererPort,
}

impl ComponentPorts {
    /// The six ports, each built from its own ports type, the `agent` component's with a caller
    /// of its own that names no one.
    pub fn new() -> Self {
        Self::with_caller(Caller::new())
    }

    /// The six ports, each built from its own ports type, the `agent` component's commands sent
    /// as whoever `caller` names when each runs.
    pub fn with_caller(caller: Caller) -> Self {
        Self::with_agent(AgentPorts::with_caller(caller))
    }

    /// The six ports, the `agent` component's over `agent`, each other one built from its own
    /// ports type.
    pub fn with_agent(agent: AgentPorts) -> Self {
        Self {
            agent: agent_port(agent),
            shell: shell_port(ShellPorts::new()),
            navigator: navigator_port(NavigatorPorts::new()),
            net: net_port(NetPorts::new()),
            document: document_port(DocumentPorts::new()),
            renderer: renderer_port(RendererPorts::new()),
        }
    }
}

impl Default for ComponentPorts {
    fn default() -> Self {
        Self::new()
    }
}
