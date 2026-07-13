//! Error handling for experimental modules.

use std::fmt;

/// Unified error type for experimental modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Memory or buffer limit exceeded.
    CapacityExceeded(String),
    /// Data layout or index is invalid.
    InvalidData(String),
    /// Simulation or stack constraints violated.
    ConstraintViolated(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded(msg) => write!(f, "Capacity Exceeded: {msg}"),
            Self::InvalidData(msg) => write!(f, "Invalid Data: {msg}"),
            Self::ConstraintViolated(msg) => write!(f, "Constraint Violated: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
