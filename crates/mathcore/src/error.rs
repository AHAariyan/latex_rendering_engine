//! Error type shared by the parser, font layer and layout engine.

use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The TeX source could not be parsed. `pos` is a byte offset into the source.
    Parse { pos: usize, msg: String },
    /// The font could not be loaded or lacks a required table.
    Font(String),
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
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = core::result::Result<T, Error>;
