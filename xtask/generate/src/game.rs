//! An idle game's types, and what serde_json writes and reads for them.
#![expect(dead_code, reason = "the types exist for their shapes")]

use std::{collections::BTreeMap, marker::PhantomData};

use serde::{Deserialize, Serialize};
use serde_json::json;
use taba::{Input, Never, Output, Procedure, Procedures, Strand};
use taba_build::{__private::Cases, Build, Declarations};

/// A machine that makes ore.
#[derive(Serialize, Deserialize, Output, Input)]
#[serde(rename_all = "lowercase")]
enum Generator {
    Drill,
    Furnace,
}

/// The mine, after a tick.
///
/// Sent once per tick.
#[derive(Serialize, Output)]
struct Mine<'a> {
    /// Ore on hand, by kind.
    ore: BTreeMap<String, u32>,

    /// The last hauls, newest first.
    hauls: Vec<(Generator, f64)>,

    /// What the shop sells.
    for_sale: Vec<shop::Generator>,

    /// The level below, dug on a `*/5` schedule.
    deeper: Option<Box<Mine<'a>>>,

    running: bool,

    /// The last survey, as the surveyor wrote it.
    survey: serde_json::Value,

    #[serde(rename = "gift-note")]
    gift_note: &'a str,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    boost: Option<Boost>,

    #[serde(flatten)]
    weather: Weather,

    /// What's on its way.
    incoming: Vec<Shipment<shop::Generator>>,

    /// The generator that dug last.
    last_digger: Id<Generator>,
}

/// The weather over the mine.
#[derive(Serialize, Deserialize, Output, Input)]
enum Weather {
    Clear,
    Storm { strength: u32 },
}

/// An id for one kind of thing.
#[derive(Serialize, Output)]
struct Id<T> {
    raw: u32,
    kind: PhantomData<T>,
}

/// Something a shipment can carry.
trait Freight {}

impl Freight for shop::Generator {}

/// A boost the mine is under.
#[derive(Serialize, Deserialize, Output, Input)]
struct Boost {
    /// How many times faster it digs.
    rate: u32,
}

/// Something on its way to the mine.
#[derive(Serialize, Deserialize, Output, Input)]
struct Shipment<T: Freight> {
    /// Ticks until it arrives.
    eta: u32,

    #[serde(flatten)]
    cargo: T,

    /// The shipment after it.
    next: Option<Box<Shipment<T>>>,
}

#[derive(Serialize, Output)]
enum Unreachable {}

mod shop {
    use serde::{Deserialize, Serialize};
    use taba::{Input, Output, Procedure, Strand};

    /// A generator for sale.
    #[derive(Serialize, Deserialize, Output, Input)]
    pub(super) struct Generator {
        /// What it costs, in ore.
        pub(super) price: u32,
    }

    /// Sells a generator back, for some of its price.
    #[derive(Deserialize, Input)]
    pub(super) struct Sell(pub(super) Generator);

    impl Procedure for Sell {
        type Output = u32;
        type Error = String;
    }

    /// Each new price, until the shop closes.
    #[derive(Deserialize, Input)]
    pub(super) struct Prices;

    impl Procedure for Prices {
        type Output = Strand<u32>;
        type Error = String;
    }
}

/// Buys generators.
#[derive(Deserialize, Input)]
#[serde(untagged)]
enum Buy {
    One(Generator),

    Batch {
        generator: Generator,

        /// How many beyond the first, none when absent.
        #[serde(default)]
        extra: u32,

        /// A note for the shop.
        note: serde_json::Value,
    },

    Delivered(Shipment<shop::Generator>),
}

impl Procedure for Buy {
    type Output = Cart;
    type Error = Never;
}

/// The mine after each tick.
#[derive(Deserialize, Input)]
struct Ticks;

impl Procedure for Ticks {
    type Output = Strand<Mine<'static>>;
    type Error = Never;
}

/// A cart of ore, by kind.
#[derive(Serialize, Deserialize, Output, Input)]
struct Cart {
    /// What pulls it.
    pulled_by: String,

    #[serde(flatten)]
    ore: BTreeMap<String, u32>,
}

/// A claim on a vein.
#[derive(Serialize, Deserialize, Output, Input)]
struct Claim {
    owner: String,

    /// Ore dug, by kind.
    #[serde(flatten)]
    dug: BTreeMap<String, u32>,

    /// What the owner noted, by kind.
    #[serde(flatten)]
    notes: BTreeMap<String, String>,
}

/// A train, and the wagon it pulls.
#[derive(Serialize, Deserialize, Output, Input)]
struct Train {
    engine: String,

    #[serde(flatten)]
    wagon: Wagon,
}

/// A wagon of ore, by kind.
#[derive(Serialize, Deserialize, Output, Input)]
struct Wagon {
    load: u32,

