//! [mdBook](https://rust-lang.github.io/mdBook/) builds the site.

use std::{
    io::{self, Error, ErrorKind},
    process::Command,
};

/// `mdbook` in `site/`, printing only warnings and errors.
fn mdbook(command: &str) -> Command {
    let mut mdbook = Command::new("mdbook");
    mdbook
        .arg(command)
        .env("MDBOOK_LOG", "warn")
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../site"));
    mdbook
}

fn mdbook_missing(error: Error) -> Error {
    match error.kind() {
        ErrorKind::NotFound => Error::other("needs mdbook (https://rust-lang.github.io/mdBook/)"),
        _ => error,
    }
}

/// `mdbook build`, failing on any warning: mdbook exits 0 on most errors.
pub(crate) fn build() -> io::Result<()> {
    let mut build = mdbook("build");
    let output = build.output().map_err(mdbook_missing)?;
    if output.status.success() && output.stderr.is_empty() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(Error::other(format!("{build:?} failed:\n{stderr}")))
    }
}
