// generated from surf v1
// contract digest 9cf563aeb85e0ebaf274c9cab36fdfa5c94c5872239a1808e0be06ff915e5e98
// do not edit: regenerate with `ess synthesize`

//! URL — `surf.url`.
//!
//! The address of a resource, as every other domain carries it.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Url — `surf.url.Url`: a distinct wrapper around `String`.
///
/// An absolute URL in its serialized form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url(pub String);
