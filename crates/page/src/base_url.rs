//! The document base URL (HTML Standard, "document base URL", from memory): the one owner of the
//! rule in surf (decision D10 of `story:agent-follow-links`). The session resolves a link's
//! target against it, and `story:agent-form-port` a form's action.
//!
//! The rule: the frozen base URL of the first HTML `base` element in tree order that has an
//! `href` attribute, which is that `href` parsed against the document's address; the document's
//! address when there is no such element or its `href` does not parse. Template contents are not
//! searched, as they are not in the document. Left out: `about:blank` and `srcdoc` documents,
//! which take their base from another document, and the check "Is base allowed for Document?"
//! of Content Security Policy, which surf does not implement.

use surf_html::{Namespace, NodeData, Tree};
use surf_url::Url;

/// The document base URL of the document `tree` loaded from `document_address`.
///
/// One walk in tree order with its own stack, which stops at the first HTML `base` element with
/// an `href` attribute: the time is linear in the number of nodes before it.
pub fn document_base_url(tree: &Tree, document_address: &Url) -> Url {
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if let NodeData::Element {
            namespace: Namespace::Html,
            local_name,
            attributes,
            ..
        } = tree.data(node)
            && local_name == "base"
            && let Some(href) = attributes
                .iter()
                .find(|attribute| attribute.namespace.is_none() && attribute.local_name == "href")
        {
            return Url::parse(&href.value, Some(document_address))
                .unwrap_or_else(|_| document_address.clone());
        }
        pending.extend(tree.children(node).iter().rev());
    }
    document_address.clone()
}
