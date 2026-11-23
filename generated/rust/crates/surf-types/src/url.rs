// generated from surf v1
// contract digest a781938fbf4be70c3e62196db4afd2b86d2f53f3955885c5d3e606169c3ba178
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