    #[serde(flatten)]
    ore: BTreeMap<String, u32>,
}

/// Ore piled up under a boost, by kind.
#[derive(Serialize, Deserialize, Output, Input)]
struct Stockpile {
    #[serde(flatten)]
    boost: Boost,

    #[serde(flatten)]
    ore: BTreeMap<String, u32>,
}

/// A hold, and its ore by kind once it has some.
#[derive(Serialize, Deserialize, Output, Input)]
struct Hold {
    owner: String,

    #[serde(flatten)]
    ore: Option<BTreeMap<String, u32>>,
}

/// The surveyor's notes, by vein.
#[derive(Serialize, Deserialize, Output, Input)]
struct Notebook {
    author: String,

    #[serde(flatten)]
    notes: BTreeMap<String, serde_json::Value>,
}

/// The next tick's forecast.
#[derive(Serialize, Deserialize, Output, Input)]
struct Forecast {
    tick: u32,

    #[serde(flatten)]
    calm: Calm,

    #[serde(flatten)]
    weather: Weather,

    #[serde(flatten)]
    boost: Option<Boost>,
}

/// No change in the weather.
#[derive(Serialize, Deserialize, Output, Input)]
struct Calm;

/// A parcel, whatever it holds.
#[derive(Serialize, Deserialize, Output, Input)]
struct Parcel<T> {
    weight: u32,

    #[serde(flatten)]
    contents: T,
}

/// Something that happened at the mine.
#[derive(Serialize, Deserialize, Output, Input)]
#[serde(tag = "kind")]
enum Event {
    Bought(shop::Generator),
    Sold(BTreeMap<String, u32>),
    Calmed(Calm),
    Changed(Weather),
    Boosted(Option<Boost>),
    Faded(Spent),
    Stored(Reserve),
}

/// A boost that ran out.
#[derive(Serialize, Deserialize, Output, Input)]
struct Spent(Option<Boost>);

/// A boost kept for later.
#[derive(Serialize, Deserialize, Output, Input)]
#[serde(transparent)]
struct Reserve(Option<Boost>);

/// Ore by kind, whatever names the kind.
#[derive(Serialize, Output)]
struct Tally<K>(BTreeMap<K, u32>);

/// The game's procedures.
fn procedures() -> Procedures {
    taba::procedures!()
        .declare::<Buy>()
        .declare::<Ticks>()
        .declare::<shop::Sell>()
        .declare::<shop::Prices>()
}

pub(crate) fn client() -> Build {
    Build::new().package(procedures(), CLIENT)
}

/// Where [`client`] writes, from the repo root.
pub(crate) const CLIENT: &str = "xtask/generate/conformance/client";

pub(crate) fn declarations() -> Declarations {
    Declarations::new("generate")
        .output::<Mine>()
        .output::<Unreachable>()
        .input::<Buy>()
        .both::<Cart>()
        .both::<Claim>()
        .both::<Train>()
        .both::<Stockpile>()
        .both::<Hold>()
        .both::<Notebook>()
        .both::<Forecast>()
        .both::<Parcel<Calm>>()
        .both::<Parcel<Weather>>()
        .both::<Parcel<Option<Boost>>>()
        .both::<Event>()
        .output::<Tally<String>>()
}

