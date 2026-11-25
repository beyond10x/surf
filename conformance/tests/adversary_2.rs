//! Adversary of `story:scripts-spec`, second pass: the refusals `invalid-end` and
//! `invalid-position` of `surf.document.AddScriptEnd`, and the order of its refusals, planted in
//! the Rust target. The target meets `AddScriptEnd` by hand (`surf_ports::add_script_end`), so a
//! plant in the specification's guard reaches only the interpreter; here each plant is a ports
//! type whose `AddScriptEnd` answers as the specification's guard would with one term removed,
//! one bound moved by one, or two refusals swapped. Each runs over the scenarios that issue
//! `AddScriptEnd` (narrowed from the suite `adversary::scripts_suite` keeps on disk) and must
//! fail at least one of them. A red test names what every scenario lets through.

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
    AddScriptEnd, AddScriptEndOutcome, AnyScript, Check, CheckOutcome, DocumentId,
    DocumentNotFound, DocumentSnapshot, DocumentState, DocumentStateConflict, ElementId,
    ElementSnapshot, FormId, FormSnapshot, InvalidScriptEnd, InvalidScriptPosition,
    ListOptionSnapshot, OptionId, Script, ScriptData, ScriptEnd, ScriptEndAdded, ScriptErrorKind,
    ScriptId, ScriptReason, ScriptSnapshot, SelectOption, SelectOptionOutcome, SubmissionId,
    SubmissionSnapshot,
};
use surf_types::net::{MediaType, RequestId};
use surf_types::obligation::UnmetObligation;
use surf_types::render::FrameId;
use surf_types::shell::{TabId, WindowId};

/// 2^32: a line and a column are below it (Q16).
const SITE: i64 = 1 << 32;

/// The terms of `invalid-end` in `spec/domains/document.yaml`, each item of an inner `any:` its
/// own term, so that removing one leaves the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Term {
    /// `Ran` or `Faulted` with a reason, an error kind, a message, a line, a column, `dynamic`.
    Bare(Field),
    /// `Limit` or `NotRun` without a reason.
    NoReason,
    /// `Limit` or `NotRun` with an error kind, a message, a line, a column, `dynamic`.
    WordWith(Field),
    /// `Threw` or `Refused` with a reason.
    ErrorWithReason,
    /// `Threw` or `Refused` without a kind.
    ErrorWithoutKind,
    /// `Limit` with a word that is not a limit's.
    LimitWord,
    /// `NotRun` with this word of a limit alone.
    NotRunWord(ScriptReason),
    /// `Threw` `Value` with a message.
    ValueMessage,
    /// `Refused` with this kind, not `SyntaxError`.
    RefusedKind(ScriptErrorKind),
    /// A message over 1,024 characters.
    LongMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Reason,
    ErrorKind,
    Message,
    Line,
    Column,
    Dynamic,
}

const LIMIT_WORDS: [ScriptReason; 8] = [
    ScriptReason::StepsPerScript,
    ScriptReason::Memory,
    ScriptReason::SourceBytes,
    ScriptReason::WallClock,
    ScriptReason::Nodes,
    ScriptReason::Depth,
    ScriptReason::Attributes,
    ScriptReason::StoredBytes,
];

const LIMIT_ALONE: [ScriptReason; 6] = [
    ScriptReason::StepsPerScript,
    ScriptReason::SourceBytes,
    ScriptReason::Nodes,
    ScriptReason::Depth,
    ScriptReason::Attributes,
    ScriptReason::StoredBytes,
];

const NOT_SYNTAX: [ScriptErrorKind; 8] = [
    ScriptErrorKind::Error,
    ScriptErrorKind::EvalError,
    ScriptErrorKind::RangeError,
    ScriptErrorKind::ReferenceError,
    ScriptErrorKind::TypeError,
    ScriptErrorKind::UriError,
    ScriptErrorKind::AggregateError,
    ScriptErrorKind::Value,
];

const FIELDS: [Field; 6] = [
    Field::Reason,
    Field::ErrorKind,
    Field::Message,
    Field::Line,
    Field::Column,
    Field::Dynamic,
];

fn has(input: &AddScriptEnd, field: Field) -> bool {
    match field {
        Field::Reason => input.reason.is_some(),
        Field::ErrorKind => input.error_kind.is_some(),
        Field::Message => input.message.is_some(),
        Field::Line => input.line.is_some(),
        Field::Column => input.column.is_some(),
        Field::Dynamic => input.dynamic.is_some(),
    }
}

