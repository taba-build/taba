//! Taba's RPC

mod error;
mod router;
mod sessions;
#[cfg(test)]
mod tests;
mod wire;

use core::marker::PhantomData;

use serde::{Serialize, Serializer};

pub use self::{
    error::Error,
    router::{Answers, BuildError, Handler, Procedures, Router, State},
    sessions::{SessionId, Sessions},
    wire::Frame,
};

#[doc(hidden)]
pub mod __private {
    pub use super::wire::{Kind, Message, Rejection, SafeInteger};
}

/// What TypeScript can call or subscribe to.
pub trait Procedure {
    /// What TypeScript gets on success.
    type Output;

    /// What TypeScript gets on error.
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
