//! Review 2: system defects planted behind the real adapter, one per test. Each test asserts what
//! a green `task conform-rust` is taken to mean: the suite notices the defect, so at least one
//! scenario does not pass. A test that is red here names a defect the gate lets through.
//!
//! Every mutant lives in this file, in a storage port or a random source handed to the generated
//! behaviours; nothing under `crates/`, `generated/` or `conformance/src` is touched. Each runs
//! over the scenarios its line of `tests/noticed.txt` names (`task conform-slow` writes the
//! list), or over the whole suite when `SURF_NOTICED_OUT` is set; the control runs the whole
//! suite.

use ess_conformance::target::ImplementationIdentity;
use surf::{Browser, System};
use surf_conformance::{
    ConformanceBrowser, Setup, SurfTarget, Unanswered, admit, note_noticed, planted_suite, run,
};
use surf_ports::{
    DocumentPorts, NavigatorPorts, NetPorts, RandomSource, RendererPorts, ShellPorts, agent_port,
    commit_navigation, document_port, navigator_port, net_port, renderer_port, shell_port,
    traverse_history,
};
use surf_types::agent::{ActionId, SessionId};
use surf_types::behaviour::{
    DocumentStorage, ElementStorage, FormStorage, FrameStorage, ListOptionStorage,
    NavigationStorage, RequestStorage, SubmissionStorage, TabStorage, TryContext, WindowStorage,
};
use surf_types::document::obligations::{CheckBehavior, SelectOptionBehavior};
use surf_types::document::{
    Check, CheckOutcome, DocumentId, DocumentSnapshot, ElementId, ElementSnapshot, FormId,
    FormSnapshot, ListOptionSnapshot, OptionId, SelectOption, SelectOptionOutcome, SubmissionId,
    SubmissionSnapshot,
};
use surf_types::navigation::obligations::{CommitNavigationBehavior, TraverseHistoryBehavior};
use surf_types::navigation::{
    CommitNavigation, CommitNavigationOutcome, NavigationSnapshot, NavigationState,
    TraverseHistory, TraverseHistoryOutcome,
};
use surf_types::net::{MediaType, RequestId, RequestSnapshot};
use surf_types::obligation::UnmetObligation;
use surf_types::render::{FrameId, FrameSnapshot};
use surf_types::shell::{
    AnyWindow, TabId, TabSnapshot, Window, WindowData, WindowId, WindowSnapshot,
};
use surf_types::url::Url;

/// The suite `task suite` writes.
const SUITE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../.out/suite.json");

/// The label of the control, which runs the whole suite; every other label is a planted defect's
/// and runs over the scenarios its line of `tests/noticed.txt` names.
const CONTROL: &str = "control";

/// The suite over the system `assemble` builds: (total, passed, the scenarios that did not pass).
/// The control runs the whole suite; a planted defect the scenarios its line of
/// `tests/noticed.txt` names, or the whole suite when `SURF_NOTICED_OUT` is set.
fn suite_over<A, D, N, Ne, R, S>(
    label: &str,
    assemble: impl Fn(&Setup) -> ConformanceBrowser<A, D, N, Ne, R, S> + 'static,
) -> (usize, usize, Vec<String>)
where
    A: surf::AgentBehaviors + 'static,
    D: surf::DocumentBehaviors + 'static,
    N: surf::NavigatorBehaviors + 'static,
    Ne: surf::NetBehaviors + 'static,
    R: surf::RendererBehaviors + 'static,
    S: surf::ShellBehaviors + 'static,
{
    let mut suite = std::fs::read_to_string(SUITE)
        .unwrap_or_else(|error| panic!("{SUITE}: {error}; run `task suite` first"));
    if label != CONTROL {
        let admitted = admit(&suite).expect("the suite is admitted");
        suite = planted_suite(&admitted, label).unwrap_or_else(|error| panic!("{error}"));
    }
    let target = SurfTarget::new(ImplementationIdentity::new("surf", label), assemble);
    let conformance = run(&suite, &target).expect("the suite runs");
    if label != CONTROL {
        note_noticed(label, &conformance.report).unwrap_or_else(|error| panic!("{error}"));
    }
    let counts = conformance.report.counts();
    let not_passed: Vec<String> = conformance
        .report
        .statuses()
        .into_iter()
        .filter(|(_, status)| *status != "passed")
        .map(|(scenario, status)| format!("{status} {scenario}"))
        .collect();
    eprintln!(
        "review2 [{label}]: {} scenarios: {} passed, {} failed, {} error, {} unsupported",
        counts.total, counts.passed, counts.failed, counts.error, counts.unsupported
    );
    for line in &not_passed {
        eprintln!("  {line}");
    }
    (counts.total as usize, counts.passed as usize, not_passed)
}

