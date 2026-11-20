# Tauri

[`taba-tauri`] carries Taba's RPC over Tauri's `invoke`.

A handler answers a procedure, here the introduction's `AddTodo`:

```rust
use taba::Never;

async fn add_todo(AddTodo(todo): AddTodo) -> Result<Todo, Never> {
    Ok(todo)
}
```

A [`Router`] gives each procedure its handler:

```rust
use taba::rpc::Router;

let procedures = taba::procedures!().declare::<AddTodo>();
let router = Router::builder([procedures]).handle(add_todo).build()?;
```

`.taba(router)` takes the place of `.invoke_handler(...)`:

```rust
use taba_tauri::BuilderExt as _;

tauri::Builder::default()
    .taba(router)
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

An app with commands already passes them as the fallback, and moves them over one at a time:

```rust
tauri::Builder::default()
    .taba_with_fallback(router, tauri::generate_handler![greet])
```

[`taba-tauri`]: api/taba_tauri/
[`Router`]: api/taba/rpc/struct.Router.html
