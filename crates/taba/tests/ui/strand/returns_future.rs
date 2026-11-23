//! A strand's handler that returns a future.

use serde::Deserialize;
use taba::{Input, Never, Procedure, Procedures, Strand, rpc::Router};

#[derive(Deserialize, Input)]
struct Ore;

impl Procedure for Ore {
    type Output = Strand<u32>;
    type Error = Never;
}

async fn ore(_: Ore) -> Result<u32, Never> {
    Ok(3)
}

fn main() {
    Router::builder([Procedures::new().declare::<Ore>()]).handle(ore);
}
