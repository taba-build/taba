//! Declarations for an idle game's types.
#![expect(dead_code, reason = "the types exist for their shapes")]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
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
struct Mine {
    /// Ore on hand, by kind.
    ore: BTreeMap<String, u32>,

    /// The last hauls, newest first.
    hauls: Vec<(Generator, f64)>,

    /// What the shop sells.
    for_sale: Vec<shop::Generator>,

    /// The level below, dug on a `*/5` schedule.
    deeper: Option<Box<Mine>>,

    running: bool,

    #[serde(rename = "gift-note")]
    gift_note: String,
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
    },
}

#[test]
fn every_shape_kind() {
    let declarations =
        Declarations::new("typescript").output::<Mine>().output::<Unreachable>().input::<Buy>();
    insta::assert_snapshot!(declarations, @r#"
    import type * as $ from "./taba.js";
    export namespace crate {
    	export namespace Buy {
    		/** Buys generators. */
    		export type Input = $.crate.Generator.Input | {
    			generator: $.crate.Generator.Input;
    			/** How many beyond the first, none when absent. */
    			extra?: number;
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
    		"gift-note": string;
    	};
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
