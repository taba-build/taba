//! Taba's generated files, written from the repo root.

mod game;

use std::{
    fs,
    io::{self, Error, Write},
    path::Path,
    process::{Command, Stdio},
    thread,
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
        ("xtask/generate/conformance/taba.d.ts", game::declarations().to_string()),
        ("xtask/generate/conformance/cases.ts", game::cases().to_string()),
    ];

    for (path, contents) in &files {
        let path = Path::new(path);
        write(path, &formatted(path, contents)?)?;
    }

    // written as an app writes its client, then formatted like the rest
    game::client().write().map_err(Error::other)?;
    format(Path::new(game::CLIENT))?;

    let client = format!("{}/**", game::CLIENT);
    let paths = files.iter().map(|(path, _)| *path).chain([client.as_str()]);
    let attributes: String = paths.map(|path| format!("{path} linguist-generated\n")).collect();
    let attributes = format!("# written by `cargo xtask generate`\n{attributes}");
    write(Path::new(".gitattributes"), attributes.as_bytes())
}

/// Formats the files under `directory` in place.
fn format(directory: &Path) -> io::Result<()> {
    let mut oxfmt = oxfmt();
    oxfmt.arg(directory);
    let status = oxfmt.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::other(format!("{oxfmt:?} failed with {status}")))
    }
}

/// Writes `contents` to `path` if they changed.
///
/// Cargo rebuilds on a new mtime, even with the same bytes.
fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    if fs::read(path).is_ok_and(|current| current == contents) {
        return Ok(());
    }
    fs::write(path, contents)
}

/// `contents` formatted by oxfmt for `path`.
fn formatted(path: &Path, contents: &str) -> io::Result<Vec<u8>> {
    let mut oxfmt = oxfmt();
    oxfmt.arg("--stdin-filepath").arg(path);
    let mut child = oxfmt.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    let mut stdin = child.stdin.take().expect("stdin is piped");

    // on its own thread, see https://doc.rust-lang.org/std/process/index.html#handling-io
    let (output, fed) = thread::scope(|scope| {
        let feed = scope.spawn(move || stdin.write_all(contents.as_bytes()));
        let output = child.wait_with_output();
        (output, feed.join().expect("writing stdin doesn't panic"))
    });

    let output = output?;
    if output.status.success() {
        fed?;
        Ok(output.stdout)
    } else {
        Err(Error::other(format!("{oxfmt:?} failed with {}", output.status)))
    }
}

/// The repo's oxfmt, through its `fmt` script.
fn oxfmt() -> Command {
    let mut oxfmt = Command::new("pnpm");
    oxfmt.args(["--silent", "run", "fmt"]);
    oxfmt
}
