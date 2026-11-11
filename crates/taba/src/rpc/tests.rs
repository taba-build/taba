//! An idle game's procedures, for tests to answer with handlers of their own.

use std::prelude::rust_2024::*;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::{Never, Procedure, Strand};

#[derive(Clone)]
pub(super) struct Game {
    pub(super) drills: u32,
}

#[derive(Deserialize, Type)]
pub(super) struct Buy {
    pub(super) generator: Generator,
}

#[derive(Deserialize, Type)]
pub(super) enum Generator {
    Drill,
    Furnace,
}

#[derive(Serialize, Type)]
pub(super) enum BuyError {
    CantAfford { missing: u32 },
}

impl Procedure for Buy {
    type Output = u32;
    type Error = BuyError;
}

#[derive(Deserialize, Type)]
pub(super) struct Sell;

impl Procedure for Sell {
    type Output = u32;
    type Error = Never;
}

#[derive(Deserialize, Type)]
pub(super) struct Hire;

impl Procedure for Hire {
    type Output = u32;
    type Error = Never;
}

#[derive(Deserialize, Type)]
pub(super) struct Ore;

impl Procedure for Ore {
    type Output = Strand<u32>;
    type Error = Never;
}
