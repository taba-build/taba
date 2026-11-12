//! Taba's build-time bundler
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "bundle")]
mod bundle;

#[cfg(feature = "bundle")]
pub use self::bundle::{Error, Result, bundle};
