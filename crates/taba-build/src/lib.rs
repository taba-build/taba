//! Taba's build-time bundler

#[cfg(feature = "bundle")]
mod bundle;

#[cfg(feature = "bundle")]
pub use self::bundle::{Error, Result, bundle};
