//! Taba's RPC

mod error;
mod router;
mod sessions;
#[cfg(test)]
mod tests;
mod wire;

pub use self::{
    error::Error,
    router::{BuildError, Builder, Handler, Replies, Router, State},
    sessions::{SessionId, Sessions},
    wire::Frame,
};

#[doc(hidden)]
pub mod __private {
    pub use super::wire::{Batch, Kind, Message, Rejection, SafeInteger};
}