fn assert_noticed(label: &str, (total, passed, _): (usize, usize, Vec<String>)) {
    assert!(total > 0, "[{label}] the suite holds no scenario");
    assert!(
        passed < total,
        "[{label}] the suite passed all {total} scenarios over a system with this defect"
    );
}

/// What a mutant storage does differently from the in-memory one it wraps.
#[derive(Clone, Copy, Default)]
struct Policy {
    drop_puts: bool,
    drop_last_row: bool,
    first_row_only: bool,
}

impl Policy {
    fn rows<T>(self, mut rows: Vec<T>) -> Vec<T> {
        if self.drop_last_row {
            rows.pop();
        }
        if self.first_row_only {
            rows.truncate(1);
        }
        rows
    }
}

/// An in-memory ports type with one [`Policy`] applied to every storage trait it implements.
struct Mutant<P> {
    inner: P,
    policy: Policy,
}

impl<P> Mutant<P> {
    fn new(inner: P, policy: Policy) -> Self {
        Self { inner, policy }
    }
}

macro_rules! mutant_storage {
    ($storage:ident, $snapshot:ty, $key:ty) => {
        impl<P: $storage> $storage for Mutant<P> {
            fn get(&self, identity: &$key) -> Option<$snapshot> {
                $storage::get(&self.inner, identity)
            }

            fn put(&mut self, snapshot: $snapshot) {
                if !self.policy.drop_puts {
                    $storage::put(&mut self.inner, snapshot);
                }
            }

            fn delete(&mut self, identity: &$key) {
                $storage::delete(&mut self.inner, identity);
            }

            fn list(&self) -> Vec<$snapshot> {
                self.policy.rows($storage::list(&self.inner))
            }
        }
    };
}

mutant_storage!(WindowStorage, WindowSnapshot, WindowId);
mutant_storage!(TabStorage, TabSnapshot, TabId);
mutant_storage!(NavigationStorage, NavigationSnapshot, RequestId);
mutant_storage!(RequestStorage, RequestSnapshot, RequestId);
mutant_storage!(DocumentStorage, DocumentSnapshot, DocumentId);
mutant_storage!(ElementStorage, ElementSnapshot, ElementId);
mutant_storage!(FormStorage, FormSnapshot, FormId);
mutant_storage!(ListOptionStorage, ListOptionSnapshot, OptionId);
mutant_storage!(SubmissionStorage, SubmissionSnapshot, SubmissionId);
mutant_storage!(FrameStorage, FrameSnapshot, FrameId);

/// The `document` component's obligations, handed through to the ports the mutant wraps: they
/// read and write its tables without the policy.
impl<P: CheckBehavior> CheckBehavior for Mutant<P> {
    fn check(&mut self, input: Check) -> Result<CheckOutcome, UnmetObligation> {
        self.inner.check(input)
    }
}

impl<P: SelectOptionBehavior> SelectOptionBehavior for Mutant<P> {
    fn select_option(
        &mut self,
        input: SelectOption,
    ) -> Result<SelectOptionOutcome, UnmetObligation> {
        self.inner.select_option(input)
    }
}

/// The `navigator` component's obligations for `$ty`, met over its own storage: they read and
/// write a tab's entries through it, as the component's generated behaviours do.
macro_rules! navigator_obligations {
    (impl$(<$param:ident: NavigationStorage>)? for $ty:ty) => {
        impl$(<$param: NavigationStorage>)? CommitNavigationBehavior for $ty {
            fn commit_navigation(
                &mut self,
                input: CommitNavigation,
            ) -> Result<CommitNavigationOutcome, UnmetObligation> {
                commit_navigation(self, input)
            }
        }

        impl$(<$param: NavigationStorage>)? TraverseHistoryBehavior for $ty {
            fn traverse_history(
                &mut self,
                input: TraverseHistory,
            ) -> Result<TraverseHistoryOutcome, UnmetObligation> {
                traverse_history(self, input)
            }
        }
    };
}

navigator_obligations!(impl<P: NavigationStorage> for Mutant<P>);

