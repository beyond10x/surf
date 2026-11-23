//! The planted defect: the suite over a system whose `Navigation` storage drops every `put` must
//! fail. A suite that passes over this system would pass over a navigator that keeps nothing.
//! It runs over the scenarios its line of `tests/noticed.txt` names (`task conform-slow` writes
//! the list), or over the whole suite when `SURF_NOTICED_OUT` is set.

use ess_conformance::target::ImplementationIdentity;
use surf::{Browser, System};
use surf_conformance::{SurfTarget, Unanswered, admit, note_noticed, planted_suite, run};
use surf_ports::{
    NavigatorPorts, NetPorts, RendererPorts, ShellPorts, agent_port, commit_navigation,
    document_port, navigator_port, net_port, renderer_port, shell_port, traverse_history,
};
use surf_types::agent::{ActionId, SessionId};
use surf_types::behaviour::{NavigationStorage, TryContext};
use surf_types::document::{DocumentId, ElementId, FormId, OptionId, SubmissionId};
use surf_types::navigation::obligations::{CommitNavigationBehavior, TraverseHistoryBehavior};
use surf_types::navigation::{
    CommitNavigation, CommitNavigationOutcome, NavigationSnapshot, TraverseHistory,
    TraverseHistoryOutcome,
};
use surf_types::net::{MediaType, RequestId};
use surf_types::obligation::UnmetObligation;
use surf_types::render::FrameId;
use surf_types::shell::{TabId, WindowId};

/// The suite `task suite` writes.
const SUITE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../.out/suite.json");

/// The navigator's in-memory ports, except that `put` stores nothing.
struct DropsEveryPut(NavigatorPorts);

impl NavigationStorage for DropsEveryPut {
    fn get(&self, identity: &RequestId) -> Option<NavigationSnapshot> {
        NavigationStorage::get(&self.0, identity)
    }

    fn put(&mut self, _snapshot: NavigationSnapshot) {}

    fn delete(&mut self, identity: &RequestId) {
        NavigationStorage::delete(&mut self.0, identity);
    }

    fn list(&self) -> Vec<NavigationSnapshot> {
        NavigationStorage::list(&self.0)
    }
}

/// The `navigator` component's obligations, met over the storage that drops every `put`, as its
/// generated behaviours are.
impl CommitNavigationBehavior for DropsEveryPut {
    fn commit_navigation(
        &mut self,
        input: CommitNavigation,
    ) -> Result<CommitNavigationOutcome, UnmetObligation> {
        commit_navigation(self, input)
    }
}

impl TraverseHistoryBehavior for DropsEveryPut {
    fn traverse_history(
        &mut self,
        input: TraverseHistory,
    ) -> Result<TraverseHistoryOutcome, UnmetObligation> {
        traverse_history(self, input)
    }
}

impl TryContext for DropsEveryPut {
    fn try_caller_session_id(&self) -> Result<Option<SessionId>, UnmetObligation> {
        self.0.try_caller_session_id()
    }

    fn try_generate_optional_string(&mut self) -> Result<Option<String>, UnmetObligation> {
        self.0.try_generate_optional_string()
    }

    fn try_generate_optional_bytes(&mut self) -> Result<Option<Vec<u8>>, UnmetObligation> {
        self.0.try_generate_optional_bytes()
    }

    fn try_generate_optional_surf_net_media_type(
        &mut self,
    ) -> Result<Option<MediaType>, UnmetObligation> {
        self.0.try_generate_optional_surf_net_media_type()
    }

    fn try_generate_surf_agent_action_id(&mut self) -> Result<ActionId, UnmetObligation> {
        self.0.try_generate_surf_agent_action_id()
    }

    fn try_generate_surf_document_document_id(&mut self) -> Result<DocumentId, UnmetObligation> {
        self.0.try_generate_surf_document_document_id()
    }

    fn try_generate_surf_document_element_id(&mut self) -> Result<ElementId, UnmetObligation> {
        self.0.try_generate_surf_document_element_id()
    }

    fn try_generate_surf_document_form_id(&mut self) -> Result<FormId, UnmetObligation> {
        self.0.try_generate_surf_document_form_id()
    }

    fn try_generate_surf_document_option_id(&mut self) -> Result<OptionId, UnmetObligation> {
        self.0.try_generate_surf_document_option_id()
    }

    fn try_generate_surf_document_submission_id(
        &mut self,
    ) -> Result<SubmissionId, UnmetObligation> {
        self.0.try_generate_surf_document_submission_id()
    }

    fn try_generate_surf_net_request_id(&mut self) -> Result<RequestId, UnmetObligation> {
        self.0.try_generate_surf_net_request_id()
    }

    fn try_generate_surf_render_frame_id(&mut self) -> Result<FrameId, UnmetObligation> {
        self.0.try_generate_surf_render_frame_id()
    }

    fn try_generate_surf_shell_tab_id(&mut self) -> Result<TabId, UnmetObligation> {
        self.0.try_generate_surf_shell_tab_id()
    }

    fn try_generate_surf_shell_window_id(&mut self) -> Result<WindowId, UnmetObligation> {
        self.0.try_generate_surf_shell_window_id()
    }
}

/// The defect's label, and its line of `tests/noticed.txt`.
const LABEL: &str = "navigation storage drops every put";

#[test]
fn a_navigation_storage_that_drops_every_put_fails_at_least_one_scenario() {
    let suite = std::fs::read_to_string(SUITE)
        .unwrap_or_else(|error| panic!("{SUITE}: {error}; run `task suite` first"));
    let admitted = admit(&suite).expect("the suite is admitted");
    let suite = planted_suite(&admitted, LABEL).unwrap_or_else(|error| panic!("{error}"));
    let target = SurfTarget::new(ImplementationIdentity::new("surf", LABEL), |setup| {
        Browser::new(
            System::new(
                agent_port(setup.agent_ports()),
                document_port(setup.document_ports()),
                navigator_port(DropsEveryPut(NavigatorPorts::new())),
                net_port(NetPorts::new()),
                renderer_port(RendererPorts::new()),
                shell_port(ShellPorts::new()),
            ),
            Unanswered,
        )
    });
    let conformance = run(&suite, &target).expect("the suite runs");
    note_noticed(LABEL, &conformance.report).unwrap_or_else(|error| panic!("{error}"));
    let counts = conformance.report.counts();
    let failed: Vec<String> = conformance
        .report
        .statuses()
        .into_iter()
        .filter(|(_, status)| *status == "failed")
        .map(|(scenario, _)| scenario)
        .collect();
    eprintln!(
        "planted defect: {} scenarios: {} passed, {} failed, {} error, {} unsupported",
        counts.total, counts.passed, counts.failed, counts.error, counts.unsupported
    );
    for scenario in &failed {
        eprintln!("  failed {scenario}");
    }
    assert!(
        counts.failed >= 1,
        "no scenario failed over a navigation storage that drops every put"
    );
}
