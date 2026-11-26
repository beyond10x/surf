//! The part of `bindings.rs` that is the same for every selection: the context type, the
//! wrapper hook, the limit result, `DOMException`'s Rust side, the conversions of Web IDL over
//! swell's host interface, the one counting copy function and the realm's setup over the
//! generated tables. [`crate::generate`] writes it after the header and before the tables; it
//! is Rust source text, compiled where the file is, in `surf-script`.

/// The text, from its `use` lines to the realm's setup.
pub(crate) const PRELUDE: &str = r##"
use std::cell::Cell;

use surf_html::limits::{Limit, Limits, NODE_BYTES};
use surf_html::{NodeId, Tree};
use swell::{
    Abrupt, Brand, Completion, Cx, ErrorKind, HostBytes, HostCall, HostData, HostError, HostFn,
    HostIntrinsic, HostObjectSpec, InterruptHandle, ObjectRef, PropertyDescriptor, PropertyKey,
    Prototype, RealmRef, Root, StringRef, Termination, Tracer, Value, WellKnownSymbol,
};

use crate::runner::ScriptHost;

/// A `DOMException` as the Rust side holds it: its name (an error name of Web IDL's table,
/// `NotSupportedError`) and its message. Thrown to a script as a `DOMException` object of the
/// realm, with an \[\[ErrorData\]\] slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomException {
    /// The name.
    pub name: String,
    /// The message.
    pub message: String,
}

impl DomException {
    /// A `DOMException` named `name` with `message`.
    pub fn new(name: &str, message: impl Into<String>) -> DomException {
        DomException {
            name: name.to_string(),
            message: message.into(),
        }
    }

    /// A `NotSupportedError` with `message`.
    pub fn not_supported(message: impl Into<String>) -> DomException {
        DomException::new("NotSupportedError", message)
    }

    /// The legacy code of the name (Web IDL's "error names" table), 0 for a name the table
    /// gives none.
    pub fn code(&self) -> u16 {
        legacy_code(&self.name)
    }
}

/// How a member of a slice ends when it does not answer (decision 9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomEnd {
    /// Throw a `DOMException` of that name to the script.
    Throw(DomException),
    /// A write would go over a limit of the parser: nothing is written,
    /// [`end_with_limit`] records the limit and ends the script there.
    Limit(Limit),
    /// The String the member would answer has more UTF-16 code units than the agent's
    /// `string_length`: the realm's `RangeError`, as for a String answered whole.
    StringLength,
    /// A defect of surf's own code that no page causes: the run ends faulted, as `defect` does,
    /// and no script can catch it.
    Fault,
}

/// What every method of a slice answers.
pub type DomResult<T> = Result<T, DomEnd>;

/// The message of the `RangeError` for a String longer than `string_length`: swell's own for a
/// String its operations would make too long.
pub const STRING_LENGTH_MESSAGE: &str = "the string would be longer than string_length";

/// The UTF-16 code units of `text`.
pub fn utf16_units(text: &str) -> u64 {
    u64::try_from(text.encode_utf16().count()).unwrap_or(u64::MAX)
}

/// Whether `text` has more UTF-16 code units than `limit`. A text of at most `limit` bytes has
/// at most that many units and is not counted; a longer one is counted once.
pub fn longer_than(text: &str, limit: u64) -> bool {
    u64::try_from(text.len()).unwrap_or(u64::MAX) > limit && utf16_units(text) > limit
}

/// The `NotSupportedError` a member answers until its slice gives it behaviour: its message
/// names the member, `<interface>.<member> is not supported`.
pub fn not_supported(member: &str) -> DomEnd {
    DomEnd::Throw(DomException::not_supported(format!(
        "{member} is not supported"
    )))
}

/// The totals of the nodes a script made, charged with [`DomCx::charge_node`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    /// The nodes charged.
    pub nodes: u64,
    /// The bytes charged for them. The bytes scripts write into nodes that exist are charged
    /// apart, through one handle ([`DomCx::charge_stored`], [`DomCx::settle_stored`]), and are
    /// not added here.
    pub bytes: u64,
}

/// The factor of a stored byte in a memory charge: the tree's heap bound counts a stored byte
/// twice, as the engine's String and the tree's copy may both be live. [`DomCx::charge_node`]
/// charges a node's stored bytes with it, and the stored handle holds it times the live bytes.
const STORED_FACTOR: u64 = 2;

/// The live bytes after a write that adds `added` stored bytes and removes `removed` from
/// `live`: `live + added` less `removed`, never below 0.
fn next_live(live: u64, added: u64, removed: u64) -> u64 {
    live.saturating_add(added).saturating_sub(removed)
}

/// A count as a `u64`.
fn bytes_u64(bytes: usize) -> u64 {
    u64::try_from(bytes).unwrap_or(u64::MAX)
}

/// The document a realm's bindings serve: its tree, the parser's limits, the totals and the
/// record of a limit reached, the agent's interrupt handle, and the node charges, which stay
/// with it until the agent is dropped (decision 13).
pub struct DomState {
    tree: Tree,
    limits: Limits,
    totals: Totals,
    limit: Option<Limit>,
    /// Whether an end of the run left a kept collection half refilled ([`DomState::cut_refill`]).
    cut_refill: bool,
    interrupt: InterruptHandle,
    charges: Vec<HostBytes>,
    /// The one handle of the bytes scripts wrote into nodes that exist
    /// ([`DomCx::charge_stored`]); `None` until the first write that adds bytes. Not an entry
    /// of `charges`.
    stored: Option<HostBytes>,
    /// The net rise of the stored bytes of existing nodes since the parse, by the writes that
    /// charge them: the bytes added less the bytes removed, never below 0.
    live: u64,
    /// The bytes the stored handle holds: `STORED_FACTOR` times `live` after each settle the
    /// engine took.
    charged: u64,
    /// The message of a `TypeError` noted while the bindings are out of the host
    /// ([`unwrap_node`]), thrown by [`in_dom`] once they are back.
    type_error: Option<String>,
}

impl DomState {
    /// The state of `tree`, parsed under `limits`, with the agent's `interrupt` handle.
    pub fn new(tree: Tree, limits: Limits, interrupt: InterruptHandle) -> DomState {
        DomState {
            tree,
            limits,
            totals: Totals::default(),
            limit: None,
            cut_refill: false,
            interrupt,
            charges: Vec::new(),
            stored: None,
            live: 0,
            charged: 0,
            type_error: None,
        }
    }

    /// The tree.
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// The limit [`end_with_limit`] recorded, taken: the record is cleared.
    pub fn take_limit(&mut self) -> Option<Limit> {
        self.limit.take()
    }

    /// The limit recorded, if any.
    pub fn limit(&self) -> Option<Limit> {
        self.limit
    }

    /// Whether an end of the run (`Abrupt::Terminated`) cut a refill of a kept collection after
    /// its first define and before its data was written, so that the object may show two lists.
    /// Set once and never cleared: no script of the document reads that object again, because
    /// the runner runs no later script of the document.
    pub fn cut_refill(&self) -> bool {
        self.cut_refill
    }

    /// The totals of the node charges.
    pub fn totals(&self) -> Totals {
        self.totals
    }

    /// The net rise of the stored bytes of existing nodes that the stored handle charges
    /// ([`DomCx::charge_stored`]).
    pub fn stored_live(&self) -> u64 {
        self.live
    }

    /// The bytes the stored handle holds charged: twice [`DomState::stored_live`] after each
    /// settle.
    pub fn stored_charged(&self) -> u64 {
        self.charged
    }

    /// The stored handles: 0 before the first write that adds bytes, then 1.
    pub fn stored_handles(&self) -> usize {
        usize::from(self.stored.is_some())
    }

    /// The node charges kept: one handle for each node a script made.
    pub fn node_charges(&self) -> usize {
        self.charges.len()
    }
}

/// The context every member of a slice receives (decision 8): the document's tree, the
/// parser's limits, the totals, the record of a limit reached, the agent's interrupt handle,
/// and the engine for the node charge and the wrappers.
pub struct DomCx<'c, 'e> {
    state: &'c mut DomState,
    installed: &'c Installed,
    engine: &'c mut Cx<'e, ScriptHost>,
}

