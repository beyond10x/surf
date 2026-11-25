//! Adversary of `story:script-error-kind-aggregate`: wrong treatments of the error kind
//! `AggregateError`, planted in the `document` component's ports of the Rust target, each run
//! over the scenarios that issue `surf.document.AddScriptEnd` (narrowed from the suite
//! `adversary::scripts_suite` keeps on disk, admitted once per test). Nothing under `crates/`,
//! `generated/` or `conformance/src` is touched: a plant stores, refuses or accepts around
//! `surf_ports::add_script_end`, which stays the target's own answer for every other input.
//!
//! A plant that every scenario lets through is a wrong treatment the suite does not notice; the
//! test that holds it is red and names it.

use ess_conformance::target::ImplementationIdentity;
use ess_conformance::{AdmittedSuite, CountReport, Runner};
use surf::{Browser, System};
use surf_conformance::{SurfTarget, Unanswered, admit, narrowed};
use surf_ports::{
    NavigatorPorts, NetPorts, RendererPorts, ScriptIdentity, ShellPorts, add_script_end,
    agent_port, document_port, navigator_port, net_port, renderer_port, shell_port,
};
use surf_types::agent::{ActionId, SessionId};
use surf_types::behaviour::{
    DocumentStorage, ElementStorage, FormStorage, ListOptionStorage, ScriptStorage,
    SubmissionStorage, TryContext,
};
use surf_types::document::obligations::{
    AddScriptEndBehavior, CheckBehavior, SelectOptionBehavior,
};
use surf_types::document::{
    AddScriptEnd, AddScriptEndOutcome, Check, CheckOutcome, DocumentId, DocumentSnapshot,
    ElementId, ElementSnapshot, FormId, FormSnapshot, InvalidScriptEnd, ListOptionSnapshot,
    OptionId, ScriptEnd, ScriptErrorKind, ScriptId, ScriptSnapshot, SelectOption,
    SelectOptionOutcome, SubmissionId, SubmissionSnapshot,
};
use surf_types::net::{MediaType, RequestId};
use surf_types::obligation::UnmetObligation;
use surf_types::render::FrameId;
use surf_types::shell::{TabId, WindowId};

const EVERY_KIND: &str = "document-records-every-error-kind";
const EXCLUDED: &str = "adversary-script-ends-q7-excluded";

/// Every kind of `surf.document.ScriptErrorKind` but `AggregateError`.
const OTHERS: [ScriptErrorKind; 8] = [
    ScriptErrorKind::Error,
    ScriptErrorKind::EvalError,
    ScriptErrorKind::RangeError,
    ScriptErrorKind::ReferenceError,
    ScriptErrorKind::SyntaxError,
    ScriptErrorKind::TypeError,
    ScriptErrorKind::UriError,
    ScriptErrorKind::Value,
];

/// One wrong treatment of `AggregateError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Plant {
    /// Nothing planted: the control.
    Nothing,
    /// A record of `AggregateError` is stored with this kind.
    AggregateStoredAs(ScriptErrorKind),
    /// A record of this kind is stored as `AggregateError`.
    StoredAsAggregate(ScriptErrorKind),
    /// `Threw` with `AggregateError` is answered `invalid-end`.
    ThrewAggregateRefused,
    /// `Refused` with `AggregateError` is added, as `Refused` with `SyntaxError` is.
    RefusedAggregateAdded,
    /// `Threw` with `AggregateError` and no message is answered `invalid-end`: an
    /// `AggregateError` taken to need a message. swell 0.9.0's `new AggregateError([])` sets no
    /// message (`story:swell-0-9-0`, decision 4: it ends `threw AggregateError 1:1`).
    ThrewAggregateWithoutMessageRefused,
    /// A record of `AggregateError` with no message is stored with the message
    /// `AggregateError`: the kind's name written where swell gave none.
    AggregateMessageInvented,
}

/// The `document` component's ports with one [`Plant`].
struct Planted<P> {
    inner: P,
    plant: Plant,
    /// The record being added is a `Refused` `AggregateError` let through as `SyntaxError`:
    /// stored with its own kind.
    restore: bool,
}

