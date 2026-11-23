use std::{
    fmt, io,
    path::{Path, PathBuf},
};

/// Failed to write a package's client.
#[derive(Debug)]
pub struct WriteError(Kind);

#[derive(Debug)]
enum Kind {
    Reserved { procedure: String, function: String },
    Unpaired { crate_: &'static str, directory: PathBuf },
    Unmarked(PathBuf),
    Io { path: PathBuf, error: io::Error },
    Transform { path: PathBuf, error: io::Error },
    Workspace(io::Error),
}

impl WriteError {
    pub(super) fn reserved(procedure: String, function: String) -> Self {
        Self(Kind::Reserved { procedure, function })
    }

    pub(super) fn unpaired(crate_: &'static str, directory: &Path) -> Self {
        Self(Kind::Unpaired { crate_, directory: directory.to_path_buf() })
    }

    pub(super) fn unmarked(path: &Path) -> Self {
        Self(Kind::Unmarked(path.to_path_buf()))
    }

    pub(super) fn io(path: &Path, error: io::Error) -> Self {
        Self(Kind::Io { path: path.to_path_buf(), error })
    }

    pub(super) fn transform(path: &Path, error: io::Error) -> Self {
        Self(Kind::Transform { path: path.to_path_buf(), error })
    }

    pub(super) fn workspace(error: io::Error) -> Self {
        Self(Kind::Workspace(error))
    }
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Kind::Reserved { procedure, function } => {
                write!(
                    f,
                    "failed client for `{procedure}`: `{function}` can't name a JavaScript function"
                )
            }
            Kind::Unpaired { crate_, directory } => {
                let directory = directory.display();
                write!(
                    f,
                    "failed client for `{crate_}` in {directory}: crates and directories don't pair"
                )
            }
            Kind::Unmarked(path) => {
                write!(f, "failed client write of {}: taba didn't write it", path.display())
            }
            Kind::Io { path, error } => {
                write!(f, "failed client write of {}: {error}", path.display())
            }
            Kind::Transform { path, error } => {
                write!(f, "failed client transform of {}: {error}", path.display())
            }
            Kind::Workspace(error) => write!(f, "failed workspace lookup for the client: {error}"),
        }
    }
}

impl std::error::Error for WriteError {}
