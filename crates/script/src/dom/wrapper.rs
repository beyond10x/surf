//! The wrapper hook (decision 8 of `story:webidl-bindings`), as `story:scripts-dom-reading`
//! writes it: one wrapper per node for the document's life, and one object per node and
//! `[SameObject]` member.
//!
//! A wrapper is made on first use, a host object of the interface its node's kind gives
//! ([`interface_of`]), and kept by a [`Root`] in a table that is never emptied: a property a
//! script set on a wrapper is there at the next read, whatever the scripts hold. The objects of
//! the `[SameObject]` collections are kept the same way. The engine charges each such object to
//! the agent's memory budget, which bounds them; the tables' own entries are surf's.

use std::collections::HashMap;

use surf_html::{NodeData, NodeId};
use swell::{Abrupt, Completion, ObjectRef, Root, Termination, Value};

use crate::bindings::{DomCx, Interface, SameObject, Wrap, new_wrapper, wrapper_node};

/// surf's wrapper hook.
#[derive(Default)]
pub struct SurfWrap {
    /// The wrapper of each node that has one.
    wrappers: HashMap<NodeId, Root>,
    /// The object of each node and `[SameObject]` member that has one.
    kept: HashMap<(NodeId, SameObject), Root>,
}

/// The interface of a node's wrapper: the most derived interface of the selection its kind
/// implements. An element's is `Element`, whatever its name (no `HTMLElement` is selected).
fn interface_of(data: &NodeData) -> Interface {
    match data {
        NodeData::Document => Interface::Document,
        NodeData::DocumentFragment => Interface::DocumentFragment,
        NodeData::Doctype { .. } => Interface::DocumentType,
        NodeData::Element { .. } => Interface::Element,
        NodeData::Text(_) => Interface::Text,
        NodeData::Comment(_) => Interface::Comment,
        NodeData::ProcessingInstruction { .. } => Interface::ProcessingInstruction,
    }
}

/// A call surf should not have made: the run ends faulted, as the bindings end it.
fn fault() -> Abrupt {
    Abrupt::Terminated(Termination::EngineFault)
}

/// The object a root holds.
fn object_of(cx: &mut DomCx<'_, '_>, root: &Root) -> Completion<ObjectRef> {
    match cx.engine().get(root).map_err(|_| fault())? {
        Value::Object(object) => Ok(object),
        Value::Undefined
        | Value::Null
        | Value::Boolean(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Symbol(_)
        | Value::BigInt(_) => Err(fault()),
    }
}

impl Wrap for SurfWrap {
    fn wrap(&mut self, cx: &mut DomCx<'_, '_>, node: NodeId) -> Completion<Value> {
        if let Some(root) = self.wrappers.get(&node) {
            return object_of(cx, root).map(Value::Object);
        }
        let interface = interface_of(cx.tree().data(node));
        let object = new_wrapper(cx, interface, node)?;
        let root = cx
            .engine()
            .root(Value::Object(object))
            .map_err(|_| fault())?;
        self.wrappers.insert(node, root);
        Ok(Value::Object(object))
    }

    fn unwrap(&self, cx: &DomCx<'_, '_>, value: Value) -> Option<NodeId> {
        wrapper_node(cx, value)
    }

    fn same_object(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        node: NodeId,
        member: SameObject,
    ) -> Completion<Option<ObjectRef>> {
        match self.kept.get(&(node, member)) {
            Some(root) => object_of(cx, root).map(Some),
            None => Ok(None),
        }
    }

    fn keep_same_object(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        node: NodeId,
        member: SameObject,
        object: ObjectRef,
    ) -> Completion<()> {
        let root = cx
            .engine()
            .root(Value::Object(object))
            .map_err(|_| fault())?;
        self.kept.insert((node, member), root);
        Ok(())
    }
}
