//! A strand of an item that isn't `Serialize`.

use futures_util::{Stream, stream};
use serde::Deserialize;
use taba::{Input, Never, Procedure, Strand, rpc::Router};

#[derive(Deserialize, Input)]
struct Ore;

struct Nugget;

impl Procedure for Ore {
    type Output = Strand<Nugget>;
    type Error = Never;
}

fn ore(_: Ore) -> impl Stream<Item = Result<Nugget, Never>> {
    stream::iter([Ok(Nugget)])
}

fn main() {
    Router::builder([taba::procedures!().declare::<Ore>()]).handle(ore);
}
