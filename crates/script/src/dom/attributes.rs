//! The slice `attributes` (`story:scripts-dom-attributes`): the members of the attributes of
//! an element. Each method calls one function of [`super::attribute_reads`] (a read) or
//! [`super::attribute_writes`] (a write) and gives its answer back; those files hold every
//! read and write of the tree and every charge.

use surf_html::NodeId;

use crate::bindings::{Attributes, DomCx, DomResult};

use super::{attribute_reads, attribute_writes};

/// surf's members of `attributes`.
pub struct SurfAttributes;

impl Attributes for SurfAttributes {
    fn element_id(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        attribute_reads::reflected(cx, this, "id")
    }

    fn set_element_id(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: String,
    ) -> DomResult<()> {
        attribute_writes::set_reflected(cx, this, "id", &value)
    }

    fn element_class_name(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        attribute_reads::reflected(cx, this, "class")
    }

    fn set_element_class_name(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: String,
    ) -> DomResult<()> {
        attribute_writes::set_reflected(cx, this, "class", &value)
    }

    fn element_slot(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<String> {
        attribute_reads::reflected(cx, this, "slot")
    }

    fn set_element_slot(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        value: String,
    ) -> DomResult<()> {
        attribute_writes::set_reflected(cx, this, "slot", &value)
    }

    fn element_has_attributes(&mut self, cx: &mut DomCx<'_, '_>, this: NodeId) -> DomResult<bool> {
        Ok(attribute_reads::any(cx, this))
    }

    fn element_get_attribute_names(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
    ) -> DomResult<Vec<String>> {
        attribute_reads::names(cx, this)
    }

    fn element_get_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<Option<String>> {
        attribute_reads::get(cx, this, &qualified_name)
    }

    fn element_get_attribute_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<Option<String>> {
        attribute_reads::get_ns(cx, this, namespace.as_deref(), &local_name)
    }

    fn element_set_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
        value: String,
    ) -> DomResult<()> {
        attribute_writes::set(cx, this, &qualified_name, &value)
    }

    fn element_set_attribute_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        qualified_name: String,
        value: String,
    ) -> DomResult<()> {
        attribute_writes::set_ns(cx, this, namespace.as_deref(), &qualified_name, &value)
    }

    fn element_remove_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<()> {
        attribute_writes::remove(cx, this, &qualified_name)
    }

    fn element_remove_attribute_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<()> {
        attribute_writes::remove_ns(cx, this, namespace.as_deref(), &local_name)
    }

    fn element_toggle_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
        force: Option<bool>,
    ) -> DomResult<bool> {
        attribute_writes::toggle(cx, this, &qualified_name, force)
    }

    fn element_has_attribute(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        qualified_name: String,
    ) -> DomResult<bool> {
        attribute_reads::has(cx, this, &qualified_name)
    }

    fn element_has_attribute_ns(
        &mut self,
        cx: &mut DomCx<'_, '_>,
        this: NodeId,
        namespace: Option<String>,
        local_name: String,
    ) -> DomResult<bool> {
        attribute_reads::has_ns(cx, this, namespace.as_deref(), &local_name)
    }
}
