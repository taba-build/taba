//! A call's handler that returns its output without a `Result`.

use serde::Deserialize;
use taba::{Input, Never, Procedure, rpc::Router};

#[derive(Deserialize, Input)]
struct Buy;

impl Procedure for Buy {
    type Output = u32;
    type Error = Never;
}

async fn buy(_: Buy) -> u32 {
    1
}

fn main() {
    Router::builder([taba::procedures!().declare::<Buy>()]).handle(buy);
}
