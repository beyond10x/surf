// generated from surf v1
// contract digest 153f12c9bc6989d7738d48778d89afd6338830f138bc654b26d0f1a53611f3d7
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