/// `TryContext` handed through to `$field` of `$ty`.
macro_rules! context_through {
    (impl$(<$param:ident: TryContext>)? for $ty:ty, $field:tt) => {
        impl$(<$param: TryContext>)? TryContext for $ty {
            fn try_caller_session_id(&self) -> Result<Option<SessionId>, UnmetObligation> {
                self.$field.try_caller_session_id()
            }

            fn try_generate_optional_string(
                &mut self,
            ) -> Result<Option<String>, UnmetObligation> {
                self.$field.try_generate_optional_string()
            }

            fn try_generate_optional_bytes(
                &mut self,
            ) -> Result<Option<Vec<u8>>, UnmetObligation> {
                self.$field.try_generate_optional_bytes()
            }

            fn try_generate_optional_surf_net_media_type(
                &mut self,
            ) -> Result<Option<MediaType>, UnmetObligation> {
                self.$field.try_generate_optional_surf_net_media_type()
            }

            fn try_generate_surf_agent_action_id(&mut self) -> Result<ActionId, UnmetObligation> {
                self.$field.try_generate_surf_agent_action_id()
            }

            fn try_generate_surf_document_document_id(
                &mut self,
            ) -> Result<DocumentId, UnmetObligation> {
                self.$field.try_generate_surf_document_document_id()
            }

            fn try_generate_surf_document_element_id(
                &mut self,
            ) -> Result<ElementId, UnmetObligation> {
                self.$field.try_generate_surf_document_element_id()
            }

            fn try_generate_surf_document_form_id(&mut self) -> Result<FormId, UnmetObligation> {
                self.$field.try_generate_surf_document_form_id()
            }

            fn try_generate_surf_document_option_id(
                &mut self,
            ) -> Result<OptionId, UnmetObligation> {
                self.$field.try_generate_surf_document_option_id()
            }

            fn try_generate_surf_document_submission_id(
                &mut self,
            ) -> Result<SubmissionId, UnmetObligation> {
                self.$field.try_generate_surf_document_submission_id()
            }

            fn try_generate_surf_net_request_id(&mut self) -> Result<RequestId, UnmetObligation> {
                self.$field.try_generate_surf_net_request_id()
            }

            fn try_generate_surf_render_frame_id(&mut self) -> Result<FrameId, UnmetObligation> {
                self.$field.try_generate_surf_render_frame_id()
            }

            fn try_generate_surf_shell_tab_id(&mut self) -> Result<TabId, UnmetObligation> {
                self.$field.try_generate_surf_shell_tab_id()
            }

            fn try_generate_surf_shell_window_id(&mut self) -> Result<WindowId, UnmetObligation> {
                self.$field.try_generate_surf_shell_window_id()
            }
        }
    };
}

context_through!(impl<P: TryContext> for Mutant<P>, inner);

/// Every component's storage with `policy`, the `net` component's included.
fn every_storage(label: &'static str, policy: Policy) -> (usize, usize, Vec<String>) {
    suite_over(label, move |setup| {
        Browser::new(
            System::new(
                agent_port(setup.agent_ports()),
                document_port(Mutant::new(setup.document_ports(), policy)),
                navigator_port(Mutant::new(NavigatorPorts::new(), policy)),
                net_port(Mutant::new(NetPorts::new(), policy)),
                renderer_port(Mutant::new(RendererPorts::new(), policy)),
                shell_port(Mutant::new(ShellPorts::new(), policy)),
            ),
            Unanswered,
        )
    })
}

/// Control: the harness over the unplanted system passes every scenario, so a red test below is
/// the defect's and not the harness's.
#[test]
fn review2_control_the_unplanted_system_passes_every_scenario() {
    let (total, passed, not_passed) = every_storage(CONTROL, Policy::default());
    assert!(total > 0 && passed == total, "{not_passed:?}");
}

#[test]
fn review2_a_net_storage_that_drops_every_put_is_noticed() {
    let label = "net storage drops every put";
    assert_noticed(
        label,
        suite_over(label, |setup| {
            Browser::new(
                System::new(
                    agent_port(setup.agent_ports()),
                    document_port(setup.document_ports()),
                    navigator_port(NavigatorPorts::new()),
                    net_port(Mutant::new(
                        NetPorts::new(),
                        Policy {
                            drop_puts: true,
                            ..Policy::default()
                        },
                    )),
                    renderer_port(RendererPorts::new()),
                    shell_port(ShellPorts::new()),
                ),
                Unanswered,
            )
        }),
    );
}

#[test]
fn review2_a_document_storage_that_drops_every_put_is_noticed() {
    let label = "document storage drops every put";
    assert_noticed(
        label,
        suite_over(label, |setup| {
            Browser::new(
                System::new(
                    agent_port(setup.agent_ports()),
                    document_port(Mutant::new(
                        setup.document_ports(),
                        Policy {
                            drop_puts: true,
                            ..Policy::default()
                        },
                    )),
                    navigator_port(NavigatorPorts::new()),
                    net_port(NetPorts::new()),
                    renderer_port(RendererPorts::new()),
                    shell_port(ShellPorts::new()),
                ),
                Unanswered,
            )
        }),
    );
}

