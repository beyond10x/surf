//! The writes of the tree for the slice `attributes` (`story:scripts-dom-attributes`): every
//! write of the document's tree that a member of [`super::SurfAttributes`] makes, the
//! validation of the names it is given, and the charges of a write. A member validates the
//! name, finds the attribute through [`attribute_reads::find`] (it reads the tree only
//! there), works out the bytes the write adds and removes by the tree's count, charges the
//! steps of the tree's own scan and of the bytes the tree copies, and then writes through
//! [`charged_write`], the one place that charges the bytes for memory, calls the tree and
//! settles the charge.

use surf_html::limits::ATTRIBUTE_BYTES;
use surf_html::{NodeId, Tree, WriteError};

use crate::bindings::{DomCx, DomEnd, DomException, DomResult};

use super::attribute_reads::{self, Scan, Wanted, byte_steps};
use super::reads;

/// The message of `setAttribute` for a name that is not a valid attribute local name.
const SET_NAME: &str = "Element.setAttribute: the name is not a valid attribute name";

/// The message of `toggleAttribute` for a name that is not a valid attribute local name.
const TOGGLE_NAME: &str = "Element.toggleAttribute: the name is not a valid attribute name";

/// The message of `setAttributeNS` for a prefix or a local name that is not valid.
const SET_NS_NAME: &str = "Element.setAttributeNS: the name is not a valid attribute name";

/// The message of `setAttributeNS` for a namespace that does not fit the name.
const NAMESPACE_MESSAGE: &str = "Element.setAttributeNS: the namespace does not fit the name";

/// The message of `setAttributeNS` for a namespace that is none of the six the tree stores.
const OUTSIDE_NAMESPACE: &str = "Element.setAttributeNS is not supported";

/// The message of an exception of the tree that no write of this slice answers for an
/// element.
const REFUSED: &str = "Element: the tree refused the attribute write";

/// Whether `byte` may be in a valid attribute local name: no ASCII whitespace, NUL, `/`, `=`
/// or `>`.
fn local_byte(byte: u8) -> bool {
    !matches!(
        byte,
        b'\t' | b'\n' | b'\x0c' | b'\r' | b' ' | b'\0' | b'/' | b'=' | b'>'
    )
}

/// Whether `byte` may be in a valid namespace prefix: no ASCII whitespace, NUL, `/` or `>`.
fn prefix_byte(byte: u8) -> bool {
    !matches!(
        byte,
        b'\t' | b'\n' | b'\x0c' | b'\r' | b' ' | b'\0' | b'/' | b'>'
    )
}

/// An `InvalidCharacterError` with `message`.
fn invalid_character(message: &str) -> DomEnd {
    DomEnd::Throw(DomException::new("InvalidCharacterError", message))
}

/// Holds `name` to a valid attribute local name (non-empty, every byte a [`local_byte`]): one
/// step per 16 bytes of it, before the pass; else the `InvalidCharacterError` with `message`.
fn validate(cx: &mut DomCx<'_, '_>, name: &str, message: &str) -> DomResult<()> {
    reads::charge(cx, byte_steps(name.len()))?;
    if name.is_empty() || !name.bytes().all(local_byte) {
        return Err(invalid_character(message));
    }
    Ok(())
}

/// The DOM's "validate and extract" of `qualified_name` up to its `NamespaceError`s, in one
/// pass charged one step per 16 bytes of the whole name before it runs: the split at the first
/// colon (a name with no colon is all local name), with the bytes of the prefix held to a
/// valid namespace prefix and those of the local name to a valid attribute local name in the
/// same loop. Answers the prefix and the local name.
fn split<'n>(
    cx: &mut DomCx<'_, '_>,
    qualified_name: &'n str,
) -> DomResult<(Option<&'n str>, &'n str)> {
    reads::charge(cx, byte_steps(qualified_name.len()))?;
    let mut colon = None;
    // Before the first colon a byte is the prefix's when a colon follows, and the local
    // name's when none does: it is held to both sets, and the split picks the answer.
    let mut head_prefix = true;
    let mut head_local = true;
    let mut tail_local = true;
    for (at, byte) in qualified_name.bytes().enumerate() {
        match colon {
            None if byte == b':' => colon = Some(at),
            None => {
                head_prefix &= prefix_byte(byte);
                head_local &= local_byte(byte);
            }
            Some(_) => tail_local &= local_byte(byte),
        }
    }
    let (prefix, local_name, local_valid) = match colon {
        Some(at) => (
            Some(&qualified_name[..at]),
            &qualified_name[at + 1..],
            tail_local,
        ),
        None => (None, qualified_name, head_local),
    };
    if let Some(prefix) = prefix
        && (prefix.is_empty() || !head_prefix)
    {
        return Err(invalid_character(SET_NS_NAME));
    }
    if local_name.is_empty() || !local_valid {
        return Err(invalid_character(SET_NS_NAME));
    }
    Ok((prefix, local_name))
}

