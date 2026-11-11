//! A call's handler that returns a stream.

use futures_util::{Stream, stream};
use serde::Deserialize;
use specta::Type;
use taba::rpc::{Never, Procedure, Procedures};

#[derive(Deserialize, Type)]
struct Buy;

impl Procedure for Buy {
    type Output = u32;
    type Error = Never;
}

fn buy(_: Buy) -> impl Stream<Item = Result<u32, Never>> {
    stream::iter([Ok(1)])
}

fn main() {
    Procedures::new().handle(buy);
}
