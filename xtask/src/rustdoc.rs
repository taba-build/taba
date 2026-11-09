//! [rustdoc](https://doc.rust-lang.org/rustdoc/) builds the API reference into the site.

use std::{
    fs,
    io::{self, Error},
    process::Command,
};

const WORKSPACE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const TARGET_DOC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/doc");
const SITE_BOOK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../site/book");

/// Builds the API reference into `site/book`, failing on any warning.
///
/// The default members' public items go in `api/`, and every item of the
/// workspace in `private/`. Runs after [`mdbook::build`](crate::mdbook::build),
/// which empties `site/book`.
pub(crate) fn build() -> io::Result<()> {
    doc("api", &[])?;
    doc("private", &["--workspace", "--document-private-items"])
}

fn doc(dir: &str, args: &[&str]) -> io::Result<()> {
    // rustdoc lists and indexes every crate already in `target/doc`
    let mut clean = Command::new("cargo");
    clean.args(["clean", "--doc", "--locked"]);
    crate::run(clean)?;

    let mut doc = Command::new("cargo");
    doc.args(["doc", "--no-deps", "--all-features", "--locked"]).args(args);
    doc.env("RUSTDOCFLAGS", "-D warnings");
    doc.current_dir(WORKSPACE); // no package flag means the default members only here
    crate::run(doc)?;

    let to = format!("{SITE_BOOK}/{dir}");
    fs::rename(TARGET_DOC, &to)
        .map_err(|error| Error::other(format!("moving {TARGET_DOC} to {to}: {error}")))
}