impl<'e> DomCx<'_, 'e> {
    /// The tree.
    pub fn tree(&self) -> &Tree {
        &self.state.tree
    }

    /// The tree, to write. A write that would make a node calls [`DomCx::charge_node`] first; a
    /// write that adds or removes stored bytes of a node that exists calls
    /// [`DomCx::charge_stored`] before it and [`DomCx::settle_stored`] after it, whatever the
    /// tree answers.
    pub fn tree_mut(&mut self) -> &mut Tree {
        &mut self.state.tree
    }

    /// The parser's limits, which every write is held to.
    pub fn limits(&self) -> &Limits {
        &self.state.limits
    }

    /// The totals of the node charges.
    pub fn totals(&self) -> Totals {
        self.state.totals
    }

    /// [`DomState::stored_live`].
    pub fn stored_live(&self) -> u64 {
        self.state.live
    }

    /// [`DomState::stored_charged`].
    pub fn stored_charged(&self) -> u64 {
        self.state.charged
    }

    /// The limit recorded by [`end_with_limit`] in the running script, if any.
    pub fn limit_reached(&self) -> Option<Limit> {
        self.state.limit
    }

    /// The agent's interrupt handle.
    pub fn interrupt(&self) -> &InterruptHandle {
        &self.state.interrupt
    }

    /// The agent's `string_length`: a String a member answers with more UTF-16 code units is
    /// the `RangeError` of [`DomEnd::StringLength`].
    pub fn string_length(&self) -> u64 {
        self.engine.host().string_length()
    }

    /// The engine, for a wrapper's host object.
    pub fn engine(&mut self) -> &mut Cx<'e, ScriptHost> {
        &mut *self.engine
    }

    /// The node charge of decision 13, made before a write that makes a node: the node's term
    /// of the tree's heap bound, 3 × `NODE_BYTES` plus twice `stored_bytes`, charged to the
    /// agent's memory budget with `charge_host_bytes` and kept until the agent is dropped. Over
    /// the budget nothing is charged and the run ends `Terminated(MemoryBudget)`; the answer is
    /// then a throw the engine does not deliver.
    pub fn charge_node(&mut self, stored_bytes: usize) -> DomResult<()> {
        let node = bytes_u64(NODE_BYTES);
        let stored = bytes_u64(stored_bytes);
        let bytes = node
            .saturating_mul(3)
            .saturating_add(stored.saturating_mul(STORED_FACTOR));
        let mut handle = None;
        Self::charge_to(&mut *self.engine, &mut handle, bytes)?;
        if let Some(handle) = handle {
            self.state.charges.push(handle);
        }
        self.state.totals.nodes = self.state.totals.nodes.saturating_add(1);
        self.state.totals.bytes = self.state.totals.bytes.saturating_add(bytes);
        Ok(())
    }

    /// Makes `handle` hold `bytes` of the agent's memory budget: with no handle it makes one
    /// (`charge_host_bytes`), with one it resizes it (`resize_host_bytes`), a grow charging the
    /// difference first and a shrink giving it back. Over the budget nothing changes and the
    /// run ends `Terminated(MemoryBudget)`; the answer is then a throw the engine does not
    /// deliver. An engine call surf should not have made (a handle of another engine, a
    /// faulted agent) answers [`DomEnd::Fault`]. It takes no `self`, so that a field of the
    /// state can be the handle while the engine is borrowed.
    fn charge_to(
        engine: &mut Cx<'e, ScriptHost>,
        handle: &mut Option<HostBytes>,
        bytes: u64,
    ) -> DomResult<()> {
        let spent = || {
            DomEnd::Throw(DomException::new(
                "InvalidStateError",
                "the memory budget is spent",
            ))
        };
        match handle {
            None => match engine.charge_host_bytes(bytes) {
                Ok(made) => {
                    *handle = Some(made);
                    Ok(())
                }
                Err(Abrupt::Throw(_) | Abrupt::Terminated(_)) => Err(spent()),
            },
            Some(held) => match engine.resize_host_bytes(held, bytes) {
                Ok(Ok(())) => Ok(()),
                Ok(Err(Abrupt::Throw(_) | Abrupt::Terminated(_))) => Err(spent()),
                // A faulted agent, or a handle of another one: surf's own defect.
                Err(HostError::Faulted | HostError::ForeignHandle) => Err(DomEnd::Fault),
                // Ends faulted: the state frees no handle while it runs. A limit or an
                // interrupt is never this outer error: the budget answers the inner completion.
                Err(HostError::StaleHandle) => Err(DomEnd::Fault),
                // End faulted: a resize takes no value, no module and no evaluation and queues
                // no job, so these answer only a defect; a limit is the inner MemoryBudget, an
                // interrupt the termination the next step reads, never this outer error. The
                // outer MemoryBudget (new at swell 0.9.0) is answered by `queue_microtask`
                // only (`host/jobs.rs:207 @ 0.10.0`); a resize ends the run instead
                // (`host/host_bytes.rs:93-94 @ 0.10.0`).
                Err(
                    HostError::WrongKind { .. }
                    | HostError::ModuleStatus { .. }
                    | HostError::ModuleTerminated
                    | HostError::EvaluationInProgress
                    | HostError::MemoryBudget,
                ) => Err(DomEnd::Fault),
            },
        }
    }

    /// The charge before a write into nodes that exist, of the sums of one tree call: `added`
    /// stored bytes and `removed` ones. The next live bytes are those of `next_live` and the
    /// wanted charge is `STORED_FACTOR` times them; when it is above what the stored handle
    /// holds, the handle is made or grown to it. Over the budget nothing changes (the live
    /// bytes, the charge and the tree) and the run ends `Terminated(MemoryBudget)`,
    /// `limit: memory`, the answer being a throw the engine does not deliver. A shrink waits
    /// for [`DomCx::settle_stored`] after the write. It changes the charge and never the live
    /// bytes, so each call is followed by one settle with the same sums before another charge.
    pub fn charge_stored(&mut self, added: usize, removed: usize) -> DomResult<()> {
        let live = next_live(self.state.live, bytes_u64(added), bytes_u64(removed));
        let wanted = live.saturating_mul(STORED_FACTOR);
        if wanted <= self.state.charged {
            return Ok(());
        }
        Self::charge_to(&mut *self.engine, &mut self.state.stored, wanted)?;
        self.state.charged = wanted;
        Ok(())
    }

    /// The settle after a write into nodes that exist, with the sums [`DomCx::charge_stored`]
    /// was given, or with `(0, 0)` after a write the tree refused: the live bytes become
    /// `next_live` of them and the handle shrinks to `STORED_FACTOR` times that, so a write
    /// that removed bytes gives them back. An equal size makes no engine call; a settle never
    /// grows the handle (a caller's defect, which a debug assertion names). The engine cannot
    /// refuse a shrink for memory, so any refusal of it is a defect: [`DomEnd::Fault`], with
    /// the live bytes and the charge as they were.
    pub fn settle_stored(&mut self, added: usize, removed: usize) -> DomResult<()> {
        let live = next_live(self.state.live, bytes_u64(added), bytes_u64(removed));
        let wanted = live.saturating_mul(STORED_FACTOR);
        debug_assert!(
            wanted <= self.state.charged,
            "a settle above the charge: charge_stored comes first"
        );
        if wanted < self.state.charged {
            let shrunk = match Self::charge_to(&mut *self.engine, &mut self.state.stored, wanted) {
                Err(DomEnd::Throw(_)) => Err(DomEnd::Fault),
                other => other,
            };
            shrunk?;
            self.state.charged = wanted;
        }
        self.state.live = live;
        Ok(())
    }
}

/// The wrapper hook (decision 8): every node a member returns goes through `wrap`, and every
/// `this` and node argument through `unwrap` after its brand check. surf's own is
/// `crate::dom::wrapper`.
///
/// It also holds `[SameObject]` for the attributes that return a `NodeList` or an
/// `HTMLCollection` ([`SameObject`]): the getter asks `same_object` for the object kept for the
/// node and the member, and hands a new one to `keep_same_object`. The getter makes and
/// refreshes the object; the hook only finds and keeps it. With the provided methods, which
/// keep nothing, such a getter makes a new object at each read.
pub trait Wrap {
    /// The wrapper of `node`.
    fn wrap(&mut self, cx: &mut DomCx<'_, '_>, node: NodeId) -> Completion<Value>;

    /// The node of a wrapper, `None` for any other value.
    fn unwrap(&self, cx: &DomCx<'_, '_>, value: Value) -> Option<NodeId>;

    /// The object kept for `node` and `member` by [`Wrap::keep_same_object`], if any.
    fn same_object(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        node: NodeId,
        member: SameObject,
    ) -> Completion<Option<ObjectRef>> {
        let _ = (cx, node, member);
        Ok(None)
    }

    /// Keeps `object` as the object of `node` and `member`.
    fn keep_same_object(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        node: NodeId,
        member: SameObject,
        object: ObjectRef,
    ) -> Completion<()> {
        let _ = (cx, node, member, object);
        Ok(())
    }
}

/// The data of a node's wrapper made by [`new_wrapper`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeData {
    /// The node.
    pub node: NodeId,
}

impl HostData for NodeData {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// The data of a `NodeList` or an `HTMLCollection`: the snapshot of its nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionData {
    /// The nodes, in order.
    pub nodes: Vec<NodeId>,
    /// The tree's `writes()` when the nodes were taken: a kept `[SameObject]` collection whose
    /// count is the tree's is up to date.
    pub written: u64,
}

impl HostData for CollectionData {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// The data of a `DOMException` object: its `name` and `message` as the engine's Strings, every
/// code unit a script gave kept (`DOMString`), no copy of them in surf. The Strings are on
/// swell's heap and charged there, under the run's `memory_bytes`. They stay alive because
/// [`ExceptionData::trace`] lists them: swell's host interface, "What keeps an object alive for
/// the host is a `Root` (decision 4), or a handle that host data the engine reaches lists in its
/// `trace` from the scope it was handed in onward (decision 15); such a handle is strong"
/// (`docs/design/host-interface.md:382-384 @ 0.8.0`, `:394-396 @ 0.9.0`), with
/// `Tracer::value(&mut self, value: Value)` in decision 15 (`:1720 @ 0.8.0`, `:1736 @ 0.9.0`).
/// The sentence names "an object" and "a handle", and the requirement under it (HOST-109) tests
/// an object only: that a String listed with `Tracer::value` lives is read from the sentence and
/// the signature, not from a requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExceptionData {
    name: StringRef,
    message: StringRef,
}

impl HostData for ExceptionData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.value(Value::String(self.name));
        tracer.value(Value::String(self.message));
    }
}

