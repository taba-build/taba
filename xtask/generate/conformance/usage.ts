// What an app writes against the client generate writes, which tsc checks like the cases.
// Each `// ^` says where go to definition takes the name above it, which `cargo xtask lint` checks.
import { buy, ticks } from "./client/game/index.js";
import { prices, sell } from "./client/game/shop/index.js";
import type { generate } from "./client/index.js";

export async function play(signal: AbortSignal): Promise<generate.game.Cart> {
  //                                                                   ^ ../src/game.rs struct [Cart]
  for await (const mine of ticks(null, signal)) {
    if (mine.running) break;
  }
  for await (const price of prices(null, signal)) {
    if (price.ok) console.log(price.value);
  }
  const stock: generate.game.shop.Generator.Input = { price: 40 };
  //                         ^ client/index.d.ts declare namespace generate.game.[shop]
  //                              ^ ../src/game.rs pub(super) struct [Generator]
  //                              ^ ../src/game.rs pub(super) struct [Generator]
  //                                                  ^ ../src/game.rs pub(super) [price]
  const sold = await sell(stock);
  //                 ^ ../src/game.rs pub(super) struct [Sell]
  if (sold.ok) console.log(sold.value);
  // @ts-expect-error a fallible call's value needs its ok checked
  console.log(sold.value);
  // @ts-expect-error serde_json can't read a shovel as a generator
  await buy("shovel");
  // @ts-expect-error a subscription needs the signal that ends it
  ticks(null);
  const order: generate.game.Buy.Input = "drill";
  //                         ^ ../src/game.rs enum [Buy]
  //                             ^ ../src/game.rs enum [Buy]
  return buy(order);
  //     ^ ../src/game.rs enum [Buy]
}

export function read(cart: generate.game.Cart, mine: generate.game.Mine) {
  return [
    cart.pulled_by,
    //   ^ ../src/game.rs [pulled_by]
    mine["gift-note"],
    //    ^ ../src/game.rs [gift_note]
    mine.for_sale[0]?.price,
    //                ^ ../src/game.rs pub(super) [price]
    "rate" in mine && mine.rate,
    //                     ^ ../src/game.rs [rate]
    "Storm" in mine && mine.Storm.strength,
    //                      ^ ../src/game.rs [Storm]
    //                            ^ ../src/game.rs Storm { [strength]
  ];
}
