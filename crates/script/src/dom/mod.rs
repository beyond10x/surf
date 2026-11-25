//! surf's DOM behind the generated bindings (`crate::bindings`): one file per slice, each with
//! the `impl` of its slice's trait, and the wrapper hook. Until a slice's story writes its
//! methods, each answers the trait's default, a `NotSupportedError` naming the member
//! (`story:webidl-bindings`); the wrapper hook throws `NotSupportedError` until
//! `story:scripts-dom-reading` writes the real one.

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

use crate::bindings::Slices;

/// surf's slices and wrapper hook.
pub fn slices() -> Slices {
    Slices {
        reading: Box::new(SurfReading),
        attributes: Box::new(SurfAttributes),
        mutation: Box::new(SurfMutation),
        selectors: Box::new(SurfSelectors),
        wrap: Box::new(SurfWrap),
    }
}
