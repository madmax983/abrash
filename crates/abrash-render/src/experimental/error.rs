use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    CapacityExceeded(&'static str),
    MeshIndexOutOfBounds(&'static str),
    MessageTooLarge(&'static str),
    FramebufferTooSmall(&'static str),
    StackOverflow(&'static str),
    StackUnderflow(&'static str),
    General(&'static str),
    GeneralString(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded(msg) => write!(f, "Capacity exceeded: {msg}"),
            Self::MeshIndexOutOfBounds(msg) => write!(f, "Mesh index out of bounds: {msg}"),
            Self::MessageTooLarge(msg) => write!(f, "Message too large: {msg}"),
            Self::FramebufferTooSmall(msg) => write!(f, "Framebuffer too small: {msg}"),
            Self::StackOverflow(msg) => write!(f, "Stack overflow: {msg}"),
            Self::StackUnderflow(msg) => write!(f, "Stack underflow: {msg}"),
            Self::General(msg) => write!(f, "{msg}"),
            Self::GeneralString(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {}
