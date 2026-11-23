import type * as $ from "./taba.js";
// serde_json writes this for a `Mine`
export const case1: $.generate.game.Mine = {
  ore: { copper: 12 },
  hauls: [
    ["drill", 1.5],
    ["furnace", null],
  ],
  for_sale: [{ price: 40 }],
  deeper: {
    ore: {},
    hauls: [],
    for_sale: [],
    deeper: null,
    running: false,
    survey: null,
    "gift-note": "",
    Clear: null,
    incoming: [],
    last_digger: { raw: 0, kind: null },
  },
  running: true,
  survey: { depth: 3, veins: ["copper"] },
  "gift-note": "for the drill",
  rate: 2,
  Storm: { strength: 4 },
  incoming: [{ eta: 2, price: 40, next: { eta: 5, price: 60, next: null } }],
  last_digger: { raw: 1, kind: null },
};
// serde_json reads this as a `Buy`
export const case2: $.generate.game.Buy.Input = "drill";
// @ts-expect-error serde_json: data did not match any variant of untagged enum Buy
export const case3: $.generate.game.Buy.Input = "shovel";
// serde_json reads this as a `Buy`
export const case4: $.generate.game.Buy.Input = {
  generator: "furnace",
  note: { from: "the shop" },
};
// serde_json reads this as a `Buy`
export const case5: $.generate.game.Buy.Input = { generator: "furnace", extra: 2, note: null };
// @ts-expect-error serde_json: data did not match any variant of untagged enum Buy
export const case6: $.generate.game.Buy.Input = { generator: "furnace" };
// serde_json reads this as a `Buy`
export const case7: $.generate.game.Buy.Input = { eta: 3, price: 40, next: { eta: 1, price: 60 } };
// @ts-expect-error serde_json: data did not match any variant of untagged enum Buy
export const case8: $.generate.game.Buy.Input = { eta: 3 };
// serde_json writes this for a `Cart`
export const case9: $.generate.game.Cart = { pulled_by: "mule", copper: 3 };
// serde_json writes this for a `Cart`
export const case10: $.generate.game.Cart = { pulled_by: 7 };
// serde_json reads this as a `Cart`
export const case11: $.generate.game.Cart.Input = { pulled_by: "mule" };
// @ts-expect-error serde_json: invalid type: string "lots", expected u32 at line 1 column 36
export const case12: $.generate.game.Cart.Input = { pulled_by: "mule", copper: "lots" };
// serde_json writes this for a `Claim`
export const case13: $.generate.game.Claim = { owner: "ana", copper: "rich" };
// serde_json reads this as a `Claim`
export const case14: $.generate.game.Claim.Input = { owner: "ana" };
// @ts-expect-error serde_json: invalid type: integer `1`, expected a string at line 1 column 26
export const case15: $.generate.game.Claim.Input = { owner: "ana", copper: 1 };
// serde_json writes this for a `Train`
export const case16: $.generate.game.Train = { engine: 5, load: 2 };
// serde_json reads this as a `Train`
export const case17: $.generate.game.Train.Input = { engine: "steam", load: 2 };
// @ts-expect-error serde_json: invalid type: string "x", expected u32 at line 1 column 40
export const case18: $.generate.game.Train.Input = { engine: "steam", load: 2, copper: "x" };
// serde_json writes this for a `Stockpile`
export const case19: $.generate.game.Stockpile = { rate: 1, copper: 2 };
// serde_json reads this as a `Stockpile`
export const case20: $.generate.game.Stockpile.Input = { rate: 1, copper: 2 };
// @ts-expect-error serde_json: missing field `rate` at line 1 column 12
export const case21: $.generate.game.Stockpile.Input = { copper: 2 };
// serde_json writes this for a `Hold`
export const case22: $.generate.game.Hold = { owner: "ana" };
// serde_json writes this for a `Hold`
export const case23: $.generate.game.Hold = { owner: "ana", copper: 1 };
// serde_json reads this as a `Hold`
export const case24: $.generate.game.Hold.Input = { owner: "ana" };
// serde_json writes this for a `Notebook`
export const case25: $.generate.game.Notebook = { author: "ana", copper: "seen twice" };
// serde_json reads this as a `Notebook`
export const case26: $.generate.game.Notebook.Input = { author: "ana", copper: "seen", tin: [1] };
// @ts-expect-error serde_json: invalid type: integer `5`, expected a string at line 1 column 11
export const case27: $.generate.game.Notebook.Input = { author: 5 };
// serde_json writes this for a `Forecast`
export const case28: $.generate.game.Forecast = { tick: 1, Clear: null };
// serde_json writes this for a `Forecast`
export const case29: $.generate.game.Forecast = { tick: 1, Storm: { strength: 2 }, rate: 3 };
// serde_json reads this as a `Forecast`
export const case30: $.generate.game.Forecast.Input = { tick: 1, Clear: null };
// serde_json reads this as a `Forecast`
export const case31: $.generate.game.Forecast.Input = { tick: 1, Storm: { strength: 2 }, rate: 3 };
// @ts-expect-error serde_json: invalid type: string "Clear", expected unit at line 1 column 26
export const case32: $.generate.game.Forecast.Input = { tick: 1, Clear: "Clear" };
// serde_json writes this for a `Parcel`
export const case33: $.generate.game.Parcel<$.generate.game.Calm> = { weight: 1 };
// serde_json writes this for a `Parcel`
export const case34: $.generate.game.Parcel<$.generate.game.Weather> = { weight: 1, Clear: null };
// serde_json writes this for a `Parcel`
export const case35: $.generate.game.Parcel<$.generate.game.Boost | null> = { weight: 1 };
// serde_json writes this for a `Parcel`
export const case36: $.generate.game.Parcel<$.generate.game.Boost | null> = { weight: 1, rate: 3 };
// serde_json reads this as a `Parcel`
export const case37: $.generate.game.Parcel.Input<$.generate.game.Calm.Input> = { weight: 1 };
// serde_json reads this as a `Parcel`
export const case38: $.generate.game.Parcel.Input<$.generate.game.Weather.Input> = {
  weight: 1,
  Clear: null,
};
// serde_json reads this as a `Parcel`
export const case39: $.generate.game.Parcel.Input<$.generate.game.Boost.Input | null> = {
  weight: 1,
};
// serde_json writes this for a `Event`
export const case40: $.generate.game.Event = { kind: "Bought", price: 40 };
// serde_json writes this for a `Event`
export const case41: $.generate.game.Event = { kind: "Sold", copper: 3 };
// serde_json writes this for a `Event`
export const case42: $.generate.game.Event = { kind: 3 };
// serde_json writes this for a `Event`
export const case43: $.generate.game.Event = { kind: "Calmed" };
// serde_json writes this for a `Event`
export const case44: $.generate.game.Event = { kind: "Changed", Clear: null };
// serde_json reads this as a `Event`
export const case45: $.generate.game.Event.Input = { kind: "Bought", price: 40 };
// @ts-expect-error serde_json: missing field `price`
export const case46: $.generate.game.Event.Input = { kind: "Bought" };
// serde_json reads this as a `Event`
export const case47: $.generate.game.Event.Input = { kind: "Sold" };
// serde_json reads this as a `Event`
export const case48: $.generate.game.Event.Input = { kind: "Calmed" };
// serde_json reads this as a `Event`
export const case49: $.generate.game.Event.Input = { kind: "Changed", Clear: null };
// serde_json reads this as a `Event`
export const case50: $.generate.game.Event.Input = { kind: "Boosted", rate: 2 };
// @ts-expect-error serde_json: missing field `rate`
export const case51: $.generate.game.Event.Input = { kind: "Boosted" };
// @ts-expect-error serde_json: missing field `rate`
export const case52: $.generate.game.Event.Input = { kind: "Faded" };
// @ts-expect-error serde_json: missing field `rate`
export const case53: $.generate.game.Event.Input = { kind: "Stored" };
