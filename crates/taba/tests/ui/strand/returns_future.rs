//! A strand's handler that returns a future.

use serde::Deserialize;
use taba::{Read, rpc::{Never, Procedure, Procedures, Strand}};

#[derive(Deserialize, Read)]
struct Ore;

impl Procedure for Ore {
    type Output = Strand<u32>;
    type Error = Never;
}

async fn ore(_: Ore) -> Result<u32, Never> {
    Ok(3)
}

fn main() {
    Procedures::new().handle(ore);
}
