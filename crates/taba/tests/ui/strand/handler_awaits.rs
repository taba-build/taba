//! A strand's handler that awaits before returning its stream.

use futures_util::{Stream, stream};
use serde::Deserialize;
use specta::Type;
use taba::rpc::{Never, Procedure, Procedures, Strand};

#[derive(Deserialize, Type)]
struct Ore;

impl Procedure for Ore {
    type Output = Strand<u32>;
    type Error = Never;
}

async fn ore(_: Ore) -> impl Stream<Item = Result<u32, Never>> {
    stream::iter([Ok(3)])
}

fn main() {
    Procedures::new().handle(ore);
}
