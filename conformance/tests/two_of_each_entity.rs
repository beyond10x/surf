//! Every entity a scenario creates two of is read back as two rows with distinct identities. One
//! defect is planted per entity, in that entity's storage or identity source alone, and the suite
//! over that system must fail at least one scenario: every instance of the entity gets one
//! identity, so a second replaces the first, or its view answers its first row only.
//!
//! Review 2 (F1, F2) planted both defects in every component at once, which one entity covered
//! would catch; this holds each entity to it. `Request` takes its identity from the navigation, so
//! it has no identity defect of its own. A defect that is not noticed names its entity.

use ess_conformance::target::ImplementationIdentity;
use surf::{Browser, System};
use surf_conformance::{SurfTarget, Unanswered, run};
use surf_ports::{
    DocumentPorts, NavigatorPorts, NetPorts, RendererPorts, ShellPorts, document_port,
    navigator_port, net_port, renderer_port, shell_port,
};
use surf_types::behaviour::{
    DocumentStorage, FrameStorage, NavigationStorage, RequestStorage, TabStorage, TryContext,
    WindowStorage,
};
use surf_types::document::{DocumentId, DocumentSnapshot};
use surf_types::navigation::NavigationSnapshot;
use surf_types::net::{RequestId, RequestSnapshot};
use surf_types::obligation::UnmetObligation;
use surf_types::primitives::Uuid;
use surf_types::render::{FrameId, FrameSnapshot};
use surf_types::shell::{TabId, TabSnapshot, WindowId, WindowSnapshot};

/// The suite `task suite` writes.
const SUITE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../.out/suite.json");

/// The one identity every instance of the planted entity gets.
const SAME: &str = "00000000-0000-4000-8000-000000000000";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entity {
    Window,
    Tab,
    Navigation,
    Request,
    Document,
    Frame,
}

#[derive(Debug, Clone, Copy)]
enum Defect {
    /// Nothing planted.
    Nothing,
    /// Every instance of the entity gets the identity [`SAME`].
    SameIdentity(Entity),
    /// The entity's view answers its first row only.
    FirstRowOnly(Entity),
}

/// In-memory ports with `defect` applied where it names one of their entities.
struct Planted<P> {
    inner: P,
    defect: Defect,
}

impl<P> Planted<P> {
    fn same(&self, entity: Entity) -> bool {
        matches!(self.defect, Defect::SameIdentity(planted) if planted == entity)
    }

    fn rows<T>(&self, entity: Entity, mut rows: Vec<T>) -> Vec<T> {
        if matches!(self.defect, Defect::FirstRowOnly(planted) if planted == entity) {
            rows.truncate(1);
        }
        rows
    }
}

macro_rules! planted_storage {
    ($storage:ident, $snapshot:ty, $key:ty, $entity:expr) => {
        impl<P: $storage> $storage for Planted<P> {
            fn get(&self, identity: &$key) -> Option<$snapshot> {
                $storage::get(&self.inner, identity)
            }

            fn put(&mut self, snapshot: $snapshot) {
                $storage::put(&mut self.inner, snapshot);
            }

            fn delete(&mut self, identity: &$key) {
                $storage::delete(&mut self.inner, identity);
            }

            fn list(&self) -> Vec<$snapshot> {
                self.rows($entity, $storage::list(&self.inner))
            }
        }
    };
}

planted_storage!(WindowStorage, WindowSnapshot, WindowId, Entity::Window);
planted_storage!(TabStorage, TabSnapshot, TabId, Entity::Tab);
planted_storage!(
    NavigationStorage,
    NavigationSnapshot,
    RequestId,
    Entity::Navigation
);
planted_storage!(RequestStorage, RequestSnapshot, RequestId, Entity::Request);
planted_storage!(
    DocumentStorage,
    DocumentSnapshot,
    DocumentId,
    Entity::Document
);
planted_storage!(FrameStorage, FrameSnapshot, FrameId, Entity::Frame);

fn same() -> Uuid {
    Uuid(SAME.to_owned())
}

impl<P: TryContext> TryContext for Planted<P> {
    fn try_generate_surf_document_document_id(&mut self) -> Result<DocumentId, UnmetObligation> {
        if self.same(Entity::Document) {
            return Ok(DocumentId(same()));
        }
        self.inner.try_generate_surf_document_document_id()
    }

