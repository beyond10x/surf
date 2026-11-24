//! Adversary of `story:scripts-spec`: defects planted in the stored record of a script's end
//! (`surf.document.Script`) and in the stored option of a session (`surf.agent.Session.scripts`),
//! behind the generated behaviours, one per test. Each test asserts what a green
//! `task conform-rust` is taken to mean for this story: the suite notices the defect, so at
//! least one scenario does not pass. A red test names a defect every scenario lets through.
//!
//! Every defect lives in this file, in a storage port the generated behaviours write and read;
//! nothing under `crates/`, `generated/` or `conformance/src` is touched. A defect in these two
//! tables changes only a scenario that writes or reads a script's end or a session, so each test
//! runs over those scenarios of `.out/suite.json` alone: the suite narrowed to them is made once
//! per suite and kept on disk under `conformance/target/adversary/`, keyed by a digest of the
//! suite's bytes, never once per test.

use std::hash::{DefaultHasher, Hasher};
use std::path::Path;

use ess_conformance::target::ImplementationIdentity;
use surf::{Browser, System};
use surf_conformance::{SurfTarget, Unanswered, admit, narrowed, run, whole_suite};
use surf_ports::{
    NavigatorPorts, NetPorts, RendererPorts, ShellPorts, agent_port, document_port, navigator_port,
    net_port, renderer_port, shell_port,
};
use surf_types::agent::obligations::{
    NoteNavigationCommittedBehavior, NoteNavigationEndedBehavior, NoteNavigationFailedBehavior,
    NoteNavigationStartedBehavior,
};
use surf_types::agent::{
    ActionId, ActionSnapshot, NoteNavigationCommitted, NoteNavigationCommittedOutcome,
    NoteNavigationEnded, NoteNavigationEndedOutcome, NoteNavigationFailed,
    NoteNavigationFailedOutcome, NoteNavigationStarted, NoteNavigationStartedOutcome, SessionId,
    SessionSnapshot,
};
use surf_types::behaviour::{
    ActionStorage, DocumentStorage, ElementStorage, FormStorage, ListOptionStorage, ScriptStorage,
    SessionStorage, SubmissionStorage, TryContext,
};
use surf_types::document::obligations::{CheckBehavior, SelectOptionBehavior};
use surf_types::document::{
    Check, CheckOutcome, DocumentId, DocumentSnapshot, ElementId, ElementSnapshot, FormId,
    FormSnapshot, ListOptionSnapshot, OptionId, ScriptErrorKind, ScriptId, ScriptReason,
    ScriptSnapshot, SelectOption, SelectOptionOutcome, SubmissionId, SubmissionSnapshot,
};
use surf_types::net::{MediaType, RequestId};
use surf_types::obligation::UnmetObligation;
use surf_types::primitives::Uuid;
use surf_types::render::FrameId;
use surf_types::shell::{TabId, WindowId};

/// Where the narrowed suite is kept, one file per digest of `.out/suite.json`.
const CACHE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/adversary");

/// A scenario is kept in the narrowed suite when its text names one of these: the command and
/// the view of a script's end, and the command, the view and the field of the option.
const MARKS: &[&str] = &[
    "surf.document.AddScriptEnd",
    "surf.document.Scripts",
    "surf.agent.OpenSession",
    "surf.agent.Sessions",
    "\"scripts\"",
];

/// The scenarios of `.out/suite.json` that write or read a script's end or a session's option,
/// as a suite: made from the whole suite once per digest of its bytes, then read from disk.
fn scripts_suite() -> String {
    let whole = whole_suite().unwrap_or_else(|error| panic!("{error}"));
    let mut hasher = DefaultHasher::new();
    hasher.write(whole.as_bytes());
    let path = Path::new(CACHE).join(format!("{:016x}-{}.json", hasher.finish(), whole.len()));
    if let Ok(text) = std::fs::read_to_string(&path) {
        return text;
    }
    let admitted = admit(&whole).unwrap_or_else(|error| panic!("{error}"));
    let ids: Vec<String> = admitted
        .suite()
        .scenarios
        .iter()
        .filter(|(_, scenario)| {
            let text = format!("{scenario:?}");
            MARKS.iter().any(|mark| text.contains(mark))
        })
        .map(|(id, _)| id.to_string())
        .collect();
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    let text = narrowed(&admitted, "adversary", &ids).unwrap_or_else(|error| panic!("{error}"));
    std::fs::create_dir_all(CACHE).unwrap_or_else(|error| panic!("{CACHE}: {error}"));
    let partial = path.with_extension(format!("{}.partial", std::process::id()));
    std::fs::write(&partial, &text).unwrap_or_else(|error| panic!("{error}"));
    std::fs::rename(&partial, &path).unwrap_or_else(|error| panic!("{error}"));
    text
}

