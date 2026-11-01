//! Taba's build-time bundler

mod error;

use std::path::Path;

use taba_rolldown::{
    rolldown::{Bundler, BundlerOptions, CodeSplittingMode, InputItem, OutputFormat},
    rolldown_common::{AddonOutputOption, Output},
    rolldown_error::EventKind,
};

pub use crate::error::Error;

/// Alias for a `Result` with the error type `taba_build::Error`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Bundles `module` and its imports into one script, the bundle `taba::script` runs.
///
/// The module reads its input from the global `_taba`. Like any ES module, it runs in strict mode.
///
/// # Errors
///
/// When `module` isn't a UTF-8 path, the async runtime rolldown runs on can't start, or rolldown
/// fails or can't resolve an import.
pub fn bundle(module: impl AsRef<Path>) -> Result<String> {
    bundle_path(module.as_ref())
}

fn bundle_path(module: &Path) -> Result<String> {
    let import = module.to_str().ok_or_else(|| Error::not_utf8(module))?;
    let options = BundlerOptions {
        input: Some(vec![InputItem { name: None, import: import.to_owned() }]),
        format: Some(OutputFormat::Iife),
        code_splitting: Some(CodeSplittingMode::Bool(false)),
        // Rolldown's `strict` goes above the IIFE, but we need it inside to keep it in effect when
        // included as part of a larger script.
        intro: Some(AddonOutputOption::String(Some(r#""use strict";"#.to_owned()))),
        ..BundlerOptions::default()
    };
    let failed = |diagnostics| Error::bundle(module, diagnostics);

    let runtime = tokio::runtime::Builder::new_current_thread().build().map_err(Error::runtime)?;
    let mut bundler = Bundler::new(options).map_err(|errors| failed(errors.into_vec()))?;
    let output =
        runtime.block_on(bundler.generate()).map_err(|errors| failed(errors.into_vec()))?;

    // rolldown leaves an import it can't resolve as a global.
    let unresolved: Vec<_> = output
        .warnings
        .into_iter()
        .filter(|warning| matches!(warning.kind(), EventKind::UnresolvedImport))
        .collect();
    if !unresolved.is_empty() {
        return Err(failed(unresolved));
    }
    // An empty chunk is a script that does nothing.
    let code = output.assets.into_iter().find_map(|asset| match asset {
        Output::Chunk(chunk) => Some(chunk.code.clone()),
        Output::Asset(_) => None,
    });
    Ok(code.unwrap_or_default())
}
