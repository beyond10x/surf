//! surf's DOM behind the generated bindings (`crate::bindings`): one file per slice, each with
//! the `impl` of its slice's trait, and the wrapper hook. The slice `reading` and the wrapper
//! hook read the document's tree (`story:scripts-dom-reading`); until a slice's story writes
//! its methods, each of the others answers the trait's default, a `NotSupportedError` naming
//! the member (`story:webidl-bindings`).

mod attributes;
mod mutation;
mod reading;
mod selectors;
mod wrapper;

pub use attributes::SurfAttributes;
pub use mutation::SurfMutation;
pub use reading::SurfReading;
pub use selectors::SurfSelectors;
pub use wrapper::SurfWrap;

use surf_url::Url;

use crate::bindings::Slices;

/// What the document's members read that its tree does not hold: its address (`URL`,
/// `documentURI`, and the fallback base URL of `baseURI`), its content type (`contentType`) and
/// the name of its encoding (`characterSet`, `charset`, `inputEncoding`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentInfo {
    /// The document's address.
    pub address: Url,
    /// The document's content type.
    pub content_type: String,
    /// The name of the document's encoding.
    pub encoding: String,
}

impl Default for DocumentInfo {
    /// `about:blank`, `text/html` and `UTF-8`.
    fn default() -> Self {
        DocumentInfo {
            address: Url::parse("about:blank", None).expect("`about:blank` is a URL"),
            content_type: "text/html".to_string(),
            encoding: "UTF-8".to_string(),
        }
    }
}

/// surf's slices and wrapper hook for a document with `info`.
pub fn slices(info: DocumentInfo) -> Slices {
    Slices {
        reading: Box::new(SurfReading::new(info)),
        attributes: Box::new(SurfAttributes),
        mutation: Box::new(SurfMutation),
        selectors: Box::new(SurfSelectors),
        wrap: Box::new(SurfWrap::default()),
    }
}
