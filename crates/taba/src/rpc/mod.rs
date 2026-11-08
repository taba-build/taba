//! Taba's RPC

mod wire;

use core::convert::Infallible;

use serde::{Serialize, Serializer, de::DeserializeOwned};
use specta::{Type, Types, datatype::DataType};

#[doc(hidden)]
pub mod __private {
    pub use super::wire::{Message, SafeInteger};
}

/// A request from the page, with its handler's return types.
pub trait Procedure: DeserializeOwned + Type + Send + 'static {
    /// A successful handler in `Ok`.
    type Output: Serialize + Type + Send;

    /// A failed handler in `Err`.
    type Error: Serialize + Type + Send;
}

/// An error that can never happen.
#[derive(Debug)]
pub enum Never {}

impl Serialize for Never {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        match *self {}
    }
}

impl Type for Never {
    fn definition(types: &mut Types) -> DataType {
        Infallible::definition(types)
    }
}
