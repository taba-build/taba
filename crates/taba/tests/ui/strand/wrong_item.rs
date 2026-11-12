//! A strand's handler that streams the wrong item.

use futures_util::{Stream, stream};
use serde::Deserialize;
use taba::{Read, rpc::{Never, Procedure, Procedures, Strand}};

#[derive(Deserialize, Read)]
struct Ore;

impl Procedure for Ore {
    type Output = Strand<u32>;
    type Error = Never;
}

fn ore(_: Ore) -> impl Stream<Item = Result<String, Never>> {
    stream::iter([Ok(String::from("3"))])
}

fn main() {
    Procedures::new().handle(ore);
}
