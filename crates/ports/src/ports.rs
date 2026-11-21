//! One ports type per component, each holding that component's rows and nothing else.

use surf_types::behaviour::{
    DocumentStorage, FrameStorage, NavigationStorage, RequestStorage, TabStorage, TryContext,
    WindowStorage,
};
use surf_types::document::{DocumentId, DocumentSnapshot};
use surf_types::navigation::NavigationSnapshot;
use surf_types::net::{RequestId, RequestSnapshot};
use surf_types::obligation::UnmetObligation;
use surf_types::render::{FrameId, FrameSnapshot};
use surf_types::shell::{TabId, TabSnapshot, WindowId, WindowSnapshot};

use crate::random::{OsRandom, RandomSource, assign};
use crate::store::{Keyed, Rows};

macro_rules! keyed {
    ($snapshot:ty, $key:ty, $field:ident) => {
        impl Keyed for $snapshot {
            type Key = $key;

            fn key(&self) -> &$key {
                &self.data.$field
            }
        }
    };
}

keyed!(WindowSnapshot, WindowId, window_id);
keyed!(TabSnapshot, TabId, tab_id);
keyed!(NavigationSnapshot, RequestId, navigation_id);
keyed!(RequestSnapshot, RequestId, request_id);
keyed!(DocumentSnapshot, DocumentId, document_id);
keyed!(FrameSnapshot, FrameId, frame_id);

/// Implements one storage trait for a ports type over one of its private tables.
macro_rules! storage {
    ($storage:ident for $ports:ty, $rows:ident: $snapshot:ty, $key:ty $(, $param:ident)?) => {
        impl$(<$param>)? $storage for $ports {
            fn get(&self, identity: &$key) -> Option<$snapshot> {
                self.$rows.get(identity)
            }

            fn put(&mut self, snapshot: $snapshot) {
                self.$rows.put(snapshot);
            }

            fn delete(&mut self, identity: &$key) {
                self.$rows.delete(identity);
            }

            fn list(&self) -> Vec<$snapshot> {
                self.$rows.list()
            }
        }
    };
}

/// Implements `TryContext` for a ports type over its random source.
///
/// `TryContext` asks for all five kinds of identity, so every ports type that answers it can
/// assign each of them; the generated behaviours of a component ask only for their own.
macro_rules! context {
    ($ports:ident) => {
        impl<R: RandomSource> TryContext for $ports<R> {
            fn try_generate_surf_document_document_id(
                &mut self,
            ) -> Result<DocumentId, UnmetObligation> {
                assign(&mut self.random, "surf.document.DocumentId").map(DocumentId)
            }

            fn try_generate_surf_net_request_id(&mut self) -> Result<RequestId, UnmetObligation> {
                assign(&mut self.random, "surf.net.RequestId").map(RequestId)
            }

            fn try_generate_surf_render_frame_id(&mut self) -> Result<FrameId, UnmetObligation> {
                assign(&mut self.random, "surf.render.FrameId").map(FrameId)
            }

            fn try_generate_surf_shell_tab_id(&mut self) -> Result<TabId, UnmetObligation> {
                assign(&mut self.random, "surf.shell.TabId").map(TabId)
            }

            fn try_generate_surf_shell_window_id(&mut self) -> Result<WindowId, UnmetObligation> {
                assign(&mut self.random, "surf.shell.WindowId").map(WindowId)
            }
        }

        impl $ports<OsRandom> {
            /// Empty tables, with identities drawn from the operating system's random source.
            pub fn new() -> Self {
                Self::with_source(OsRandom::new())
            }
        }

        impl Default for $ports<OsRandom> {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// The `shell` component's ports: its windows and tabs, and the random source its identities
/// come from.
#[derive(Debug)]
pub struct ShellPorts<R = OsRandom> {
    windows: Rows<WindowSnapshot>,
    tabs: Rows<TabSnapshot>,
    random: R,
}

impl<R: RandomSource> ShellPorts<R> {
    /// Empty tables, with identities drawn from `source`.
    pub fn with_source(source: R) -> Self {
        Self {
            windows: Rows::default(),
            tabs: Rows::default(),
            random: source,
        }
    }
}

storage!(WindowStorage for ShellPorts<R>, windows: WindowSnapshot, WindowId, R);
storage!(TabStorage for ShellPorts<R>, tabs: TabSnapshot, TabId, R);
context!(ShellPorts);

/// The `navigator` component's ports: its navigations, and the random source its identities
/// come from.
#[derive(Debug)]
pub struct NavigatorPorts<R = OsRandom> {
    navigations: Rows<NavigationSnapshot>,
    random: R,
}

impl<R: RandomSource> NavigatorPorts<R> {
    /// An empty table, with identities drawn from `source`.
    pub fn with_source(source: R) -> Self {
        Self {
            navigations: Rows::default(),
            random: source,
        }
    }
}

storage!(NavigationStorage for NavigatorPorts<R>, navigations: NavigationSnapshot, RequestId, R);
context!(NavigatorPorts);

/// The `net` component's ports: its requests.
///
/// No random source and no `TryContext`: the `net` behaviours assign no identity, a request
/// carries the one its `Fetch` names.
#[derive(Debug, Default)]
pub struct NetPorts {
    requests: Rows<RequestSnapshot>,
}

impl NetPorts {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }
}

storage!(RequestStorage for NetPorts, requests: RequestSnapshot, RequestId);

/// The `document` component's ports: its documents, and the random source its identities come
/// from.
#[derive(Debug)]
pub struct DocumentPorts<R = OsRandom> {
    documents: Rows<DocumentSnapshot>,
    random: R,
}

impl<R: RandomSource> DocumentPorts<R> {
    /// An empty table, with identities drawn from `source`.
    pub fn with_source(source: R) -> Self {
        Self {
            documents: Rows::default(),
            random: source,
        }
    }
}

storage!(DocumentStorage for DocumentPorts<R>, documents: DocumentSnapshot, DocumentId, R);
context!(DocumentPorts);

/// The `renderer` component's ports: its frames, and the random source its identities come
/// from.
#[derive(Debug)]
pub struct RendererPorts<R = OsRandom> {
    frames: Rows<FrameSnapshot>,
    random: R,
}

impl<R: RandomSource> RendererPorts<R> {
    /// An empty table, with identities drawn from `source`.
    pub fn with_source(source: R) -> Self {
        Self {
            frames: Rows::default(),
            random: source,
        }
    }
}

storage!(FrameStorage for RendererPorts<R>, frames: FrameSnapshot, FrameId, R);
context!(RendererPorts);
