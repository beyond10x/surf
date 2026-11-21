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
//! | [`DocumentPorts`] | `document` | `Document` | yes |
//! | [`RendererPorts`] | `renderer` | `Frame` | yes |
//!
//! Each ports type keeps its tables private: a component's rows are reachable only through the
//! storage traits of its own ports type, so no component's behaviours read another's.
//!
//! An assigned identity is a version 4 UUID (RFC 9562) in its canonical lower-case text form,
//! drawn from the [`RandomSource`] the ports type was built with; [`OsRandom`] by default. A
//! source that cannot be read is answered as an [`UnmetObligation`] through `TryContext`, never
//! with a value of the ports type's own making.
//!
//! [`shell_port`], [`navigator_port`], [`net_port`], [`document_port`] and [`renderer_port`] build
//! each component's generated port over a ports type; [`ComponentPorts::new`] builds all five
//! with the default source.
//!
//! The navigator's port, built from its own ports type:
//!
//! ```
//! use surf_ports::{NavigatorPorts, navigator_port};
//! use surf_types::navigation::{Navigate, NavigateOutcome};
//! use surf_types::primitives::Uuid;
//! use surf_types::shell::TabId;
//! use surf_types::url::Url;
//!
//! let mut navigator = navigator_port(NavigatorPorts::new());
//! let outcome = navigator.navigate(Navigate {
//!     tab_id: TabId(Uuid("6f1c2d3e-4b5a-4c6d-8e7f-001122334455".to_owned())),
//!     url: Url("https://example.org/".to_owned()),
//! });
//! assert!(matches!(outcome, Ok(NavigateOutcome::Started { .. })));
//! ```
//!
//! The same port built from the shell's ports type does not compile:
//!
//! ```compile_fail,E0277
//! use surf_ports::{ShellPorts, navigator_port};
//! use surf_types::navigation::{Navigate, NavigateOutcome};
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

pub use ports::{DocumentPorts, NavigatorPorts, NetPorts, RendererPorts, ShellPorts};
pub use random::{OsRandom, RandomSource};

use surf_types::behaviour::{
    DocumentStorage, FrameStorage, Generated, NavigationStorage, RequestStorage, TabStorage,
    TryContext, WindowStorage,
};

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

/// The `shell` component's port, its behaviours generated over `ports`.
pub fn shell_port<P>(ports: P) -> shell::Shell<Generated<P>>
where
    P: TryContext + WindowStorage + TabStorage,
{
    shell::Shell::new(Generated::new(ports))
}

/// The `navigator` component's port, its behaviours generated over `ports`.
pub fn navigator_port<P>(ports: P) -> navigator::Navigator<Generated<P>>
where
    P: TryContext + NavigationStorage,
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

/// The `document` component's port, its behaviours generated over `ports`.
pub fn document_port<P>(ports: P) -> document::Document<Generated<P>>
where
    P: TryContext + DocumentStorage,
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

/// All five component ports, each over empty tables of its own and the operating system's
/// random source.
pub struct ComponentPorts {
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
    /// The five ports, each built from its own ports type.
    pub fn new() -> Self {
        Self {
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