/// The end of a write the tree refused: a limit ends the script, `Namespace` is the
/// `NamespaceError`, and each other refusal the exception its doc comment names.
fn ended(error: WriteError) -> DomEnd {
    let name = match error {
        WriteError::Limit(limit) => return DomEnd::Limit(limit),
        WriteError::Namespace => {
            return DomEnd::Throw(DomException::new("NamespaceError", NAMESPACE_MESSAGE));
        }
        WriteError::HierarchyRequest => "HierarchyRequestError",
        WriteError::NotFound => "NotFoundError",
        WriteError::IndexSize => "IndexSizeError",
        WriteError::InvalidNodeType => "InvalidNodeTypeError",
        WriteError::NotSupported => "NotSupportedError",
    };
    DomEnd::Throw(DomException::new(name, REFUSED))
}

/// The bytes a write adds and removes by the tree's count, from what [`attribute_reads::find`]
/// found: a value replaced adds the new value's bytes and removes the old one's (the tree keeps
/// the name and the prefix); an attribute added adds its record, its name's bytes (`name`, the
/// prefix and the local name it stores), its value's, and the room the list's allocation then
/// grows by as the tree says it ([`Scan::room_grown`]), so that a list the parser built at its
/// exact length is charged for the records its first add makes room for.
fn put_bytes(scan: &Scan, name: usize, value: usize) -> (usize, usize) {
    match scan.found {
        Some(found) => (value, found.value_bytes),
        None => (
            ATTRIBUTE_BYTES
                .saturating_add(name)
                .saturating_add(value)
                .saturating_add(scan.room_grown),
            0,
        ),
    }
}

/// The bytes a removal gives back by the tree's count: the attribute's own (`stored_bytes`),
/// and the room the list's allocation gives back beyond its record as the tree says it
/// ([`Scan::room_freed`]); a list that keeps its allocation gives back no room.
fn removed_bytes(scan: &Scan, stored_bytes: usize) -> usize {
    stored_bytes.saturating_add(scan.room_freed)
}

/// The steps of a write that puts a value, after its scan: one call for the tree's own scan,
/// `c / A + k * n / S` with the scan's numbers (`scanned` the bytes the scan compares), then one
/// step per 16 bytes the tree copies (`name` and `value`), the last charge before the write.
fn charge_put(
    cx: &mut DomCx<'_, '_>,
    scan: &Scan,
    scanned: usize,
    name: usize,
    value: usize,
) -> DomResult<()> {
    reads::charge(cx, scan.steps(scanned))?;
    reads::charge(cx, byte_steps(name.saturating_add(value)))
}

/// The setter of `id`, `className` or `slot`: the tree's `set_attribute_ns` in no namespace
/// with the local name `local_name`, which finds the attribute by namespace and local name as
/// the DOM's "set an attribute value" does. Validates nothing.
pub(super) fn set_reflected(
    cx: &mut DomCx<'_, '_>,
    element: NodeId,
    local_name: &str,
    value: &str,
) -> DomResult<()> {
    let scan = attribute_reads::find(cx, element, Wanted::Local(None, local_name))?;
    let (added, removed) = put_bytes(&scan, local_name.len(), value.len());
    charge_put(cx, &scan, local_name.len(), local_name.len(), value.len())?;
    charged_write(cx, added, removed, |tree| {
        tree.set_attribute_ns(element, None, local_name, value)
    })
}

/// `setAttribute`: the name validated, then the tree's `set_attribute`, which lowers it on an
/// HTML element (an ASCII lowering keeps its length).
pub(super) fn set(
    cx: &mut DomCx<'_, '_>,
    element: NodeId,
    name: &str,
    value: &str,
) -> DomResult<()> {
    validate(cx, name, SET_NAME)?;
    let scan = attribute_reads::find(cx, element, Wanted::Qualified(name))?;
    let (added, removed) = put_bytes(&scan, name.len(), value.len());
    charge_put(cx, &scan, name.len(), name.len(), value.len())?;
    charged_write(cx, added, removed, |tree| {
        tree.set_attribute(element, name, value)
    })
}