    fn try_generate_surf_net_request_id(&mut self) -> Result<RequestId, UnmetObligation> {
        if self.same(Entity::Navigation) {
            return Ok(RequestId(same()));
        }
        self.inner.try_generate_surf_net_request_id()
    }

    fn try_generate_surf_render_frame_id(&mut self) -> Result<FrameId, UnmetObligation> {
        if self.same(Entity::Frame) {
            return Ok(FrameId(same()));
        }
        self.inner.try_generate_surf_render_frame_id()
    }

    fn try_generate_surf_shell_tab_id(&mut self) -> Result<TabId, UnmetObligation> {
        if self.same(Entity::Tab) {
            return Ok(TabId(same()));
        }
        self.inner.try_generate_surf_shell_tab_id()
    }

    fn try_generate_surf_shell_window_id(&mut self) -> Result<WindowId, UnmetObligation> {
        if self.same(Entity::Window) {
            return Ok(WindowId(same()));
        }
        self.inner.try_generate_surf_shell_window_id()
    }
}

/// The suite over every component's ports with `defect` planted: the scenarios that failed.
fn failed_with(defect: Defect) -> Vec<String> {
    let suite = std::fs::read_to_string(SUITE)
        .unwrap_or_else(|error| panic!("{SUITE}: {error}; run `task suite` first"));
    let label = format!("{defect:?}");
    let target = SurfTarget::new(
        ImplementationIdentity::new("surf", label.clone()),
        move || {
            Browser::new(
                System::new(
                    document_port(Planted {
                        inner: DocumentPorts::new(),
                        defect,
                    }),
                    navigator_port(Planted {
                        inner: NavigatorPorts::new(),
                        defect,
                    }),
                    net_port(Planted {
                        inner: NetPorts::new(),
                        defect,
                    }),
                    renderer_port(Planted {
                        inner: RendererPorts::new(),
                        defect,
                    }),
                    shell_port(Planted {
                        inner: ShellPorts::new(),
                        defect,
                    }),
                ),
                Unanswered,
            )
        },
    );
    let conformance = run(&suite, &target).expect("the suite runs");
    let counts = conformance.report.counts();
    let failed: Vec<String> = conformance
        .report
        .statuses()
        .into_iter()
        .filter(|(_, status)| *status != "passed")
        .map(|(scenario, status)| format!("{status} {scenario}"))
        .collect();
    eprintln!(
        "{label}: {} scenarios: {} passed, {} failed, {} error, {} unsupported",
        counts.total, counts.passed, counts.failed, counts.error, counts.unsupported
    );
    for line in &failed {
        eprintln!("  {line}");
    }
    assert!(counts.total > 0, "{label}: the suite holds no scenario");
    failed
}

fn assert_each_noticed(defects: &[Defect]) {
    let missed: Vec<String> = defects
        .iter()
        .filter(|defect| failed_with(**defect).is_empty())
        .map(|defect| format!("{defect:?}"))
        .collect();
    assert!(
        missed.is_empty(),
        "the suite passed every scenario over a system with each of {missed:?}"
    );
}

/// Control: nothing planted, nothing fails, so a failure above is the defect's.
#[test]
fn nothing_planted_fails_no_scenario() {
    let failed = failed_with(Defect::Nothing);
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
fn one_identity_for_every_instance_of_an_entity_is_noticed_for_each_entity() {
    assert_each_noticed(&[
        Defect::SameIdentity(Entity::Window),
        Defect::SameIdentity(Entity::Tab),
        Defect::SameIdentity(Entity::Navigation),
        Defect::SameIdentity(Entity::Document),
        Defect::SameIdentity(Entity::Frame),
    ]);
}

#[test]
fn a_view_cut_to_its_first_row_is_noticed_for_each_entity() {
    assert_each_noticed(&[
        Defect::FirstRowOnly(Entity::Window),
        Defect::FirstRowOnly(Entity::Tab),
        Defect::FirstRowOnly(Entity::Navigation),
        Defect::FirstRowOnly(Entity::Request),
        Defect::FirstRowOnly(Entity::Document),
        Defect::FirstRowOnly(Entity::Frame),
    ]);
}
