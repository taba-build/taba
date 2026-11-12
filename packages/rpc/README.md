# @taba-build/rpc

The TypeScript side of [Taba](https://github.com/taba-build/taba)'s RPC. It calls a Rust app's
procedures by name.

```sh
pnpm add @taba-build/rpc
```

```ts
import { call, subscribe } from "@taba-build/rpc";

const drills = await call<number>("game::Buy", { generator: "Drill" });

const controller = new AbortController();
for await (const ore of subscribe<number>("game::Ore", null, controller.signal)) {
  console.log(ore);
}
```

`tryCall` and `trySubscribe` do the same for a procedure that can fail, and wrap each answer in a
`Result`.

```ts
export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };
```

TypeScript needs a session from a Taba backend, such as the
[`taba-tauri`](https://crates.io/crates/taba-tauri) crate in a Tauri app. Without one, each function
throws "taba can't find its session".

---

<sup>Licensed under either of <a href="LICENSE-APACHE">Apache License, Version 2.0</a> or
<a href="LICENSE-MIT">MIT license</a> at your option.</sup> <br /> <sub>Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined
in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or
conditions.</sub>
