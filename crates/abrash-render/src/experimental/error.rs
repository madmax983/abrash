use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    CapacityExceeded(&'static str),
    MeshIndexOutOfBounds(usize, usize),
    InvalidInput(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded(msg) => write!(f, "Capacity exceeded: {msg}"),
            Self::MeshIndexOutOfBounds(idx, max) => write!(f, "Mesh index {idx} out of bounds (max {max})"),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
