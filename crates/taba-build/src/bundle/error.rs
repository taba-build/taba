use std::{
    fmt, io,
    path::{Path, PathBuf},
};

use taba_rolldown::rolldown_error::BuildDiagnostic;

/// Failed to bundle a module.
#[derive(Debug)]
pub struct Error(Kind);

#[derive(Debug)]
enum Kind {
    NotUtf8(PathBuf),
    Runtime(io::Error),
    Bundle { module: PathBuf, diagnostics: Vec<BuildDiagnostic> },
}

impl Error {
    pub(crate) fn not_utf8(module: &Path) -> Self {
        Self(Kind::NotUtf8(module.to_path_buf()))
    }

    pub(crate) fn runtime(error: io::Error) -> Self {
        Self(Kind::Runtime(error))
    }

    pub(crate) fn bundle(module: &Path, diagnostics: Vec<BuildDiagnostic>) -> Self {
        Self(Kind::Bundle { module: module.to_path_buf(), diagnostics })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Kind::NotUtf8(module) => {
                write!(f, "failed bundle of {}: rolldown reads only UTF-8 paths", module.display())
            }
            Kind::Runtime(error) => write!(f, "failed async runtime start: {error}"),
            Kind::Bundle { module, diagnostics } => {
                write!(f, "failed bundle of {}:", module.display())?;
                diagnostics.iter().try_for_each(|diagnostic| write!(f, "\n{diagnostic}"))
            }
        }
    }
}

impl std::error::Error for Error {}
