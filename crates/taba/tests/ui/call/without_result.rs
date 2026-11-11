//! A call's handler that returns its output without a `Result`.

use serde::Deserialize;
use specta::Type;
use taba::rpc::{Never, Procedure, Procedures};

#[derive(Deserialize, Type)]
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