/// `setAttributeNS`: "validate and extract" of the qualified name (the split, the characters,
/// then the four `NamespaceError` conditions of [`Tree::namespace_error`]), then a namespace
/// outside the six the tree stores is the `NotSupportedError`, all before the scan and any
/// charge for memory; then the tree's `set_attribute_ns`.
pub(super) fn set_ns(
    cx: &mut DomCx<'_, '_>,
    element: NodeId,
    namespace: Option<&str>,
    qualified_name: &str,
    value: &str,
) -> DomResult<()> {
    let (prefix, local_name) = split(cx, qualified_name)?;
    let url = namespace.filter(|url| !url.is_empty());
    if Tree::namespace_error(url, prefix, local_name) {
        return Err(DomEnd::Throw(DomException::new(
            "NamespaceError",
            NAMESPACE_MESSAGE,
        )));
    }
    let Some(namespace) = attribute_reads::namespace_of(namespace) else {
        return Err(DomEnd::Throw(DomException::not_supported(
            OUTSIDE_NAMESPACE,
        )));
    };
    let scan = attribute_reads::find(cx, element, Wanted::Local(namespace, local_name))?;
    let name = prefix.map_or(0, str::len).saturating_add(local_name.len());
    let (added, removed) = put_bytes(&scan, name, value.len());
    charge_put(cx, &scan, local_name.len(), name, value.len())?;
    charged_write(cx, added, removed, |tree| {
        tree.set_attribute_ns(element, namespace, qualified_name, value)
    })
}

/// `removeAttribute`: validates nothing; an attribute found is removed and its bytes refunded,
/// and a name found nowhere makes no further charge and no tree call.
pub(super) fn remove(cx: &mut DomCx<'_, '_>, element: NodeId, name: &str) -> DomResult<()> {
    let scan = attribute_reads::find(cx, element, Wanted::Qualified(name))?;
    let Some(found) = scan.found else {
        return Ok(());
    };
    reads::charge(cx, scan.steps(name.len()))?;
    charged_write(cx, 0, removed_bytes(&scan, found.stored_bytes), |tree| {
        tree.remove_attribute(element, name)
    })
}

/// `removeAttributeNS`: as [`remove`], by namespace (the empty string is null) and local name;
/// a namespace outside the six holds no attribute, so nothing is scanned.
pub(super) fn remove_ns(
    cx: &mut DomCx<'_, '_>,
    element: NodeId,
    namespace: Option<&str>,
    local_name: &str,
) -> DomResult<()> {
    let Some(namespace) = attribute_reads::namespace_of(namespace) else {
        return Ok(());
    };
    let scan = attribute_reads::find(cx, element, Wanted::Local(namespace, local_name))?;
    let Some(found) = scan.found else {
        return Ok(());
    };
    reads::charge(cx, scan.steps(local_name.len()))?;
    charged_write(cx, 0, removed_bytes(&scan, found.stored_bytes), |tree| {
        tree.remove_attribute_ns(element, namespace, local_name)
    })
}

/// `toggleAttribute`: the name validated; an attribute found is removed unless `force` is
/// `true` (then nothing is written), and one not found is added with the empty value unless
/// `force` is `false`. Answers whether the attribute is present after.
pub(super) fn toggle(
    cx: &mut DomCx<'_, '_>,
    element: NodeId,
    name: &str,
    force: Option<bool>,
) -> DomResult<bool> {
    validate(cx, name, TOGGLE_NAME)?;
    let scan = attribute_reads::find(cx, element, Wanted::Qualified(name))?;
    match (scan.found, force) {
        (Some(_), Some(true)) => Ok(true),
        (None, Some(false)) => Ok(false),
        (Some(found), Some(false) | None) => {
            reads::charge(cx, scan.steps(name.len()))?;
            charged_write(cx, 0, removed_bytes(&scan, found.stored_bytes), |tree| {
                tree.toggle_attribute(element, name, force)
            })
        }
        (None, Some(true) | None) => {
            let (added, removed) = put_bytes(&scan, name.len(), 0);
            charge_put(cx, &scan, name.len(), name.len(), 0)?;
            charged_write(cx, added, removed, |tree| {
                tree.toggle_attribute(element, name, force)
            })
        }
    }
}

/// The one place of the slice that charges the bytes of a write for memory, calls the tree
/// and settles the charge: `charge_stored(added, removed)` before any change, then the tree
/// call, whose answer is held; after a refusal `settle_stored(0, 0)` puts the handle back to
/// the bytes of the tree as it was, and the refusal ends as [`ended`] says; after a write
/// `settle_stored(added, removed)` refunds what it removed. A settle that answers a fault ends
/// the run faulted, and the tree's refusal is then dropped.
fn charged_write<T>(
    cx: &mut DomCx<'_, '_>,
    added: usize,
    removed: usize,
    write: impl FnOnce(&mut Tree) -> Result<T, WriteError>,
) -> DomResult<T> {
    cx.charge_stored(added, removed)?;
    match write(cx.tree_mut()) {
        Err(error) => {
            cx.settle_stored(0, 0)?;
            Err(ended(error))
        }
        Ok(value) => {
            cx.settle_stored(added, removed)?;
            Ok(value)
        }
    }
}