/// One defect, planted in the `document` or the `agent` component's storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Defect {
    /// Nothing planted: the control.
    Nothing,
    /// The reason `WallClock` is stored as `Memory`: two reasons, one stored value.
    WallClockStoredAsMemory,
    /// `dynamic: true` is stored as `false`.
    DynamicStoredFalse,
    /// The line and the column of an error are stored swapped.
    LineAndColumnSwapped,
    /// Each record is stored under another document than the one named.
    RecordOnAnotherDocument,
    /// The records' storage lists all but its last row.
    ScriptsDropLastRow,
    /// Every session reads back the opposite of the `scripts` it was opened with.
    ScriptsInverted,
    /// The error kind `UriError` is stored as `Error`: two kinds, one stored value.
    UriErrorStoredAsError,
    /// Every session but the first one opened reads back the first one's `scripts`.
    ScriptsOfTheFirstSession,
}

/// A ports type with one [`Defect`] planted.
struct Planted<P> {
    inner: P,
    defect: Defect,
    /// The first session put, with the `scripts` it was put with.
    first: Option<(SessionId, bool)>,
}

impl<P> Planted<P> {
    fn new(inner: P, defect: Defect) -> Self {
        Self {
            inner,
            defect,
            first: None,
        }
    }

    fn session(&self, mut snapshot: SessionSnapshot) -> SessionSnapshot {
        match self.defect {
            Defect::ScriptsInverted => snapshot.data.scripts = !snapshot.data.scripts,
            Defect::ScriptsOfTheFirstSession => {
                if let Some((first, scripts)) = &self.first
                    && *first != snapshot.data.session_id
                {
                    snapshot.data.scripts = *scripts;
                }
            }
            _ => {}
        }
        snapshot
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
through_storage!(ActionStorage, ActionSnapshot, ActionId);

impl<P: ScriptStorage> ScriptStorage for Planted<P> {
    fn get(&self, identity: &ScriptId) -> Option<ScriptSnapshot> {
        ScriptStorage::get(&self.inner, identity)
    }

    fn put(&mut self, mut snapshot: ScriptSnapshot) {
        let data = &mut snapshot.data;
        match self.defect {
            Defect::WallClockStoredAsMemory if data.reason == Some(ScriptReason::WallClock) => {
                data.reason = Some(ScriptReason::Memory);
            }
            Defect::UriErrorStoredAsError if data.error_kind == Some(ScriptErrorKind::UriError) => {
                data.error_kind = Some(ScriptErrorKind::Error);
            }
            Defect::DynamicStoredFalse if data.dynamic == Some(true) => {
                data.dynamic = Some(false);
            }
            Defect::LineAndColumnSwapped => std::mem::swap(&mut data.line, &mut data.column),
            Defect::RecordOnAnotherDocument => {
                data.document_id = DocumentId(Uuid("5c41e7d0-0000-4000-8000-0000000000ff".into()));
            }
            _ => {}
        }
        ScriptStorage::put(&mut self.inner, snapshot);
    }

    fn delete(&mut self, identity: &ScriptId) {
        ScriptStorage::delete(&mut self.inner, identity);
    }

    fn list(&self) -> Vec<ScriptSnapshot> {
        let mut rows = ScriptStorage::list(&self.inner);
        if self.defect == Defect::ScriptsDropLastRow {
            rows.pop();
        }
        rows
    }
}

impl<P: SessionStorage> SessionStorage for Planted<P> {
    fn get(&self, identity: &SessionId) -> Option<SessionSnapshot> {
        SessionStorage::get(&self.inner, identity).map(|snapshot| self.session(snapshot))
    }

    fn put(&mut self, snapshot: SessionSnapshot) {
        if self.first.is_none() {
            self.first = Some((snapshot.data.session_id.clone(), snapshot.data.scripts));
        }
        SessionStorage::put(&mut self.inner, snapshot);
    }

    fn delete(&mut self, identity: &SessionId) {
        SessionStorage::delete(&mut self.inner, identity);
    }

    fn list(&self) -> Vec<SessionSnapshot> {
        SessionStorage::list(&self.inner)
            .into_iter()
            .map(|snapshot| self.session(snapshot))
            .collect()
    }
}

macro_rules! through_obligation {
    ($behavior:ident, $method:ident, $input:ty, $outcome:ty) => {
        impl<P: $behavior> $behavior for Planted<P> {
            fn $method(&mut self, input: $input) -> Result<$outcome, UnmetObligation> {
                self.inner.$method(input)
            }
        }
    };
}

through_obligation!(
    NoteNavigationStartedBehavior,
    note_navigation_started,
    NoteNavigationStarted,
    NoteNavigationStartedOutcome
);
through_obligation!(
    NoteNavigationCommittedBehavior,
    note_navigation_committed,
    NoteNavigationCommitted,
    NoteNavigationCommittedOutcome
);
through_obligation!(
    NoteNavigationFailedBehavior,
    note_navigation_failed,
    NoteNavigationFailed,
    NoteNavigationFailedOutcome
);
through_obligation!(
    NoteNavigationEndedBehavior,
    note_navigation_ended,
    NoteNavigationEnded,
    NoteNavigationEndedOutcome
);
through_obligation!(CheckBehavior, check, Check, CheckOutcome);
through_obligation!(
    SelectOptionBehavior,
    select_option,
    SelectOption,
    SelectOptionOutcome
);

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

    fn try_generate_surf_document_script_id(&mut self) -> Result<ScriptId, UnmetObligation> {
        self.inner.try_generate_surf_document_script_id()
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

/// The scenarios of [`scripts_suite`] over the system with `defect` planted in the `agent` and
/// the `document` component's ports: (total, the scenarios that did not pass).
fn over(defect: Defect) -> (usize, Vec<String>) {
    let suite = scripts_suite();
    let target = SurfTarget::new(
        ImplementationIdentity::new("surf", format!("adversary {defect:?}")),
        move |setup| {
            Browser::new(
                System::new(
                    agent_port(Planted::new(setup.agent_ports(), defect)),
                    document_port(Planted::new(setup.document_ports(), defect)),
                    navigator_port(NavigatorPorts::new()),
                    net_port(NetPorts::new()),
                    renderer_port(RendererPorts::new()),
                    shell_port(ShellPorts::new()),
                ),
                Unanswered,
            )
        },
    );
    let conformance = run(&suite, &target).expect("the suite runs");
    let counts = conformance.report.counts();
    let not_passed: Vec<String> = conformance
        .report
        .statuses()
        .into_iter()
        .filter(|(_, status)| *status != "passed")
        .map(|(scenario, status)| format!("{status} {scenario}"))
        .collect();
    eprintln!(
        "adversary [{defect:?}]: {} scenarios: {} passed, {} failed, {} error, {} unsupported",
        counts.total, counts.passed, counts.failed, counts.error, counts.unsupported
    );
    (counts.total as usize, not_passed)
}

fn assert_noticed(defect: Defect) {
    let (total, not_passed) = over(defect);
    assert!(
        total > 0,
        "[{defect:?}] the narrowed suite holds no scenario"
    );
    assert!(
        !not_passed.is_empty(),
        "{defect:?}: all {total} scenarios that write or read a script's end or a session \
         passed over a system with this defect"
    );
}

/// Control: the narrowed suite holds the story's scenarios, and the unplanted system passes
/// each, so a red test below is its defect's and not the harness's.
#[test]
fn adversary_control_the_unplanted_system_passes_the_narrowed_suite() {
    let suite = scripts_suite();
    for name in [
        "document-records-script-ends",
        "document-refuses-script-end-after-parsing",
        "document-refuses-script-end-of-unknown-document",
        "agent-open-session-with-scripts",
        "agent-open-session-without-scripts",
        "two-of-each-entity",
        "surf.document.AddScriptEnd/outcome/added",
    ] {
        assert!(suite.contains(name), "the narrowed suite lacks {name}");
    }
    let (total, not_passed) = over(Defect::Nothing);
    assert!(total > 0 && not_passed.is_empty(), "{not_passed:?}");
}

/// Held since `adversary-script-ends-every-reason`: Q7 has 23 reason words, and
/// `document-records-script-ends` writes two (`StepsPerScript`, `Eof`); that scenario writes
/// each, so a store that keeps `WallClock` as `Memory` answers a wrong row there.
#[test]
fn adversary_wall_clock_stored_as_memory_is_noticed() {
    assert_noticed(Defect::WallClockStoredAsMemory);
}

/// Held: Q16's `dynamic` (the line and column count in `eval` or `Function` code). The authored
/// scenarios write only `dynamic: false`; the synthesized `AddScriptEnd/outcome/added` writes
/// `true` and reads it back.
#[test]
fn adversary_dynamic_true_stored_false_is_noticed() {
    assert_noticed(Defect::DynamicStoredFalse);
}

/// Q7 and Q16 name eight error kinds; the scenarios write five (`TypeError`, `SyntaxError`,
/// `Error`, `RangeError`, `Value`) and none writes `URIError`, `EvalError` or
/// `ReferenceError`. A store that keeps `URIError` as `Error` (the note's wildcard arm,
/// misapplied) answers a wrong row that no scenario reads.
#[test]
fn adversary_uri_error_stored_as_error_is_noticed() {
    assert_noticed(Defect::UriErrorStoredAsError);
}

/// Q7, the option: one value per session. No scenario opens two sessions with different values
/// (`two-of-each-entity` opens two, both `false`), so the value of the wrong session of two is
/// not seen.
#[test]
fn adversary_scripts_of_the_first_session_read_on_the_second_is_noticed() {
    assert_noticed(Defect::ScriptsOfTheFirstSession);
}

/// `surf.document.Scripts` declares `order_by: [position asc]`. Synthesis refuses the scenario
/// that would check it (`ESS-SYNTH-014` on `AddScriptEnd/added`, `REFUSALS_MAX` 57), and
/// `AGENTS.md` says `document-records-script-ends` covers it, "read back in order"; that
/// scenario reads its rows with `contains` and `counts` only. Some scenario of the suite must
/// expect the view's rows ranked by its order, as `parsed-page-with-title-links-and-form` does
/// for `surf.document.Elements`; else a view answering its rows in any order passes.
#[test]
fn adversary_some_scenario_expects_the_scripts_view_ranked() {
    let compact: String = whole_suite()
        .unwrap_or_else(|error| panic!("{error}"))
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let ranked =
        |view: &str| format!("\"view\":\"{view}\",\"expectation\":{{\"expect\":\"ranked\"");
    assert!(
        compact.contains(&ranked("surf.document.Elements")),
        "the form this test reads is not the suite's: no ranked check of Elements found"
    );
    assert!(
        compact.contains(&ranked("surf.document.Scripts")),
        "no scenario of the suite expects surf.document.Scripts ranked by position"
    );
}

/// Held: `document-records-script-ends` writes line 3, column 5.
#[test]
fn adversary_line_and_column_swapped_is_noticed() {
    assert_noticed(Defect::LineAndColumnSwapped);
}

/// Held: the view of the named document answers no row.
#[test]
fn adversary_record_on_another_document_is_noticed() {
    assert_noticed(Defect::RecordOnAnotherDocument);
}

/// Held: `document-records-script-ends` counts six rows.
#[test]
fn adversary_scripts_view_one_row_short_is_noticed() {
    assert_noticed(Defect::ScriptsDropLastRow);
}

/// Held: one scenario opens with `true`, one with `false`.
#[test]
fn adversary_scripts_inverted_is_noticed() {
    assert_noticed(Defect::ScriptsInverted);
}
