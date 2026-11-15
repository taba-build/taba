//! One instantiation of a generic type as a procedure.

use serde::Deserialize;
use taba::{Input, rpc::{Never, Procedure}};

#[derive(Deserialize, Input)]
struct Find<T> {
    query: T,
}

impl Procedure for Find<String> {
    type Output = u32;
    type Error = Never;
}

fn main() {}
