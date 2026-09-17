//! Error type shared by the parser, font layer and layout engine.

use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The TeX source could not be parsed. `pos` is a byte offset into the source.
    Parse { pos: usize, msg: String },
    /// The font could not be loaded or lacks a required table.
    Font(String),
    /// The formula exceeded a `Budget`. Hosts that render untrusted input rely
    /// on this instead of discovering the problem as an out-of-memory kill.
    TooLarge { what: &'static str, limit: usize },
}

impl Error {
    pub(crate) fn parse(pos: usize, msg: impl Into<String>) -> Self {
        Error::Parse { pos, msg: msg.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse { pos, msg } => write!(f, "parse error at byte {pos}: {msg}"),
            Error::Font(msg) => write!(f, "font error: {msg}"),
            Error::TooLarge { what, limit } => write!(f, "formula is too large: more than {limit} {what}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = core::result::Result<T, Error>;
