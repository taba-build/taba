# Taba

{{#include ../../README.md:pitch}}

Taba parses serde attributes with serde's own parser, and `serde_json` does every read and write.
TypeScript names each type by its Rust path, so `crate::Todo` is `crate.Todo` and
`serde_json::value::Value` is `serde_json.value.Value`.

A serde type, as TypeScript sees it:

```rust
use serde::{Deserialize, Serialize};
use taba::{Input, Output};

#[derive(Serialize, Deserialize, Output, Input)]
struct Todo {
    #[serde(default)]
    done: bool,

    note: Option<String>,

    title: String,
}
```

`Todo` is what [`serde_json`] writes, and `Todo.Input` is what it reads:

```ts
import type { crate } from "./taba";

const todo: crate.Todo = { title: "ship docs", done: false, note: null };
const newTodo: crate.Todo.Input = { title: "ship docs" };
```

## RPC

Taba's RPC lets TypeScript call Rust. A procedure is a type it can call:

```rust
use serde::Deserialize;
use taba::{Input, Never, Procedure};

/// Adds a todo.
#[derive(Deserialize, Input)]
struct AddTodo(Todo);

impl Procedure for AddTodo {
    type Output = Todo;
    type Error = Never;
}
```

Taba generates a TypeScript function for it:

```ts
import { addTodo } from "./bindings";

const todo = await addTodo({ title: "ship docs" });
```

The RPC has no transport of its own. [`taba-tauri`](tauri.md) carries it over Tauri's `invoke` and
passes every command Taba doesn't answer to your existing handler, so an app moves over one command
at a time.

[`serde_json`]: https://docs.rs/serde_json
