//! Procedures, as a types crate declares them.

use alloc::vec::Vec;
use core::marker::PhantomData;

use serde_core::{Serialize, Serializer};

use crate::{
    Output,
    typescript::{Named, Request, Shape},
};

#[doc(hidden)]
pub mod __private {
    pub use super::{Declaration, ReplyShape};
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

/// A [`Reply`](IntoReply::Reply) TypeScript can see.
pub trait Reply {
    const SHAPE: ReplyShape;
}

impl<T: Output> Reply for Once<T> {
    const SHAPE: ReplyShape = ReplyShape::Once(T::__SHAPE);
}

impl<T: Output> Reply for Strand<T> {
    const SHAPE: ReplyShape = ReplyShape::Strand(T::__SHAPE);
}

/// A reply, as TypeScript sees it.
#[derive(Debug)]
pub enum ReplyShape {
    /// A call's value.
    Once(Shape),

    /// Each of a subscription's values.
    Strand(Shape),
}

/// A list of procedures.
#[derive(Debug, Default)]
pub struct Procedures {
    pub(crate) declarations: Vec<Declaration>,
}

/// One procedure in a [`Procedures`] list, as TypeScript sees it.
#[derive(Debug)]
pub struct Declaration {
    /// Its request type's `Named`.
    pub named: fn() -> &'static Named,

    /// What it replies with.
    pub reply: &'static ReplyShape,

    /// What it fails with, `never` if it can't.
    pub error: &'static Shape,
}

impl Procedures {
    /// A list with no procedures.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `P`.
    pub fn declare<P: Procedure>(mut self) -> Self
    where
        P::Output: IntoReply<Reply: Reply>,
        P::Error: Output,
    {
        let reply = const { &<<P::Output as IntoReply>::Reply as Reply>::SHAPE };
        let error = const { &<P::Error as Output>::__SHAPE };
        self.declarations.push(Declaration { named: P::named, reply, error });
        self
    }

    #[doc(hidden)]
    pub fn __declarations(&self) -> &[Declaration] {
        &self.declarations
    }
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

impl Output for Never {
    const __SHAPE: Shape = Shape::Never;
}
