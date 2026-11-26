// Copyright © WHATWG (Apple, Google, Mozilla, Microsoft).
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived from
//    this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! The DOM's interfaces for the script engine swell, generated from the Web IDL of the
//! vendored DOM, Web IDL and HTML Standards (`crates/webidl/data/whatwg/`, `SOURCE`) and the
//! selection `crates/webidl/selection.txt`: EventTarget, Node, Element, Document, CharacterData, Text, Comment, ProcessingInstruction, DocumentType, DocumentFragment, NodeList, HTMLCollection, DOMException, Window. A portion of the standards
//! incorporated into source code, under the BSD 3-Clause License above (WHATWG IPR policy,
//! <https://whatwg.org/ipr-policy>).
//!
//! Written by `cargo run -p surf-webidl --bin bindings` (`story:webidl-bindings`). Never
//! edited by hand: a member enters or leaves by a change of the selection, and gets
//! behaviour in the hand-written slice behind its trait (`crate::dom`). `task webidl-check`
//! recomputes the sha256 of each source below and of this whole file (its line at the end of
//! this header), and compares; `task webidl-slow` writes the file again and compares the bytes.
//!
//! The sha256 of each source, its path from the repository root:
//!
//! ```text
//! 80cf04abd88405c6c1f0ec7c28a0a53634df5fdd55e8833c6b6fe68511ca2aa6  crates/webidl/data/whatwg/dom.html
//! 3915f58aa923a43fd03dd6fee040bbf8468856ef51270071b0a8bf11378658fe  crates/webidl/data/whatwg/webidl.html
//! cc3aaecd4aaf3d0e63c7405191056e99d9e16eb64d356e1c8d59d5bc01265371  crates/webidl/data/whatwg/html-idl.html
//! 9e816eebc8bf8fa91016e42ace74903fd5d660240f5b80976f93fd4aed3a56a8  crates/webidl/selection.txt
//! e8b1d6e0efc188db4e1608564a7fcf305e3781c42884985e77e99f7e91d3fe5f  crates/webidl/Cargo.toml
//! 143aaa6923355a7101127f3db0aecd9b7b3d9adeff8689c23628335884d31df1  crates/html/Cargo.toml
//! b9c8d2d59f6f46474c0cfa0b8b77bea21d94fc73a30ad93d5bf6385c422197d2  rust-toolchain.toml
//! f32d2f03b3f4b1e295c79ed49d543f1205c7dac7c3dc74f05a5a27af4fea0584  rustfmt.toml
//! 75291cf6fa7a72d5062db3fca1f659463b3187e258d6a2d34f4f7bcf6ab86b68  crates/webidl/src/bin/bindings.rs
//! 91c0cd2b8dc96f725518046a15c01115e61e0dec125f0ae36656c3a4e73b7356  crates/webidl/src/extract.rs
//! bd7ad49af55281f3d8b2722b1a34a4465ed8b119a9359a1344392fb7afc69a96  crates/webidl/src/generate.rs
//! bb39103dd61d3152cfb7258514bc234c3992f1aec09d79b47b30ef3acb365e10  crates/webidl/src/html.rs
//! 2f5c4d359f13f497b0e1d87d12b0163b20cd7c972a22cf4761106f889ecde900  crates/webidl/src/lexer.rs
//! 9803d546bb474bc834b29498ec0e075e0ef88ceed3de1989601c2ae8cf12eeed  crates/webidl/src/lib.rs
//! fa6e285bfc241f9e68384ebbfd5f04e1c9172fbc254baca4c4fc8e1a154cd930  crates/webidl/src/model.rs
//! 8db6c1d01d7d0b8dc68530edc4f5bf982f1b5cc2b1c49fbbb38496b9ff60dacd  crates/webidl/src/parser.rs
//! 6a24493aa4b7653ab265e5a77051a688da3709842e1f3aa86d053c439d04751e  crates/webidl/src/prelude.rs
//! 22baac7ca4bee83065ca31df9ddab4faf4d6227668cb327360b0473564ed85a3  crates/webidl/src/selection.rs
//! ```
//!
//! What the generator read of each page through `surf-html`: the FNV-1a (64 bits) of the
//! Web IDL text, its bytes and the page's path (a test of `surf-webidl` reads them again):
//!
//! ```text
//! 2bb8d6c403026eb3  23424  crates/webidl/data/whatwg/dom.html
//! 35b61f04d5105dbe  2279  crates/webidl/data/whatwg/webidl.html
//! 36612fcd0caea180  6266  crates/webidl/data/whatwg/html-idl.html
//! ```
//!
//! The sha256 of this whole file, with the 64 digits of this line read as zeros: f1b60e45f89d4b0510107eb818615e9e91a15c80eac7317bbaf03156a9155e29

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
                // End faulted: a resize takes no value, no module and no evaluation, so these
                // answer only a defect; a limit is the inner MemoryBudget, an interrupt the
                // termination the next step reads, never this outer error.
                Err(
                    HostError::WrongKind { .. }
                    | HostError::ModuleStatus { .. }
                    | HostError::ModuleTerminated
                    | HostError::EvaluationInProgress,
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
fn prefix_units(
    cx: &mut Cx<'_, ScriptHost>,
    string: StringRef,
    units: u32,
) -> Completion<Vec<u16>> {
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
/// run's termination (`object/builtin_function.rs:497 @ 0.8.0`); that answer works at the tag
/// but swell does not promise it there.
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
    error_of(
        cx.engine,
        realm,
        ErrorKind::RangeError,
        STRING_LENGTH_MESSAGE,
    )
}

/// An error of `kind` of `realm` with `message`, thrown.
fn error_of(
    cx: &mut Cx<'_, ScriptHost>,
    realm: RealmRef,
    kind: ErrorKind,
    message: &str,
) -> Abrupt {
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
fn required(
    cx: &mut Cx<'_, ScriptHost>,
    call: &HostCall<'_>,
    required: usize,
    member: &str,
) -> Completion<()> {
    if call.args.len() < required {
        return Err(type_error(
            cx,
            &format!(
                "{member}: {required} argument(s) required, {} passed",
                call.args.len()
            ),
        ));
    }
    Ok(())
}

/// The interface the brand of `value` names, if it is an object of a generated brand.
fn interface_of(
    cx: &Cx<'_, ScriptHost>,
    value: Value,
) -> Completion<Option<(ObjectRef, Interface)>> {
    let Value::Object(object) = value else {
        return Ok(None);
    };
    let brand = cx.brand(object).map_err(defect)?;
    Ok(brand
        .and_then(Interface::of_brand)
        .map(|interface| (object, interface)))
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
    match cx
        .host_data::<CollectionData>(object, brand)
        .map_err(defect)?
    {
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
fn to_legacy_null_string(
    cx: &mut Cx<'_, ScriptHost>,
    value: Value,
) -> Completion<Option<StringRef>> {
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
        | Value::BigInt(_) => Err(type_error(
            cx,
            &format!("{member}: a dictionary is an object"),
        )),
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
    Ok(PropertyKey::String(
        cx.engine.new_string(&index.to_string())?,
    ))
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
    refused(engine(
        cx.engine.define_property_or_throw(object, key, descriptor),
    ))
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
    cx.engine.charge_steps(
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
fn data_property(
    value: Value,
    writable: bool,
    enumerable: bool,
    configurable: bool,
) -> PropertyDescriptor {
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
fn replace_on_global(
    cx: &mut Cx<'_, ScriptHost>,
    call: &HostCall<'_>,
    name: &str,
) -> Completion<Value> {
    this_global(cx, call.this, name)?;
    let value = argument(call, 0);
    let global = with_dom(cx, |dcx, _| {
        dcx.engine.get(&dcx.installed.global).map_err(defect)
    })
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
        define(
            cx,
            object,
            "prototype",
            data_property(Value::Object(prototype), false, false, false),
        )?;
        define(
            cx,
            prototype,
            "constructor",
            data_property(Value::Object(object), true, false, true),
        )?;
        let name = Value::String(cx.new_string(interface.name())?);
        engine(cx.define_property_or_throw(
            prototype,
            PropertyKey::Symbol(tag),
            data_property(name, false, false, true),
        ))?;
        let holder = if interface == GLOBAL {
            global
        } else {
            prototype
        };
        for member in spec.members {
            match *member {
                MemberSpec::Constant { name, value } => {
                    define(
                        cx,
                        object,
                        name,
                        data_property(number(value), false, true, false),
                    )?;
                    define(
                        cx,
                        prototype,
                        name,
                        data_property(number(value), false, true, false),
                    )?;
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
                    define(
                        cx,
                        on,
                        name,
                        accessor_property(Value::Object(get), set, !unforgeable),
                    )?;
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
                define(
                    cx,
                    list,
                    name,
                    data_property(Value::Boolean(true), true, true, true),
                )?;
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
                define(
                    cx,
                    prototype,
                    name,
                    data_property(Value::Object(method), true, true, true),
                )?;
            }
        }
        define(
            cx,
            global,
            interface.name(),
            data_property(Value::Object(object), true, false, true),
        )?;
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
            | Value::BigInt(_) => {
                prototype_object(dcx.engine, dcx.installed, Interface::DomException)?
            }
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
fn this_exception(
    cx: &mut Cx<'_, ScriptHost>,
    this: Value,
    member: &str,
) -> Completion<ExceptionData> {
    let object = this_object(cx, this, Interface::DomException, member)?;
    match cx
        .host_data::<ExceptionData>(object, Interface::DomException.brand())
        .map_err(defect)?
    {
        Some(data) => Ok(*data),
        None => Err(type_error(
            cx,
            &format!("{member}: `this` holds no exception"),
        )),
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
fn no_constructor(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
    name: &str,
) -> Completion<Value> {
    let _ = call;
    Err(type_error(cx, &format!("{name} has no constructor")))
}

/// The steps of a constructor of `throws`: a `TypeError` without `new`, else the
/// `NotSupportedError` with its reason.
fn throwing_constructor(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
    name: &str,
    reason: &str,
) -> Completion<Value> {
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

/// The attributes marked `[SameObject]` that return a `NodeList` or an `HTMLCollection`: the key, with the node, of the one object `Wrap` keeps for each (`Wrap::same_object`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SameObject {
    /// `Node.childNodes`.
    NodeChildNodes,
    /// `Element.children`.
    ElementChildren,
    /// `Document.children`.
    DocumentChildren,
    /// `DocumentFragment.children`.
    DocumentFragmentChildren,
}
/// The generated interfaces, parents first. Each is the brand of the objects that implement it, `Brand(index + 1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Interface {
    /// `EventTarget`.
    EventTarget,
    /// `Node`.
    Node,
    /// `Element`.
    Element,
    /// `Document`.
    Document,
    /// `CharacterData`.
    CharacterData,
    /// `Text`.
    Text,
    /// `Comment`.
    Comment,
    /// `ProcessingInstruction`.
    ProcessingInstruction,
    /// `DocumentType`.
    DocumentType,
    /// `DocumentFragment`.
    DocumentFragment,
    /// `NodeList`.
    NodeList,
    /// `HTMLCollection`.
    HtmlCollection,
    /// `DOMException`.
    DomException,
    /// `Window`.
    Window,
}
impl Interface {
    /// Every interface, parents first.
    pub const ALL: [Interface; 14] = [
        Interface::EventTarget,
        Interface::Node,
        Interface::Element,
        Interface::Document,
        Interface::CharacterData,
        Interface::Text,
        Interface::Comment,
        Interface::ProcessingInstruction,
        Interface::DocumentType,
        Interface::DocumentFragment,
        Interface::NodeList,
        Interface::HtmlCollection,
        Interface::DomException,
        Interface::Window,
    ];
    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Interface::EventTarget => "EventTarget",
            Interface::Node => "Node",
            Interface::Element => "Element",
            Interface::Document => "Document",
            Interface::CharacterData => "CharacterData",
            Interface::Text => "Text",
            Interface::Comment => "Comment",
            Interface::ProcessingInstruction => "ProcessingInstruction",
            Interface::DocumentType => "DocumentType",
            Interface::DocumentFragment => "DocumentFragment",
            Interface::NodeList => "NodeList",
            Interface::HtmlCollection => "HTMLCollection",
            Interface::DomException => "DOMException",
            Interface::Window => "Window",
        }
    }
    /// Its place in [`Interface::ALL`].
    pub fn index(self) -> usize {
        match self {
            Interface::EventTarget => 0,
            Interface::Node => 1,
            Interface::Element => 2,
            Interface::Document => 3,
            Interface::CharacterData => 4,
            Interface::Text => 5,
            Interface::Comment => 6,
            Interface::ProcessingInstruction => 7,
            Interface::DocumentType => 8,
            Interface::DocumentFragment => 9,
            Interface::NodeList => 10,
            Interface::HtmlCollection => 11,
            Interface::DomException => 12,
            Interface::Window => 13,
        }
    }
    /// The brand of its objects.
    pub fn brand(self) -> Brand {
        match self {
            Interface::EventTarget => Brand(1),
            Interface::Node => Brand(2),
            Interface::Element => Brand(3),
            Interface::Document => Brand(4),
            Interface::CharacterData => Brand(5),
            Interface::Text => Brand(6),
            Interface::Comment => Brand(7),
            Interface::ProcessingInstruction => Brand(8),
            Interface::DocumentType => Brand(9),
            Interface::DocumentFragment => Brand(10),
            Interface::NodeList => Brand(11),
            Interface::HtmlCollection => Brand(12),
            Interface::DomException => Brand(13),
            Interface::Window => Brand(14),
        }
    }
    /// The interface of `brand`, if it is one of the generated brands.
    pub fn of_brand(brand: Brand) -> Option<Interface> {
        match brand {
            Brand(1) => Some(Interface::EventTarget),
            Brand(2) => Some(Interface::Node),
            Brand(3) => Some(Interface::Element),
            Brand(4) => Some(Interface::Document),
            Brand(5) => Some(Interface::CharacterData),
            Brand(6) => Some(Interface::Text),
            Brand(7) => Some(Interface::Comment),
            Brand(8) => Some(Interface::ProcessingInstruction),
            Brand(9) => Some(Interface::DocumentType),
            Brand(10) => Some(Interface::DocumentFragment),
            Brand(11) => Some(Interface::NodeList),
            Brand(12) => Some(Interface::HtmlCollection),
            Brand(13) => Some(Interface::DomException),
            Brand(14) => Some(Interface::Window),
            Brand(_) => None,
        }
    }
    /// The interface it inherits from.
    pub fn parent(self) -> Option<Interface> {
        match self {
            Interface::EventTarget => None,
            Interface::Node => Some(Interface::EventTarget),
            Interface::Element => Some(Interface::Node),
            Interface::Document => Some(Interface::Node),
            Interface::CharacterData => Some(Interface::Node),
            Interface::Text => Some(Interface::CharacterData),
            Interface::Comment => Some(Interface::CharacterData),
            Interface::ProcessingInstruction => Some(Interface::CharacterData),
            Interface::DocumentType => Some(Interface::Node),
            Interface::DocumentFragment => Some(Interface::Node),
            Interface::NodeList => None,
            Interface::HtmlCollection => None,
            Interface::DomException => None,
            Interface::Window => Some(Interface::EventTarget),
        }
    }
    /// The generated table of which brand inherits which members: whether an object of this interface's brand implements `other` (is it, or inherits from it).
    pub fn implements(self, other: Interface) -> bool {
        match self {
            Interface::EventTarget => matches!(other, Interface::EventTarget),
            Interface::Node => matches!(other, Interface::Node | Interface::EventTarget),
            Interface::Element => matches!(
                other,
                Interface::Element | Interface::Node | Interface::EventTarget
            ),
            Interface::Document => matches!(
                other,
                Interface::Document | Interface::Node | Interface::EventTarget
            ),
            Interface::CharacterData => matches!(
                other,
                Interface::CharacterData | Interface::Node | Interface::EventTarget
            ),
            Interface::Text => matches!(
                other,
                Interface::Text
                    | Interface::CharacterData
                    | Interface::Node
                    | Interface::EventTarget
            ),
            Interface::Comment => matches!(
                other,
                Interface::Comment
                    | Interface::CharacterData
                    | Interface::Node
                    | Interface::EventTarget
            ),
            Interface::ProcessingInstruction => matches!(
                other,
                Interface::ProcessingInstruction
                    | Interface::CharacterData
                    | Interface::Node
                    | Interface::EventTarget
            ),
            Interface::DocumentType => matches!(
                other,
                Interface::DocumentType | Interface::Node | Interface::EventTarget
            ),
            Interface::DocumentFragment => matches!(
                other,
                Interface::DocumentFragment | Interface::Node | Interface::EventTarget
            ),
            Interface::NodeList => matches!(other, Interface::NodeList),
            Interface::HtmlCollection => matches!(other, Interface::HtmlCollection),
            Interface::DomException => matches!(other, Interface::DomException),
            Interface::Window => matches!(other, Interface::Window | Interface::EventTarget),
        }
    }
}
/// The interface of the global object.
const GLOBAL: Interface = Interface::Window;
/// The legacy code of an error name (Web IDL's table of error names), 0 for a name without one.
fn legacy_code(name: &str) -> u16 {
    match name {
        "IndexSizeError" => 1,
        "HierarchyRequestError" => 3,
        "WrongDocumentError" => 4,
        "InvalidCharacterError" => 5,
        "NoModificationAllowedError" => 7,
        "NotFoundError" => 8,
        "NotSupportedError" => 9,
        "InUseAttributeError" => 10,
        "InvalidStateError" => 11,
        "SyntaxError" => 12,
        "InvalidModificationError" => 13,
        "NamespaceError" => 14,
        "InvalidAccessError" => 15,
        "TypeMismatchError" => 17,
        "SecurityError" => 18,
        "NetworkError" => 19,
        "AbortError" => 20,
        "URLMismatchError" => 21,
        "QuotaExceededError" => 22,
        "TimeoutError" => 23,
        "InvalidNodeTypeError" => 24,
        "DataCloneError" => 25,
        _ => 0,
    }
}
/// The dictionary `ElementCreationOptions` converted; a member of an interface type the bindings do not generate is held only as absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementCreationOptions {
    /// `is`.
    pub is: Option<String>,
}
struct RawElementCreationOptions {
    is: Option<StringRef>,
}
fn convert_element_creation_options(
    cx: &mut Cx<'_, ScriptHost>,
    value: Value,
    member: &str,
) -> Completion<RawElementCreationOptions> {
    let object = to_dictionary(cx, value, member)?;
    let m0 = dictionary_member(cx, object, "customElementRegistry")?;
    absent_member(cx, m0, true, member, "CustomElementRegistry")?;
    let m1 = dictionary_member(cx, object, "is")?;
    let m1 = if let Value::Undefined = m1 {
        None
    } else {
        Some(to_string(cx, m1)?)
    };
    Ok(RawElementCreationOptions { is: m1 })
}
fn finish_element_creation_options(
    cx: &mut DomCx<'_, '_>,
    copies: &mut Copies,
    raw: RawElementCreationOptions,
) -> Completion<ElementCreationOptions> {
    let _ = (&cx, &copies);
    Ok(ElementCreationOptions {
        is: match raw.is {
            Some(string) => Some(copies.copy(cx.engine(), string)?),
            None => None,
        },
    })
}
/// The dictionary `GetRootNodeOptions` converted; a member of an interface type the bindings do not generate is held only as absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetRootNodeOptions {
    /// `composed`.
    pub composed: bool,
}
struct RawGetRootNodeOptions {
    composed: bool,
}
fn convert_get_root_node_options(
    cx: &mut Cx<'_, ScriptHost>,
    value: Value,
    member: &str,
) -> Completion<RawGetRootNodeOptions> {
    let object = to_dictionary(cx, value, member)?;
    let m0 = dictionary_member(cx, object, "composed")?;
    let m0 = if let Value::Undefined = m0 {
        false
    } else {
        to_boolean(cx, m0)?
    };
    Ok(RawGetRootNodeOptions { composed: m0 })
}
fn finish_get_root_node_options(
    cx: &mut DomCx<'_, '_>,
    copies: &mut Copies,
    raw: RawGetRootNodeOptions,
) -> Completion<GetRootNodeOptions> {
    let _ = (&cx, &copies);
    Ok(GetRootNodeOptions {
        composed: raw.composed,
    })
}
/// The dictionary `ImportNodeOptions` converted; a member of an interface type the bindings do not generate is held only as absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportNodeOptions {
    /// `selfOnly`.
    pub self_only: bool,
}
struct RawImportNodeOptions {
    self_only: bool,
}
fn convert_import_node_options(
    cx: &mut Cx<'_, ScriptHost>,
    value: Value,
    member: &str,
) -> Completion<RawImportNodeOptions> {
    let object = to_dictionary(cx, value, member)?;
    let m0 = dictionary_member(cx, object, "customElementRegistry")?;
    absent_member(cx, m0, false, member, "CustomElementRegistry")?;
    let m1 = dictionary_member(cx, object, "selfOnly")?;
    let m1 = if let Value::Undefined = m1 {
        false
    } else {
        to_boolean(cx, m1)?
    };
    Ok(RawImportNodeOptions { self_only: m1 })
}
fn finish_import_node_options(
    cx: &mut DomCx<'_, '_>,
    copies: &mut Copies,
    raw: RawImportNodeOptions,
) -> Completion<ImportNodeOptions> {
    let _ = (&cx, &copies);
    Ok(ImportNodeOptions {
        self_only: raw.self_only,
    })
}
/// The union `(boolean or ImportNodeOptions)` converted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BooleanOrImportNodeOptions {
    /// The boolean.
    Boolean(bool),
    /// The dictionary.
    Dictionary(ImportNodeOptions),
}
enum RawBooleanOrImportNodeOptions {
    Boolean(bool),
    Dictionary(RawImportNodeOptions),
}
fn finish_boolean_or_import_node_options(
    cx: &mut DomCx<'_, '_>,
    copies: &mut Copies,
    raw: RawBooleanOrImportNodeOptions,
) -> Completion<BooleanOrImportNodeOptions> {
    Ok(match raw {
        RawBooleanOrImportNodeOptions::Boolean(raw) => BooleanOrImportNodeOptions::Boolean(raw),
        RawBooleanOrImportNodeOptions::Dictionary(raw) => {
            BooleanOrImportNodeOptions::Dictionary(finish_import_node_options(cx, copies, raw)?)
        }
    })
}
/// The union `(DOMString or ElementCreationOptions)` converted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringOrElementCreationOptions {
    /// The string.
    String(String),
    /// The dictionary.
    Dictionary(ElementCreationOptions),
}
enum RawStringOrElementCreationOptions {
    String(StringRef),
    Dictionary(RawElementCreationOptions),
}
fn finish_string_or_element_creation_options(
    cx: &mut DomCx<'_, '_>,
    copies: &mut Copies,
    raw: RawStringOrElementCreationOptions,
) -> Completion<StringOrElementCreationOptions> {
    Ok(match raw {
        RawStringOrElementCreationOptions::String(raw) => {
            StringOrElementCreationOptions::String(copies.copy(cx.engine(), raw)?)
        }
        RawStringOrElementCreationOptions::Dictionary(raw) => {
            StringOrElementCreationOptions::Dictionary(finish_element_creation_options(
                cx, copies, raw,
            )?)
        }
    })
}
/// The members of the slice `reading`: each method's default answers `NotSupportedError` naming the member, until the slice's story writes it in `crate::dom::reading`.
pub trait Reading {
    /// The getter of `Node.nodeType`, `unsigned short`.
    fn node_node_type(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<u16> {
        let _ = (cx, this);
        Err(not_supported("Node.nodeType"))
    }
    /// The getter of `Node.nodeName`, `DOMString`.
    fn node_node_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Node.nodeName"))
    }
    /// The getter of `Node.baseURI`, `USVString`; deviation: no check of a Content Security Policy base-uri directive, surf reads none; deviation: no creator base URL for a document at about:blank or about:srcdoc, surf has no nested document; deviation: the base href parsed as UTF-8 whatever the document's encoding.
    fn node_base_uri(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Node.baseURI"))
    }
    /// The getter of `Node.isConnected`, `boolean`.
    fn node_is_connected(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<bool> {
        let _ = (cx, this);
        Err(not_supported("Node.isConnected"))
    }
    /// The getter of `Node.ownerDocument`, `Document?`.
    fn node_owner_document(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Node.ownerDocument"))
    }
    /// `Node.getRootNode`: `Node getRootNode(optional GetRootNodeOptions options)`.
    fn node_get_root_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        options: GetRootNodeOptions,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, options);
        Err(not_supported("Node.getRootNode"))
    }
    /// The getter of `Node.parentNode`, `Node?`.
    fn node_parent_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Node.parentNode"))
    }
    /// The getter of `Node.parentElement`, `Element?`.
    fn node_parent_element(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Node.parentElement"))
    }
    /// `Node.hasChildNodes`: `boolean hasChildNodes()`.
    fn node_has_child_nodes(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<bool> {
        let _ = (cx, this);
        Err(not_supported("Node.hasChildNodes"))
    }
    /// The getter of `Node.childNodes`, `NodeList`; deviation: a snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114); deviation: \[SameObject\] as the same snapshot object refreshed when read; deviation: a refresh the script blocked by freezing, sealing or making the object non-extensible or an index non-configurable keeps the old snapshot whole and the getter throws TypeError, where a browser refuses that change on the collection itself.
    fn node_child_nodes(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Node.childNodes"))
    }
    /// The getter of `Node.firstChild`, `Node?`.
    fn node_first_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Node.firstChild"))
    }
    /// The getter of `Node.lastChild`, `Node?`.
    fn node_last_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Node.lastChild"))
    }
    /// The getter of `Node.previousSibling`, `Node?`.
    fn node_previous_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Node.previousSibling"))
    }
    /// The getter of `Node.nextSibling`, `Node?`.
    fn node_next_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Node.nextSibling"))
    }
    /// The getter of `Node.nodeValue`, `DOMString?`.
    fn node_node_value(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<String>> {
        let _ = (cx, this);
        Err(not_supported("Node.nodeValue"))
    }
    /// The getter of `Node.textContent`, `DOMString?`.
    fn node_text_content(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<String>> {
        let _ = (cx, this);
        Err(not_supported("Node.textContent"))
    }
    /// `Node.isEqualNode`: `boolean isEqualNode(Node? otherNode)`.
    fn node_is_equal_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        other_node: Option<NodeId>,
    ) -> DomResult<bool> {
        let _ = (cx, this, other_node);
        Err(not_supported("Node.isEqualNode"))
    }
    /// `Node.isSameNode`: `boolean isSameNode(Node? otherNode)`.
    fn node_is_same_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        other_node: Option<NodeId>,
    ) -> DomResult<bool> {
        let _ = (cx, this, other_node);
        Err(not_supported("Node.isSameNode"))
    }
    /// `Node.compareDocumentPosition`: `unsigned short compareDocumentPosition(Node other)`.
    fn node_compare_document_position(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        other: NodeId,
    ) -> DomResult<u16> {
        let _ = (cx, this, other);
        Err(not_supported("Node.compareDocumentPosition"))
    }
    /// `Node.contains`: `boolean contains(Node? other)`.
    fn node_contains(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        other: Option<NodeId>,
    ) -> DomResult<bool> {
        let _ = (cx, this, other);
        Err(not_supported("Node.contains"))
    }
    /// `Node.lookupPrefix`: `DOMString? lookupPrefix(DOMString? namespace)`.
    fn node_lookup_prefix(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
    ) -> DomResult<Option<String>> {
        let _ = (cx, this, namespace);
        Err(not_supported("Node.lookupPrefix"))
    }
    /// `Node.lookupNamespaceURI`: `DOMString? lookupNamespaceURI(DOMString? prefix)`.
    fn node_lookup_namespace_uri(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        prefix: Option<String>,
    ) -> DomResult<Option<String>> {
        let _ = (cx, this, prefix);
        Err(not_supported("Node.lookupNamespaceURI"))
    }
    /// `Node.isDefaultNamespace`: `boolean isDefaultNamespace(DOMString? namespace)`.
    fn node_is_default_namespace(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
    ) -> DomResult<bool> {
        let _ = (cx, this, namespace);
        Err(not_supported("Node.isDefaultNamespace"))
    }
    /// The getter of `Element.namespaceURI`, `DOMString?`.
    fn element_namespace_uri(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<String>> {
        let _ = (cx, this);
        Err(not_supported("Element.namespaceURI"))
    }
    /// The getter of `Element.prefix`, `DOMString?`.
    fn element_prefix(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<String>> {
        let _ = (cx, this);
        Err(not_supported("Element.prefix"))
    }
    /// The getter of `Element.localName`, `DOMString`.
    fn element_local_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Element.localName"))
    }
    /// The getter of `Element.tagName`, `DOMString`.
    fn element_tag_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Element.tagName"))
    }
    /// The getter of `Element.shadowRoot`, `ShadowRoot?`.
    fn element_shadow_root(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Element.shadowRoot"))
    }
    /// `Element.getElementsByTagName`: `HTMLCollection getElementsByTagName(DOMString qualifiedName)`; deviation: a snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114).
    fn element_get_elements_by_tag_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this, qualified_name);
        Err(not_supported("Element.getElementsByTagName"))
    }
    /// `Element.getElementsByTagNameNS`: `HTMLCollection getElementsByTagNameNS(DOMString? namespace, DOMString localName)`; deviation: a snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114).
    fn element_get_elements_by_tag_name_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this, namespace, local_name);
        Err(not_supported("Element.getElementsByTagNameNS"))
    }
    /// `Element.getElementsByClassName`: `HTMLCollection getElementsByClassName(DOMString classNames)`; deviation: a snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114).
    fn element_get_elements_by_class_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        class_names: String,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this, class_names);
        Err(not_supported("Element.getElementsByClassName"))
    }
    /// The getter of `Element.children`, `HTMLCollection`; deviation: a snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114); deviation: \[SameObject\] as the same snapshot object refreshed when read; deviation: a refresh the script blocked by freezing, sealing or making the object non-extensible or an index non-configurable keeps the old snapshot whole and the getter throws TypeError, where a browser refuses that change on the collection itself.
    fn element_children(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Element.children"))
    }
    /// The getter of `Element.firstElementChild`, `Element?`.
    fn element_first_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Element.firstElementChild"))
    }
    /// The getter of `Element.lastElementChild`, `Element?`.
    fn element_last_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Element.lastElementChild"))
    }
    /// The getter of `Element.childElementCount`, `unsigned long`.
    fn element_child_element_count(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<u32> {
        let _ = (cx, this);
        Err(not_supported("Element.childElementCount"))
    }
    /// The getter of `Element.previousElementSibling`, `Element?`.
    fn element_previous_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Element.previousElementSibling"))
    }
    /// The getter of `Element.nextElementSibling`, `Element?`.
    fn element_next_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Element.nextElementSibling"))
    }
    /// The getter of `Element.assignedSlot`, `HTMLSlotElement?`.
    fn element_assigned_slot(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Element.assignedSlot"))
    }
    /// The getter of `Document.URL`, `USVString`.
    fn document_url(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Document.URL"))
    }
    /// The getter of `Document.documentURI`, `USVString`.
    fn document_document_uri(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Document.documentURI"))
    }
    /// The getter of `Document.compatMode`, `DOMString`.
    fn document_compat_mode(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Document.compatMode"))
    }
    /// The getter of `Document.characterSet`, `DOMString`.
    fn document_character_set(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Document.characterSet"))
    }
    /// The getter of `Document.charset`, `DOMString`.
    fn document_charset(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Document.charset"))
    }
    /// The getter of `Document.inputEncoding`, `DOMString`.
    fn document_input_encoding(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Document.inputEncoding"))
    }
    /// The getter of `Document.contentType`, `DOMString`.
    fn document_content_type(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Document.contentType"))
    }
    /// The getter of `Document.doctype`, `DocumentType?`.
    fn document_doctype(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Document.doctype"))
    }
    /// The getter of `Document.documentElement`, `Element?`.
    fn document_document_element(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Document.documentElement"))
    }
    /// `Document.getElementsByTagName`: `HTMLCollection getElementsByTagName(DOMString qualifiedName)`; deviation: a snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114).
    fn document_get_elements_by_tag_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this, qualified_name);
        Err(not_supported("Document.getElementsByTagName"))
    }
    /// `Document.getElementsByTagNameNS`: `HTMLCollection getElementsByTagNameNS(DOMString? namespace, DOMString localName)`; deviation: a snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114).
    fn document_get_elements_by_tag_name_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this, namespace, local_name);
        Err(not_supported("Document.getElementsByTagNameNS"))
    }
    /// `Document.getElementsByClassName`: `HTMLCollection getElementsByClassName(DOMString classNames)`; deviation: a snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114).
    fn document_get_elements_by_class_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        class_names: String,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this, class_names);
        Err(not_supported("Document.getElementsByClassName"))
    }
    /// `Document.getElementById`: `Element? getElementById(DOMString elementId)`.
    fn document_get_element_by_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        element_id: String,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this, element_id);
        Err(not_supported("Document.getElementById"))
    }
    /// The getter of `Document.children`, `HTMLCollection`; deviation: a snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114); deviation: \[SameObject\] as the same snapshot object refreshed when read; deviation: a refresh the script blocked by freezing, sealing or making the object non-extensible or an index non-configurable keeps the old snapshot whole and the getter throws TypeError, where a browser refuses that change on the collection itself.
    fn document_children(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Document.children"))
    }
    /// The getter of `Document.firstElementChild`, `Element?`.
    fn document_first_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Document.firstElementChild"))
    }
    /// The getter of `Document.lastElementChild`, `Element?`.
    fn document_last_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Document.lastElementChild"))
    }
    /// The getter of `Document.childElementCount`, `unsigned long`.
    fn document_child_element_count(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<u32> {
        let _ = (cx, this);
        Err(not_supported("Document.childElementCount"))
    }
    /// The getter of `Document.title`, `DOMString`.
    fn document_title(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Document.title"))
    }
    /// The getter of `CharacterData.data`, `DOMString`.
    fn character_data_data(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("CharacterData.data"))
    }
    /// The getter of `CharacterData.length`, `unsigned long`.
    fn character_data_length(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<u32> {
        let _ = (cx, this);
        Err(not_supported("CharacterData.length"))
    }
    /// `CharacterData.substringData`: `DOMString substringData(unsigned long offset, unsigned long count)`.
    fn character_data_substring_data(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        offset: u32,
        count: u32,
    ) -> DomResult<Vec<u16>> {
        let _ = (cx, this, offset, count);
        Err(not_supported("CharacterData.substringData"))
    }
    /// The getter of `CharacterData.previousElementSibling`, `Element?`.
    fn character_data_previous_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("CharacterData.previousElementSibling"))
    }
    /// The getter of `CharacterData.nextElementSibling`, `Element?`.
    fn character_data_next_element_sibling(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("CharacterData.nextElementSibling"))
    }
    /// The getter of `Text.wholeText`, `DOMString`.
    fn text_whole_text(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Text.wholeText"))
    }
    /// The getter of `Text.assignedSlot`, `HTMLSlotElement?`.
    fn text_assigned_slot(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("Text.assignedSlot"))
    }
    /// The getter of `ProcessingInstruction.target`, `DOMString`.
    fn processing_instruction_target(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("ProcessingInstruction.target"))
    }
    /// The getter of `DocumentType.name`, `DOMString`.
    fn document_type_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("DocumentType.name"))
    }
    /// The getter of `DocumentType.publicId`, `DOMString`.
    fn document_type_public_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("DocumentType.publicId"))
    }
    /// The getter of `DocumentType.systemId`, `DOMString`.
    fn document_type_system_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("DocumentType.systemId"))
    }
    /// `DocumentFragment.getElementById`: `Element? getElementById(DOMString elementId)`.
    fn document_fragment_get_element_by_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        element_id: String,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this, element_id);
        Err(not_supported("DocumentFragment.getElementById"))
    }
    /// The getter of `DocumentFragment.children`, `HTMLCollection`; deviation: a snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114); deviation: \[SameObject\] as the same snapshot object refreshed when read; deviation: a refresh the script blocked by freezing, sealing or making the object non-extensible or an index non-configurable keeps the old snapshot whole and the getter throws TypeError, where a browser refuses that change on the collection itself.
    fn document_fragment_children(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("DocumentFragment.children"))
    }
    /// The getter of `DocumentFragment.firstElementChild`, `Element?`.
    fn document_fragment_first_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("DocumentFragment.firstElementChild"))
    }
    /// The getter of `DocumentFragment.lastElementChild`, `Element?`.
    fn document_fragment_last_element_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this);
        Err(not_supported("DocumentFragment.lastElementChild"))
    }
    /// The getter of `DocumentFragment.childElementCount`, `unsigned long`.
    fn document_fragment_child_element_count(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<u32> {
        let _ = (cx, this);
        Err(not_supported("DocumentFragment.childElementCount"))
    }
    /// `NodeList.item`: `Node? item(unsigned long index)`.
    fn node_list_item(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: &[NodeId],
        index: u32,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this, index);
        Err(not_supported("NodeList.item"))
    }
    /// The getter of `NodeList.length`, `unsigned long`.
    fn node_list_length(&mut self, cx: &mut DomCx<'_, '_>, this: &[NodeId]) -> DomResult<u32> {
        let _ = (cx, this);
        Err(not_supported("NodeList.length"))
    }
    /// The getter of `HTMLCollection.length`, `unsigned long`; deviation: no named properties (collection\["id"\] is undefined, namedItem answers) until a later story.
    fn html_collection_length(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: &[NodeId],
    ) -> DomResult<u32> {
        let _ = (cx, this);
        Err(not_supported("HTMLCollection.length"))
    }
    /// `HTMLCollection.item`: `Element? item(unsigned long index)`; deviation: no named properties (collection\["id"\] is undefined, namedItem answers) until a later story.
    fn html_collection_item(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: &[NodeId],
        index: u32,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this, index);
        Err(not_supported("HTMLCollection.item"))
    }
    /// `HTMLCollection.namedItem`: `Element? namedItem(DOMString name)`; deviation: no named properties (collection\["id"\] is undefined, namedItem answers) until a later story.
    fn html_collection_named_item(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: &[NodeId],
        name: String,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this, name);
        Err(not_supported("HTMLCollection.namedItem"))
    }
    /// The getter of `Window.window`, `WindowProxy`; deviation: the global this is the Window, no WindowProxy.
    fn window_window(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<TheWindow> {
        let _ = cx;
        Err(not_supported("Window.window"))
    }
    /// The getter of `Window.self`, `WindowProxy`; deviation: the global this is the Window, no WindowProxy.
    fn window_self(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<TheWindow> {
        let _ = cx;
        Err(not_supported("Window.self"))
    }
    /// The getter of `Window.document`, `Document`.
    fn window_document(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<NodeId> {
        let _ = cx;
        Err(not_supported("Window.document"))
    }
    /// The getter of `Window.event`, `(Event or undefined)`.
    fn window_event(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<()> {
        let _ = cx;
        Err(not_supported("Window.event"))
    }
}
/// The members of the slice `attributes`: each method's default answers `NotSupportedError` naming the member, until the slice's story writes it in `crate::dom::attributes`.
pub trait Attributes {
    /// The getter of `Element.id`, `DOMString`.
    fn element_id(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Element.id"))
    }
    /// The setter of `Element.id`, `DOMString`.
    fn set_element_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: String,
    ) -> DomResult<()> {
        let _ = (cx, this, value);
        Err(not_supported("Element.id"))
    }
    /// The getter of `Element.className`, `DOMString`.
    fn element_class_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Element.className"))
    }
    /// The setter of `Element.className`, `DOMString`.
    fn set_element_class_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: String,
    ) -> DomResult<()> {
        let _ = (cx, this, value);
        Err(not_supported("Element.className"))
    }
    /// The getter of `Element.slot`, `DOMString`.
    fn element_slot(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        let _ = (cx, this);
        Err(not_supported("Element.slot"))
    }
    /// The setter of `Element.slot`, `DOMString`.
    fn set_element_slot(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: String,
    ) -> DomResult<()> {
        let _ = (cx, this, value);
        Err(not_supported("Element.slot"))
    }
    /// `Element.hasAttributes`: `boolean hasAttributes()`.
    fn element_has_attributes(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<bool> {
        let _ = (cx, this);
        Err(not_supported("Element.hasAttributes"))
    }
    /// `Element.getAttributeNames`: `sequence<DOMString> getAttributeNames()`.
    fn element_get_attribute_names(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Vec<String>> {
        let _ = (cx, this);
        Err(not_supported("Element.getAttributeNames"))
    }
    /// `Element.getAttribute`: `DOMString? getAttribute(DOMString qualifiedName)`.
    fn element_get_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<Option<String>> {
        let _ = (cx, this, qualified_name);
        Err(not_supported("Element.getAttribute"))
    }
    /// `Element.getAttributeNS`: `DOMString? getAttributeNS(DOMString? namespace, DOMString localName)`.
    fn element_get_attribute_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<Option<String>> {
        let _ = (cx, this, namespace, local_name);
        Err(not_supported("Element.getAttributeNS"))
    }
    /// `Element.setAttribute`: `undefined setAttribute(DOMString qualifiedName, (TrustedType or DOMString) value)`; deviation: (TrustedType or DOMString) read as DOMString.
    fn element_set_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
        value: String,
    ) -> DomResult<()> {
        let _ = (cx, this, qualified_name, value);
        Err(not_supported("Element.setAttribute"))
    }
    /// `Element.setAttributeNS`: `undefined setAttributeNS(DOMString? namespace, DOMString qualifiedName, (TrustedType or DOMString) value)`; deviation: (TrustedType or DOMString) read as DOMString; deviation: a namespace that is none of the six the tree stores (HTML, MathML, SVG, XLink, XML, XMLNS) throws NotSupportedError after validate and extract, before the attribute is looked up.
    fn element_set_attribute_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        qualified_name: String,
        value: String,
    ) -> DomResult<()> {
        let _ = (cx, this, namespace, qualified_name, value);
        Err(not_supported("Element.setAttributeNS"))
    }
    /// `Element.removeAttribute`: `undefined removeAttribute(DOMString qualifiedName)`.
    fn element_remove_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<()> {
        let _ = (cx, this, qualified_name);
        Err(not_supported("Element.removeAttribute"))
    }
    /// `Element.removeAttributeNS`: `undefined removeAttributeNS(DOMString? namespace, DOMString localName)`.
    fn element_remove_attribute_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<()> {
        let _ = (cx, this, namespace, local_name);
        Err(not_supported("Element.removeAttributeNS"))
    }
    /// `Element.toggleAttribute`: `boolean toggleAttribute(DOMString qualifiedName, optional boolean force)`.
    fn element_toggle_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
        force: Option<bool>,
    ) -> DomResult<bool> {
        let _ = (cx, this, qualified_name, force);
        Err(not_supported("Element.toggleAttribute"))
    }
    /// `Element.hasAttribute`: `boolean hasAttribute(DOMString qualifiedName)`.
    fn element_has_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<bool> {
        let _ = (cx, this, qualified_name);
        Err(not_supported("Element.hasAttribute"))
    }
    /// `Element.hasAttributeNS`: `boolean hasAttributeNS(DOMString? namespace, DOMString localName)`.
    fn element_has_attribute_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<bool> {
        let _ = (cx, this, namespace, local_name);
        Err(not_supported("Element.hasAttributeNS"))
    }
}
/// The members of the slice `mutation`: each method's default answers `NotSupportedError` naming the member, until the slice's story writes it in `crate::dom::mutation`.
pub trait Mutation {
    /// The setter of `Node.nodeValue`, `DOMString?`.
    fn set_node_node_value(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: Option<String>,
    ) -> DomResult<()> {
        let _ = (cx, this, value);
        Err(not_supported("Node.nodeValue"))
    }
    /// The setter of `Node.textContent`, `DOMString?`.
    fn set_node_text_content(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: Option<String>,
    ) -> DomResult<()> {
        let _ = (cx, this, value);
        Err(not_supported("Node.textContent"))
    }
    /// `Node.normalize`: `undefined normalize()`.
    fn node_normalize(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<()> {
        let _ = (cx, this);
        Err(not_supported("Node.normalize"))
    }
    /// `Node.cloneNode`: `Node cloneNode(optional boolean subtree)`.
    fn node_clone_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        subtree: bool,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, subtree);
        Err(not_supported("Node.cloneNode"))
    }
    /// `Node.insertBefore`: `Node insertBefore(Node node, Node? child)`.
    fn node_insert_before(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
        child: Option<NodeId>,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, node, child);
        Err(not_supported("Node.insertBefore"))
    }
    /// `Node.appendChild`: `Node appendChild(Node node)`.
    fn node_append_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, node);
        Err(not_supported("Node.appendChild"))
    }
    /// `Node.replaceChild`: `Node replaceChild(Node node, Node child)`.
    fn node_replace_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
        child: NodeId,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, node, child);
        Err(not_supported("Node.replaceChild"))
    }
    /// `Node.removeChild`: `Node removeChild(Node child)`.
    fn node_remove_child(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        child: NodeId,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, child);
        Err(not_supported("Node.removeChild"))
    }
    /// `Element.insertAdjacentElement`: `Element? insertAdjacentElement(DOMString where, Element element)`.
    fn element_insert_adjacent_element(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        r#where: String,
        element: NodeId,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this, r#where, element);
        Err(not_supported("Element.insertAdjacentElement"))
    }
    /// `Element.insertAdjacentText`: `undefined insertAdjacentText(DOMString where, DOMString data)`.
    fn element_insert_adjacent_text(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        r#where: String,
        data: String,
    ) -> DomResult<()> {
        let _ = (cx, this, r#where, data);
        Err(not_supported("Element.insertAdjacentText"))
    }
    /// `Element.prepend`: `undefined prepend((Node or DOMString)... nodes)`.
    fn element_prepend(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("Element.prepend"))
    }
    /// `Element.append`: `undefined append((Node or DOMString)... nodes)`.
    fn element_append(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("Element.append"))
    }
    /// `Element.replaceChildren`: `undefined replaceChildren((Node or DOMString)... nodes)`.
    fn element_replace_children(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("Element.replaceChildren"))
    }
    /// `Element.moveBefore`: `undefined moveBefore(Node node, Node? child)`.
    fn element_move_before(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
        child: Option<NodeId>,
    ) -> DomResult<()> {
        let _ = (cx, this, node, child);
        Err(not_supported("Element.moveBefore"))
    }
    /// `Element.before`: `undefined before((Node or DOMString)... nodes)`.
    fn element_before(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("Element.before"))
    }
    /// `Element.after`: `undefined after((Node or DOMString)... nodes)`.
    fn element_after(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("Element.after"))
    }
    /// `Element.replaceWith`: `undefined replaceWith((Node or DOMString)... nodes)`.
    fn element_replace_with(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("Element.replaceWith"))
    }
    /// `Element.remove`: `undefined remove()`.
    fn element_remove(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<()> {
        let _ = (cx, this);
        Err(not_supported("Element.remove"))
    }
    /// `Document.createElement`: `Element createElement(DOMString localName, optional (DOMString or ElementCreationOptions) options)`; deviation: an element's wrapper is an Element.
    fn document_create_element(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        local_name: String,
        options: StringOrElementCreationOptions,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, local_name, options);
        Err(not_supported("Document.createElement"))
    }
    /// `Document.createElementNS`: `Element createElementNS(DOMString? namespace, DOMString qualifiedName, optional (DOMString or ElementCreationOptions) options)`; deviation: an element's wrapper is an Element.
    fn document_create_element_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        qualified_name: String,
        options: StringOrElementCreationOptions,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, namespace, qualified_name, options);
        Err(not_supported("Document.createElementNS"))
    }
    /// `Document.createDocumentFragment`: `DocumentFragment createDocumentFragment()`.
    fn document_create_document_fragment(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<NodeId> {
        let _ = (cx, this);
        Err(not_supported("Document.createDocumentFragment"))
    }
    /// `Document.createTextNode`: `Text createTextNode(DOMString data)`.
    fn document_create_text_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        data: String,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, data);
        Err(not_supported("Document.createTextNode"))
    }
    /// `Document.createCDATASection`: `CDATASection createCDATASection(DOMString data)`.
    fn document_create_cdata_section(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        data: String,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, data);
        Err(not_supported("Document.createCDATASection"))
    }
    /// `Document.createComment`: `Comment createComment(DOMString data)`.
    fn document_create_comment(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        data: String,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, data);
        Err(not_supported("Document.createComment"))
    }
    /// `Document.createProcessingInstruction`: `ProcessingInstruction createProcessingInstruction(DOMString target, DOMString data)`.
    fn document_create_processing_instruction(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        target: String,
        data: String,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, target, data);
        Err(not_supported("Document.createProcessingInstruction"))
    }
    /// `Document.importNode`: `Node importNode(Node node, optional (boolean or ImportNodeOptions) options)`.
    fn document_import_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
        options: BooleanOrImportNodeOptions,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, node, options);
        Err(not_supported("Document.importNode"))
    }
    /// `Document.adoptNode`: `Node adoptNode(Node node)`.
    fn document_adopt_node(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, node);
        Err(not_supported("Document.adoptNode"))
    }
    /// `Document.prepend`: `undefined prepend((Node or DOMString)... nodes)`.
    fn document_prepend(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("Document.prepend"))
    }
    /// `Document.append`: `undefined append((Node or DOMString)... nodes)`.
    fn document_append(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("Document.append"))
    }
    /// `Document.replaceChildren`: `undefined replaceChildren((Node or DOMString)... nodes)`.
    fn document_replace_children(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("Document.replaceChildren"))
    }
    /// `Document.moveBefore`: `undefined moveBefore(Node node, Node? child)`.
    fn document_move_before(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
        child: Option<NodeId>,
    ) -> DomResult<()> {
        let _ = (cx, this, node, child);
        Err(not_supported("Document.moveBefore"))
    }
    /// The setter of `Document.title`, `DOMString`.
    fn set_document_title(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: String,
    ) -> DomResult<()> {
        let _ = (cx, this, value);
        Err(not_supported("Document.title"))
    }
    /// The setter of `CharacterData.data`, `DOMString`.
    fn set_character_data_data(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: String,
    ) -> DomResult<()> {
        let _ = (cx, this, value);
        Err(not_supported("CharacterData.data"))
    }
    /// `CharacterData.appendData`: `undefined appendData(DOMString data)`.
    fn character_data_append_data(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        data: String,
    ) -> DomResult<()> {
        let _ = (cx, this, data);
        Err(not_supported("CharacterData.appendData"))
    }
    /// `CharacterData.insertData`: `undefined insertData(unsigned long offset, DOMString data)`.
    fn character_data_insert_data(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        offset: u32,
        data: String,
    ) -> DomResult<()> {
        let _ = (cx, this, offset, data);
        Err(not_supported("CharacterData.insertData"))
    }
    /// `CharacterData.deleteData`: `undefined deleteData(unsigned long offset, unsigned long count)`.
    fn character_data_delete_data(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        offset: u32,
        count: u32,
    ) -> DomResult<()> {
        let _ = (cx, this, offset, count);
        Err(not_supported("CharacterData.deleteData"))
    }
    /// `CharacterData.replaceData`: `undefined replaceData(unsigned long offset, unsigned long count, DOMString data)`.
    fn character_data_replace_data(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        offset: u32,
        count: u32,
        data: String,
    ) -> DomResult<()> {
        let _ = (cx, this, offset, count, data);
        Err(not_supported("CharacterData.replaceData"))
    }
    /// `CharacterData.before`: `undefined before((Node or DOMString)... nodes)`.
    fn character_data_before(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("CharacterData.before"))
    }
    /// `CharacterData.after`: `undefined after((Node or DOMString)... nodes)`.
    fn character_data_after(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("CharacterData.after"))
    }
    /// `CharacterData.replaceWith`: `undefined replaceWith((Node or DOMString)... nodes)`.
    fn character_data_replace_with(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("CharacterData.replaceWith"))
    }
    /// `CharacterData.remove`: `undefined remove()`.
    fn character_data_remove(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<()> {
        let _ = (cx, this);
        Err(not_supported("CharacterData.remove"))
    }
    /// `new Text(...)`.
    fn text_constructor(&mut self, cx: &mut DomCx<'_, '_>, data: String) -> DomResult<NodeId> {
        let _ = (cx, data);
        Err(not_supported("Text constructor"))
    }
    /// `Text.splitText`: `Text splitText(unsigned long offset)`.
    fn text_split_text(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        offset: u32,
    ) -> DomResult<NodeId> {
        let _ = (cx, this, offset);
        Err(not_supported("Text.splitText"))
    }
    /// `new Comment(...)`.
    fn comment_constructor(&mut self, cx: &mut DomCx<'_, '_>, data: String) -> DomResult<NodeId> {
        let _ = (cx, data);
        Err(not_supported("Comment constructor"))
    }
    /// `new ProcessingInstruction(...)`.
    fn processing_instruction_constructor(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        target: String,
        data: String,
    ) -> DomResult<NodeId> {
        let _ = (cx, target, data);
        Err(not_supported("ProcessingInstruction constructor"))
    }
    /// `DocumentType.before`: `undefined before((Node or DOMString)... nodes)`.
    fn document_type_before(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("DocumentType.before"))
    }
    /// `DocumentType.after`: `undefined after((Node or DOMString)... nodes)`.
    fn document_type_after(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("DocumentType.after"))
    }
    /// `DocumentType.replaceWith`: `undefined replaceWith((Node or DOMString)... nodes)`.
    fn document_type_replace_with(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("DocumentType.replaceWith"))
    }
    /// `DocumentType.remove`: `undefined remove()`.
    fn document_type_remove(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<()> {
        let _ = (cx, this);
        Err(not_supported("DocumentType.remove"))
    }
    /// `new DocumentFragment(...)`.
    fn document_fragment_constructor(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<NodeId> {
        let _ = cx;
        Err(not_supported("DocumentFragment constructor"))
    }
    /// `DocumentFragment.prepend`: `undefined prepend((Node or DOMString)... nodes)`.
    fn document_fragment_prepend(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("DocumentFragment.prepend"))
    }
    /// `DocumentFragment.append`: `undefined append((Node or DOMString)... nodes)`.
    fn document_fragment_append(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("DocumentFragment.append"))
    }
    /// `DocumentFragment.replaceChildren`: `undefined replaceChildren((Node or DOMString)... nodes)`.
    fn document_fragment_replace_children(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        nodes: Vec<NodeOrString>,
    ) -> DomResult<()> {
        let _ = (cx, this, nodes);
        Err(not_supported("DocumentFragment.replaceChildren"))
    }
    /// `DocumentFragment.moveBefore`: `undefined moveBefore(Node node, Node? child)`.
    fn document_fragment_move_before(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        node: NodeId,
        child: Option<NodeId>,
    ) -> DomResult<()> {
        let _ = (cx, this, node, child);
        Err(not_supported("DocumentFragment.moveBefore"))
    }
}
/// The members of the slice `selectors`: each method's default answers `NotSupportedError` naming the member, until the slice's story writes it in `crate::dom::selectors`.
pub trait Selectors {
    /// `Element.closest`: `Element? closest(DOMString selectors)`.
    fn element_closest(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this, selectors);
        Err(not_supported("Element.closest"))
    }
    /// `Element.matches`: `boolean matches(DOMString selectors)`.
    fn element_matches(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<bool> {
        let _ = (cx, this, selectors);
        Err(not_supported("Element.matches"))
    }
    /// `Element.webkitMatchesSelector`: `boolean webkitMatchesSelector(DOMString selectors)`.
    fn element_webkit_matches_selector(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<bool> {
        let _ = (cx, this, selectors);
        Err(not_supported("Element.webkitMatchesSelector"))
    }
    /// `Element.querySelector`: `Element? querySelector(DOMString selectors)`.
    fn element_query_selector(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this, selectors);
        Err(not_supported("Element.querySelector"))
    }
    /// `Element.querySelectorAll`: `NodeList querySelectorAll(DOMString selectors)`.
    fn element_query_selector_all(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this, selectors);
        Err(not_supported("Element.querySelectorAll"))
    }
    /// `Document.querySelector`: `Element? querySelector(DOMString selectors)`.
    fn document_query_selector(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this, selectors);
        Err(not_supported("Document.querySelector"))
    }
    /// `Document.querySelectorAll`: `NodeList querySelectorAll(DOMString selectors)`.
    fn document_query_selector_all(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this, selectors);
        Err(not_supported("Document.querySelectorAll"))
    }
    /// `DocumentFragment.querySelector`: `Element? querySelector(DOMString selectors)`.
    fn document_fragment_query_selector(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Option<NodeId>> {
        let _ = (cx, this, selectors);
        Err(not_supported("DocumentFragment.querySelector"))
    }
    /// `DocumentFragment.querySelectorAll`: `NodeList querySelectorAll(DOMString selectors)`.
    fn document_fragment_query_selector_all(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        selectors: String,
    ) -> DomResult<Vec<NodeId>> {
        let _ = (cx, this, selectors);
        Err(not_supported("DocumentFragment.querySelectorAll"))
    }
}
fn interface_event_target(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    throwing_constructor(cx, call, "EventTarget", "events")
}
/// `EventTarget.addEventListener`, of `throws` (events): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_event_target_add_event_listener(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::EventTarget,
        "EventTarget.addEventListener",
    )?;
    throws(cx, "EventTarget.addEventListener", "events")
}
/// `EventTarget.removeEventListener`, of `throws` (events): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_event_target_remove_event_listener(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::EventTarget,
        "EventTarget.removeEventListener",
    )?;
    throws(cx, "EventTarget.removeEventListener", "events")
}
/// `EventTarget.dispatchEvent`, of `throws` (events): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_event_target_dispatch_event(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::EventTarget,
        "EventTarget.dispatchEvent",
    )?;
    throws(cx, "EventTarget.dispatchEvent", "events")
}
fn getter_node_node_type(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.nodeType";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_node_type(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| {
            Ok(number(f64::from(value)))
        })
    })
}
fn getter_node_node_name(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.nodeName";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_node_name(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_node_base_uri(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.baseURI";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_base_uri(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_node_is_connected(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.isConnected";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_is_connected(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
fn getter_node_owner_document(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.ownerDocument";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_owner_document(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_node_get_root_node(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.getRootNode";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    let a0 = convert_get_root_node_options(cx, argument(&call, 0), MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_get_root_node_options(dcx, &mut copies, a0)?;
        let answer = slices.reading.node_get_root_node(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn getter_node_parent_node(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.parentNode";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_parent_node(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_node_parent_element(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.parentElement";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_parent_element(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_node_has_child_nodes(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.hasChildNodes";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_has_child_nodes(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
fn getter_node_child_nodes(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.childNodes";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let writes = dcx.tree().writes();
        let kept = slices
            .wrap
            .same_object(dcx, this, SameObject::NodeChildNodes)?;
        if let Some(object) = kept
            && collection_written(dcx, object, Interface::NodeList)? == writes
        {
            return Ok(Value::Object(object));
        }
        let answer = slices.reading.node_child_nodes(dcx, this);
        let nodes = answered(dcx, answer)?;
        match kept {
            Some(object) => {
                refresh_collection(dcx, slices, object, Interface::NodeList, nodes)?;
                Ok(Value::Object(object))
            }
            None => {
                let object = collection_object(dcx, slices, Interface::NodeList, nodes)?;
                slices
                    .wrap
                    .keep_same_object(dcx, this, SameObject::NodeChildNodes, object)?;
                Ok(Value::Object(object))
            }
        }
    })
}
fn getter_node_first_child(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.firstChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_first_child(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_node_last_child(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.lastChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_last_child(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_node_previous_sibling(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.previousSibling";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_previous_sibling(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_node_next_sibling(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.nextSibling";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_next_sibling(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_node_node_value(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.nodeValue";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_node_value(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            nullable_string_value(dcx, value)
        })
    })
}
fn setter_node_node_value(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.nodeValue";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let answer = slices.mutation.set_node_node_value(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn getter_node_text_content(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.textContent";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.node_text_content(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            nullable_string_value(dcx, value)
        })
    })
}
fn setter_node_text_content(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.textContent";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let answer = slices.mutation.set_node_text_content(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_node_normalize(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.normalize";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.mutation.node_normalize(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_node_clone_node(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.cloneNode";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    let a0 = if present(&call, 0) {
        Some(to_boolean(cx, argument(&call, 0))?)
    } else {
        None
    };
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = a0.unwrap_or_default();
        let answer = slices.mutation.node_clone_node(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_node_is_equal_node(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.isEqualNode";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_nullable_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(object) => Some(unwrap_node(dcx, slices, object, MEMBER)?),
            None => None,
        };
        let answer = slices.reading.node_is_equal_node(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
fn operation_node_is_same_node(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.isSameNode";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_nullable_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(object) => Some(unwrap_node(dcx, slices, object, MEMBER)?),
            None => None,
        };
        let answer = slices.reading.node_is_same_node(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
fn operation_node_compare_document_position(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.compareDocumentPosition";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = unwrap_node(dcx, slices, a0, MEMBER)?;
        let answer = slices.reading.node_compare_document_position(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| {
            Ok(number(f64::from(value)))
        })
    })
}
fn operation_node_contains(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Node.contains";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_nullable_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(object) => Some(unwrap_node(dcx, slices, object, MEMBER)?),
            None => None,
        };
        let answer = slices.reading.node_contains(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
fn operation_node_lookup_prefix(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.lookupPrefix";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let answer = slices.reading.node_lookup_prefix(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            nullable_string_value(dcx, value)
        })
    })
}
fn operation_node_lookup_namespace_uri(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.lookupNamespaceURI";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let answer = slices.reading.node_lookup_namespace_uri(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            nullable_string_value(dcx, value)
        })
    })
}
fn operation_node_is_default_namespace(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.isDefaultNamespace";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let answer = slices.reading.node_is_default_namespace(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
fn operation_node_insert_before(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.insertBefore";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    let a1 = to_nullable_node(cx, argument(&call, 1), Interface::Node, MEMBER, 1)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = unwrap_node(dcx, slices, a0, MEMBER)?;
        let a1 = match a1 {
            Some(object) => Some(unwrap_node(dcx, slices, object, MEMBER)?),
            None => None,
        };
        let answer = slices.mutation.node_insert_before(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_node_append_child(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.appendChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = unwrap_node(dcx, slices, a0, MEMBER)?;
        let answer = slices.mutation.node_append_child(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_node_replace_child(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.replaceChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    let a1 = to_node(cx, argument(&call, 1), Interface::Node, MEMBER, 1)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = unwrap_node(dcx, slices, a0, MEMBER)?;
        let a1 = unwrap_node(dcx, slices, a1, MEMBER)?;
        let answer = slices.mutation.node_replace_child(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_node_remove_child(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Node.removeChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Node, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = unwrap_node(dcx, slices, a0, MEMBER)?;
        let answer = slices.mutation.node_remove_child(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn interface_node(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    no_constructor(cx, call, "Node")
}
fn getter_element_namespace_uri(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.namespaceURI";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_namespace_uri(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            nullable_string_value(dcx, value)
        })
    })
}
fn getter_element_prefix(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.prefix";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_prefix(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            nullable_string_value(dcx, value)
        })
    })
}
fn getter_element_local_name(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.localName";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_local_name(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_element_tag_name(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.tagName";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_tag_name(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_element_id(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.id";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.attributes.element_id(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn setter_element_id(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.id";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.attributes.set_element_id(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn getter_element_class_name(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.className";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.attributes.element_class_name(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn setter_element_class_name(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.className";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.attributes.set_element_class_name(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
/// `Element.classList`, of `throws` (token lists): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn getter_element_class_list(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(cx, call.this, Interface::Element, "Element.classList")?;
    throws(cx, "Element.classList", "token lists")
}
/// `Element.classList`, of `throws` (token lists): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn setter_element_class_list(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(cx, call.this, Interface::Element, "Element.classList")?;
    throws(cx, "Element.classList", "token lists")
}
fn getter_element_slot(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.slot";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.attributes.element_slot(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn setter_element_slot(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.slot";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.attributes.set_element_slot(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_has_attributes(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.hasAttributes";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.attributes.element_has_attributes(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
/// `Element.attributes`, of `throws` (attribute nodes): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn getter_element_attributes(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(cx, call.this, Interface::Element, "Element.attributes")?;
    throws(cx, "Element.attributes", "attribute nodes")
}
fn operation_element_get_attribute_names(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.getAttributeNames";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.attributes.element_get_attribute_names(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            strings_value(dcx, value)
        })
    })
}
fn operation_element_get_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.getAttribute";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.attributes.element_get_attribute(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            nullable_string_value(dcx, value)
        })
    })
}
fn operation_element_get_attribute_ns(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.getAttributeNS";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    let a1 = to_string(cx, argument(&call, 1))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let a1 = copies.copy(dcx.engine(), a1)?;
        let answer = slices
            .attributes
            .element_get_attribute_ns(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            nullable_string_value(dcx, value)
        })
    })
}
fn operation_element_set_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.setAttribute";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    let a1 = to_string(cx, argument(&call, 1))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let a1 = copies.copy(dcx.engine(), a1)?;
        let answer = slices.attributes.element_set_attribute(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_set_attribute_ns(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.setAttributeNS";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 3, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    let a1 = to_string(cx, argument(&call, 1))?;
    let a2 = to_string(cx, argument(&call, 2))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let a1 = copies.copy(dcx.engine(), a1)?;
        let a2 = copies.copy(dcx.engine(), a2)?;
        let answer = slices
            .attributes
            .element_set_attribute_ns(dcx, this, a0, a1, a2);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_remove_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.removeAttribute";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.attributes.element_remove_attribute(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_remove_attribute_ns(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.removeAttributeNS";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    let a1 = to_string(cx, argument(&call, 1))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let a1 = copies.copy(dcx.engine(), a1)?;
        let answer = slices
            .attributes
            .element_remove_attribute_ns(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_toggle_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.toggleAttribute";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    let a1 = if present(&call, 1) {
        Some(to_boolean(cx, argument(&call, 1))?)
    } else {
        None
    };
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices
            .attributes
            .element_toggle_attribute(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
fn operation_element_has_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.hasAttribute";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.attributes.element_has_attribute(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
fn operation_element_has_attribute_ns(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.hasAttributeNS";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    let a1 = to_string(cx, argument(&call, 1))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let a1 = copies.copy(dcx.engine(), a1)?;
        let answer = slices
            .attributes
            .element_has_attribute_ns(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
/// `Element.getAttributeNode`, of `throws` (attribute nodes): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_element_get_attribute_node(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Element,
        "Element.getAttributeNode",
    )?;
    throws(cx, "Element.getAttributeNode", "attribute nodes")
}
/// `Element.getAttributeNodeNS`, of `throws` (attribute nodes): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_element_get_attribute_node_ns(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Element,
        "Element.getAttributeNodeNS",
    )?;
    throws(cx, "Element.getAttributeNodeNS", "attribute nodes")
}
/// `Element.setAttributeNode`, of `throws` (attribute nodes): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_element_set_attribute_node(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Element,
        "Element.setAttributeNode",
    )?;
    throws(cx, "Element.setAttributeNode", "attribute nodes")
}
/// `Element.setAttributeNodeNS`, of `throws` (attribute nodes): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_element_set_attribute_node_ns(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Element,
        "Element.setAttributeNodeNS",
    )?;
    throws(cx, "Element.setAttributeNodeNS", "attribute nodes")
}
/// `Element.removeAttributeNode`, of `throws` (attribute nodes): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_element_remove_attribute_node(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Element,
        "Element.removeAttributeNode",
    )?;
    throws(cx, "Element.removeAttributeNode", "attribute nodes")
}
/// `Element.attachShadow`, of `throws` (shadow trees): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_element_attach_shadow(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(cx, call.this, Interface::Element, "Element.attachShadow")?;
    throws(cx, "Element.attachShadow", "shadow trees")
}
fn getter_element_shadow_root(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.shadowRoot";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_shadow_root(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
/// `Element.customElementRegistry`, of `throws` (custom elements): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn getter_element_custom_element_registry(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Element,
        "Element.customElementRegistry",
    )?;
    throws(cx, "Element.customElementRegistry", "custom elements")
}
fn operation_element_closest(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.closest";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.selectors.element_closest(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_element_matches(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.matches";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.selectors.element_matches(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
fn operation_element_webkit_matches_selector(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.webkitMatchesSelector";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices
            .selectors
            .element_webkit_matches_selector(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| Ok(Value::Boolean(value)))
    })
}
fn operation_element_get_elements_by_tag_name(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.getElementsByTagName";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices
            .reading
            .element_get_elements_by_tag_name(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |dcx, slices, value| {
            collection_value(dcx, slices, Interface::HtmlCollection, value)
        })
    })
}
fn operation_element_get_elements_by_tag_name_ns(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.getElementsByTagNameNS";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    let a1 = to_string(cx, argument(&call, 1))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let a1 = copies.copy(dcx.engine(), a1)?;
        let answer = slices
            .reading
            .element_get_elements_by_tag_name_ns(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |dcx, slices, value| {
            collection_value(dcx, slices, Interface::HtmlCollection, value)
        })
    })
}
fn operation_element_get_elements_by_class_name(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.getElementsByClassName";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices
            .reading
            .element_get_elements_by_class_name(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |dcx, slices, value| {
            collection_value(dcx, slices, Interface::HtmlCollection, value)
        })
    })
}
fn operation_element_insert_adjacent_element(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.insertAdjacentElement";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    let a1 = to_node(cx, argument(&call, 1), Interface::Element, MEMBER, 1)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let a1 = unwrap_node(dcx, slices, a1, MEMBER)?;
        let answer = slices
            .mutation
            .element_insert_adjacent_element(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_element_insert_adjacent_text(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.insertAdjacentText";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    let a1 = to_string(cx, argument(&call, 1))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let a1 = copies.copy(dcx.engine(), a1)?;
        let answer = slices
            .mutation
            .element_insert_adjacent_text(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn getter_element_children(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.children";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let writes = dcx.tree().writes();
        let kept = slices
            .wrap
            .same_object(dcx, this, SameObject::ElementChildren)?;
        if let Some(object) = kept
            && collection_written(dcx, object, Interface::HtmlCollection)? == writes
        {
            return Ok(Value::Object(object));
        }
        let answer = slices.reading.element_children(dcx, this);
        let nodes = answered(dcx, answer)?;
        match kept {
            Some(object) => {
                refresh_collection(dcx, slices, object, Interface::HtmlCollection, nodes)?;
                Ok(Value::Object(object))
            }
            None => {
                let object = collection_object(dcx, slices, Interface::HtmlCollection, nodes)?;
                slices
                    .wrap
                    .keep_same_object(dcx, this, SameObject::ElementChildren, object)?;
                Ok(Value::Object(object))
            }
        }
    })
}
fn getter_element_first_element_child(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.firstElementChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_first_element_child(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_element_last_element_child(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.lastElementChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_last_element_child(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_element_child_element_count(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.childElementCount";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_child_element_count(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| {
            Ok(number(f64::from(value)))
        })
    })
}
fn operation_element_prepend(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.prepend";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.element_prepend(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_append(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.append";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.element_append(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_replace_children(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.replaceChildren";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.element_replace_children(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_move_before(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.moveBefore";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    let a1 = to_nullable_node(cx, argument(&call, 1), Interface::Node, MEMBER, 1)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = unwrap_node(dcx, slices, a0, MEMBER)?;
        let a1 = match a1 {
            Some(object) => Some(unwrap_node(dcx, slices, object, MEMBER)?),
            None => None,
        };
        let answer = slices.mutation.element_move_before(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_query_selector(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.querySelector";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.selectors.element_query_selector(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_element_query_selector_all(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.querySelectorAll";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.selectors.element_query_selector_all(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |dcx, slices, value| {
            collection_value(dcx, slices, Interface::NodeList, value)
        })
    })
}
fn getter_element_previous_element_sibling(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.previousElementSibling";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_previous_element_sibling(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_element_next_element_sibling(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.nextElementSibling";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_next_element_sibling(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_element_before(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.before";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.element_before(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_after(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.after";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.element_after(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_replace_with(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.replaceWith";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.element_replace_with(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_element_remove(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Element.remove";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.mutation.element_remove(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn getter_element_assigned_slot(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Element.assignedSlot";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Element, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.element_assigned_slot(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn interface_element(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    no_constructor(cx, call, "Element")
}
fn interface_document(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    throwing_constructor(cx, call, "Document", "documents made by script")
}
/// `Document.implementation`, of `throws` (DOMImplementation is not generated): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn getter_document_implementation(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Document,
        "Document.implementation",
    )?;
    throws(
        cx,
        "Document.implementation",
        "DOMImplementation is not generated",
    )
}
fn getter_document_url(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Document.URL";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_url(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_document_document_uri(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.documentURI";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_document_uri(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_document_compat_mode(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.compatMode";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_compat_mode(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_document_character_set(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.characterSet";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_character_set(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_document_charset(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Document.charset";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_charset(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_document_input_encoding(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.inputEncoding";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_input_encoding(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_document_content_type(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.contentType";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_content_type(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_document_doctype(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Document.doctype";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_doctype(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_document_document_element(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.documentElement";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_document_element(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_document_get_elements_by_tag_name(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.getElementsByTagName";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices
            .reading
            .document_get_elements_by_tag_name(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |dcx, slices, value| {
            collection_value(dcx, slices, Interface::HtmlCollection, value)
        })
    })
}
fn operation_document_get_elements_by_tag_name_ns(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.getElementsByTagNameNS";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    let a1 = to_string(cx, argument(&call, 1))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let a1 = copies.copy(dcx.engine(), a1)?;
        let answer = slices
            .reading
            .document_get_elements_by_tag_name_ns(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |dcx, slices, value| {
            collection_value(dcx, slices, Interface::HtmlCollection, value)
        })
    })
}
fn operation_document_get_elements_by_class_name(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.getElementsByClassName";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices
            .reading
            .document_get_elements_by_class_name(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |dcx, slices, value| {
            collection_value(dcx, slices, Interface::HtmlCollection, value)
        })
    })
}
fn operation_document_create_element(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.createElement";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    let a1 = if union_takes_dictionary(argument(&call, 1)) {
        RawStringOrElementCreationOptions::Dictionary(convert_element_creation_options(
            cx,
            argument(&call, 1),
            MEMBER,
        )?)
    } else {
        RawStringOrElementCreationOptions::String(to_string(cx, argument(&call, 1))?)
    };
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let a1 = finish_string_or_element_creation_options(dcx, &mut copies, a1)?;
        let answer = slices.mutation.document_create_element(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_document_create_element_ns(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.createElementNS";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_nullable_string(cx, argument(&call, 0))?;
    let a1 = to_string(cx, argument(&call, 1))?;
    let a2 = if union_takes_dictionary(argument(&call, 2)) {
        RawStringOrElementCreationOptions::Dictionary(convert_element_creation_options(
            cx,
            argument(&call, 2),
            MEMBER,
        )?)
    } else {
        RawStringOrElementCreationOptions::String(to_string(cx, argument(&call, 2))?)
    };
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => Some(copies.copy(dcx.engine(), string)?),
            None => None,
        };
        let a1 = copies.copy(dcx.engine(), a1)?;
        let a2 = finish_string_or_element_creation_options(dcx, &mut copies, a2)?;
        let answer = slices
            .mutation
            .document_create_element_ns(dcx, this, a0, a1, a2);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_document_create_document_fragment(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.createDocumentFragment";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.mutation.document_create_document_fragment(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_document_create_text_node(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.createTextNode";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.mutation.document_create_text_node(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_document_create_cdata_section(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.createCDATASection";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.mutation.document_create_cdata_section(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_document_create_comment(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.createComment";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.mutation.document_create_comment(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_document_create_processing_instruction(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.createProcessingInstruction";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    let a1 = to_string(cx, argument(&call, 1))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let a1 = copies.copy(dcx.engine(), a1)?;
        let answer = slices
            .mutation
            .document_create_processing_instruction(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_document_import_node(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.importNode";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    let a1 = if present(&call, 1) {
        Some(if union_takes_dictionary(argument(&call, 1)) {
            RawBooleanOrImportNodeOptions::Dictionary(convert_import_node_options(
                cx,
                argument(&call, 1),
                MEMBER,
            )?)
        } else {
            RawBooleanOrImportNodeOptions::Boolean(to_boolean(cx, argument(&call, 1))?)
        })
    } else {
        None
    };
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = unwrap_node(dcx, slices, a0, MEMBER)?;
        let a1 = match a1 {
            Some(raw) => finish_boolean_or_import_node_options(dcx, &mut copies, raw)?,
            None => BooleanOrImportNodeOptions::Boolean(false),
        };
        let answer = slices.mutation.document_import_node(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_document_adopt_node(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.adoptNode";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = unwrap_node(dcx, slices, a0, MEMBER)?;
        let answer = slices.mutation.document_adopt_node(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
/// `Document.createAttribute`, of `throws` (attribute nodes): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_document_create_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Document,
        "Document.createAttribute",
    )?;
    throws(cx, "Document.createAttribute", "attribute nodes")
}
/// `Document.createAttributeNS`, of `throws` (attribute nodes): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_document_create_attribute_ns(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Document,
        "Document.createAttributeNS",
    )?;
    throws(cx, "Document.createAttributeNS", "attribute nodes")
}
/// `Document.createEvent`, of `throws` (events): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_document_create_event(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(cx, call.this, Interface::Document, "Document.createEvent")?;
    throws(cx, "Document.createEvent", "events")
}
/// `Document.createRange`, of `throws` (ranges): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_document_create_range(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(cx, call.this, Interface::Document, "Document.createRange")?;
    throws(cx, "Document.createRange", "ranges")
}
/// `Document.createNodeIterator`, of `throws` (traversal): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_document_create_node_iterator(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Document,
        "Document.createNodeIterator",
    )?;
    throws(cx, "Document.createNodeIterator", "traversal")
}
/// `Document.createTreeWalker`, of `throws` (traversal): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_document_create_tree_walker(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Document,
        "Document.createTreeWalker",
    )?;
    throws(cx, "Document.createTreeWalker", "traversal")
}
fn operation_document_get_element_by_id(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.getElementById";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.reading.document_get_element_by_id(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
/// `Document.customElementRegistry`, of `throws` (custom elements): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn getter_document_custom_element_registry(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Document,
        "Document.customElementRegistry",
    )?;
    throws(cx, "Document.customElementRegistry", "custom elements")
}
fn getter_document_children(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Document.children";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let writes = dcx.tree().writes();
        let kept = slices
            .wrap
            .same_object(dcx, this, SameObject::DocumentChildren)?;
        if let Some(object) = kept
            && collection_written(dcx, object, Interface::HtmlCollection)? == writes
        {
            return Ok(Value::Object(object));
        }
        let answer = slices.reading.document_children(dcx, this);
        let nodes = answered(dcx, answer)?;
        match kept {
            Some(object) => {
                refresh_collection(dcx, slices, object, Interface::HtmlCollection, nodes)?;
                Ok(Value::Object(object))
            }
            None => {
                let object = collection_object(dcx, slices, Interface::HtmlCollection, nodes)?;
                slices
                    .wrap
                    .keep_same_object(dcx, this, SameObject::DocumentChildren, object)?;
                Ok(Value::Object(object))
            }
        }
    })
}
fn getter_document_first_element_child(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.firstElementChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_first_element_child(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_document_last_element_child(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.lastElementChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_last_element_child(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_document_child_element_count(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.childElementCount";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_child_element_count(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| {
            Ok(number(f64::from(value)))
        })
    })
}
fn operation_document_prepend(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.prepend";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.document_prepend(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_append(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Document.append";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.document_append(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_replace_children(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.replaceChildren";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.document_replace_children(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_move_before(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.moveBefore";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    let a1 = to_nullable_node(cx, argument(&call, 1), Interface::Node, MEMBER, 1)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = unwrap_node(dcx, slices, a0, MEMBER)?;
        let a1 = match a1 {
            Some(object) => Some(unwrap_node(dcx, slices, object, MEMBER)?),
            None => None,
        };
        let answer = slices.mutation.document_move_before(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_query_selector(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.querySelector";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.selectors.document_query_selector(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_document_query_selector_all(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "Document.querySelectorAll";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.selectors.document_query_selector_all(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |dcx, slices, value| {
            collection_value(dcx, slices, Interface::NodeList, value)
        })
    })
}
/// `Document.createExpression`, of `throws` (XPath): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_document_create_expression(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Document,
        "Document.createExpression",
    )?;
    throws(cx, "Document.createExpression", "XPath")
}
/// `Document.createNSResolver`, of `throws` (XPath): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_document_create_ns_resolver(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::Document,
        "Document.createNSResolver",
    )?;
    throws(cx, "Document.createNSResolver", "XPath")
}
/// `Document.evaluate`, of `throws` (XPath): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_document_evaluate(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(cx, call.this, Interface::Document, "Document.evaluate")?;
    throws(cx, "Document.evaluate", "XPath")
}
fn getter_document_title(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Document.title";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_title(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn setter_document_title(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Document.title";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Document, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.mutation.set_document_title(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn getter_character_data_data(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.data";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.character_data_data(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn setter_character_data_data(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.data";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    let a0 = to_legacy_null_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = match a0 {
            Some(string) => copies.copy(dcx.engine(), string)?,
            None => String::new(),
        };
        let answer = slices.mutation.set_character_data_data(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn getter_character_data_length(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.length";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.character_data_length(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| {
            Ok(number(f64::from(value)))
        })
    })
}
fn operation_character_data_substring_data(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.substringData";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_unsigned(cx, argument(&call, 0), 32)?;
    let a1 = to_unsigned(cx, argument(&call, 1), 32)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices
            .reading
            .character_data_substring_data(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            units_value(dcx, &value)
        })
    })
}
fn operation_character_data_append_data(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.appendData";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices.mutation.character_data_append_data(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_character_data_insert_data(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.insertData";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_unsigned(cx, argument(&call, 0), 32)?;
    let a1 = to_string(cx, argument(&call, 1))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a1 = copies.copy(dcx.engine(), a1)?;
        let answer = slices
            .mutation
            .character_data_insert_data(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_character_data_delete_data(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.deleteData";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_unsigned(cx, argument(&call, 0), 32)?;
    let a1 = to_unsigned(cx, argument(&call, 1), 32)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices
            .mutation
            .character_data_delete_data(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_character_data_replace_data(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.replaceData";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    required(cx, &call, 3, MEMBER)?;
    let a0 = to_unsigned(cx, argument(&call, 0), 32)?;
    let a1 = to_unsigned(cx, argument(&call, 1), 32)?;
    let a2 = to_string(cx, argument(&call, 2))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a2 = copies.copy(dcx.engine(), a2)?;
        let answer = slices
            .mutation
            .character_data_replace_data(dcx, this, a0, a1, a2);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn getter_character_data_previous_element_sibling(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.previousElementSibling";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices
            .reading
            .character_data_previous_element_sibling(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_character_data_next_element_sibling(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.nextElementSibling";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices
            .reading
            .character_data_next_element_sibling(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_character_data_before(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.before";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.character_data_before(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_character_data_after(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.after";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.character_data_after(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_character_data_replace_with(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.replaceWith";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.character_data_replace_with(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_character_data_remove(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "CharacterData.remove";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::CharacterData, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.mutation.character_data_remove(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn interface_character_data(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    no_constructor(cx, call, "CharacterData")
}
fn interface_text(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Text.constructor";
    cx.charge_steps(1)?;
    if call.new_target.is_none() {
        return Err(type_error(cx, &format!("{MEMBER}: called without new")));
    }
    let a0 = if present(&call, 0) {
        Some(to_string(cx, argument(&call, 0))?)
    } else {
        None
    };
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let a0 = match a0 {
            Some(raw) => copies.copy(dcx.engine(), raw)?,
            None => "".to_string(),
        };
        let answer = slices.mutation.text_constructor(dcx, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_text_split_text(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Text.splitText";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Text, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_unsigned(cx, argument(&call, 0), 32)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.mutation.text_split_text(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn getter_text_whole_text(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Text.wholeText";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Text, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.text_whole_text(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_text_assigned_slot(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Text.assignedSlot";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::Text, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.text_assigned_slot(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn interface_comment(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Comment.constructor";
    cx.charge_steps(1)?;
    if call.new_target.is_none() {
        return Err(type_error(cx, &format!("{MEMBER}: called without new")));
    }
    let a0 = if present(&call, 0) {
        Some(to_string(cx, argument(&call, 0))?)
    } else {
        None
    };
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let a0 = match a0 {
            Some(raw) => copies.copy(dcx.engine(), raw)?,
            None => "".to_string(),
        };
        let answer = slices.mutation.comment_constructor(dcx, a0);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn interface_processing_instruction(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "ProcessingInstruction.constructor";
    cx.charge_steps(1)?;
    if call.new_target.is_none() {
        return Err(type_error(cx, &format!("{MEMBER}: called without new")));
    }
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    let a1 = if present(&call, 1) {
        Some(to_string(cx, argument(&call, 1))?)
    } else {
        None
    };
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let a0 = copies.copy(dcx.engine(), a0)?;
        let a1 = match a1 {
            Some(raw) => copies.copy(dcx.engine(), raw)?,
            None => "".to_string(),
        };
        let answer = slices
            .mutation
            .processing_instruction_constructor(dcx, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn getter_processing_instruction_target(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "ProcessingInstruction.target";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::ProcessingInstruction, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.processing_instruction_target(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
/// `ProcessingInstruction.hasAttributes`, of `throws` (attributes of a processing instruction): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_processing_instruction_has_attributes(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::ProcessingInstruction,
        "ProcessingInstruction.hasAttributes",
    )?;
    throws(
        cx,
        "ProcessingInstruction.hasAttributes",
        "attributes of a processing instruction",
    )
}
/// `ProcessingInstruction.getAttributeNames`, of `throws` (attributes of a processing instruction): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_processing_instruction_get_attribute_names(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::ProcessingInstruction,
        "ProcessingInstruction.getAttributeNames",
    )?;
    throws(
        cx,
        "ProcessingInstruction.getAttributeNames",
        "attributes of a processing instruction",
    )
}
/// `ProcessingInstruction.getAttribute`, of `throws` (attributes of a processing instruction): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_processing_instruction_get_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::ProcessingInstruction,
        "ProcessingInstruction.getAttribute",
    )?;
    throws(
        cx,
        "ProcessingInstruction.getAttribute",
        "attributes of a processing instruction",
    )
}
/// `ProcessingInstruction.setAttribute`, of `throws` (attributes of a processing instruction): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_processing_instruction_set_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::ProcessingInstruction,
        "ProcessingInstruction.setAttribute",
    )?;
    throws(
        cx,
        "ProcessingInstruction.setAttribute",
        "attributes of a processing instruction",
    )
}
/// `ProcessingInstruction.removeAttribute`, of `throws` (attributes of a processing instruction): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_processing_instruction_remove_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::ProcessingInstruction,
        "ProcessingInstruction.removeAttribute",
    )?;
    throws(
        cx,
        "ProcessingInstruction.removeAttribute",
        "attributes of a processing instruction",
    )
}
/// `ProcessingInstruction.toggleAttribute`, of `throws` (attributes of a processing instruction): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_processing_instruction_toggle_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::ProcessingInstruction,
        "ProcessingInstruction.toggleAttribute",
    )?;
    throws(
        cx,
        "ProcessingInstruction.toggleAttribute",
        "attributes of a processing instruction",
    )
}
/// `ProcessingInstruction.hasAttribute`, of `throws` (attributes of a processing instruction): the brand check of `this`, then `NotSupportedError`.
/// Web IDL converts the arguments first; here no argument is converted, so a call with an
/// argument whose conversion throws gets `NotSupportedError`, not that throw.
fn operation_processing_instruction_has_attribute(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    cx.charge_steps(1)?;
    this_object(
        cx,
        call.this,
        Interface::ProcessingInstruction,
        "ProcessingInstruction.hasAttribute",
    )?;
    throws(
        cx,
        "ProcessingInstruction.hasAttribute",
        "attributes of a processing instruction",
    )
}
fn getter_document_type_name(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "DocumentType.name";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentType, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_type_name(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_document_type_public_id(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentType.publicId";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentType, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_type_public_id(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn getter_document_type_system_id(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentType.systemId";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentType, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.reading.document_type_system_id(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, value| {
            string_value(dcx, &value)
        })
    })
}
fn operation_document_type_before(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentType.before";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentType, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.document_type_before(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_type_after(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentType.after";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentType, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.document_type_after(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_type_replace_with(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentType.replaceWith";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentType, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.document_type_replace_with(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_type_remove(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentType.remove";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentType, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices.mutation.document_type_remove(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn interface_document_type(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    no_constructor(cx, call, "DocumentType")
}
fn interface_document_fragment(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.constructor";
    cx.charge_steps(1)?;
    if call.new_target.is_none() {
        return Err(type_error(cx, &format!("{MEMBER}: called without new")));
    }
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let answer = slices.mutation.document_fragment_constructor(dcx);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn operation_document_fragment_get_element_by_id(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.getElementById";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices
            .reading
            .document_fragment_get_element_by_id(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_document_fragment_children(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.children";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let writes = dcx.tree().writes();
        let kept = slices
            .wrap
            .same_object(dcx, this, SameObject::DocumentFragmentChildren)?;
        if let Some(object) = kept
            && collection_written(dcx, object, Interface::HtmlCollection)? == writes
        {
            return Ok(Value::Object(object));
        }
        let answer = slices.reading.document_fragment_children(dcx, this);
        let nodes = answered(dcx, answer)?;
        match kept {
            Some(object) => {
                refresh_collection(dcx, slices, object, Interface::HtmlCollection, nodes)?;
                Ok(Value::Object(object))
            }
            None => {
                let object = collection_object(dcx, slices, Interface::HtmlCollection, nodes)?;
                slices.wrap.keep_same_object(
                    dcx,
                    this,
                    SameObject::DocumentFragmentChildren,
                    object,
                )?;
                Ok(Value::Object(object))
            }
        }
    })
}
fn getter_document_fragment_first_element_child(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.firstElementChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices
            .reading
            .document_fragment_first_element_child(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_document_fragment_last_element_child(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.lastElementChild";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices
            .reading
            .document_fragment_last_element_child(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_document_fragment_child_element_count(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.childElementCount";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let answer = slices
            .reading
            .document_fragment_child_element_count(dcx, this);
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| {
            Ok(number(f64::from(value)))
        })
    })
}
fn operation_document_fragment_prepend(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.prepend";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.document_fragment_prepend(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_fragment_append(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.append";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices.mutation.document_fragment_append(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_fragment_replace_children(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.replaceChildren";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    let a0 = to_nodes_or_strings(cx, &call, 0)?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = finish_nodes_or_strings(dcx, slices, &mut copies, a0, MEMBER)?;
        let answer = slices
            .mutation
            .document_fragment_replace_children(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_fragment_move_before(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.moveBefore";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    required(cx, &call, 2, MEMBER)?;
    let a0 = to_node(cx, argument(&call, 0), Interface::Node, MEMBER, 0)?;
    let a1 = to_nullable_node(cx, argument(&call, 1), Interface::Node, MEMBER, 1)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = unwrap_node(dcx, slices, a0, MEMBER)?;
        let a1 = match a1 {
            Some(object) => Some(unwrap_node(dcx, slices, object, MEMBER)?),
            None => None,
        };
        let answer = slices
            .mutation
            .document_fragment_move_before(dcx, this, a0, a1);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn operation_document_fragment_query_selector(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.querySelector";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices
            .selectors
            .document_fragment_query_selector(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_document_fragment_query_selector_all(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "DocumentFragment.querySelectorAll";
    cx.charge_steps(1)?;
    let this = this_object(cx, call.this, Interface::DocumentFragment, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let this = unwrap_node(dcx, slices, this, MEMBER)?;
        let a0 = copies.copy(dcx.engine(), a0)?;
        let answer = slices
            .selectors
            .document_fragment_query_selector_all(dcx, this, a0);
        drop(copies);
        finish(dcx, slices, answer, |dcx, slices, value| {
            collection_value(dcx, slices, Interface::NodeList, value)
        })
    })
}
fn operation_node_list_item(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "NodeList.item";
    cx.charge_steps(1)?;
    let this = this_collection(cx, call.this, Interface::NodeList, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_unsigned(cx, argument(&call, 0), 32)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let nodes = take_nodes(dcx, this, Interface::NodeList)?;
        let answer = slices.reading.node_list_item(dcx, &nodes, a0);
        restore_nodes(dcx, this, Interface::NodeList, nodes)?;
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn getter_node_list_length(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "NodeList.length";
    cx.charge_steps(1)?;
    let this = this_collection(cx, call.this, Interface::NodeList, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let nodes = take_nodes(dcx, this, Interface::NodeList)?;
        let answer = slices.reading.node_list_length(dcx, &nodes);
        restore_nodes(dcx, this, Interface::NodeList, nodes)?;
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| {
            Ok(number(f64::from(value)))
        })
    })
}
fn interface_node_list(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    no_constructor(cx, call, "NodeList")
}
fn getter_html_collection_length(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "HTMLCollection.length";
    cx.charge_steps(1)?;
    let this = this_collection(cx, call.this, Interface::HtmlCollection, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let nodes = take_nodes(dcx, this, Interface::HtmlCollection)?;
        let answer = slices.reading.html_collection_length(dcx, &nodes);
        restore_nodes(dcx, this, Interface::HtmlCollection, nodes)?;
        drop(copies);
        finish(dcx, slices, answer, |_, _, value| {
            Ok(number(f64::from(value)))
        })
    })
}
fn operation_html_collection_item(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "HTMLCollection.item";
    cx.charge_steps(1)?;
    let this = this_collection(cx, call.this, Interface::HtmlCollection, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_unsigned(cx, argument(&call, 0), 32)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let nodes = take_nodes(dcx, this, Interface::HtmlCollection)?;
        let answer = slices.reading.html_collection_item(dcx, &nodes, a0);
        restore_nodes(dcx, this, Interface::HtmlCollection, nodes)?;
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn operation_html_collection_named_item(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    const MEMBER: &str = "HTMLCollection.namedItem";
    cx.charge_steps(1)?;
    let this = this_collection(cx, call.this, Interface::HtmlCollection, MEMBER)?;
    required(cx, &call, 1, MEMBER)?;
    let a0 = to_string(cx, argument(&call, 0))?;
    in_dom(cx, |dcx, slices| {
        let mut copies = Copies::new();
        let a0 = copies.copy(dcx.engine(), a0)?;
        let nodes = take_nodes(dcx, this, Interface::HtmlCollection)?;
        let answer = slices.reading.html_collection_named_item(dcx, &nodes, a0);
        restore_nodes(dcx, this, Interface::HtmlCollection, nodes)?;
        drop(copies);
        finish(dcx, slices, answer, nullable_node_value)
    })
}
fn interface_html_collection(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    no_constructor(cx, call, "HTMLCollection")
}
fn getter_dom_exception_name(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    dom_exception_name(cx, call)
}
fn getter_dom_exception_message(
    cx: &mut Cx<'_, ScriptHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    dom_exception_message(cx, call)
}
fn getter_dom_exception_code(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    dom_exception_code(cx, call)
}
fn getter_window_window(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Window.window";
    cx.charge_steps(1)?;
    this_global(cx, call.this, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let answer = slices.reading.window_window(dcx);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, TheWindow| global_value(dcx))
    })
}
fn getter_window_self(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Window.self";
    cx.charge_steps(1)?;
    this_global(cx, call.this, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let answer = slices.reading.window_self(dcx);
        drop(copies);
        finish(dcx, slices, answer, |dcx, _, TheWindow| global_value(dcx))
    })
}
fn setter_window_self(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.charge_steps(1)?;
    replace_on_global(cx, &call, "self")
}
fn getter_window_document(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Window.document";
    cx.charge_steps(1)?;
    this_global(cx, call.this, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let answer = slices.reading.window_document(dcx);
        drop(copies);
        finish(dcx, slices, answer, node_value)
    })
}
fn getter_window_event(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    const MEMBER: &str = "Window.event";
    cx.charge_steps(1)?;
    this_global(cx, call.this, MEMBER)?;
    in_dom(cx, |dcx, slices| {
        let copies = Copies::new();
        let answer = slices.reading.window_event(dcx);
        drop(copies);
        finish(dcx, slices, answer, |_, _, ()| Ok(Value::Undefined))
    })
}
fn setter_window_event(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.charge_steps(1)?;
    replace_on_global(cx, &call, "event")
}
fn interface_window(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    no_constructor(cx, call, "Window")
}
/// The interfaces, parents first, for the realm's setup.
static INTERFACES: [InterfaceSpec; 14] = [
    InterfaceSpec {
        interface: Interface::EventTarget,
        steps: interface_event_target,
        length: 0,
        members: &[
            MemberSpec::Operation {
                name: "addEventListener",
                steps: operation_event_target_add_event_listener,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "removeEventListener",
                steps: operation_event_target_remove_event_listener,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "dispatchEvent",
                steps: operation_event_target_dispatch_event,
                length: 1,
                unforgeable: false,
            },
        ],
        unscopables: &[],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::Node,
        steps: interface_node,
        length: 0,
        members: &[
            MemberSpec::Constant {
                name: "ELEMENT_NODE",
                value: 1.0,
            },
            MemberSpec::Constant {
                name: "ATTRIBUTE_NODE",
                value: 2.0,
            },
            MemberSpec::Constant {
                name: "TEXT_NODE",
                value: 3.0,
            },
            MemberSpec::Constant {
                name: "CDATA_SECTION_NODE",
                value: 4.0,
            },
            MemberSpec::Constant {
                name: "ENTITY_REFERENCE_NODE",
                value: 5.0,
            },
            MemberSpec::Constant {
                name: "ENTITY_NODE",
                value: 6.0,
            },
            MemberSpec::Constant {
                name: "PROCESSING_INSTRUCTION_NODE",
                value: 7.0,
            },
            MemberSpec::Constant {
                name: "COMMENT_NODE",
                value: 8.0,
            },
            MemberSpec::Constant {
                name: "DOCUMENT_NODE",
                value: 9.0,
            },
            MemberSpec::Constant {
                name: "DOCUMENT_TYPE_NODE",
                value: 10.0,
            },
            MemberSpec::Constant {
                name: "DOCUMENT_FRAGMENT_NODE",
                value: 11.0,
            },
            MemberSpec::Constant {
                name: "NOTATION_NODE",
                value: 12.0,
            },
            MemberSpec::Attribute {
                name: "nodeType",
                getter: getter_node_node_type,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "nodeName",
                getter: getter_node_node_name,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "baseURI",
                getter: getter_node_base_uri,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "isConnected",
                getter: getter_node_is_connected,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "ownerDocument",
                getter: getter_node_owner_document,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getRootNode",
                steps: operation_node_get_root_node,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "parentNode",
                getter: getter_node_parent_node,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "parentElement",
                getter: getter_node_parent_element,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "hasChildNodes",
                steps: operation_node_has_child_nodes,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "childNodes",
                getter: getter_node_child_nodes,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "firstChild",
                getter: getter_node_first_child,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "lastChild",
                getter: getter_node_last_child,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "previousSibling",
                getter: getter_node_previous_sibling,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "nextSibling",
                getter: getter_node_next_sibling,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "nodeValue",
                getter: getter_node_node_value,
                setter: Some(setter_node_node_value),
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "textContent",
                getter: getter_node_text_content,
                setter: Some(setter_node_text_content),
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "normalize",
                steps: operation_node_normalize,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "cloneNode",
                steps: operation_node_clone_node,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "isEqualNode",
                steps: operation_node_is_equal_node,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "isSameNode",
                steps: operation_node_is_same_node,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Constant {
                name: "DOCUMENT_POSITION_DISCONNECTED",
                value: 1.0,
            },
            MemberSpec::Constant {
                name: "DOCUMENT_POSITION_PRECEDING",
                value: 2.0,
            },
            MemberSpec::Constant {
                name: "DOCUMENT_POSITION_FOLLOWING",
                value: 4.0,
            },
            MemberSpec::Constant {
                name: "DOCUMENT_POSITION_CONTAINS",
                value: 8.0,
            },
            MemberSpec::Constant {
                name: "DOCUMENT_POSITION_CONTAINED_BY",
                value: 16.0,
            },
            MemberSpec::Constant {
                name: "DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC",
                value: 32.0,
            },
            MemberSpec::Operation {
                name: "compareDocumentPosition",
                steps: operation_node_compare_document_position,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "contains",
                steps: operation_node_contains,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "lookupPrefix",
                steps: operation_node_lookup_prefix,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "lookupNamespaceURI",
                steps: operation_node_lookup_namespace_uri,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "isDefaultNamespace",
                steps: operation_node_is_default_namespace,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "insertBefore",
                steps: operation_node_insert_before,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "appendChild",
                steps: operation_node_append_child,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "replaceChild",
                steps: operation_node_replace_child,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "removeChild",
                steps: operation_node_remove_child,
                length: 1,
                unforgeable: false,
            },
        ],
        unscopables: &[],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::Element,
        steps: interface_element,
        length: 0,
        members: &[
            MemberSpec::Attribute {
                name: "namespaceURI",
                getter: getter_element_namespace_uri,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "prefix",
                getter: getter_element_prefix,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "localName",
                getter: getter_element_local_name,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "tagName",
                getter: getter_element_tag_name,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "id",
                getter: getter_element_id,
                setter: Some(setter_element_id),
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "className",
                getter: getter_element_class_name,
                setter: Some(setter_element_class_name),
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "classList",
                getter: getter_element_class_list,
                setter: Some(setter_element_class_list),
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "slot",
                getter: getter_element_slot,
                setter: Some(setter_element_slot),
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "hasAttributes",
                steps: operation_element_has_attributes,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "attributes",
                getter: getter_element_attributes,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getAttributeNames",
                steps: operation_element_get_attribute_names,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getAttribute",
                steps: operation_element_get_attribute,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getAttributeNS",
                steps: operation_element_get_attribute_ns,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "setAttribute",
                steps: operation_element_set_attribute,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "setAttributeNS",
                steps: operation_element_set_attribute_ns,
                length: 3,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "removeAttribute",
                steps: operation_element_remove_attribute,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "removeAttributeNS",
                steps: operation_element_remove_attribute_ns,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "toggleAttribute",
                steps: operation_element_toggle_attribute,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "hasAttribute",
                steps: operation_element_has_attribute,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "hasAttributeNS",
                steps: operation_element_has_attribute_ns,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getAttributeNode",
                steps: operation_element_get_attribute_node,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getAttributeNodeNS",
                steps: operation_element_get_attribute_node_ns,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "setAttributeNode",
                steps: operation_element_set_attribute_node,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "setAttributeNodeNS",
                steps: operation_element_set_attribute_node_ns,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "removeAttributeNode",
                steps: operation_element_remove_attribute_node,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "attachShadow",
                steps: operation_element_attach_shadow,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "shadowRoot",
                getter: getter_element_shadow_root,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "customElementRegistry",
                getter: getter_element_custom_element_registry,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "closest",
                steps: operation_element_closest,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "matches",
                steps: operation_element_matches,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "webkitMatchesSelector",
                steps: operation_element_webkit_matches_selector,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getElementsByTagName",
                steps: operation_element_get_elements_by_tag_name,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getElementsByTagNameNS",
                steps: operation_element_get_elements_by_tag_name_ns,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getElementsByClassName",
                steps: operation_element_get_elements_by_class_name,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "insertAdjacentElement",
                steps: operation_element_insert_adjacent_element,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "insertAdjacentText",
                steps: operation_element_insert_adjacent_text,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "children",
                getter: getter_element_children,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "firstElementChild",
                getter: getter_element_first_element_child,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "lastElementChild",
                getter: getter_element_last_element_child,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "childElementCount",
                getter: getter_element_child_element_count,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "prepend",
                steps: operation_element_prepend,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "append",
                steps: operation_element_append,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "replaceChildren",
                steps: operation_element_replace_children,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "moveBefore",
                steps: operation_element_move_before,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "querySelector",
                steps: operation_element_query_selector,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "querySelectorAll",
                steps: operation_element_query_selector_all,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "previousElementSibling",
                getter: getter_element_previous_element_sibling,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "nextElementSibling",
                getter: getter_element_next_element_sibling,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "before",
                steps: operation_element_before,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "after",
                steps: operation_element_after,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "replaceWith",
                steps: operation_element_replace_with,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "remove",
                steps: operation_element_remove,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "assignedSlot",
                getter: getter_element_assigned_slot,
                setter: None,
                unforgeable: false,
            },
        ],
        unscopables: &[
            "slot",
            "prepend",
            "append",
            "replaceChildren",
            "before",
            "after",
            "replaceWith",
            "remove",
        ],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::Document,
        steps: interface_document,
        length: 0,
        members: &[
            MemberSpec::Attribute {
                name: "implementation",
                getter: getter_document_implementation,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "URL",
                getter: getter_document_url,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "documentURI",
                getter: getter_document_document_uri,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "compatMode",
                getter: getter_document_compat_mode,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "characterSet",
                getter: getter_document_character_set,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "charset",
                getter: getter_document_charset,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "inputEncoding",
                getter: getter_document_input_encoding,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "contentType",
                getter: getter_document_content_type,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "doctype",
                getter: getter_document_doctype,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "documentElement",
                getter: getter_document_document_element,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getElementsByTagName",
                steps: operation_document_get_elements_by_tag_name,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getElementsByTagNameNS",
                steps: operation_document_get_elements_by_tag_name_ns,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getElementsByClassName",
                steps: operation_document_get_elements_by_class_name,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createElement",
                steps: operation_document_create_element,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createElementNS",
                steps: operation_document_create_element_ns,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createDocumentFragment",
                steps: operation_document_create_document_fragment,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createTextNode",
                steps: operation_document_create_text_node,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createCDATASection",
                steps: operation_document_create_cdata_section,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createComment",
                steps: operation_document_create_comment,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createProcessingInstruction",
                steps: operation_document_create_processing_instruction,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "importNode",
                steps: operation_document_import_node,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "adoptNode",
                steps: operation_document_adopt_node,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createAttribute",
                steps: operation_document_create_attribute,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createAttributeNS",
                steps: operation_document_create_attribute_ns,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createEvent",
                steps: operation_document_create_event,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createRange",
                steps: operation_document_create_range,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createNodeIterator",
                steps: operation_document_create_node_iterator,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createTreeWalker",
                steps: operation_document_create_tree_walker,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getElementById",
                steps: operation_document_get_element_by_id,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "customElementRegistry",
                getter: getter_document_custom_element_registry,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "children",
                getter: getter_document_children,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "firstElementChild",
                getter: getter_document_first_element_child,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "lastElementChild",
                getter: getter_document_last_element_child,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "childElementCount",
                getter: getter_document_child_element_count,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "prepend",
                steps: operation_document_prepend,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "append",
                steps: operation_document_append,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "replaceChildren",
                steps: operation_document_replace_children,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "moveBefore",
                steps: operation_document_move_before,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "querySelector",
                steps: operation_document_query_selector,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "querySelectorAll",
                steps: operation_document_query_selector_all,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createExpression",
                steps: operation_document_create_expression,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "createNSResolver",
                steps: operation_document_create_ns_resolver,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "evaluate",
                steps: operation_document_evaluate,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "title",
                getter: getter_document_title,
                setter: Some(setter_document_title),
                unforgeable: false,
            },
        ],
        unscopables: &["prepend", "append", "replaceChildren"],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::CharacterData,
        steps: interface_character_data,
        length: 0,
        members: &[
            MemberSpec::Attribute {
                name: "data",
                getter: getter_character_data_data,
                setter: Some(setter_character_data_data),
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "length",
                getter: getter_character_data_length,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "substringData",
                steps: operation_character_data_substring_data,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "appendData",
                steps: operation_character_data_append_data,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "insertData",
                steps: operation_character_data_insert_data,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "deleteData",
                steps: operation_character_data_delete_data,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "replaceData",
                steps: operation_character_data_replace_data,
                length: 3,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "previousElementSibling",
                getter: getter_character_data_previous_element_sibling,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "nextElementSibling",
                getter: getter_character_data_next_element_sibling,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "before",
                steps: operation_character_data_before,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "after",
                steps: operation_character_data_after,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "replaceWith",
                steps: operation_character_data_replace_with,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "remove",
                steps: operation_character_data_remove,
                length: 0,
                unforgeable: false,
            },
        ],
        unscopables: &["before", "after", "replaceWith", "remove"],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::Text,
        steps: interface_text,
        length: 0,
        members: &[
            MemberSpec::Operation {
                name: "splitText",
                steps: operation_text_split_text,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "wholeText",
                getter: getter_text_whole_text,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "assignedSlot",
                getter: getter_text_assigned_slot,
                setter: None,
                unforgeable: false,
            },
        ],
        unscopables: &[],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::Comment,
        steps: interface_comment,
        length: 0,
        members: &[],
        unscopables: &[],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::ProcessingInstruction,
        steps: interface_processing_instruction,
        length: 1,
        members: &[
            MemberSpec::Attribute {
                name: "target",
                getter: getter_processing_instruction_target,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "hasAttributes",
                steps: operation_processing_instruction_has_attributes,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getAttributeNames",
                steps: operation_processing_instruction_get_attribute_names,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "getAttribute",
                steps: operation_processing_instruction_get_attribute,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "setAttribute",
                steps: operation_processing_instruction_set_attribute,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "removeAttribute",
                steps: operation_processing_instruction_remove_attribute,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "toggleAttribute",
                steps: operation_processing_instruction_toggle_attribute,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "hasAttribute",
                steps: operation_processing_instruction_has_attribute,
                length: 1,
                unforgeable: false,
            },
        ],
        unscopables: &[],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::DocumentType,
        steps: interface_document_type,
        length: 0,
        members: &[
            MemberSpec::Attribute {
                name: "name",
                getter: getter_document_type_name,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "publicId",
                getter: getter_document_type_public_id,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "systemId",
                getter: getter_document_type_system_id,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "before",
                steps: operation_document_type_before,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "after",
                steps: operation_document_type_after,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "replaceWith",
                steps: operation_document_type_replace_with,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "remove",
                steps: operation_document_type_remove,
                length: 0,
                unforgeable: false,
            },
        ],
        unscopables: &["before", "after", "replaceWith", "remove"],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::DocumentFragment,
        steps: interface_document_fragment,
        length: 0,
        members: &[
            MemberSpec::Operation {
                name: "getElementById",
                steps: operation_document_fragment_get_element_by_id,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "children",
                getter: getter_document_fragment_children,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "firstElementChild",
                getter: getter_document_fragment_first_element_child,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "lastElementChild",
                getter: getter_document_fragment_last_element_child,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "childElementCount",
                getter: getter_document_fragment_child_element_count,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "prepend",
                steps: operation_document_fragment_prepend,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "append",
                steps: operation_document_fragment_append,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "replaceChildren",
                steps: operation_document_fragment_replace_children,
                length: 0,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "moveBefore",
                steps: operation_document_fragment_move_before,
                length: 2,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "querySelector",
                steps: operation_document_fragment_query_selector,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "querySelectorAll",
                steps: operation_document_fragment_query_selector_all,
                length: 1,
                unforgeable: false,
            },
        ],
        unscopables: &["prepend", "append", "replaceChildren"],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::NodeList,
        steps: interface_node_list,
        length: 0,
        members: &[
            MemberSpec::Operation {
                name: "item",
                steps: operation_node_list_item,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "length",
                getter: getter_node_list_length,
                setter: None,
                unforgeable: false,
            },
        ],
        unscopables: &[],
        indexed: true,
        iterable: true,
    },
    InterfaceSpec {
        interface: Interface::HtmlCollection,
        steps: interface_html_collection,
        length: 0,
        members: &[
            MemberSpec::Attribute {
                name: "length",
                getter: getter_html_collection_length,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "item",
                steps: operation_html_collection_item,
                length: 1,
                unforgeable: false,
            },
            MemberSpec::Operation {
                name: "namedItem",
                steps: operation_html_collection_named_item,
                length: 1,
                unforgeable: false,
            },
        ],
        unscopables: &[],
        indexed: true,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::DomException,
        steps: dom_exception_constructor,
        length: 0,
        members: &[
            MemberSpec::Attribute {
                name: "name",
                getter: getter_dom_exception_name,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "message",
                getter: getter_dom_exception_message,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "code",
                getter: getter_dom_exception_code,
                setter: None,
                unforgeable: false,
            },
            MemberSpec::Constant {
                name: "INDEX_SIZE_ERR",
                value: 1.0,
            },
            MemberSpec::Constant {
                name: "DOMSTRING_SIZE_ERR",
                value: 2.0,
            },
            MemberSpec::Constant {
                name: "HIERARCHY_REQUEST_ERR",
                value: 3.0,
            },
            MemberSpec::Constant {
                name: "WRONG_DOCUMENT_ERR",
                value: 4.0,
            },
            MemberSpec::Constant {
                name: "INVALID_CHARACTER_ERR",
                value: 5.0,
            },
            MemberSpec::Constant {
                name: "NO_DATA_ALLOWED_ERR",
                value: 6.0,
            },
            MemberSpec::Constant {
                name: "NO_MODIFICATION_ALLOWED_ERR",
                value: 7.0,
            },
            MemberSpec::Constant {
                name: "NOT_FOUND_ERR",
                value: 8.0,
            },
            MemberSpec::Constant {
                name: "NOT_SUPPORTED_ERR",
                value: 9.0,
            },
            MemberSpec::Constant {
                name: "INUSE_ATTRIBUTE_ERR",
                value: 10.0,
            },
            MemberSpec::Constant {
                name: "INVALID_STATE_ERR",
                value: 11.0,
            },
            MemberSpec::Constant {
                name: "SYNTAX_ERR",
                value: 12.0,
            },
            MemberSpec::Constant {
                name: "INVALID_MODIFICATION_ERR",
                value: 13.0,
            },
            MemberSpec::Constant {
                name: "NAMESPACE_ERR",
                value: 14.0,
            },
            MemberSpec::Constant {
                name: "INVALID_ACCESS_ERR",
                value: 15.0,
            },
            MemberSpec::Constant {
                name: "VALIDATION_ERR",
                value: 16.0,
            },
            MemberSpec::Constant {
                name: "TYPE_MISMATCH_ERR",
                value: 17.0,
            },
            MemberSpec::Constant {
                name: "SECURITY_ERR",
                value: 18.0,
            },
            MemberSpec::Constant {
                name: "NETWORK_ERR",
                value: 19.0,
            },
            MemberSpec::Constant {
                name: "ABORT_ERR",
                value: 20.0,
            },
            MemberSpec::Constant {
                name: "URL_MISMATCH_ERR",
                value: 21.0,
            },
            MemberSpec::Constant {
                name: "QUOTA_EXCEEDED_ERR",
                value: 22.0,
            },
            MemberSpec::Constant {
                name: "TIMEOUT_ERR",
                value: 23.0,
            },
            MemberSpec::Constant {
                name: "INVALID_NODE_TYPE_ERR",
                value: 24.0,
            },
            MemberSpec::Constant {
                name: "DATA_CLONE_ERR",
                value: 25.0,
            },
        ],
        unscopables: &[],
        indexed: false,
        iterable: false,
    },
    InterfaceSpec {
        interface: Interface::Window,
        steps: interface_window,
        length: 0,
        members: &[
            MemberSpec::Attribute {
                name: "window",
                getter: getter_window_window,
                setter: None,
                unforgeable: true,
            },
            MemberSpec::Attribute {
                name: "self",
                getter: getter_window_self,
                setter: Some(setter_window_self),
                unforgeable: false,
            },
            MemberSpec::Attribute {
                name: "document",
                getter: getter_window_document,
                setter: None,
                unforgeable: true,
            },
            MemberSpec::Attribute {
                name: "event",
                getter: getter_window_event,
                setter: Some(setter_window_event),
                unforgeable: false,
            },
        ],
        unscopables: &[],
        indexed: false,
        iterable: false,
    },
];
