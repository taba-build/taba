//! A strand's handler that streams items without a `Result`.

use futures_util::{Stream, stream};
use serde::Deserialize;
use taba::{Input, Never, Procedure, Strand, rpc::Router};

#[derive(Deserialize, Input)]
struct Ore;

impl Procedure for Ore {
    type Output = Strand<u32>;
    type Error = Never;
}

fn ore(_: Ore) -> impl Stream<Item = u32> {
    stream::iter([3])
}

fn main() {
    Router::builder([taba::procedures!().declare::<Ore>()]).handle(ore);
}