impl<P: ScriptIdentity + DocumentStorage + ScriptStorage> AddScriptEndBehavior for Planted<P> {
    fn add_script_end(
        &mut self,
        mut input: AddScriptEnd,
    ) -> Result<AddScriptEndOutcome, UnmetObligation> {
        let aggregate = input.error_kind == Some(ScriptErrorKind::AggregateError);
        let refuse = match self.plant {
            Plant::ThrewAggregateRefused => aggregate && input.end == ScriptEnd::Threw,
            Plant::ThrewAggregateWithoutMessageRefused => {
                aggregate && input.end == ScriptEnd::Threw && input.message.is_none()
            }
            _ => false,
        };
        if refuse {
            return Ok(AddScriptEndOutcome::InvalidEnd {
                error: InvalidScriptEnd { end: input.end },
            });
        }
        if self.plant == Plant::RefusedAggregateAdded
            && aggregate
            && input.end == ScriptEnd::Refused
        {
            input.error_kind = Some(ScriptErrorKind::SyntaxError);
            self.restore = true;
        }
        let outcome = add_script_end(self, input);
        self.restore = false;
        outcome
    }
}

impl<P: ScriptStorage> ScriptStorage for Planted<P> {
    fn get(&self, identity: &ScriptId) -> Option<ScriptSnapshot> {
        ScriptStorage::get(&self.inner, identity)
    }

    fn put(&mut self, mut snapshot: ScriptSnapshot) {
        let data = &mut snapshot.data;
        if self.restore {
            data.error_kind = Some(ScriptErrorKind::AggregateError);
        }
        let aggregate = data.error_kind == Some(ScriptErrorKind::AggregateError);
        match self.plant {
            Plant::AggregateStoredAs(kind) if aggregate => data.error_kind = Some(kind),
            Plant::StoredAsAggregate(kind) if data.error_kind == Some(kind) => {
                data.error_kind = Some(ScriptErrorKind::AggregateError);
            }
            Plant::AggregateMessageInvented if aggregate && data.message.is_none() => {
                data.message = Some("AggregateError".to_owned());
            }
            _ => {}
        }
        ScriptStorage::put(&mut self.inner, snapshot);
    }

    fn delete(&mut self, identity: &ScriptId) {
        ScriptStorage::delete(&mut self.inner, identity);
    }

    fn list(&self) -> Vec<ScriptSnapshot> {
        ScriptStorage::list(&self.inner)
    }
}

macro_rules! through_storage {
    ($storage:ident, $snapshot:ty, $key:ty) => {
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
                $storage::list(&self.inner)
            }
        }
    };
}

through_storage!(DocumentStorage, DocumentSnapshot, DocumentId);
through_storage!(ElementStorage, ElementSnapshot, ElementId);
through_storage!(FormStorage, FormSnapshot, FormId);
through_storage!(ListOptionStorage, ListOptionSnapshot, OptionId);
through_storage!(SubmissionStorage, SubmissionSnapshot, SubmissionId);

impl<P: ScriptIdentity> ScriptIdentity for Planted<P> {
    fn assign_script_id(&mut self) -> Result<ScriptId, UnmetObligation> {
        self.inner.assign_script_id()
    }
}

impl<P: CheckBehavior> CheckBehavior for Planted<P> {
    fn check(&mut self, input: Check) -> Result<CheckOutcome, UnmetObligation> {
        self.inner.check(input)
    }
}

impl<P: SelectOptionBehavior> SelectOptionBehavior for Planted<P> {
    fn select_option(
        &mut self,
        input: SelectOption,
    ) -> Result<SelectOptionOutcome, UnmetObligation> {
        self.inner.select_option(input)
    }
}

impl<P: TryContext> TryContext for Planted<P> {
    fn try_caller_session_id(&self) -> Result<Option<SessionId>, UnmetObligation> {
        self.inner.try_caller_session_id()
    }

    fn try_generate_optional_string(&mut self) -> Result<Option<String>, UnmetObligation> {
        self.inner.try_generate_optional_string()
    }

    fn try_generate_optional_bytes(&mut self) -> Result<Option<Vec<u8>>, UnmetObligation> {
        self.inner.try_generate_optional_bytes()
    }

    fn try_generate_optional_surf_net_media_type(
        &mut self,
    ) -> Result<Option<MediaType>, UnmetObligation> {
        self.inner.try_generate_optional_surf_net_media_type()
    }

    fn try_generate_surf_agent_action_id(&mut self) -> Result<ActionId, UnmetObligation> {
        self.inner.try_generate_surf_agent_action_id()
    }

    fn try_generate_surf_document_document_id(&mut self) -> Result<DocumentId, UnmetObligation> {
        self.inner.try_generate_surf_document_document_id()
    }

    fn try_generate_surf_document_element_id(&mut self) -> Result<ElementId, UnmetObligation> {
        self.inner.try_generate_surf_document_element_id()
    }

