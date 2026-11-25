//! `surf.document.AddScriptEnd`, met by hand: its `invalid-end` refusal reads `message.count`, and
//! the generation keeps a command with such a guard an obligation.

use surf_types::behaviour::{DocumentStorage, ScriptStorage};
use surf_types::document::obligations::AddScriptEndBehavior;
use surf_types::document::{
    AddScriptEnd, AddScriptEndOutcome, AnyScript, DocumentNotFound, DocumentState,
    DocumentStateConflict, InvalidScriptEnd, InvalidScriptPosition, Script, ScriptData, ScriptEnd,
    ScriptEndAdded, ScriptErrorKind, ScriptId, ScriptReason,
};
use surf_types::obligation::UnmetObligation;

use crate::{DocumentPorts, RandomSource};

/// The most characters (Unicode scalar values) of a message a record keeps
/// (`docs/design/scripts.md`, Q16).
const MESSAGE_CHARACTERS: usize = 1024;

/// A line and a column are below 2^32 (`docs/design/scripts.md`, Q16).
const SITE_BOUND: i64 = 1 << 32;

/// The identity of a script end's record, assigned when it is added. `TryContext` does not ask
/// for it: the generation asks only for the identities its own behaviours mint, and
/// `AddScriptEnd` is not one of them.
pub trait ScriptIdentity {
    /// A new identity for the record of a script's end.
    ///
    /// # Errors
    ///
    /// An [`UnmetObligation`] when no identity can be assigned.
    fn assign_script_id(&mut self) -> Result<ScriptId, UnmetObligation>;
}

/// The end of one script element of a document being parsed, its outcomes chosen in the order
/// `spec/` declares them and the interpreter answers them: the input's own refusals first
/// (`invalid-end`, then `invalid-position`), then a document no row holds (`no-such-document`),
/// and one already `Parsed` (`already-parsed`); otherwise the record is added with an identity
/// from `ports`.
///
/// It reads and writes through `ports`' storage traits only, so a ports type that wraps a
/// [`DocumentStorage`] and a [`ScriptStorage`] meets the obligation over its own storage.
///
/// # Errors
///
/// The [`UnmetObligation`] of an identity `ports` cannot assign; nothing is stored then.
pub fn add_script_end<P: ScriptIdentity + DocumentStorage + ScriptStorage + ?Sized>(
    ports: &mut P,
    input: AddScriptEnd,
) -> Result<AddScriptEndOutcome, UnmetObligation> {
    if invalid_end(&input) {
        return Ok(AddScriptEndOutcome::InvalidEnd {
            error: InvalidScriptEnd { end: input.end },
        });
    }
    if invalid_position(&input) {
        return Ok(AddScriptEndOutcome::InvalidPosition {
            error: InvalidScriptPosition {
                position: input.position,
            },
        });
    }
    let Some(document) = DocumentStorage::get(ports, &input.document_id) else {
        return Ok(AddScriptEndOutcome::NoSuchDocument {
            error: DocumentNotFound {
                document_id: input.document_id,
            },
        });
    };
    if document.state == DocumentState::Parsed {
        return Ok(AddScriptEndOutcome::AlreadyParsed {
            error: DocumentStateConflict {
                state: DocumentState::Parsed,
            },
        });
    }
    let script_id = ports.assign_script_id()?;
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
    ScriptStorage::put(ports, AnyScript::Added(Script::new(data)).snapshot());
    Ok(AddScriptEndOutcome::Added {
        script_end_added: added,
    })
}

/// `invalid-end`: Q7's closed list. `Ran` and `Faulted` carry nothing; `Limit` has one of the
/// eight limit words and `NotRun` one of the seventeen not-run words; `Threw` has a kind, and a
/// thrown value that is no error object (`Value`) no message; `Refused` has the `SyntaxError` of
/// the refusal. Only `Threw` and `Refused` carry an error's fields, and no message is longer
/// than Q16 keeps.
fn invalid_end(input: &AddScriptEnd) -> bool {
    let error = input.error_kind.is_some()
        || input.message.is_some()
        || input.line.is_some()
        || input.column.is_some()
        || input.dynamic.is_some();
    let paired = match input.end {
        ScriptEnd::Ran | ScriptEnd::Faulted => input.reason.is_none() && !error,
        ScriptEnd::Limit => input.reason.is_some_and(limit_word) && !error,
        ScriptEnd::NotRun => input.reason.is_some_and(not_run_word) && !error,
        ScriptEnd::Threw => {
            input.reason.is_none()
                && input.error_kind.is_some()
                && !(input.error_kind == Some(ScriptErrorKind::Value) && input.message.is_some())
        }
        ScriptEnd::Refused => {
            input.reason.is_none() && input.error_kind == Some(ScriptErrorKind::SyntaxError)
        }
    };
    let too_long = input
        .message
        .as_ref()
        .is_some_and(|message| message.chars().count() > MESSAGE_CHARACTERS);
    !paired || too_long
}

/// The words of Q4's column `outcome when reached` that end a running script (`Limit`).
fn limit_word(reason: ScriptReason) -> bool {
    matches!(
        reason,
        ScriptReason::StepsPerScript
            | ScriptReason::Memory
            | ScriptReason::SourceBytes
            | ScriptReason::WallClock
            | ScriptReason::Nodes
            | ScriptReason::Depth
            | ScriptReason::Attributes
            | ScriptReason::StoredBytes
    )
}

/// The words of Q1's line `reasons:` and Q4 that keep a script from running (`NotRun`):
/// every word but the six of a limit alone (`WallClock` and `Memory` are both).
fn not_run_word(reason: ScriptReason) -> bool {
    !matches!(
        reason,
        ScriptReason::StepsPerScript
            | ScriptReason::SourceBytes
            | ScriptReason::Nodes
            | ScriptReason::Depth
            | ScriptReason::Attributes
            | ScriptReason::StoredBytes
    )
}

/// `invalid-position`: a position from 1 (Q7), a line and a column from 1 and below 2^32 (Q16).
fn invalid_position(input: &AddScriptEnd) -> bool {
    let outside = |value: Option<i64>| value.is_some_and(|value| !(1..SITE_BOUND).contains(&value));
    input.position < 1 || outside(input.line) || outside(input.column)
}

impl<R: RandomSource> AddScriptEndBehavior for DocumentPorts<R> {
    fn add_script_end(
        &mut self,
        input: AddScriptEnd,
    ) -> Result<AddScriptEndOutcome, UnmetObligation> {
        add_script_end(self, input)
    }
}
