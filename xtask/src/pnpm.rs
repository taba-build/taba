//! [`pnpm`] handles all Node tooling, through the scripts in the root `package.json`.
//!
//! [`pnpm`]: https://pnpm.io/

use std::{
    io::{self, Error, ErrorKind},
    process::Command,
};

const NEEDS_PNPM: &str = "needs pnpm (https://pnpm.io/installation)";

fn pnpm() -> Command {
    let mut pnpm = Command::new("pnpm");
    pnpm.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    pnpm
}

/// `pnpm run` from any cwd, failing on a stale `pnpm-lock.yaml`.
fn script(name: &str, args: &[&str]) -> io::Result<()> {
    let mut install = pnpm();
    install.args(["install", "--frozen-lockfile", "--silent"]);
    crate::run(install)?;

    let mut script = pnpm();
    script.args(["--silent", "run", name]).args(args);
    crate::run(script)
}

fn pnpm_missing(error: Error) -> Error {
    match error.kind() {
        ErrorKind::NotFound => Error::other(NEEDS_PNPM),
        _ => error,
    }
}

/// [`script()`], explaining a missing `pnpm`.
pub(crate) fn run(name: &str, args: &[&str]) -> io::Result<()> {
    script(name, args).map_err(pnpm_missing)
}

/// Ensure `pnpm` is available.
pub(crate) fn require() -> io::Result<()> {
    let mut version = pnpm();
    version.arg("--version");
    crate::run(version).map_err(pnpm_missing)
}

/// [`run()`], only if `pnpm` is available.
pub(crate) fn opt(name: &str, args: &[&str]) -> io::Result<()> {
    script(name, args).or_else(|error| match error.kind() {
        ErrorKind::NotFound => {
            eprintln!("xtask warning: skipping `{name}`: {NEEDS_PNPM}");
            Ok(())
        }
        _ => Err(error),
    })
}
