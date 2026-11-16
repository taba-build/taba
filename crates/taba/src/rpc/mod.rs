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
    router::{BuildError, Builder, Handler, Procedures, Replies, Router, State},
    sessions::{SessionId, Sessions},
    wire::Frame,
};
use crate::typescript::Request;

#[doc(hidden)]
pub mod __private {
    pub use super::wire::{Batch, Kind, Message, Rejection, SafeInteger};
}

/// Starts a list of procedures for [`Router::builder`].
///
/// ```rust,ignore
/// pub fn procedures() -> Procedures {
///     taba::procedures!().declare::<shop::Buy>().declare::<Hauls>()
/// }
/// ```
#[macro_export]
macro_rules! procedures {
    () => {
        $crate::rpc::Procedures::__new()
    };
}

/// What TypeScript can call or subscribe to.
///
/// TypeScript calls a procedure by its type's path, which a generic type's instantiations share:
///
/// ```rust,ignore
/// #[derive(Deserialize, Input)]
/// struct Find<T> {
///     query: T,
/// }
///
/// #[derive(Deserialize, Input)]
/// struct Id(String);
///
/// // both would be `shop::Find`, sent as `{ query: string }`
/// impl Procedure for Find<String> { ... }
/// impl Procedure for Find<Id> { ... }
/// ```
///
/// To call each instantiation, wrap it in a struct of its own:
///
/// ```rust,ignore
/// #[derive(Deserialize, Input)]
/// struct FindByName(Find<String>);
///
/// #[derive(Deserialize, Input)]
/// struct FindById(Find<Id>);
///
/// impl Procedure for FindByName { ... }
/// impl Procedure for FindById { ... }
/// ```
pub trait Procedure: Request {
    /// What TypeScript gets on success.
    type Output;

    /// What TypeScript gets on error.
    type Error;
}

/// How a procedure replies with its [`Output`](Procedure::Output).
pub trait IntoReply {
    /// [`Once`] for a call, [`Strand`] for a subscription.
    type Reply;
}

impl<T: Serialize> IntoReply for T {
    type Reply = Once<T>;
}

impl<T> IntoReply for Strand<T> {
    type Reply = Strand<T>;
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