/// Every term of `invalid-end`.
fn terms() -> Vec<Term> {
    let mut all: Vec<Term> = FIELDS.iter().map(|field| Term::Bare(*field)).collect();
    all.push(Term::NoReason);
    all.extend(FIELDS[1..].iter().map(|field| Term::WordWith(*field)));
    all.extend([
        Term::ErrorWithReason,
        Term::ErrorWithoutKind,
        Term::LimitWord,
    ]);
    all.extend(LIMIT_ALONE.iter().map(|word| Term::NotRunWord(*word)));
    all.push(Term::ValueMessage);
    all.extend(NOT_SYNTAX[..7].iter().map(|kind| Term::RefusedKind(*kind)));
    all.push(Term::RefusedKind(ScriptErrorKind::Value));
    all.push(Term::LongMessage);
    all.dedup();
    all
}

/// Whether `term` holds for `input`, with the message bound `bound`.
fn holds(term: Term, input: &AddScriptEnd, bound: usize) -> bool {
    let end = input.end;
    match term {
        Term::Bare(field) => {
            matches!(end, ScriptEnd::Ran | ScriptEnd::Faulted) && has(input, field)
        }
        Term::NoReason => {
            matches!(end, ScriptEnd::Limit | ScriptEnd::NotRun) && input.reason.is_none()
        }
        Term::WordWith(field) => {
            matches!(end, ScriptEnd::Limit | ScriptEnd::NotRun) && has(input, field)
        }
        Term::ErrorWithReason => {
            matches!(end, ScriptEnd::Threw | ScriptEnd::Refused) && input.reason.is_some()
        }
        Term::ErrorWithoutKind => {
            matches!(end, ScriptEnd::Threw | ScriptEnd::Refused) && input.error_kind.is_none()
        }
        Term::LimitWord => {
            end == ScriptEnd::Limit
                && input
                    .reason
                    .is_some_and(|reason| !LIMIT_WORDS.contains(&reason))
        }
        Term::NotRunWord(word) => end == ScriptEnd::NotRun && input.reason == Some(word),
        Term::ValueMessage => {
            end == ScriptEnd::Threw
                && input.error_kind == Some(ScriptErrorKind::Value)
                && input.message.is_some()
        }
        Term::RefusedKind(kind) => end == ScriptEnd::Refused && input.error_kind == Some(kind),
        Term::LongMessage => input
            .message
            .as_ref()
            .is_some_and(|message| message.chars().count() > bound),
    }
}

/// The bounds of `invalid-position`: (position from, line and column from, line and column
/// below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Bounds {
    position_from: i64,
    line_from: i64,
    line_below: i64,
    column_from: i64,
    column_below: i64,
    message_most: usize,
}

const BOUNDS: Bounds = Bounds {
    position_from: 1,
    line_from: 1,
    line_below: SITE,
    column_from: 1,
    column_below: SITE,
    message_most: 1024,
};

/// The order the target answers its refusals in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Order {
    /// The specification's: `invalid-end`, `invalid-position`, `no-such-document`,
    /// `already-parsed`.
    Declared,
    /// `invalid-position` before `invalid-end`.
    PositionFirst,
    /// The document's existence and state before the input's refusals.
    DocumentFirst,
    /// `already-parsed` of a document that exists before the input's refusals.
    ParsedFirst,
    /// `already-parsed` of a document that exists between `invalid-end` and
    /// `invalid-position`.
    ParsedBeforePosition,
}

/// One plant of the Rust target's `AddScriptEnd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Plant {
    real: bool,
    without: Option<Term>,
    bounds: Bounds,
    order: Order,
}

const NOTHING: Plant = Plant {
    real: false,
    without: None,
    bounds: BOUNDS,
    order: Order::Declared,
};

/// The target as built: `surf_ports::add_script_end`.
const REAL: Plant = Plant {
    real: true,
    ..NOTHING
};

/// The `document` component's ports with `AddScriptEnd` answered by [`Plant`]; everything else
/// goes to the ports it wraps.
struct Planted<P> {
    inner: P,
    plant: Plant,
}