    fn try_generate_surf_document_form_id(&mut self) -> Result<FormId, UnmetObligation> {
        self.inner.try_generate_surf_document_form_id()
    }

    fn try_generate_surf_document_option_id(&mut self) -> Result<OptionId, UnmetObligation> {
        self.inner.try_generate_surf_document_option_id()
    }

    fn try_generate_surf_document_submission_id(
        &mut self,
    ) -> Result<SubmissionId, UnmetObligation> {
        self.inner.try_generate_surf_document_submission_id()
    }

    fn try_generate_surf_net_request_id(&mut self) -> Result<RequestId, UnmetObligation> {
        self.inner.try_generate_surf_net_request_id()
    }

    fn try_generate_surf_render_frame_id(&mut self) -> Result<FrameId, UnmetObligation> {
        self.inner.try_generate_surf_render_frame_id()
    }

    fn try_generate_surf_shell_tab_id(&mut self) -> Result<TabId, UnmetObligation> {
        self.inner.try_generate_surf_shell_tab_id()
    }

    fn try_generate_surf_shell_window_id(&mut self) -> Result<WindowId, UnmetObligation> {
        self.inner.try_generate_surf_shell_window_id()
    }
}

/// The scenarios of the suite that issue `AddScriptEnd`, admitted once per test.
fn script_end_suite() -> AdmittedSuite {
    let scripts = crate::adversary::scripts_suite();
    let admitted = admit(&scripts).unwrap_or_else(|error| panic!("{error}"));
    let ids: Vec<String> = admitted
        .suite()
        .scenarios
        .iter()
        .filter(|(_, scenario)| format!("{scenario:?}").contains("surf.document.AddScriptEnd"))
        .map(|(id, _)| id.to_string())
        .collect();
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    let text = narrowed(&admitted, "adversary_3", &ids).unwrap_or_else(|error| panic!("{error}"));
    admit(&text).unwrap_or_else(|error| panic!("{error}"))
}

/// The scenarios of `suite` that do not pass over the target with `plant`, as
/// `<status> <scenario>`; and the total.
fn not_passed(suite: &AdmittedSuite, plant: Plant) -> (usize, Vec<String>) {
    let target = SurfTarget::new(
        ImplementationIdentity::new("surf", format!("adversary_3 {plant:?}")),
        move |setup| {
            Browser::new(
                System::new(
                    agent_port(setup.agent_ports()),
                    document_port(Planted {
                        inner: setup.document_ports(),
                        plant,
                        restore: false,
                    }),
                    navigator_port(NavigatorPorts::new()),
                    net_port(NetPorts::new()),
                    renderer_port(RendererPorts::new()),
                    shell_port(ShellPorts::new()),
                ),
                Unanswered,
            )
        },
    );
    let run = Runner::for_suite(suite.suite()).run_admitted(suite, &target);
    let report = CountReport::from_run(&run, suite).expect("a report");
    let total = report.counts().total as usize;
    let failed: Vec<String> = report
        .statuses()
        .into_iter()
        .filter(|(_, status)| *status != "passed")
        .map(|(scenario, status)| format!("{status} {scenario}"))
        .collect();
    eprintln!(
        "adversary_3 [{plant:?}]: {} of {total} not passed: {failed:?}",
        failed.len()
    );
    (total, failed)
}

/// Each of `plants` over `suite`: the plants that no scenario named `by` fails, with what did
/// not pass over them.
fn unnoticed_by(suite: &AdmittedSuite, plants: &[Plant], by: &str) -> Vec<String> {
    plants
        .iter()
        .filter_map(|plant| {
            let (total, failed) = not_passed(suite, *plant);
            assert!(total > 0, "the narrowed suite holds no scenario");
            let noticed = failed.iter().any(|line| line.ends_with(by));
            (!noticed).then(|| format!("{plant:?} (not passed: {failed:?})"))
        })
        .collect()
}

/// Control: the narrowed suite holds the two scenarios that write `AggregateError`, and the
/// target with nothing planted passes every scenario, so a red test below is its plant's.
#[test]
fn adversary3_control_nothing_planted_passes_and_both_scenarios_are_there() {
    let suite = script_end_suite();
    let ids: Vec<String> = suite
        .suite()
        .scenarios
        .keys()
        .map(ToString::to_string)
        .collect();
    for name in [EVERY_KIND, EXCLUDED] {
        assert!(
            ids.iter().any(|id| id.ends_with(name)),
            "the narrowed suite lacks {name}: {ids:?}"
        );
    }
    let (total, failed) = not_passed(&suite, Plant::Nothing);
    assert!(total >= 10, "{total} scenarios");
    assert!(failed.is_empty(), "nothing planted: {failed:?}");
}

