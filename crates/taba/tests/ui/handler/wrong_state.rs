//! A handler that reads a state other than the one it's given.

use serde::Deserialize;
use taba::{Read, rpc::{Never, Procedure, Procedures, State}};

#[derive(Clone)]
struct Game;

#[derive(Clone)]
struct Shop;

#[derive(Deserialize, Read)]
struct Buy;

impl Procedure for Buy {
    type Output = u32;
    type Error = Never;
}

async fn buy(State(_): State<Shop>, _: Buy) -> Result<u32, Never> {
    Ok(1)
}

fn main() {
    Procedures::new().with_state(Game).handle(buy);
}