impl<P: ScriptIdentity + DocumentStorage + ScriptStorage> AddScriptEndBehavior for Planted<P> {
    fn add_script_end(
        &mut self,
        input: AddScriptEnd,
    ) -> Result<AddScriptEndOutcome, UnmetObligation> {
        if self.plant.real {
            return add_script_end(&mut self.inner, input);
        }
        let plant = self.plant;
        let bounds = plant.bounds;
        let invalid_end = terms()
            .into_iter()
            .filter(|term| Some(*term) != plant.without)
            .any(|term| holds(term, &input, bounds.message_most));
        let site = |value: Option<i64>, from: i64, below: i64| {
            value.is_some_and(|value| value < from || value >= below)
        };
        let invalid_position = input.position < bounds.position_from
            || site(input.line, bounds.line_from, bounds.line_below)
            || site(input.column, bounds.column_from, bounds.column_below);
        let document = DocumentStorage::get(&self.inner, &input.document_id);
        let parsed = document
            .as_ref()
            .is_some_and(|document| document.state == DocumentState::Parsed);
        let end = || {
            Ok(AddScriptEndOutcome::InvalidEnd {
                error: InvalidScriptEnd { end: input.end },
            })
        };
        let position = || {
            Ok(AddScriptEndOutcome::InvalidPosition {
                error: InvalidScriptPosition {
                    position: input.position,
                },
            })
        };
        let missing = || {
            Ok(AddScriptEndOutcome::NoSuchDocument {
                error: DocumentNotFound {
                    document_id: input.document_id.clone(),
                },
            })
        };
        let conflict = || {
            Ok(AddScriptEndOutcome::AlreadyParsed {
                error: DocumentStateConflict {
                    state: DocumentState::Parsed,
                },
            })
        };
        let answer = match plant.order {
            Order::Declared if invalid_end => Some(end()),
            Order::Declared if invalid_position => Some(position()),
            Order::PositionFirst if invalid_position => Some(position()),
            Order::PositionFirst if invalid_end => Some(end()),
            Order::DocumentFirst if document.is_none() => Some(missing()),
            Order::DocumentFirst if parsed => Some(conflict()),
            Order::DocumentFirst if invalid_end => Some(end()),
            Order::DocumentFirst if invalid_position => Some(position()),
            Order::ParsedFirst if parsed => Some(conflict()),
            Order::ParsedFirst if invalid_end => Some(end()),
            Order::ParsedFirst if invalid_position => Some(position()),
            Order::ParsedBeforePosition if invalid_end => Some(end()),
            Order::ParsedBeforePosition if parsed => Some(conflict()),
            Order::ParsedBeforePosition if invalid_position => Some(position()),
            _ => None,
        };
        if let Some(answer) = answer {
            return answer;
        }
        if document.is_none() {
            return missing();
        }
        if parsed {
            return conflict();
        }
        let script_id = self.inner.assign_script_id()?;
        let added = ScriptEndAdded {
            script_id: script_id.clone(),
            document_id: input.document_id.clone(),
            position: input.position,
            end: input.end,
        };
        let data = ScriptData {
            script_id,
            document_id: input.document_id,
            position: input.position,
            end: input.end,
            reason: input.reason,
            error_kind: input.error_kind,
            message: input.message,
            line: input.line,
            column: input.column,
            dynamic: input.dynamic,
        };
        ScriptStorage::put(
            &mut self.inner,
            AnyScript::Added(Script::new(data)).snapshot(),
        );
        Ok(AddScriptEndOutcome::Added {
            script_end_added: added,
        })
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
through_storage!(ScriptStorage, ScriptSnapshot, ScriptId);

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
    let text = narrowed(&admitted, "adversary_2", &ids).unwrap_or_else(|error| panic!("{error}"));
    admit(&text).unwrap_or_else(|error| panic!("{error}"))
}

/// The scenarios of `suite` that do not pass over the target with `plant`.
fn failed(suite: &AdmittedSuite, plant: Plant) -> (usize, Vec<String>) {
    let target = SurfTarget::new(
        ImplementationIdentity::new("surf", format!("adversary_2 {plant:?}")),
        move |setup| {
            Browser::new(
                System::new(
                    agent_port(setup.agent_ports()),
                    document_port(Planted {
                        inner: setup.document_ports(),
                        plant,
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
    let failed = report
        .statuses()
        .into_iter()
        .filter(|(_, status)| *status != "passed")
        .map(|(scenario, status)| format!("{status} {scenario}"))
        .collect();
    (total, failed)
}

/// Each of `plants` over the scenarios that issue `AddScriptEnd`: the plants every one passes.
fn unnoticed(plants: &[(String, Plant)]) -> Vec<String> {
    let suite = script_end_suite();
    plants
        .iter()
        .filter_map(|(label, plant)| {
            let (total, failed) = failed(&suite, *plant);
            assert!(total > 0, "the narrowed suite holds no scenario");
            eprintln!(
                "adversary_2 [{label}]: {} of {total} not passed",
                failed.len()
            );
            failed.is_empty().then(|| label.clone())
        })
        .collect()
}

/// Control: `surf_ports::add_script_end` and the model of the guard with nothing planted both
/// pass every scenario that issues `AddScriptEnd`, so a red test below is its plant's.
#[test]
fn adversary2_control_the_real_target_and_the_unplanted_model_pass() {
    let suite = script_end_suite();
    let (total, real) = failed(&suite, REAL);
    assert!(total >= 10, "{total} scenarios");
    assert!(real.is_empty(), "the real target: {real:?}");
    let (_, model) = failed(&suite, NOTHING);
    assert!(model.is_empty(), "the unplanted model: {model:?}");
}
/// Each term of `invalid-end` removed alone, one at a time: a target that accepts that one pair
/// must fail a scenario.
#[test]
fn adversary2_each_term_of_invalid_end_removed_is_noticed() {
    let plants: Vec<(String, Plant)> = terms()
        .into_iter()
        .map(|term| {
            (
                format!("without {term:?}"),
                Plant {
                    without: Some(term),
                    ..NOTHING
                },
            )
        })
        .collect();
    let missed = unnoticed(&plants);
    assert!(
        missed.is_empty(),
        "{} of {} terms of invalid-end removed pass every scenario: {missed:?}",
        missed.len(),
        plants.len()
    );
}

/// Each bound of `invalid-position`, and the message's 1,024 characters, moved by one each way:
/// a target that refuses a valid value at the edge, or adds the first invalid one, must fail a
/// scenario.
#[test]
fn adversary2_each_bound_moved_by_one_is_noticed() {
    let moved = |label: &str, bounds: Bounds| (label.to_owned(), Plant { bounds, ..NOTHING });
    let plants = vec![
        moved(
            "position from 0",
            Bounds {
                position_from: 0,
                ..BOUNDS
            },
        ),
        moved(
            "position from 2",
            Bounds {
                position_from: 2,
                ..BOUNDS
            },
        ),
        moved(
            "line from 0",
            Bounds {
                line_from: 0,
                ..BOUNDS
            },
        ),
        moved(
            "line from 2",
            Bounds {
                line_from: 2,
                ..BOUNDS
            },
        ),
        moved(
            "line below 2^32 - 1",
            Bounds {
                line_below: SITE - 1,
                ..BOUNDS
            },
        ),
        moved(
            "line below 2^32 + 1",
            Bounds {
                line_below: SITE + 1,
                ..BOUNDS
            },
        ),
        moved(
            "column from 0",
            Bounds {
                column_from: 0,
                ..BOUNDS
            },
        ),
        moved(
            "column from 2",
            Bounds {
                column_from: 2,
                ..BOUNDS
            },
        ),
        moved(
            "column below 2^32 - 1",
            Bounds {
                column_below: SITE - 1,
                ..BOUNDS
            },
        ),
        moved(
            "column below 2^32 + 1",
            Bounds {
                column_below: SITE + 1,
                ..BOUNDS
            },
        ),
        moved(
            "message at most 1023",
            Bounds {
                message_most: 1023,
                ..BOUNDS
            },
        ),
        moved(
            "message at most 1025",
            Bounds {
                message_most: 1025,
                ..BOUNDS
            },
        ),
    ];
    let missed = unnoticed(&plants);
    assert!(
        missed.is_empty(),
        "{} of {} bounds moved by one pass every scenario: {missed:?}",
        missed.len(),
        plants.len()
    );
}

/// The specification answers `invalid-end`, then `invalid-position`, then `no-such-document`,
/// then `already-parsed`. Three other orders each fail a scenario.
#[test]
fn adversary2_refusals_in_another_order_are_noticed() {
    let plants: Vec<(String, Plant)> = [
        Order::PositionFirst,
        Order::DocumentFirst,
        Order::ParsedFirst,
    ]
    .into_iter()
    .map(|order| (format!("{order:?}"), Plant { order, ..NOTHING }))
    .collect();
    let missed = unnoticed(&plants);
    assert!(
        missed.is_empty(),
        "orders that pass every scenario: {missed:?}"
    );
}

/// `invalid-position` answers before `already-parsed`: a position below 1 on a document already
/// `Parsed` is `invalid-position`. `document-script-end-refusals-in-order` holds the order
/// against `no-such-document` and holds `invalid-end` against `already-parsed`, not
/// `invalid-position` against `already-parsed`.
#[test]
fn adversary2_already_parsed_before_invalid_position_is_noticed() {
    let missed = unnoticed(&[(
        "ParsedBeforePosition".to_owned(),
        Plant {
            order: Order::ParsedBeforePosition,
            ..NOTHING
        },
    )]);
    assert!(
        missed.is_empty(),
        "already-parsed answered before invalid-position passes every scenario"
    );
}
