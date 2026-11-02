//! Taba's repo tasks

use std::{
    io::{self, Error},
    process::Command,
};

use clap::{Parser, Subcommand};

mod mdbook;
mod pnpm;

#[derive(Parser)]
enum Xtask {
    /// Run all formatters, or check them.
    Fmt {
        /// Check instead of write.
        #[arg(long)]
        check: bool,
    },

    /// Run all linters.
    Lint,

    /// Run all tests.
    Test,

    /// Run all checks.
    Ci,

    /// Pin taba-rolldown to a Rolldown release, or see which fit an MSRV.
    #[command(subcommand)]
    Rolldown(Rolldown),
}

#[derive(Subcommand)]
enum Rolldown {
    /// Pin all Rolldown crates to the same version, and related Oxc version.
    Pin {
        /// The Rolldown version, e.g. `1.2.5`.
        version: String,
    },

    /// List Rolldown releases with the Oxc each requires and that Oxc's MSRV.
    Msrv,
}

fn main() {
    let result = match Xtask::parse() {
        Xtask::Fmt { check } => fmt(check),
        Xtask::Lint => lint(),
        Xtask::Test => test(),
        Xtask::Ci => ci(),
        Xtask::Rolldown(Rolldown::Pin { version }) => pnpm::xtask("rolldown", &["pin", &version]),
        Xtask::Rolldown(Rolldown::Msrv) => pnpm::xtask("rolldown", &["msrv"]),
    };

    if let Err(error) = result {
        eprintln!("xtask error: {error}");
        std::process::exit(1);
    }
}

fn run(mut command: Command) -> io::Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::other(format!("{command:?} failed with {status}")))
    }
}

fn fmt(check: bool) -> io::Result<()> {
    let mut rustfmt = Command::new("cargo");
    rustfmt.arg("+nightly-2026-02-28"); // our MSRV's nightly, for nightly-only rustfmt settings
    rustfmt.args(["fmt", "--all"]); // otherwise only the current dir's crate
    let mut oxfmt = vec![];
    if check {
        rustfmt.arg("--check");
        oxfmt.push("--check");
    }

    run(rustfmt)?;
    pnpm::run("fmt", &oxfmt)
}

fn lint() -> io::Result<()> {
    let mut clippy = Command::new("cargo");
    clippy.args([
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ]);
    run(clippy)?;

    let mut no_std = Command::new("cargo");
    no_std.args(["check", "--package", "taba", "--target", "x86_64-unknown-none", "--locked"]);
    run(no_std)?;

    let mut doc = Command::new("cargo");
    doc.args(["doc", "--workspace", "--no-deps", "--all-features", "--locked"]);
    doc.env("RUSTDOCFLAGS", "-D warnings");
    run(doc)?;

    pnpm::run("lint", &[])?;
    pnpm::run("typecheck", &[])?;
    mdbook::build()
}

fn test() -> io::Result<()> {
    let mut test = Command::new("cargo");
    test.args(["test", "--workspace", "--all-features", "--locked"]);
    run(test)?;
    pnpm::run("test", &[])?;
    mdbook::test()
}

fn ci() -> io::Result<()> {
    fmt(true)?;
    lint()?;
    test()
}
