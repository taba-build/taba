//! A call's handler whose future isn't `Send`.

use std::rc::Rc;

use serde::Deserialize;
use specta::Type;
use taba::rpc::{Never, Procedure, Procedures};

#[derive(Deserialize, Type)]
struct Buy;

impl Procedure for Buy {
    type Output = u32;
    type Error = Never;
}

async fn buy(_: Buy) -> Result<u32, Never> {
    let drills = Rc::new(1);
    core::future::ready(()).await;
    Ok(*drills)
}

fn main() {
    Procedures::new().handle(buy);
}
