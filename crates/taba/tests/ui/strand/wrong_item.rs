//! A strand's handler that streams the wrong item.

use futures_util::{Stream, stream};
use serde::Deserialize;
use taba::{Input, Never, Procedure, Procedures, Strand, rpc::Router};

#[derive(Deserialize, Input)]
struct Ore;

impl Procedure for Ore {
    type Output = Strand<u32>;
    type Error = Never;
}

fn ore(_: Ore) -> impl Stream<Item = Result<String, Never>> {
    stream::iter([Ok(String::from("3"))])
}

fn main() {
    Router::builder([Procedures::new().declare::<Ore>()]).handle(ore);
}
