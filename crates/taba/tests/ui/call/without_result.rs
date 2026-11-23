//! A call's handler that returns its output without a `Result`.

use serde::Deserialize;
use taba::{Input, Never, Procedure, Procedures, rpc::Router};

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
    Router::builder([Procedures::new().declare::<Buy>()]).handle(buy);
}
