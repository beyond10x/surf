//! The wrapper hook (decision 8 of `story:webidl-bindings`): until
//! `story:scripts-dom-reading` writes the real one, a node has no wrapper, and asking for one
//! throws `NotSupportedError`.

use surf_html::NodeId;
use swell::{Completion, Value};

use crate::bindings::{DomCx, DomException, Wrap, throw, wrapper_node};

/// surf's wrapper hook.
pub struct SurfWrap;

impl Wrap for SurfWrap {
    fn wrap(&mut self, cx: &mut DomCx<'_, '_>, _node: NodeId) -> Completion<Value> {
        Err(throw(
            cx,
            &DomException::not_supported("a node's wrapper is not supported"),
        ))
    }

    fn unwrap(&self, cx: &DomCx<'_, '_>, value: Value) -> Option<NodeId> {
        wrapper_node(cx, value)
    }
}
