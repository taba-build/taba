//! Declarations for an idle game's types.
#![expect(dead_code, reason = "the types exist for their shapes")]

use std::{collections::BTreeMap, marker::PhantomData};

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use taba::{Input, Output};
use taba_build::Declarations;

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
#[derive(Serialize, Output)]
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
#[derive(Serialize, Output)]
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
    use taba::{Input, Output};

    /// A generator for sale.
    #[derive(Serialize, Deserialize, Output, Input)]
    pub(super) struct Generator {
        /// What it costs, in ore.
        pub(super) price: u32,
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

        /// A note for the shop, kept as sent.
        note: Box<RawValue>,
    },

    Delivered(Shipment<shop::Generator>),
}

#[test]
fn every_shape_kind() {
    let declarations =
        Declarations::new("typescript").output::<Mine>().output::<Unreachable>().input::<Buy>();
    insta::assert_snapshot!(declarations, @r#"
    import type * as $ from "./taba.js";
    export type $Output<
    	Declared,
    	Parts
    > = $Expanded<$OutputWithRest<Declared & $AllKnown<Parts>, $AllOutputRest<Parts>>>;
    export type $Input<
    	Declared,
    	Parts
    > = $Expanded<$InputWithRest<Declared & $AllKnown<Parts>, $AllInputRest<Parts>>>;
    export type $Expanded<T> = T extends unknown ? { [Key in keyof T]: T[Key] } : never;
    export type $Flattened<Part> = Part extends null ? {} : Part extends string ? $UnitVariant<Part> : Part;
    export type $UnitVariant<Name extends string> = string extends Name ? never : Record<Name, null>;
    export type $AllKnown<Parts> = Parts extends [infer Part, ...infer Tail] ? $Known<$Flattened<Part>> & $AllKnown<Tail> : {};
    export type $Known<Part> = Part extends unknown ? { [Key in keyof Part as string extends Key ? never : Key]: Part[Key] } : never;
    export type $OutputWithRest<
    	Known,
    	Rest
    > = [Rest] extends [never] ? Known : Known extends unknown ? { [Key in keyof Known]: Known[Key] | Rest } & {
    	[key: string]: Rest | Known[keyof Known];
    } : never;
    export type $AllOutputRest<Parts> = Parts extends [infer Part, ...infer Tail] ? $OutputRest<$Flattened<Part>> | $AllOutputRest<Tail> : never;
    export type $OutputRest<Part> = Part extends unknown ? string extends keyof Part ? Part[string] : never : never;
    export type $InputWithRest<
    	Known,
    	Rest
    > = unknown extends Rest ? Known : Known extends {
    	[key: string]: Rest;
    } ? Known & {
    	[key: string]: Rest;
    } : Known;
    export type $AllInputRest<Parts> = Parts extends [infer Part, ...infer Tail] ? $InputRest<$Flattened<Part>> & $AllInputRest<Tail> : unknown;
    export type $InputRest<Part> = Part extends unknown ? string extends keyof Part ? Part[string] : unknown : never;
    export namespace serde_json.value {
    	export type Value = null | boolean | number | string | $.serde_json.value.Value[] | {
    		[key: string]: $.serde_json.value.Value;
    	};
    	export namespace Value {
    		export type Input = null | boolean | number | string | $.serde_json.value.Value.Input[] | {
    			[key: string]: $.serde_json.value.Value.Input;
    		};
    	}
    }
    export namespace crate {
    	/** A boost the mine is under. */
    	export type Boost = {
    		/** How many times faster it digs. */
    		rate: number;
    	};
    	export namespace Buy {
    		/** Buys generators. */
    		export type Input = $.crate.Generator.Input | {
    			generator: $.crate.Generator.Input;
    			/** How many beyond the first, none when absent. */
    			extra?: number;
    			/** A note for the shop, kept as sent. */
    			note: $.serde_json.value.Value.Input;
    		} | $.crate.Shipment.Input<$.crate.shop.Generator.Input>;
    	}
    	/** A machine that makes ore. */
    	export type Generator = "drill" | "furnace";
    	export namespace Generator {
    		/** A machine that makes ore. */
    		export type Input = "drill" | "furnace";
    	}
    	/** An id for one kind of thing. */
    	export type Id = {
    		raw: number;
    		kind: null;
    	};
    	/**
    	* The mine, after a tick.
    	*
    	* Sent once per tick.
    	*/
    	export type Mine = $Output<{
    		/** Ore on hand, by kind. */
    		ore: {
    			[key: string]: number;
    		};
    		/** The last hauls, newest first. */
    		hauls: [$.crate.Generator, number | null][];
    		/** What the shop sells. */
    		for_sale: $.crate.shop.Generator[];
    		/** The level below, dug on a `*\/5` schedule. */
    		deeper: $.crate.Mine | null;
    		running: boolean;
    		/** The last survey, as the surveyor wrote it. */
    		survey: $.serde_json.value.Value;
    		"gift-note": string;
    		/** What's on its way. */
    		incoming: $.crate.Shipment<$.crate.shop.Generator>[];
    		/** The generator that dug last. */
    		last_digger: $.crate.Id;
    	}, [$.crate.Boost | null, $.crate.Weather]>;
    	/** Something on its way to the mine. */
    	export type Shipment<T> = $Output<{
    		/** Ticks until it arrives. */
    		eta: number;
    		/** The shipment after it. */
    		next: $.crate.Shipment<T> | null;
    	}, [T]>;
    	export namespace Shipment {
    		/** Something on its way to the mine. */
    		export type Input<T> = $Input<{
    			/** Ticks until it arrives. */
    			eta: number;
    			/** The shipment after it. */
    			next?: $.crate.Shipment.Input<T> | null;
    		}, [T]>;
    	}
    	export type Unreachable = never;
    	/** The weather over the mine. */
    	export type Weather = "Clear" | {
    		Storm: {
    			strength: number;
    		};
    	};
    }
    export namespace crate.shop {
    	/** A generator for sale. */
    	export type Generator = {
    		/** What it costs, in ore. */
    		price: number;
    	};
    	export namespace Generator {
    		/** A generator for sale. */
    		export type Input = {
    			/** What it costs, in ore. */
    			price: number;
    		};
    	}
    }
    "#);
}