#[test]
fn review2_a_navigations_view_one_row_short_is_noticed() {
    let label = "Navigations view drops its last row";
    assert_noticed(
        label,
        suite_over(label, |setup| {
            Browser::new(
                System::new(
                    agent_port(setup.agent_ports()),
                    document_port(setup.document_ports()),
                    navigator_port(Mutant::new(
                        NavigatorPorts::new(),
                        Policy {
                            drop_last_row: true,
                            ..Policy::default()
                        },
                    )),
                    net_port(NetPorts::new()),
                    renderer_port(RendererPorts::new()),
                    shell_port(ShellPorts::new()),
                ),
                Unanswered,
            )
        }),
    );
}

/// Every view of every component answers its first row only: one row too few as soon as a view
/// holds two.
#[test]
fn review2_every_view_cut_to_its_first_row_is_noticed() {
    let label = "every view answers its first row only";
    assert_noticed(
        label,
        every_storage(
            label,
            Policy {
                first_row_only: true,
                ..Policy::default()
            },
        ),
    );
}

/// A random source that answers zeros: every identity a ports type assigns is the same one.
struct Zeros;

impl RandomSource for Zeros {
    fn fill(&mut self, buffer: &mut [u8]) -> std::io::Result<()> {
        buffer.fill(0);
        Ok(())
    }
}

/// Every window, tab, navigation, document and frame gets the same identity, so a second one of
/// a kind replaces the first.
#[test]
fn review2_every_assigned_identity_the_same_is_noticed() {
    let label = "every assigned identity is the same";
    assert_noticed(
        label,
        suite_over(label, |setup| {
            Browser::new(
                System::new(
                    agent_port(setup.agent_ports()),
                    document_port(setup.document_entities(DocumentPorts::with_source(Zeros))),
                    navigator_port(NavigatorPorts::with_source(Zeros)),
                    net_port(NetPorts::new()),
                    renderer_port(RendererPorts::with_source(Zeros)),
                    shell_port(ShellPorts::with_source(Zeros)),
                ),
                Unanswered,
            )
        }),
    );
}

/// A navigation storage that changes one stored navigation on its way in, or on its way out.
struct Navigations {
    inner: NavigatorPorts,
    on_put: fn(NavigationSnapshot) -> Option<NavigationSnapshot>,
    on_get: fn(NavigationSnapshot) -> NavigationSnapshot,
    /// Every version ever put, oldest first, when the store keeps them all.
    versions: Option<Vec<NavigationSnapshot>>,
}

impl Navigations {
    fn new(
        on_put: fn(NavigationSnapshot) -> Option<NavigationSnapshot>,
        on_get: fn(NavigationSnapshot) -> NavigationSnapshot,
    ) -> Self {
        Self {
            inner: NavigatorPorts::new(),
            on_put,
            on_get,
            versions: None,
        }
    }
}

impl NavigationStorage for Navigations {
    fn get(&self, identity: &RequestId) -> Option<NavigationSnapshot> {
        let held = match &self.versions {
            Some(versions) => versions
                .iter()
                .rev()
                .find(|held| &held.data.navigation_id == identity)
                .cloned(),
            None => NavigationStorage::get(&self.inner, identity),
        };
        held.map(self.on_get)
    }

    fn put(&mut self, snapshot: NavigationSnapshot) {
        let Some(snapshot) = (self.on_put)(snapshot) else {
            return;
        };
        match &mut self.versions {
            Some(versions) => versions.push(snapshot),
            None => NavigationStorage::put(&mut self.inner, snapshot),
        }
    }

    fn delete(&mut self, identity: &RequestId) {
        if let Some(versions) = &mut self.versions {
            versions.retain(|held| &held.data.navigation_id != identity);
        }
        NavigationStorage::delete(&mut self.inner, identity);
    }

    fn list(&self) -> Vec<NavigationSnapshot> {
        match &self.versions {
            Some(versions) => versions.clone(),
            None => NavigationStorage::list(&self.inner),
        }
    }
}

context_through!(impl for Navigations, inner);
navigator_obligations!(impl for Navigations);

fn keep(snapshot: NavigationSnapshot) -> Option<NavigationSnapshot> {
    Some(snapshot)
}

fn same(snapshot: NavigationSnapshot) -> NavigationSnapshot {
    snapshot
}

