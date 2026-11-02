//! [`pnpm`] handles all Node tooling, through the scripts in each `package.json`.
//!
//! [`pnpm`]: https://pnpm.io/

use std::{
    io::{self, Error, ErrorKind},
    process::Command,
};

fn pnpm() -> Command {
    let mut pnpm = Command::new("pnpm");
    pnpm.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    pnpm
}

fn pnpm_missing(error: Error) -> Error {
    match error.kind() {
        ErrorKind::NotFound => Error::other("needs pnpm (https://pnpm.io/installation)"),
        _ => error,
    }
}

/// `pnpm run` from any cwd, failing on a stale `pnpm-lock.yaml`.
fn script(filter: Option<&str>, name: &str, args: &[&str]) -> io::Result<()> {
    let mut install = pnpm();
    install.args(["install", "--frozen-lockfile", "--silent"]);
    crate::run(install).map_err(pnpm_missing)?;

    let mut script = pnpm();
    script.arg("--silent");
    if let Some(filter) = filter {
        script.args(["--filter", filter]);
    }
    script.args(["run", name]).args(args);
    crate::run(script)
}

/// A script of the root package.
pub(crate) fn run(name: &str, args: &[&str]) -> io::Result<()> {
    script(None, name, args)
}

/// A script of the `xtask` package.
pub(crate) fn xtask(name: &str, args: &[&str]) -> io::Result<()> {
    script(Some("xtask"), name, args)
}