pub(crate) fn cases() -> Cases {
    let deeper = Mine {
        ore: BTreeMap::new(),
        hauls: vec![],
        for_sale: vec![],
        deeper: None,
        running: false,
        survey: json!(null),
        gift_note: "",
        boost: None,
        weather: Weather::Clear,
        incoming: vec![],
        last_digger: Id { raw: 0, kind: PhantomData },
    };
    let mut cases = Cases::new("generate");
    cases.output(&Mine {
        ore: BTreeMap::from([("copper".to_owned(), 12)]),
        hauls: vec![(Generator::Drill, 1.5), (Generator::Furnace, f64::NAN)],
        for_sale: vec![shop::Generator { price: 40 }],
        deeper: Some(Box::new(deeper)),
        running: true,
        survey: json!({ "depth": 3, "veins": ["copper"] }),
        gift_note: "for the drill",
        boost: Some(Boost { rate: 2 }),
        weather: Weather::Storm { strength: 4 },
        incoming: vec![Shipment {
            eta: 2,
            cargo: shop::Generator { price: 40 },
            next: Some(Box::new(Shipment {
                eta: 5,
                cargo: shop::Generator { price: 60 },
                next: None,
            })),
        }],
        last_digger: Id { raw: 1, kind: PhantomData },
    });
    cases.input::<Buy>(r#""drill""#);
    cases.input::<Buy>(r#""shovel""#);
    cases.input::<Buy>(r#"{"generator":"furnace","note":{"from":"the shop"}}"#);
    cases.input::<Buy>(r#"{"generator":"furnace","extra":2,"note":null}"#);
    cases.input::<Buy>(r#"{"generator":"furnace"}"#);
    cases.input::<Buy>(r#"{"eta":3,"price":40,"next":{"eta":1,"price":60}}"#);
    cases.input::<Buy>(r#"{"eta":3}"#);

    cases.output(&Cart { pulled_by: "mule".to_owned(), ore: ore([("copper", 3)]) });
    cases.output(&Cart { pulled_by: "mule".to_owned(), ore: ore([("pulled_by", 7)]) });
    cases.input::<Cart>(r#"{"pulled_by":"mule"}"#);
    cases.input::<Cart>(r#"{"pulled_by":"mule","copper":"lots"}"#);

    let notes = BTreeMap::from([("copper".to_owned(), "rich".to_owned())]);
    cases.output(&Claim { owner: "ana".to_owned(), dug: ore([("copper", 1)]), notes });
    cases.input::<Claim>(r#"{"owner":"ana"}"#);
    cases.input::<Claim>(r#"{"owner":"ana","copper":1}"#);

    let wagon = Wagon { load: 2, ore: ore([("engine", 5)]) };
    cases.output(&Train { engine: "steam".to_owned(), wagon });
    cases.input::<Train>(r#"{"engine":"steam","load":2}"#);
    cases.input::<Train>(r#"{"engine":"steam","load":2,"copper":"x"}"#);

    cases.output(&Stockpile { boost: Boost { rate: 1 }, ore: ore([("copper", 2)]) });
    cases.input::<Stockpile>(r#"{"rate":1,"copper":2}"#);
    cases.input::<Stockpile>(r#"{"copper":2}"#);

    cases.output(&Hold { owner: "ana".to_owned(), ore: None });
    cases.output(&Hold { owner: "ana".to_owned(), ore: Some(ore([("copper", 1)])) });
    cases.input::<Hold>(r#"{"owner":"ana"}"#);

    let notes = BTreeMap::from([("copper".to_owned(), json!("seen twice"))]);
    cases.output(&Notebook { author: "ana".to_owned(), notes });
    cases.input::<Notebook>(r#"{"author":"ana","copper":"seen","tin":[1]}"#);
    cases.input::<Notebook>(r#"{"author":5}"#);

    cases.output(&Forecast { tick: 1, calm: Calm, weather: Weather::Clear, boost: None });
    let storm = Weather::Storm { strength: 2 };
    cases.output(&Forecast { tick: 1, calm: Calm, weather: storm, boost: Some(Boost { rate: 3 }) });
    cases.input::<Forecast>(r#"{"tick":1,"Clear":null}"#);
    cases.input::<Forecast>(r#"{"tick":1,"Storm":{"strength":2},"rate":3}"#);
    cases.input::<Forecast>(r#"{"tick":1,"Clear":"Clear"}"#);

    cases.output(&Parcel { weight: 1, contents: Calm });
    cases.output(&Parcel { weight: 1, contents: Weather::Clear });
    cases.output(&Parcel { weight: 1, contents: None::<Boost> });
    cases.output(&Parcel { weight: 1, contents: Some(Boost { rate: 3 }) });
    cases.input::<Parcel<Calm>>(r#"{"weight":1}"#);
    cases.input::<Parcel<Weather>>(r#"{"weight":1,"Clear":null}"#);
    cases.input::<Parcel<Option<Boost>>>(r#"{"weight":1}"#);

    cases.output(&Event::Bought(shop::Generator { price: 40 }));
    cases.output(&Event::Sold(ore([("copper", 3)])));
    cases.output(&Event::Sold(ore([("kind", 3)])));
    cases.output(&Event::Calmed(Calm));
    cases.output(&Event::Changed(Weather::Clear));
    cases.input::<Event>(r#"{"kind":"Bought","price":40}"#);
    cases.input::<Event>(r#"{"kind":"Bought"}"#);
    cases.input::<Event>(r#"{"kind":"Sold"}"#);
    cases.input::<Event>(r#"{"kind":"Calmed"}"#);
    cases.input::<Event>(r#"{"kind":"Changed","Clear":null}"#);
    cases.input::<Event>(r#"{"kind":"Boosted","rate":2}"#);
    cases.input::<Event>(r#"{"kind":"Boosted"}"#);
    cases.input::<Event>(r#"{"kind":"Faded"}"#);
    cases.input::<Event>(r#"{"kind":"Stored"}"#);
    cases
}

/// Ore by kind.
fn ore<const N: usize>(kinds: [(&str, u32); N]) -> BTreeMap<String, u32> {
    kinds.into_iter().map(|(kind, count)| (kind.to_owned(), count)).collect()
}
