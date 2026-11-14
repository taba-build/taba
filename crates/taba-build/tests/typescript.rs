//! Declarations for an idle game's types.
#![expect(dead_code, reason = "the types exist for their shapes")]

use std::collections::BTreeMap;

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
}

/// A boost the mine is under.
#[derive(Serialize, Output)]
struct Boost {
    /// How many times faster it digs.
    rate: u32,
}

#[derive(Serialize, Output)]
enum Unreachable {}

mod shop {
    use serde::Serialize;
    use taba::Output;

    /// A generator for sale.
    #[derive(Serialize, Output)]
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
}

#[test]
fn every_shape_kind() {
    let declarations =
        Declarations::new("typescript").output::<Mine>().output::<Unreachable>().input::<Buy>();
    insta::assert_snapshot!(declarations, @r#"
    import type * as $ from "./taba.js";
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
    		};
    	}
    	/** A machine that makes ore. */
    	export type Generator = "drill" | "furnace";
    	export namespace Generator {
    		/** A machine that makes ore. */
    		export type Input = "drill" | "furnace";
    	}
    	/**
    	* The mine, after a tick.
    	*
    	* Sent once per tick.
    	*/
    	export type Mine = {
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
    	} & (($.crate.Boost | {}) | {});
    	export type Unreachable = never;
    }
    export namespace crate.shop {
    	/** A generator for sale. */
    	export type Generator = {
    		/** What it costs, in ore. */
    		price: number;
    	};
    }
    "#);
}
