//! A handler whose request comes before its state.

use serde::Deserialize;
use specta::Type;
use taba::rpc::{Never, Procedure, Procedures, State};

#[derive(Clone)]
struct Game;

#[derive(Deserialize, Type)]
struct Buy;

impl Procedure for Buy {
    type Output = u32;
    type Error = Never;
}

async fn buy(_: Buy, State(_): State<Game>) -> Result<u32, Never> {
    Ok(1)
}

fn main() {
    Procedures::new().with_state(Game).handle(buy);
}
