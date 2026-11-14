//! A call's handler that returns its output without a `Result`.

use serde::Deserialize;
use taba::{Input, rpc::{Never, Procedure, Procedures}};

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
    Procedures::new().handle(buy);
}