fn navigations_over(
    label: &'static str,
    navigations: fn() -> Navigations,
) -> (usize, usize, Vec<String>) {
    suite_over(label, move |setup| {
        Browser::new(
            System::new(
                agent_port(setup.agent_ports()),
                document_port(setup.document_ports()),
                navigator_port(navigations()),
                net_port(NetPorts::new()),
                renderer_port(RendererPorts::new()),
                shell_port(ShellPorts::new()),
            ),
            Unanswered,
        )
    })
}

/// `CommitNavigation` answers `committed` and publishes `NavigationCommitted`, and the stored
/// navigation stays `Started`.
#[test]
fn review2_a_navigation_left_started_after_its_commit_is_noticed() {
    assert_noticed(
        "commit leaves the navigation Started",
        navigations_over("commit leaves the navigation Started", || {
            Navigations::new(
                |snapshot| (snapshot.state != NavigationState::Committed).then_some(snapshot),
                same,
            )
        }),
    );
}

/// Every stored navigation reads back as `Started`, so a commit, a failure or a cancellation of a
/// navigation that already ended is accepted instead of refused.
#[test]
fn review2_a_refusal_answered_as_acceptance_is_noticed() {
    assert_noticed(
        "an ended navigation reads back Started",
        navigations_over("an ended navigation reads back Started", || {
            Navigations::new(keep, |mut snapshot| {
                snapshot.state = NavigationState::Started;
                snapshot
            })
        }),
    );
}

/// The stored navigation's address is changed on its way in, so `NavigationCommitted`,
/// `NavigationFailed` and the `Navigations` view carry another address than the one navigated to.
#[test]
fn review2_an_event_field_from_a_corrupted_row_is_noticed() {
    assert_noticed(
        "the stored address is rewritten",
        navigations_over("the stored address is rewritten", || {
            Navigations::new(
                |mut snapshot| {
                    snapshot.data.url = Url(format!("{}#rewritten", snapshot.data.url.0));
                    Some(snapshot)
                },
                same,
            )
        }),
    );
}

/// A store that keeps every version it was given: a navigation reads back as its latest version,
/// and the `Navigations` view shows every version, so a committed navigation is listed both
/// `Started` and `Committed`.
#[test]
fn review2_a_navigations_view_that_keeps_superseded_rows_is_noticed() {
    assert_noticed(
        "Navigations view keeps every superseded row",
        navigations_over("Navigations view keeps every superseded row", || {
            let mut navigations = Navigations::new(keep, same);
            navigations.versions = Some(Vec::new());
            navigations
        }),
    );
}

/// A window storage that answers every identity: `OpenTab` into a window that was never opened is
/// accepted instead of refused.
struct EveryWindow(ShellPorts);

impl WindowStorage for EveryWindow {
    fn get(&self, identity: &WindowId) -> Option<WindowSnapshot> {
        WindowStorage::get(&self.0, identity).or_else(|| {
            Some(
                AnyWindow::Open(Window::new(WindowData {
                    window_id: identity.clone(),
                }))
                .snapshot(),
            )
        })
    }

    fn put(&mut self, snapshot: WindowSnapshot) {
        WindowStorage::put(&mut self.0, snapshot);
    }

    fn delete(&mut self, identity: &WindowId) {
        WindowStorage::delete(&mut self.0, identity);
    }

    fn list(&self) -> Vec<WindowSnapshot> {
        WindowStorage::list(&self.0)
    }
}

impl TabStorage for EveryWindow {
    fn get(&self, identity: &TabId) -> Option<TabSnapshot> {
        TabStorage::get(&self.0, identity)
    }

    fn put(&mut self, snapshot: TabSnapshot) {
        TabStorage::put(&mut self.0, snapshot);
    }

    fn delete(&mut self, identity: &TabId) {
        TabStorage::delete(&mut self.0, identity);
    }

    fn list(&self) -> Vec<TabSnapshot> {
        TabStorage::list(&self.0)
    }
}

context_through!(impl for EveryWindow, 0);

#[test]
fn review2_open_tab_into_a_window_never_opened_is_noticed() {
    let label = "every window identity is found";
    assert_noticed(
        label,
        suite_over(label, |setup| {
            Browser::new(
                System::new(
                    agent_port(setup.agent_ports()),
                    document_port(setup.document_ports()),
                    navigator_port(NavigatorPorts::new()),
                    net_port(NetPorts::new()),
                    renderer_port(RendererPorts::new()),
                    shell_port(EveryWindow(ShellPorts::new())),
                ),
                Unanswered,
            )
        }),
    );
}
