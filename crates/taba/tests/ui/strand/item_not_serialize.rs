//! A strand of an item that isn't `Serialize`.

use futures_util::{Stream, stream};
use serde::Deserialize;
use taba::{Read, rpc::{Never, Procedure, Procedures, Strand}};

#[derive(Deserialize, Read)]
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
    Procedures::new().handle(ore);
}
