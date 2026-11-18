// What an app writes against the client generate writes, which tsc checks like the cases.
import { buy, ticks } from "./client/game/index.js";
import { prices, sell } from "./client/game/shop/index.js";
import type { crate } from "./client/index.js";

export async function play(signal: AbortSignal): Promise<crate.game.Cart> {
  for await (const mine of ticks(null, signal)) {
    if (mine.running) break;
  }
  for await (const price of prices(null, signal)) {
    if (price.ok) console.log(price.value);
  }
  const sold = await sell({ price: 40 });
  if (sold.ok) console.log(sold.value);
  // @ts-expect-error a fallible call's value needs its ok checked
  console.log(sold.value);
  // @ts-expect-error serde_json can't read a shovel as a generator
  await buy("shovel");
  // @ts-expect-error a subscription needs the signal that ends it
  ticks(null);
  return buy("drill");
}
