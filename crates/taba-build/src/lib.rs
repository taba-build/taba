//! Taba's build-time bundler
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "bundle")]
mod bundle;
#[cfg(feature = "typescript")]
mod typescript;

#[cfg(feature = "bundle")]
pub use self::bundle::{Error, Result, bundle};
#[cfg(feature = "typescript")]
pub use self::typescript::Declarations;

#[cfg(feature = "test")]
#[doc(hidden)]
pub mod __private {
    pub use crate::typescript::cases::Cases;
}
