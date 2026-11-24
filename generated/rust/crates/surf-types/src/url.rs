// generated from surf v1
// contract digest d6c21bcc6a807c12b12c5e62e83d4b3cf21db6c45d1dd9135d8affd63032a0ae
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