/// The most bytes of the name and of the message of a `DOMException` the bindings make
/// themselves (a member's label with its reason from `selection.txt`, a fixed sentence, or the
/// [`DomException`] a slice answers): a longer text is cut at a character's boundary. The text
/// lives in surf only until `new_exception` made it the engine's String, before any script
/// runs.
pub const EXCEPTION_TEXT_BYTES: usize = 1024;

/// `text` cut to at most [`EXCEPTION_TEXT_BYTES`] bytes at a character's boundary.
fn bounded(text: &str) -> &str {
    let mut end = text.len().min(EXCEPTION_TEXT_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// A new wrapper of `node` implementing `interface`: a host object of the interface's brand,
/// whose prototype is the interface's prototype object and whose data is [`NodeData`].
pub fn new_wrapper(
    cx: &mut DomCx<'_, '_>,
    interface: Interface,
    node: NodeId,
) -> Completion<ObjectRef> {
    let prototype = prototype_object(cx.engine, cx.installed, interface)?;
    engine(cx.engine.new_host_object(HostObjectSpec {
        prototype: Prototype::Object(prototype),
        brand: interface.brand(),
        data: Some(Box::new(NodeData { node })),
        error_kind: None,
    }))
}

/// The node of a wrapper made by [`new_wrapper`], read from its data with its brand.
pub fn wrapper_node(cx: &DomCx<'_, '_>, value: Value) -> Option<NodeId> {
    let Value::Object(object) = value else {
        return None;
    };
    let brand = cx.engine.brand(object).ok()??;
    cx.engine
        .host_data::<NodeData>(object, brand)
        .ok()?
        .map(|data| data.node)
}

/// What a `DOMException` holds, for the runner's record of one a script did not catch (Q16):
/// `<name>: <message>`, read from the exception's own data without running script, whether
/// the bindings or a script made it, a lone surrogate written U+FFFD; `None` for any other
/// value. At most `chars` characters: of the name and of the message at most `2 × chars` code
/// units each are read (`prefix_units`; 2,048 each for the runner's 1,024 characters), so no
/// copy of a longer text is made. No script of the page runs while they are read: it runs after
/// the script's end, and a length and an own index property of a String run no code. `None`
/// too when an engine call fails: the memory budget spent by the one-unit Strings the read
/// makes, or an interrupt set after the throw, which the read's first engine call takes and
/// which ends the read; the record is then an `Error` without a message.
pub fn exception_text(cx: &mut Cx<'_, ScriptHost>, value: Value, chars: usize) -> Option<String> {
    let Value::Object(object) = value else {
        return None;
    };
    let data = *cx
        .host_data::<ExceptionData>(object, Interface::DomException.brand())
        .ok()??;
    let units = u32::try_from(chars.saturating_mul(2)).unwrap_or(u32::MAX);
    let name = prefix_units(cx, data.name, units).ok()?;
    let message = prefix_units(cx, data.message, units).ok()?;
    let text = format!(
        "{}: {}",
        String::from_utf16_lossy(&name),
        String::from_utf16_lossy(&message)
    );
    Some(text.chars().take(chars).collect())
}

/// The first `units` code units of `string`, or all of them when it has no more. A longer
/// String is not copied: each of its first `units` code units is read as an own index property
/// of its String object (`ToObject` and a Get: StringGetOwnProperty, ECMA-262 10.4.3.5, runs no
/// script), a String of one code unit on swell's heap, and copied alone.
fn prefix_units(cx: &mut Cx<'_, ScriptHost>, string: StringRef, units: u32) -> Completion<Vec<u16>> {
    if string_length(cx, string)? <= f64::from(units) {
        return cx.to_utf16(string).map_err(defect);
    }
    let object = engine(cx.to_object(Value::String(string)))?;
    let mut out = Vec::new();
    for index in 0..units {
        let key = PropertyKey::String(cx.new_string(&index.to_string())?);
        match engine(cx.get_property(object, key))? {
            Value::String(unit) => out.extend(cx.to_utf16(unit).map_err(defect)?),
            Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::Number(_)
            | Value::Symbol(_)
            | Value::BigInt(_)
            | Value::Object(_) => return Err(missing()),
        }
    }
    Ok(out)
}

/// The throw of `exception`: a new `DOMException` object of the realm.
pub fn throw(cx: &mut DomCx<'_, '_>, exception: &DomException) -> Abrupt {
    throw_exception(cx.engine, cx.installed, exception)
}

/// Decision 9, as ruled in fix round 1: a write over the parser's `limit` changes nothing in
/// the tree, and the limit is the script's end. This records the limit, sets the agent's
/// interrupt handle and reads it at once with `Cx::charge_steps` (swell's HOST-177: "an
/// interrupt set before the charge ends it in `Terminated(Interrupted)`"; the charge clears the
/// flag), and answers that termination. The termination is then sticky for the run, and what
/// the host function returns is not used (swell's host interface, decision 10, HOST-72): no
/// `catch` and no `finally` of the page runs after the limit, and the runner reads the
/// recorded limit as the script's end (`Runner::document_end`). Should the charge answer `Ok`,
/// the function answers `Terminated(Interrupted)` itself, which swell takes and makes the
/// run's termination with the agent alive (`object/builtin_function.rs:876 @ 0.10.0`); swell
/// promises that answer since `0.9.1` (HOST-217, `conformance/host/requirements.txt:217 @ 0.10.0`).
pub fn end_with_limit(cx: &mut DomCx<'_, '_>, limit: Limit) -> Abrupt {
    cx.state.limit = Some(limit);
    cx.state.interrupt.interrupt();
    match cx.engine.charge_steps(0) {
        Err(abrupt) => abrupt,
        Ok(()) => Abrupt::Terminated(Termination::Interrupted),
    }
}

/// The slices, each a trait object, and the wrapper hook: what the realm's bindings call.
pub struct Slices {
    /// The members of `reading`.
    pub reading: Box<dyn Reading>,
    /// The members of `attributes`.
    pub attributes: Box<dyn Attributes>,
    /// The members of `mutation`.
    pub mutation: Box<dyn Mutation>,
    /// The members of `selectors`.
    pub selectors: Box<dyn Selectors>,
    /// The wrapper hook.
    pub wrap: Box<dyn Wrap>,
}

/// What the realm's setup made and keeps: the realm, the prototype object of each interface,
/// the global object and the realm's original `Array`, each held by a root.
pub struct Installed {
    realm: RealmRef,
    prototypes: Vec<Root>,
    global: Root,
    array: Root,
}

/// The bindings of one realm, held by the host value: the document and the slices; the setup
/// adds what it made.
pub struct Dom {
    /// The document.
    pub state: DomState,
    /// The slices.
    pub slices: Slices,
    installed: Option<Installed>,
}

impl Dom {
    /// The bindings of `state` with `slices`, not yet set up in a realm ([`install`]).
    pub fn new(state: DomState, slices: Slices) -> Dom {
        Dom {
            state,
            slices,
            installed: None,
        }
    }
}

/// Runs `f` with the document's context and the slices, the bindings taken out of the host
/// for the call and put back after it. `None` when the host holds no bindings set up.
pub fn with_dom<R>(
    cx: &mut Cx<'_, ScriptHost>,
    f: impl FnOnce(&mut DomCx<'_, '_>, &mut Slices) -> R,
) -> Option<R> {
    let mut dom = cx.host_mut().dom.take()?;
    let answer = {
        let Dom {
            state,
            slices,
            installed,
        } = &mut *dom;
        match installed.as_ref() {
            Some(installed) => {
                let mut dcx = DomCx {
                    state,
                    installed,
                    engine: cx,
                };
                Some(f(&mut dcx, slices))
            }
            None => None,
        }
    };
    cx.host_mut().dom = Some(dom);
    answer
}

thread_local! {
    /// The copies of a String out of the engine that [`Copies::copy`] made and that are not
    /// yet handed to a slice (decision 12).
    static UNCHARGED: Cell<usize> = const { Cell::new(0) };
}

/// The uncharged copies of a String out of the engine alive on this thread: made by the one
/// copy function and not yet handed to a slice. A generated member converts every argument
/// before it copies any String, so it is 0 whenever a conversion runs script.
pub fn uncharged_copies() -> usize {
    UNCHARGED.with(Cell::get)
}


/// The ledger of a member's copies: the count of [`uncharged_copies`] holds them from the copy
/// until the ledger is dropped, after the slice's method.
struct Copies(usize);

impl Copies {
    fn new() -> Copies {
        Copies(0)
    }

    /// The one function that copies a String out of the engine (decision 12): the host
    /// interface's `to_utf16`, uncharged, counted until the ledger is dropped. Every code unit
    /// is copied (`DOMString`); a lone surrogate is then replaced by U+FFFD, since the tree
    /// stores UTF-8. A step is charged per [`STRING_BYTES_A_STEP`] bytes of the code units to
    /// be copied (8 units), the price of a String answer's copy, before the copy: the length
    /// is read first as [`string_length`] reads it, which copies nothing, so a call past its
    /// steps ends before any unit is copied.
    fn copy(&mut self, cx: &mut Cx<'_, ScriptHost>, string: StringRef) -> Completion<String> {
        let length = string_length(cx, string)?;
        let count = if length.is_finite() && length > 0.0 {
            length as u64
        } else {
            0
        };
        cx.charge_steps(count.saturating_mul(2) / STRING_BYTES_A_STEP)?;
        let units = cx.to_utf16(string).map_err(defect)?;
        let text = String::from_utf16_lossy(&units);
        self.0 = self.0.saturating_add(1);
        UNCHARGED.with(|count| count.set(count.get().saturating_add(1)));
        Ok(text)
    }
}

impl Drop for Copies {
    fn drop(&mut self) {
        let made = self.0;
        UNCHARGED.with(|count| count.set(count.get().saturating_sub(made)));
    }
}

/// A call surf should not have made, a defect of surf's own code that no page causes: the run
/// ends faulted.
fn defect(_error: HostError) -> Abrupt {
    Abrupt::Terminated(Termination::EngineFault)
}

/// The engine's answer with its `HostError` read as a defect.
fn engine<T>(answer: Result<Completion<T>, HostError>) -> Completion<T> {
    answer.map_err(defect)?
}

/// The bindings are missing from the host while a member runs: a defect.
fn missing() -> Abrupt {
    Abrupt::Terminated(Termination::EngineFault)
}

/// The realm of the bindings, read from the host.
fn realm_of(cx: &Cx<'_, ScriptHost>) -> Completion<RealmRef> {
    match cx.host().dom.as_deref() {
        Some(Dom {
            installed: Some(installed),
            ..
        }) => Ok(installed.realm),
        Some(_) | None => Err(missing()),
    }
}

/// A `TypeError` of the realm with `message`, thrown.
fn type_error(cx: &mut Cx<'_, ScriptHost>, message: &str) -> Abrupt {
    match realm_of(cx) {
        Ok(realm) => error_of(cx, realm, ErrorKind::TypeError, message),
        Err(abrupt) => abrupt,
    }
}

/// The `RangeError` of the realm for a String longer than the agent's `string_length`, thrown,
/// with swell's message ([`STRING_LENGTH_MESSAGE`]).
fn string_length_error(cx: &mut DomCx<'_, '_>) -> Abrupt {
    let realm = cx.installed.realm;
    error_of(cx.engine, realm, ErrorKind::RangeError, STRING_LENGTH_MESSAGE)
}

/// An error of `kind` of `realm` with `message`, thrown.
fn error_of(cx: &mut Cx<'_, ScriptHost>, realm: RealmRef, kind: ErrorKind, message: &str) -> Abrupt {
    match engine(cx.new_error(realm, kind, message)) {
        Ok(error) => Abrupt::Throw(Value::Object(error)),
        Err(abrupt) => abrupt,
    }
}

/// The prototype object of `interface`, from its root.
fn prototype_object(
    cx: &mut Cx<'_, ScriptHost>,
    installed: &Installed,
    interface: Interface,
) -> Completion<ObjectRef> {
    let Some(root) = installed.prototypes.get(interface.index()) else {
        return Err(missing());
    };
    match cx.get(root).map_err(defect)? {
        Value::Object(object) => Ok(object),
        Value::Undefined
        | Value::Null
        | Value::Boolean(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Symbol(_)
        | Value::BigInt(_) => Err(missing()),
    }
}

/// A new `DOMException` object of the realm from `exception`, thrown.
fn throw_exception(
    cx: &mut Cx<'_, ScriptHost>,
    installed: &Installed,
    exception: &DomException,
) -> Abrupt {
    let prototype = match prototype_object(cx, installed, Interface::DomException) {
        Ok(prototype) => prototype,
        Err(abrupt) => return abrupt,
    };
    let made = (|| {
        let name = cx.new_string(bounded(&exception.name))?;
        let message = cx.new_string(bounded(&exception.message))?;
        new_exception(cx, prototype, name, message)
    })();
    match made {
        Ok(object) => Abrupt::Throw(Value::Object(object)),
        Err(abrupt) => abrupt,
    }
}

/// A `DOMException` object with `prototype`, `name` and `message`: a host object of the brand
/// of `DOMException` with an \[\[ErrorData\]\] slot (`HostObjectSpec::error_kind`), whose data
/// holds the two Strings ([`ExceptionData`]).
fn new_exception(
    cx: &mut Cx<'_, ScriptHost>,
    prototype: ObjectRef,
    name: StringRef,
    message: StringRef,
) -> Completion<ObjectRef> {
    engine(cx.new_host_object(HostObjectSpec {
        prototype: Prototype::Object(prototype),
        brand: Interface::DomException.brand(),
        data: Some(Box::new(ExceptionData { name, message })),
        error_kind: Some(ErrorKind::Error),
    }))
}

/// The throw of `exception` from a member while the bindings are in the host.
fn throw_now(cx: &mut Cx<'_, ScriptHost>, exception: &DomException) -> Abrupt {
    match with_dom(cx, |dcx, _| throw(dcx, exception)) {
        Some(abrupt) => abrupt,
        None => missing(),
    }
}

/// The argument at `index`, `undefined` past the end.
fn argument(call: &HostCall<'_>, index: usize) -> Value {
    call.args.get(index).copied().unwrap_or(Value::Undefined)
}

/// Whether the argument at `index` was passed and is not `undefined`: an optional argument
/// that is not takes its default.
fn present(call: &HostCall<'_>, index: usize) -> bool {
    match call.args.get(index) {
        None | Some(Value::Undefined) => false,
        Some(
            Value::Null
            | Value::Boolean(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Symbol(_)
            | Value::BigInt(_)
            | Value::Object(_),
        ) => true,
    }
}

/// Web IDL's check of the arguments: fewer than `required` throws a `TypeError`.
fn required(cx: &mut Cx<'_, ScriptHost>, call: &HostCall<'_>, required: usize, member: &str) -> Completion<()> {
    if call.args.len() < required {
        return Err(type_error(
            cx,
            &format!("{member}: {required} argument(s) required, {} passed", call.args.len()),
        ));
    }
    Ok(())
}

/// The interface the brand of `value` names, if it is an object of a generated brand.
fn interface_of(cx: &Cx<'_, ScriptHost>, value: Value) -> Completion<Option<(ObjectRef, Interface)>> {
    let Value::Object(object) = value else {
        return Ok(None);
    };
    let brand = cx.brand(object).map_err(defect)?;
    Ok(brand.and_then(Interface::of_brand).map(|interface| (object, interface)))
}

/// The brand check of `this` for a member of `interface` (Web IDL: a `TypeError` for an object
/// that does not implement it), with the object.
fn this_object(
    cx: &mut Cx<'_, ScriptHost>,
    this: Value,
    interface: Interface,
    member: &str,
) -> Completion<ObjectRef> {
    match interface_of(cx, this)? {
        Some((object, found)) if found.implements(interface) => Ok(object),
        Some(_) | None => Err(type_error(
            cx,
            &format!("{member}: `this` does not implement {}", interface.name()),
        )),
    }
}

/// The brand check of `this` for a member of the global interface: the global object, or
/// `undefined` or `null`, which stand for it.
fn this_global(cx: &mut Cx<'_, ScriptHost>, this: Value, member: &str) -> Completion<()> {
    match this {
        Value::Undefined | Value::Null => Ok(()),
        Value::Boolean(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Symbol(_)
        | Value::BigInt(_)
        | Value::Object(_) => this_object(cx, this, GLOBAL, member).map(|_| ()),
    }
}

/// A `NodeList` or `HTMLCollection` `this`, after its brand check, whose data holds its nodes.
fn this_collection(
    cx: &mut Cx<'_, ScriptHost>,
    this: Value,
    interface: Interface,
    member: &str,
) -> Completion<ObjectRef> {
    let object = this_object(cx, this, interface, member)?;
    let brand = interface.brand();
    match cx.host_data::<CollectionData>(object, brand).map_err(defect)? {
        Some(_) => Ok(object),
        None => Err(type_error(cx, &format!("{member}: `this` holds no nodes"))),
    }
}

/// The nodes of the collection `object`, taken out of its data for a slice's call: the slice
/// reads them in place, and [`restore_nodes`] puts them back after it, whatever it answered.
/// Nothing is copied.
fn take_nodes(
    cx: &mut DomCx<'_, '_>,
    object: ObjectRef,
    interface: Interface,
) -> Completion<Vec<NodeId>> {
    match cx
        .engine
        .host_data_mut::<CollectionData>(object, interface.brand())
        .map_err(defect)?
    {
        Some(data) => Ok(std::mem::take(&mut data.nodes)),
        None => Err(missing()),
    }
}

/// Puts the nodes [`take_nodes`] took back into the data of `object`.
fn restore_nodes(
    cx: &mut DomCx<'_, '_>,
    object: ObjectRef,
    interface: Interface,
    nodes: Vec<NodeId>,
) -> Completion<()> {
    match cx
        .engine
        .host_data_mut::<CollectionData>(object, interface.brand())
        .map_err(defect)?
    {
        Some(data) => {
            data.nodes = nodes;
            Ok(())
        }
        None => Err(missing()),
    }
}

/// The `written` of the collection `object`.
fn collection_written(
    cx: &mut DomCx<'_, '_>,
    object: ObjectRef,
    interface: Interface,
) -> Completion<u64> {
    match cx
        .engine
        .host_data::<CollectionData>(object, interface.brand())
        .map_err(defect)?
    {
        Some(data) => Ok(data.written),
        None => Err(missing()),
    }
}

/// The node of a wrapper whose brand was checked, through the wrapper hook. When the hook knows
/// no node of it, the `TypeError` is noted, and [`in_dom`] throws it once the bindings are back
/// in the host, where [`type_error`] finds the realm; the answer here only leaves the member's
/// closure.
fn unwrap_node(
    cx: &mut DomCx<'_, '_>,
    slices: &Slices,
    object: ObjectRef,
    member: &str,
) -> Completion<NodeId> {
    match slices.wrap.unwrap(cx, Value::Object(object)) {
        Some(node) => Ok(node),
        None => {
            cx.state.type_error = Some(format!(
                "{member}: an object that is no node of this document"
            ));
            Err(Abrupt::Throw(Value::Undefined))
        }
    }
}

/// The value of a slice's answer, or its end thrown.
fn finish<T>(
    cx: &mut DomCx<'_, '_>,
    slices: &mut Slices,
    answer: DomResult<T>,
    convert: impl FnOnce(&mut DomCx<'_, '_>, &mut Slices, T) -> Completion<Value>,
) -> Completion<Value> {
    let value = answered(cx, answer)?;
    convert(cx, slices, value)
}

/// A slice's answer, or its end thrown.
fn answered<T>(cx: &mut DomCx<'_, '_>, answer: DomResult<T>) -> Completion<T> {
    match answer {
        Ok(value) => Ok(value),
        Err(DomEnd::Throw(exception)) => Err(throw(cx, &exception)),
        Err(DomEnd::Limit(limit)) => Err(end_with_limit(cx, limit)),
        Err(DomEnd::StringLength) => Err(string_length_error(cx)),
        Err(DomEnd::Fault) => Err(Abrupt::Terminated(Termination::EngineFault)),
    }
}

/// Runs the slice part of a member: the bindings out of the host, then `f`; then, the bindings
/// back, the `TypeError` [`unwrap_node`] noted, if any.
fn in_dom(
    cx: &mut Cx<'_, ScriptHost>,
    f: impl FnOnce(&mut DomCx<'_, '_>, &mut Slices) -> Completion<Value>,
) -> Completion<Value> {
    let answer = with_dom(cx, f).unwrap_or_else(|| Err(missing()));
    let noted = cx
        .host_mut()
        .dom
        .as_deref_mut()
        .and_then(|dom| dom.state.type_error.take());
    match (noted, answer) {
        (Some(message), Err(Abrupt::Throw(_))) => Err(type_error(cx, &message)),
        (_, answer) => answer,
    }
}

/// ToString (Web IDL's `DOMString`), the String in the engine.
fn to_string(cx: &mut Cx<'_, ScriptHost>, value: Value) -> Completion<StringRef> {
    engine(cx.to_string(value))
}

/// `DOMString?`: `null` and `undefined` are `None`.
fn to_nullable_string(cx: &mut Cx<'_, ScriptHost>, value: Value) -> Completion<Option<StringRef>> {
    match value {
        Value::Undefined | Value::Null => Ok(None),
        Value::Boolean(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Symbol(_)
        | Value::BigInt(_)
        | Value::Object(_) => to_string(cx, value).map(Some),
    }
}

/// `[LegacyNullToEmptyString] DOMString`: `null` is the empty string (`None` here).
fn to_legacy_null_string(cx: &mut Cx<'_, ScriptHost>, value: Value) -> Completion<Option<StringRef>> {
    match value {
        Value::Null => Ok(None),
        Value::Undefined
        | Value::Boolean(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Symbol(_)
        | Value::BigInt(_)
        | Value::Object(_) => to_string(cx, value).map(Some),
    }
}

/// The length of a String, read as `ToObject(string).length`, without copying it.
fn string_length(cx: &mut Cx<'_, ScriptHost>, string: StringRef) -> Completion<f64> {
    let object = engine(cx.to_object(Value::String(string)))?;
    let key = PropertyKey::String(cx.new_string("length")?);
    match engine(cx.get_property(object, key))? {
        Value::Number(length) => Ok(length),
        Value::Undefined
        | Value::Null
        | Value::Boolean(_)
        | Value::String(_)
        | Value::Symbol(_)
        | Value::BigInt(_)
        | Value::Object(_) => Err(missing()),
    }
}

/// ToBoolean (Web IDL's `boolean`).
fn to_boolean(cx: &mut Cx<'_, ScriptHost>, value: Value) -> Completion<bool> {
    match value {
        Value::Undefined | Value::Null => Ok(false),
        Value::Boolean(boolean) => Ok(boolean),
        Value::Number(number) => Ok(!(number == 0.0 || number.is_nan())),
        Value::String(string) => Ok(string_length(cx, string)? != 0.0),
        Value::Symbol(_) | Value::Object(_) => Ok(true),
        Value::BigInt(_) => {
            let digits = to_string(cx, value)?;
            if string_length(cx, digits)? != 1.0 {
                return Ok(true);
            }
            let mut copies = Copies::new();
            Ok(copies.copy(cx, digits)? != "0")
        }
    }
}

/// ConvertToInt (Web IDL) for an unsigned integer of `bits` bits, without `[EnforceRange]` or
/// `[Clamp]`: ToNumber, then the integer part modulo 2^bits.
fn to_unsigned(cx: &mut Cx<'_, ScriptHost>, value: Value, bits: i32) -> Completion<u32> {
    let number = engine(cx.to_number(value))?;
    if !number.is_finite() {
        return Ok(0);
    }
    let modulo = 2f64.powi(bits);
    let integer = number.trunc().rem_euclid(modulo);
    Ok(integer as u32)
}

/// The brand check of a node argument: an object whose brand implements `interface`.
fn to_node(
    cx: &mut Cx<'_, ScriptHost>,
    value: Value,
    interface: Interface,
    member: &str,
    index: usize,
) -> Completion<ObjectRef> {
    match interface_of(cx, value)? {
        Some((object, found)) if found.implements(interface) => Ok(object),
        Some(_) | None => Err(type_error(
            cx,
            &format!(
                "{member}: argument {} does not implement {}",
                index + 1,
                interface.name()
            ),
        )),
    }
}

/// A nullable node argument: `null` and `undefined` are `None`.
fn to_nullable_node(
    cx: &mut Cx<'_, ScriptHost>,
    value: Value,
    interface: Interface,
    member: &str,
    index: usize,
) -> Completion<Option<ObjectRef>> {
    match value {
        Value::Undefined | Value::Null => Ok(None),
        Value::Boolean(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Symbol(_)
        | Value::BigInt(_)
        | Value::Object(_) => to_node(cx, value, interface, member, index).map(Some),
    }
}

/// `(Node or DOMString)` before its copy: a node's wrapper, or a String in the engine.
#[derive(Clone, Copy, Debug)]
enum RawNodeOrString {
    Node(ObjectRef),
    String(StringRef),
}

/// A node, or the text of a new `Text` node: `(Node or DOMString)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeOrString {
    /// A node.
    Node(NodeId),
    /// A string.
    String(String),
}

/// `(Node or DOMString)`: an object whose brand implements `Node` is the node, anything else
/// goes through ToString.
fn to_node_or_string(cx: &mut Cx<'_, ScriptHost>, value: Value) -> Completion<RawNodeOrString> {
    match interface_of(cx, value)? {
        Some((object, found)) if found.implements(Interface::Node) => {
            Ok(RawNodeOrString::Node(object))
        }
        Some(_) | None => to_string(cx, value).map(RawNodeOrString::String),
    }
}

/// The variadic `(Node or DOMString)... nodes` from the argument at `from` on.
fn to_nodes_or_strings(
    cx: &mut Cx<'_, ScriptHost>,
    call: &HostCall<'_>,
    from: usize,
) -> Completion<Vec<RawNodeOrString>> {
    let mut out = Vec::new();
    for &value in call.args.iter().skip(from) {
        out.push(to_node_or_string(cx, value)?);
    }
    Ok(out)
}

/// The copies and the nodes of the variadic `(Node or DOMString)...`.
fn finish_nodes_or_strings(
    cx: &mut DomCx<'_, '_>,
    slices: &Slices,
    copies: &mut Copies,
    raw: Vec<RawNodeOrString>,
    member: &str,
) -> Completion<Vec<NodeOrString>> {
    let mut out = Vec::new();
    for item in raw {
        out.push(match item {
            RawNodeOrString::Node(object) => {
                NodeOrString::Node(unwrap_node(cx, slices, object, member)?)
            }
            RawNodeOrString::String(string) => {
                NodeOrString::String(copies.copy(cx.engine, string)?)
            }
        });
    }
    Ok(out)
}

/// A dictionary member's value: `undefined` when the dictionary is `null` or `undefined`, else
/// Get of the member's name.
fn dictionary_member(
    cx: &mut Cx<'_, ScriptHost>,
    dictionary: Option<ObjectRef>,
    name: &str,
) -> Completion<Value> {
    let Some(object) = dictionary else {
        return Ok(Value::Undefined);
    };
    let key = PropertyKey::String(cx.new_string(name)?);
    engine(cx.get_property(object, key))
}

/// The object of a dictionary argument: `None` for `null` and `undefined`, a `TypeError` for
/// any other value that is not an object.
fn to_dictionary(
    cx: &mut Cx<'_, ScriptHost>,
    value: Value,
    member: &str,
) -> Completion<Option<ObjectRef>> {
    match value {
        Value::Undefined | Value::Null => Ok(None),
        Value::Object(object) => Ok(Some(object)),
        Value::Boolean(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Symbol(_)
        | Value::BigInt(_) => Err(type_error(cx, &format!("{member}: a dictionary is an object"))),
    }
}

/// A dictionary member of an interface type the bindings do not generate: absent
/// (`undefined`) or, when nullable, `null`; any other value throws a `TypeError`, as no value
/// implements that interface.
fn absent_member(
    cx: &mut Cx<'_, ScriptHost>,
    value: Value,
    nullable: bool,
    member: &str,
    interface: &str,
) -> Completion<()> {
    match value {
        Value::Undefined => Ok(()),
        Value::Null if nullable => Ok(()),
        Value::Null
        | Value::Boolean(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Symbol(_)
        | Value::BigInt(_)
        | Value::Object(_) => Err(type_error(
            cx,
            &format!("{member}: no value implements {interface}"),
        )),
    }
}

/// Whether a union argument with a dictionary takes the dictionary: `null`, `undefined` and
/// every object do (Web IDL's conversion of a union).
fn union_takes_dictionary(value: Value) -> bool {
    match value {
        Value::Undefined | Value::Null | Value::Object(_) => true,
        Value::Boolean(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Symbol(_)
        | Value::BigInt(_) => false,
    }
}

/// A Number of `value`.
fn number(value: f64) -> Value {
    Value::Number(value)
}

/// The steps of one node of a collection made or refilled: its wrapper, found or made, and the
/// define of its index. One step a node bought work of about forty steps of a plain walk in the
/// test profile (measured on the reading slice), so a node is charged as eight.
pub const COLLECTION_NODE_STEPS: u64 = 8;

/// The steps of one element of a `sequence<DOMString>` answered, beside the bytes of its
/// String: the String made and the define of its index, charged as a collection node is
/// ([`COLLECTION_NODE_STEPS`]), so a list of many short Strings is not answered for nothing.
pub const STRING_ELEMENT_STEPS: u64 = 8;

/// The bytes of a String answer whose copy into the engine is charged one step: the copy
/// converts each byte to UTF-16 code units, about the time of a plain walk's step for 16 bytes
/// in the test profile (measured on the reading slice). The member charges its own reading of
/// the bytes beside this.
pub const STRING_BYTES_A_STEP: u64 = 16;

/// A new String of `text`: the one conversion of a member's String answer, charged one step per
/// [`STRING_BYTES_A_STEP`] bytes before its code units are counted and before the copy. swell
/// applies `string_length` to a String its own operations create, not to one `Cx::new_string`
/// makes, so a text longer than the agent's `string_length` is the realm's `RangeError` here,
/// before the engine copies it.
fn string_value(cx: &mut DomCx<'_, '_>, text: &str) -> Completion<Value> {
    let bytes = u64::try_from(text.len()).unwrap_or(u64::MAX);
    cx.engine.charge_steps(bytes / STRING_BYTES_A_STEP)?;
    if longer_than(text, cx.string_length()) {
        return Err(string_length_error(cx));
    }
    cx.engine.new_string(text).map(Value::String)
}

/// A new String of the UTF-16 code units `units`: the answer of a member whose String is code
/// units, so a lone half of a surrogate pair stays as it is. Held to `string_length` as
/// [`string_value`]; the count of units is the length.
fn units_value(cx: &mut DomCx<'_, '_>, units: &[u16]) -> Completion<Value> {
    let count = u64::try_from(units.len()).unwrap_or(u64::MAX);
    if count > cx.string_length() {
        return Err(string_length_error(cx));
    }
    cx.engine
        .charge_steps(count.saturating_mul(2) / STRING_BYTES_A_STEP)?;
    cx.engine.new_string_utf16(units).map(Value::String)
}

/// `DOMString?` returned.
fn nullable_string_value(cx: &mut DomCx<'_, '_>, text: Option<String>) -> Completion<Value> {
    match text {
        Some(text) => string_value(cx, &text),
        None => Ok(Value::Null),
    }
}

/// A node returned, through the wrapper hook.
fn node_value(cx: &mut DomCx<'_, '_>, slices: &mut Slices, node: NodeId) -> Completion<Value> {
    slices.wrap.wrap(cx, node)
}

/// A nullable node returned.
fn nullable_node_value(
    cx: &mut DomCx<'_, '_>,
    slices: &mut Slices,
    node: Option<NodeId>,
) -> Completion<Value> {
    match node {
        Some(node) => node_value(cx, slices, node),
        None => Ok(Value::Null),
    }
}

/// A `NodeList` or an `HTMLCollection` returned: a new host object of the interface's brand
/// holding the snapshot of `nodes`, with an indexed own property for each, its wrapper (the
/// deviation of decision 6, until swell's slice 9). [`COLLECTION_NODE_STEPS`] are charged a
/// node.
fn collection_value(
    cx: &mut DomCx<'_, '_>,
    slices: &mut Slices,
    interface: Interface,
    nodes: Vec<NodeId>,
) -> Completion<Value> {
    collection_object(cx, slices, interface, nodes).map(Value::Object)
}

/// The object of [`collection_value`], its `written` the tree's `writes()`.
fn collection_object(
    cx: &mut DomCx<'_, '_>,
    slices: &mut Slices,
    interface: Interface,
    nodes: Vec<NodeId>,
) -> Completion<ObjectRef> {
    let prototype = prototype_object(cx.engine, cx.installed, interface)?;
    let count = u64::try_from(nodes.len()).unwrap_or(u64::MAX);
    cx.engine
        .charge_steps(count.saturating_mul(COLLECTION_NODE_STEPS))?;
    let object = engine(cx.engine.new_host_object(HostObjectSpec {
        prototype: Prototype::Object(prototype),
        brand: interface.brand(),
        data: Some(Box::new(CollectionData {
            nodes: nodes.clone(),
            written: cx.state.tree.writes(),
        })),
        error_kind: None,
    }))?;
    define_indices(cx, slices, object, &nodes)?;
    Ok(object)
}

/// The index properties of a collection: `nodes[index]`'s wrapper at each index.
fn define_indices(
    cx: &mut DomCx<'_, '_>,
    slices: &mut Slices,
    object: ObjectRef,
    nodes: &[NodeId],
) -> Completion<()> {
    for (index, node) in nodes.iter().enumerate() {
        let wrapper = slices.wrap.wrap(cx, *node)?;
        let key = PropertyKey::String(cx.engine.new_string(&index.to_string())?);
        let descriptor = data_property(wrapper, false, true, true);
        engine(cx.engine.define_property_or_throw(object, key, descriptor))?;
    }
    Ok(())
}

/// The message of the `TypeError` a `[SameObject]` getter throws when the script changed its
/// kept collection so that it cannot be refreshed.
pub const NOT_REFRESHED: &str = "the kept collection cannot be refreshed";

/// The key of the index `index`.
fn index_key(cx: &mut DomCx<'_, '_>, index: usize) -> Completion<PropertyKey> {
    Ok(PropertyKey::String(cx.engine.new_string(&index.to_string())?))
}

/// Defines the index `index` of `object` as the glue's data property of `value`: define, never
/// set, so no accessor a script put there runs. `Ok(false)` when the object refuses it.
fn define_index(
    cx: &mut DomCx<'_, '_>,
    object: ObjectRef,
    index: usize,
    value: Value,
) -> Completion<bool> {
    let key = index_key(cx, index)?;
    let descriptor = data_property(value, false, true, true);
    refused(engine(cx.engine.define_property_or_throw(object, key, descriptor)))
}

/// Deletes the index `index` of `object`. `Ok(false)` when the object refuses it.
fn delete_index(cx: &mut DomCx<'_, '_>, object: ObjectRef, index: usize) -> Completion<bool> {
    let key = index_key(cx, index)?;
    refused(engine(cx.engine.delete_property_or_throw(object, key)))
}

/// `Ok(true)` for a step the object took, `Ok(false)` for one it refused with a throw; an end
/// of the run stays an end.
fn refused(step: Completion<()>) -> Completion<bool> {
    match step {
        Ok(()) => Ok(true),
        Err(Abrupt::Throw(_)) => Ok(false),
        Err(end @ Abrupt::Terminated(_)) => Err(end),
    }
}

/// A kept `[SameObject]` collection brought up to the tree, all or nothing: its index
/// properties and its data always show the same list. [`COLLECTION_NODE_STEPS`] are charged per
/// old and per new node, before the work. In this order:
///   1. each old index at or past the new length is defined again as the glue's property of
///      its old node, which fails at once on an index the script made non-configurable, and
///      makes every such index one the glue can delete;
///   2. the new nodes are defined over the indices `0..new` (define, never set);
///   3. the indices `new..old` are deleted, which cannot fail after step 1;
///   4. the data and `written` change last.
///
/// All or nothing holds for a step the object refuses: such a step (frozen, sealed,
/// non-extensible, an index the script made non-configurable) puts back what step 2 wrote (the
/// old node at an old index, nothing past the old length), leaves the data and `written` as
/// they were, and throws a `TypeError` ([`NOT_REFRESHED`]): the object is the old snapshot,
/// whole, and the next read tries again. No index is deleted before every define has
/// succeeded, so a put-back never has to add a property to an object that may refuse new ones.
///
/// An end of the run inside a refill ends the document's scripts instead: an
/// `Abrupt::Terminated` between the first define and the write of the data (the memory limit,
/// met by a wrapper or a key String) is not put back, since a put-back needs the Strings and
/// wrappers the limit just refused. It is recorded ([`DomState::cut_refill`]), and the runner
/// runs no later script of the document, so no script reads the object again.
fn refresh_collection(
    cx: &mut DomCx<'_, '_>,
    slices: &mut Slices,
    object: ObjectRef,
    interface: Interface,
    nodes: Vec<NodeId>,
) -> Completion<()> {
    let old_nodes = match cx
        .engine
        .host_data::<CollectionData>(object, interface.brand())
        .map_err(defect)?
    {
        Some(data) => data.nodes.clone(),
        None => return Err(missing()),
    };
    let count = old_nodes.len().saturating_add(nodes.len());
    cx.engine
        .charge_steps(
            u64::try_from(count)
                .unwrap_or(u64::MAX)
                .saturating_mul(COLLECTION_NODE_STEPS),
        )?;
    let mut changed = false;
    match refill(cx, slices, object, &old_nodes, &nodes, &mut changed) {
        Ok(()) => {}
        Err(end @ Abrupt::Terminated(_)) if changed => {
            cx.state.cut_refill = true;
            return Err(end);
        }
        Err(abrupt) => return Err(abrupt),
    }
    let written = cx.state.tree.writes();
    match cx
        .engine
        .host_data_mut::<CollectionData>(object, interface.brand())
        .map_err(defect)?
    {
        Some(data) => {
            data.nodes = nodes;
            data.written = written;
            Ok(())
        }
        None => Err(missing()),
    }
}

/// Steps 1 to 3 of [`refresh_collection`]. `changed` is true from the first define the object
/// took until a put-back has restored the old snapshot: while it is, the object may differ from
/// its data.
fn refill(
    cx: &mut DomCx<'_, '_>,
    slices: &mut Slices,
    object: ObjectRef,
    old_nodes: &[NodeId],
    nodes: &[NodeId],
    changed: &mut bool,
) -> Completion<()> {
    for (index, node) in old_nodes.iter().enumerate().skip(nodes.len()) {
        let key = index_key(cx, index)?;
        if !engine(cx.engine.has_property(object, key))? {
            continue;
        }
        let wrapper = slices.wrap.wrap(cx, *node)?;
        let taken = define_index(cx, object, index, wrapper)?;
        *changed |= taken;
        if !taken {
            return Err(not_refreshed(cx));
        }
    }
    for (index, node) in nodes.iter().enumerate() {
        let wrapper = slices.wrap.wrap(cx, *node)?;
        let taken = define_index(cx, object, index, wrapper)?;
        *changed |= taken;
        if !taken {
            put_back(cx, slices, object, old_nodes, index)?;
            *changed = false;
            return Err(not_refreshed(cx));
        }
    }
    for index in nodes.len()..old_nodes.len() {
        if !delete_index(cx, object, index)? {
            return Err(missing());
        }
    }
    Ok(())
}

/// Puts back what a refill wrote over the indices `0..failed`: the old node at each index of
/// the old list, nothing at an index past it. Each of these indices holds the glue's
/// configurable property the refill just defined, so the object takes every step.
fn put_back(
    cx: &mut DomCx<'_, '_>,
    slices: &mut Slices,
    object: ObjectRef,
    old_nodes: &[NodeId],
    failed: usize,
) -> Completion<()> {
    for index in 0..failed {
        let taken = match old_nodes.get(index) {
            Some(node) => {
                let wrapper = slices.wrap.wrap(cx, *node)?;
                define_index(cx, object, index, wrapper)?
            }
            None => delete_index(cx, object, index)?,
        };
        if !taken {
            return Err(missing());
        }
    }
    Ok(())
}

/// The `TypeError` of a kept collection the script changed so that it cannot be refreshed.
fn not_refreshed(cx: &mut DomCx<'_, '_>) -> Abrupt {
    let realm = cx.installed.realm;
    error_of(cx.engine, realm, ErrorKind::TypeError, NOT_REFRESHED)
}

/// A `sequence<DOMString>` returned: a new Array of the realm's original `Array` with a String
/// per element. [`STRING_ELEMENT_STEPS`] are charged for each element, in one charge before
/// the loop, and each String's bytes as [`string_value`] charges them.
fn strings_value(cx: &mut DomCx<'_, '_>, strings: Vec<String>) -> Completion<Value> {
    let elements = u64::try_from(strings.len()).unwrap_or(u64::MAX);
    cx.engine
        .charge_steps(elements.saturating_mul(STRING_ELEMENT_STEPS))?;
    let array = cx.engine.get(&cx.installed.array).map_err(defect)?;
    let created = engine(cx.engine.construct(array, &[], None))?;
    let Value::Object(object) = created else {
        return Err(missing());
    };
    for (index, text) in strings.iter().enumerate() {
        let value = string_value(cx, text)?;
        let key = PropertyKey::String(cx.engine.new_string(&index.to_string())?);
        let descriptor = data_property(value, true, true, true);
        engine(cx.engine.define_property_or_throw(object, key, descriptor))?;
    }
    Ok(created)
}

/// The global object, for `window` and `self` (the deviation of decision 6: no `WindowProxy`).
fn global_value(cx: &mut DomCx<'_, '_>) -> Completion<Value> {
    cx.engine.get(&cx.installed.global).map_err(defect)
}

/// The answer of `window` and `self` from the `reading` slice: the global object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TheWindow;

/// A data property's descriptor.
fn data_property(value: Value, writable: bool, enumerable: bool, configurable: bool) -> PropertyDescriptor {
    PropertyDescriptor {
        value: Some(value),
        writable: Some(writable),
        get: None,
        set: None,
        enumerable: Some(enumerable),
        configurable: Some(configurable),
    }
}

/// An accessor property's descriptor.
fn accessor_property(get: Value, set: Value, configurable: bool) -> PropertyDescriptor {
    PropertyDescriptor {
        value: None,
        writable: None,
        get: Some(get),
        set: Some(set),
        enumerable: Some(true),
        configurable: Some(configurable),
    }
}

/// A key of `name`.
fn key(cx: &mut Cx<'_, ScriptHost>, name: &str) -> Completion<PropertyKey> {
    cx.new_string(name).map(PropertyKey::String)
}

/// Defines `name` on `object` with `descriptor`.
fn define(
    cx: &mut Cx<'_, ScriptHost>,
    object: ObjectRef,
    name: &str,
    descriptor: PropertyDescriptor,
) -> Completion<()> {
    let key = key(cx, name)?;
    engine(cx.define_property_or_throw(object, key, descriptor))
}

/// The `[Replaceable]` setter's steps (Web IDL): define an own data property of that name on
/// the global object, holding the value.
fn replace_on_global(cx: &mut Cx<'_, ScriptHost>, call: &HostCall<'_>, name: &str) -> Completion<Value> {
    this_global(cx, call.this, name)?;
    let value = argument(call, 0);
    let global = with_dom(cx, |dcx, _| dcx.engine.get(&dcx.installed.global).map_err(defect))
        .unwrap_or_else(|| Err(missing()))?;
    let Value::Object(global) = global else {
        return Err(missing());
    };
    define(cx, global, name, data_property(value, true, true, true))?;
    Ok(Value::Undefined)
}

/// What a member of an interface is, for the realm's setup.
#[derive(Clone, Copy)]
enum MemberSpec {
    /// A constant, on the interface object and its prototype.
    Constant { name: &'static str, value: f64 },
    /// An attribute: its getter and setter functions.
    Attribute {
        name: &'static str,
        getter: HostFn<ScriptHost>,
        setter: Option<HostFn<ScriptHost>>,
        unforgeable: bool,
    },
    /// An operation, with its `length`.
    Operation {
        name: &'static str,
        steps: HostFn<ScriptHost>,
        length: u32,
        unforgeable: bool,
    },
}

/// One interface for the realm's setup.
struct InterfaceSpec {
    interface: Interface,
    /// The interface object's steps and `length`.
    steps: HostFn<ScriptHost>,
    length: u32,
    members: &'static [MemberSpec],
    /// The names of the members declared `[Unscopable]`.
    unscopables: &'static [&'static str],
    /// Whether it has an indexed property getter: its prototype gets `@@iterator` (Web IDL
    /// 3.7.9).
    indexed: bool,
    /// Whether it has `iterable<T>` and an indexed property getter: its prototype gets the
    /// iteration methods.
    iterable: bool,
}

/// The realm's setup (the epic's "a `with_cx` right after `create_realm`"): for each interface,
/// parents first, its prototype object (a host object chained to its parent's, to
/// `%Error.prototype%` for `DOMException`, to `%Object.prototype%` at the top), its interface
/// object (a host function whose [[Prototype]] is its parent's interface object), its members
/// with Web IDL's property attributes, constants on both, `@@toStringTag`, `@@unscopables`,
/// `@@iterator` of an interface with an indexed getter, the iteration methods of an iterable
/// with one, and the interface object on the global. The global object, made with [`window_spec`] by `create_realm`'s setup, gets the
/// global interface's prototype and its members. Then the bindings go into the host.
pub fn install(
    cx: &mut Cx<'_, ScriptHost>,
    realm: RealmRef,
    global: ObjectRef,
    mut dom: Dom,
) -> Completion<()> {
    let mut prototypes: Vec<Option<ObjectRef>> = vec![None; Interface::ALL.len()];
    let mut objects: Vec<Option<ObjectRef>> = vec![None; Interface::ALL.len()];
    let tag = cx
        .well_known_symbol(WellKnownSymbol::ToStringTag)
        .map_err(defect)?;
    let unscopables_key = cx
        .well_known_symbol(WellKnownSymbol::Unscopables)
        .map_err(defect)?;
    // `@@iterator` of an interface with an indexed getter: `%Array.prototype.values%`, the
    // realm's intrinsic read once here, never through a property a script can replace, and the
    // same function object on every such prototype.
    let iterator = cx
        .well_known_symbol(WellKnownSymbol::Iterator)
        .map_err(defect)?;
    let values = cx
        .intrinsic(realm, HostIntrinsic::ArrayPrototypeValues)
        .map_err(defect)?;
    for spec in &INTERFACES {
        let interface = spec.interface;
        let parent_prototype = match interface.parent() {
            Some(parent) => match prototypes[parent.index()] {
                Some(prototype) => Prototype::Object(prototype),
                None => return Err(missing()),
            },
            None if interface == Interface::DomException => Prototype::Object(
                cx.intrinsic(realm, HostIntrinsic::ErrorPrototype)
                    .map_err(defect)?,
            ),
            None => Prototype::ObjectPrototypeOf(realm),
        };
        let prototype = engine(cx.new_host_object(HostObjectSpec {
            prototype: parent_prototype,
            brand: PROTOTYPE_BRAND,
            data: None,
            error_kind: None,
        }))?;
        let object = engine(cx.new_host_function(
            realm,
            interface.name(),
            spec.length,
            spec.steps,
            true,
            None,
        ))?;
        if let Some(parent) = interface.parent() {
            let Some(parent_object) = objects[parent.index()] else {
                return Err(missing());
            };
            if !engine(cx.set_prototype_of(object, Some(parent_object)))? {
                return Err(missing());
            }
        }
        define(cx, object, "prototype", data_property(Value::Object(prototype), false, false, false))?;
        define(cx, prototype, "constructor", data_property(Value::Object(object), true, false, true))?;
        let name = Value::String(cx.new_string(interface.name())?);
        engine(cx.define_property_or_throw(
            prototype,
            PropertyKey::Symbol(tag),
            data_property(name, false, false, true),
        ))?;
        let holder = if interface == GLOBAL { global } else { prototype };
        for member in spec.members {
            match *member {
                MemberSpec::Constant { name, value } => {
                    define(cx, object, name, data_property(number(value), false, true, false))?;
                    define(cx, prototype, name, data_property(number(value), false, true, false))?;
                }
                MemberSpec::Attribute {
                    name,
                    getter,
                    setter,
                    unforgeable,
                } => {
                    let get = engine(cx.new_host_function(
                        realm,
                        &format!("get {name}"),
                        0,
                        getter,
                        false,
                        None,
                    ))?;
                    let set = match setter {
                        Some(setter) => Value::Object(engine(cx.new_host_function(
                            realm,
                            &format!("set {name}"),
                            1,
                            setter,
                            false,
                            None,
                        ))?),
                        None => Value::Undefined,
                    };
                    let on = if unforgeable { global } else { holder };
                    define(cx, on, name, accessor_property(Value::Object(get), set, !unforgeable))?;
                }
                MemberSpec::Operation {
                    name,
                    steps,
                    length,
                    unforgeable,
                } => {
                    let function =
                        engine(cx.new_host_function(realm, name, length, steps, false, None))?;
                    let on = if unforgeable { global } else { holder };
                    define(
                        cx,
                        on,
                        name,
                        data_property(Value::Object(function), !unforgeable, true, !unforgeable),
                    )?;
                }
            }
        }
        if !spec.unscopables.is_empty() {
            let list = engine(cx.new_host_object(HostObjectSpec {
                prototype: Prototype::Null,
                brand: UNSCOPABLES_BRAND,
                data: None,
                error_kind: None,
            }))?;
            for name in spec.unscopables {
                define(cx, list, name, data_property(Value::Boolean(true), true, true, true))?;
            }
            engine(cx.define_property_or_throw(
                prototype,
                PropertyKey::Symbol(unscopables_key),
                data_property(Value::Object(list), false, false, true),
            ))?;
        }
        if spec.indexed {
            engine(cx.define_property_or_throw(
                prototype,
                PropertyKey::Symbol(iterator),
                data_property(Value::Object(values), true, false, true),
            ))?;
        }
        if spec.iterable {
            for (name, which) in [
                ("entries", HostIntrinsic::ArrayPrototypeEntries),
                ("keys", HostIntrinsic::ArrayPrototypeKeys),
                ("values", HostIntrinsic::ArrayPrototypeValues),
                ("forEach", HostIntrinsic::ArrayPrototypeForEach),
            ] {
                let method = cx.intrinsic(realm, which).map_err(defect)?;
                define(cx, prototype, name, data_property(Value::Object(method), true, true, true))?;
            }
        }
        define(cx, global, interface.name(), data_property(Value::Object(object), true, false, true))?;
        prototypes[interface.index()] = Some(prototype);
        objects[interface.index()] = Some(object);
    }
    let Some(window) = prototypes[GLOBAL.index()] else {
        return Err(missing());
    };
    if !engine(cx.set_prototype_of(global, Some(window)))? {
        return Err(missing());
    }
    let array_key = key(cx, "Array")?;
    let array = engine(cx.get_property(global, array_key))?;
    let mut roots = Vec::new();
    for prototype in prototypes {
        let Some(prototype) = prototype else {
            return Err(missing());
        };
        roots.push(cx.root(Value::Object(prototype)).map_err(defect)?);
    }
    dom.installed = Some(Installed {
        realm,
        prototypes: roots,
        global: cx.root(Value::Object(global)).map_err(defect)?,
        array: cx.root(array).map_err(defect)?,
    });
    cx.host_mut().dom = Some(Box::new(dom));
    Ok(())
}

/// The global object `create_realm`'s setup makes: a host object of the global interface's
/// brand on `%Object.prototype%`, which [`install`] chains to the interface's prototype.
pub fn window_spec(realm: RealmRef) -> HostObjectSpec {
    HostObjectSpec {
        prototype: Prototype::ObjectPrototypeOf(realm),
        brand: GLOBAL.brand(),
        data: None,
        error_kind: None,
    }
}

/// The brand of the interface prototype objects: no member accepts one as `this`.
pub const PROTOTYPE_BRAND: Brand = Brand(0);

/// The brand of the objects of `@@unscopables`.
pub const UNSCOPABLES_BRAND: Brand = Brand(1000);

/// The `DOMException` constructor's steps (Web IDL): `new DOMException(message = "", name =
/// "Error")`, its prototype from `new_target`'s `prototype` when that is an object.
fn dom_exception_constructor(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    let Some(new_target) = call.new_target else {
        return Err(type_error(cx, "DOMException: called without new"));
    };
    let message = if present(&call, 0) {
        Some(to_string(cx, argument(&call, 0))?)
    } else {
        None
    };
    let name = if present(&call, 1) {
        Some(to_string(cx, argument(&call, 1))?)
    } else {
        None
    };
    let prototype_key = key(cx, "prototype")?;
    let from_target = engine(cx.get_property(new_target, prototype_key))?;
    in_dom(cx, |dcx, _| {
        let prototype = match from_target {
            Value::Object(object) => object,
            Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Symbol(_)
            | Value::BigInt(_) => prototype_object(dcx.engine, dcx.installed, Interface::DomException)?,
        };
        let message = match message {
            Some(message) => message,
            None => dcx.engine.new_string("")?,
        };
        let name = match name {
            Some(name) => name,
            None => dcx.engine.new_string("Error")?,
        };
        new_exception(dcx.engine, prototype, name, message).map(Value::Object)
    })
}

/// What a `DOMException` `this` holds, after its brand check.
fn this_exception(cx: &mut Cx<'_, ScriptHost>, this: Value, member: &str) -> Completion<ExceptionData> {
    let object = this_object(cx, this, Interface::DomException, member)?;
    match cx
        .host_data::<ExceptionData>(object, Interface::DomException.brand())
        .map_err(defect)?
    {
        Some(data) => Ok(*data),
        None => Err(type_error(cx, &format!("{member}: `this` holds no exception"))),
    }
}

fn dom_exception_name(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    let data = this_exception(cx, call.this, "DOMException.name")?;
    Ok(Value::String(data.name))
}

fn dom_exception_message(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    let data = this_exception(cx, call.this, "DOMException.message")?;
    Ok(Value::String(data.message))
}

/// The code units of the longest legacy name, `NoModificationAllowedError`.
const LEGACY_NAME_UNITS: f64 = 26.0;

/// The legacy code of the name. A name longer than [`LEGACY_NAME_UNITS`] code units, read with
/// [`string_length`], which copies nothing, has none: 0 without a copy. A shorter name is
/// copied for the comparison with the ledger of [`Copies`] (a step per 16 bytes), and the copy
/// is gone when the getter returns.
fn dom_exception_code(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    let data = this_exception(cx, call.this, "DOMException.code")?;
    if string_length(cx, data.name)? > LEGACY_NAME_UNITS {
        return Ok(number(0.0));
    }
    let mut copies = Copies::new();
    let name = copies.copy(cx, data.name)?;
    Ok(number(f64::from(legacy_code(&name))))
}

/// The steps of an interface object without a constructor: Web IDL throws a `TypeError`.
fn no_constructor(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>, name: &str) -> Completion<Value> {
    let _ = call;
    Err(type_error(cx, &format!("{name} has no constructor")))
}

/// The steps of a constructor of `throws`: a `TypeError` without `new`, else the
/// `NotSupportedError` with its reason.
fn throwing_constructor(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>, name: &str, reason: &str) -> Completion<Value> {
    if call.new_target.is_none() {
        return Err(type_error(cx, &format!("{name}: called without new")));
    }
    Err(throw_now(
        cx,
        &DomException::not_supported(format!("{name} constructor is not supported: {reason}")),
    ))
}

/// The steps of a member of `throws` after its brand check: the `NotSupportedError` naming the
/// member and its reason. It converts no argument.
fn throws(cx: &mut Cx<'_, ScriptHost>, member: &str, reason: &str) -> Completion<Value> {
    Err(throw_now(
        cx,
        &DomException::not_supported(format!("{member} is not supported: {reason}")),
    ))
}
"##;
