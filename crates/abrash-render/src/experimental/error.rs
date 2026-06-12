use core::fmt;

/// Centralized error type for experimental modules.
#[derive(Debug)]
pub enum Error {
    /// L-System expansion exceeded maximum capacity limit or memory limits.
    CapacityExceeded,

    /// L-System stack overflow or exceeded maximum stack depth.
    StackOverflow,

    /// L-System stack underflow.
    StackUnderflow,

    /// Spring indices or Mesh indices out of bounds in jelly physics.
    OutOfBounds(String),

    /// Framebuffer too small to hold the message in steganography.
    FramebufferTooSmall,

    /// Message payload too large to fit in steganography or memory limit.
    MessageTooLarge,

    /// Invalid UTF-8 sequence during text processing.
    Utf8DecodeError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded => write!(f, "L-System expansion exceeded maximum capacity limit"),
            Self::StackOverflow => write!(f, "L-System stack overflow"),
            Self::StackUnderflow => write!(f, "L-System stack underflow"),
            Self::OutOfBounds(msg) => write!(f, "{msg}"),
            Self::FramebufferTooSmall => write!(f, "Framebuffer too small to hold the message"),
            Self::MessageTooLarge => write!(f, "Message payload too large"),
            Self::Utf8DecodeError => write!(f, "Invalid UTF-8 sequence"),
        }
    }
}

impl std::error::Error for Error {}
