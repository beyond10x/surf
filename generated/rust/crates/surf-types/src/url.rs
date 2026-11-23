// generated from surf v1
// contract digest 0fe2c0b6e437654e024a069cbcca8147e4a9c7adcd5b10150e01611989dfa058
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
