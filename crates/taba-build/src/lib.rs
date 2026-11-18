//! Taba's build-time bundler
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "bundle")]
mod bundle;
#[cfg(feature = "typescript")]
mod typescript;

#[cfg(feature = "bundle")]
pub use self::bundle::{BundleError, bundle};
#[cfg(feature = "typescript")]
pub use self::typescript::{
    Declarations,
    client::{Build, WriteError},
};

#[cfg(feature = "test")]
#[doc(hidden)]
pub mod __private {
    pub use crate::typescript::cases::Cases;
}