/// `plants` over the scenarios that issue `AddScriptEnd`: each fails
/// `document-records-every-error-kind`, which reads each kind back by its word.
fn assert_each_fails_every_kind(plants: &[Plant]) {
    let suite = script_end_suite();
    let missed = unnoticed_by(&suite, plants, EVERY_KIND);
    assert!(
        missed.is_empty(),
        "{} of {} plants pass {EVERY_KIND}: {missed:?}",
        missed.len(),
        plants.len()
    );
}

/// `AggregateError` stored as `Error`, as `Value` and as `UriError`.
#[test]
fn adversary3_aggregate_error_stored_as_error_value_or_uri_error_is_noticed() {
    assert_each_fails_every_kind(&[
        Plant::AggregateStoredAs(ScriptErrorKind::Error),
        Plant::AggregateStoredAs(ScriptErrorKind::Value),
        Plant::AggregateStoredAs(ScriptErrorKind::UriError),
    ]);
}

/// `AggregateError` stored as each of the five other kinds.
#[test]
fn adversary3_aggregate_error_stored_as_the_five_other_kinds_is_noticed() {
    assert_each_fails_every_kind(&[
        Plant::AggregateStoredAs(ScriptErrorKind::EvalError),
        Plant::AggregateStoredAs(ScriptErrorKind::RangeError),
        Plant::AggregateStoredAs(ScriptErrorKind::ReferenceError),
        Plant::AggregateStoredAs(ScriptErrorKind::SyntaxError),
        Plant::AggregateStoredAs(ScriptErrorKind::TypeError),
    ]);
}

/// Each of the first four other kinds stored as `AggregateError`.
#[test]
fn adversary3_first_four_kinds_stored_as_aggregate_error_are_noticed() {
    let plants: Vec<Plant> = OTHERS[..4]
        .iter()
        .map(|kind| Plant::StoredAsAggregate(*kind))
        .collect();
    assert_each_fails_every_kind(&plants);
}

/// Each of the last four other kinds stored as `AggregateError`.
#[test]
fn adversary3_last_four_kinds_stored_as_aggregate_error_are_noticed() {
    let plants: Vec<Plant> = OTHERS[4..]
        .iter()
        .map(|kind| Plant::StoredAsAggregate(*kind))
        .collect();
    assert_each_fails_every_kind(&plants);
}

/// `Threw` with `AggregateError` refused: `document-records-every-error-kind` adds it at
/// position 9 and fails.
#[test]
fn adversary3_threw_aggregate_error_refused_is_noticed() {
    let suite = script_end_suite();
    let missed = unnoticed_by(&suite, &[Plant::ThrewAggregateRefused], EVERY_KIND);
    assert!(missed.is_empty(), "{missed:?}");
}

/// `Refused` with `AggregateError` added: `adversary-script-ends-q7-excluded` (its position 71,
/// D420) expects `invalid-end` and fails.
#[test]
fn adversary3_refused_aggregate_error_added_is_noticed() {
    let suite = script_end_suite();
    let missed = unnoticed_by(&suite, &[Plant::RefusedAggregateAdded], EXCLUDED);
    assert!(missed.is_empty(), "{missed:?}");
}

/// `Threw` with `AggregateError` and no message refused. The record swell 0.9.0 gives for
/// `throw new AggregateError([])` has no message; a scenario that adds one fails this plant.
#[test]
fn adversary3_threw_aggregate_error_without_message_refused_is_noticed() {
    let suite = script_end_suite();
    let (total, failed) = not_passed(&suite, Plant::ThrewAggregateWithoutMessageRefused);
    assert!(total > 0, "the narrowed suite holds no scenario");
    assert!(
        !failed.is_empty(),
        "Threw AggregateError without a message answered invalid-end passes all {total} \
         scenarios that issue AddScriptEnd"
    );
}

/// A record of `AggregateError` without a message stored with the message `AggregateError`.
#[test]
fn adversary3_aggregate_error_message_invented_is_noticed() {
    let suite = script_end_suite();
    let (total, failed) = not_passed(&suite, Plant::AggregateMessageInvented);
    assert!(total > 0, "the narrowed suite holds no scenario");
    assert!(
        !failed.is_empty(),
        "an AggregateError without a message stored with the message \"AggregateError\" passes \
         all {total} scenarios that issue AddScriptEnd"
    );
}
