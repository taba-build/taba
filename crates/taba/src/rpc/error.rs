use core::fmt;

/// Failed to answer a send or poll.
#[derive(Debug)]
pub struct Error(Problem);

#[derive(Debug)]
enum Problem {
    Ended,
    Malformed(serde_json::Error),
}

impl Error {
    /// The request's session has ended, or isn't at its address.
    pub(crate) fn ended() -> Self {
        Self(Problem::Ended)
    }

    /// A body that isn't taba's.
    pub(crate) fn malformed(error: serde_json::Error) -> Self {
        Self(Problem::Malformed(error))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Problem::Ended => f.write_str("this session has ended"),
            Problem::Malformed(error) => write!(f, "can't read what was sent: {error}"),
        }
    }
}

impl core::error::Error for Error {}
