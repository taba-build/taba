//! Taba's RPC

mod router;
mod wire;

use core::{convert::Infallible, marker::PhantomData};

use serde::{Serialize, Serializer};
use specta::{Type, Types, datatype::DataType};

pub use self::router::{Answers, Error, Handler, Procedures, Router, State};

#[doc(hidden)]
pub mod __private {
    pub use super::wire::{Frame, Kind, Message, Rejection, SafeInteger};
}

/// What the page can call or subscribe to.
pub trait Procedure {
    /// What the page gets on success.
    type Output;

    /// What the page gets on error.
    type Error;
}

/// How a procedure's [`Output`](Procedure::Output) is answered.
pub trait Output {
    /// [`Once`] for a call, [`Strand`] for a subscription.
    type Shape;
}

impl<T: Serialize> Output for T {
    type Shape = Once<T>;
}

impl<T> Output for Strand<T> {
    type Shape = Strand<T>;
}

/// A call's `T`, wrapped so rustc names the bound a handler missed.
#[derive(Debug)]
pub struct Once<T>(PhantomData<T>);

/// A subscription's `T`s, as a procedure's [`Output`](Procedure::Output).
#[derive(Debug)]
pub struct Strand<T>(PhantomData<T>);

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
