//! Taba's generated files, written from the repo root.

use std::{
    fs,
    io::{self, Error},
    path::Path,
    process::Command,
};

use taba::{
    __private::rpc::Batch,
    rpc::{Frame, SessionId},
};
use taba_build::{Declarations, bundle};

fn main() -> io::Result<()> {
    let wire = Declarations::new("taba").output::<Frame>().input::<Batch>().both::<SessionId>();
    let injected = bundle("crates/taba-tauri/src/injected.ts").map_err(Error::other)?;
    let files = [
        ("crates/taba-tauri/src/injected.js", injected),
        ("packages/rpc/src/taba.ts", wire.to_string()),
    ];

    for (path, contents) in &files {
        write(Path::new(path), contents)?;
    }
    let attributes = files.map(|(path, _)| format!("{path} linguist-generated\n")).concat();
    fs::write(".gitattributes", format!("# written by `cargo xtask generate`\n{attributes}"))
}

/// Writes formatted `contents` to `path`.
fn write(path: &Path, contents: &str) -> io::Result<()> {
    fs::write(path, contents)?;
    let mut oxfmt = Command::new("pnpm");
    oxfmt.args(["--silent", "run", "fmt"]).arg(path);
    let status = oxfmt.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::other(format!("{oxfmt:?} failed with {status}")))
    }
}
