//! Taba's RPC

mod wire;

use core::{convert::Infallible, marker::PhantomData};

use serde::{Serialize, Serializer, de::DeserializeOwned};
use specta::{Type, Types, datatype::DataType};

#[doc(hidden)]
pub mod __private {
    pub use super::wire::{Frame, Kind, Message, SafeInteger};
}

/// A request from the page, with its handler's return types.
pub trait Procedure: DeserializeOwned + Type + Send + 'static {
    /// A successful handler in `Ok`.
    type Output: Output;

    /// A failed handler in `Err`.
    type Error: Serialize + Type + Send;
}

/// What a procedure can output: one value, or a [`Strand`] of them.
#[diagnostic::on_unimplemented(message = "`{Self}` can't be a procedure's output")]
pub trait Output {}

impl<T: Serialize + Type + Send> Output for T {}

/// A subscription's `T`s, as a procedure's [`Output`](Procedure::Output).
#[derive(Debug)]
pub struct Strand<T>(PhantomData<T>);

impl<T: Serialize + Type + Send> Output for Strand<T> {}

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
