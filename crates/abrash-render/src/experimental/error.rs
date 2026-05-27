use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    CapacityExceeded,
    MeshIndexOutOfBounds,
    Utf8Error,
    MessageTooLarge,
    StackOverflow,
    StackUnderflow,
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded => write!(f, "Capacity exceeded"),
            Self::MeshIndexOutOfBounds => write!(f, "Mesh index out of bounds"),
            Self::Utf8Error => write!(f, "UTF-8 decoding error"),
            Self::MessageTooLarge => write!(f, "Message too large to process/encode"),
            Self::StackOverflow => write!(f, "Stack overflow"),
            Self::StackUnderflow => write!(f, "Stack underflow"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {}
