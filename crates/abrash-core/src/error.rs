//! Core error types for the Abrash engine.

use std::fmt;

/// The central error type for all operations in the `abrash-core` crate.
#[derive(Debug)]
pub enum CoreError {
    /// A required capacity was exceeded (e.g., parsing too many vertices from an OBJ).
    CapacityExceeded(String),
    /// Dimension or bounds constraints were violated (e.g., width > `i32::MAX` or 0).
    InvalidDimensions(&'static str),
    /// Invalid coordinate data.
    InvalidCoordinate(String),
    /// Input data format was invalid (e.g., malformed OBJ face).
    InvalidFormat(String),
    /// General underlying IO error.
    Io(std::io::Error),
    /// Misc string based errors to convert old API.
    Message(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded(msg) => write!(f, "capacity exceeded: {msg}"),
            Self::InvalidDimensions(msg) => write!(f, "invalid dimensions: {msg}"),
            Self::InvalidCoordinate(msg) => write!(f, "invalid coordinate: {msg}"),
            Self::InvalidFormat(msg) => write!(f, "invalid format: {msg}"),
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}
