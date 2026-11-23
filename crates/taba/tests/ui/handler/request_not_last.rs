//! A handler whose request comes before its state.

use serde::Deserialize;
use taba::{Input, Never, Procedure, Procedures, rpc::{Router, State}};

#[derive(Clone)]
struct Game;

#[derive(Deserialize, Input)]
struct Buy;

impl Procedure for Buy {
    type Output = u32;
    type Error = Never;
}

async fn buy(_: Buy, State(_): State<Game>) -> Result<u32, Never> {
    Ok(1)
}

fn main() {
    Router::builder([Procedures::new().declare::<Buy>()]).with_state(Game).handle(buy);
}
