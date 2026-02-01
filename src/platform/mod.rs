#[cfg(target_os = "windows")]
pub mod win32;

#[cfg(target_os = "windows")]
pub use win32::Window;

#[cfg(not(target_os = "windows"))]
pub mod dummy;

#[cfg(not(target_os = "windows"))]
pub use dummy::Window;

/// Window events
#[derive(Debug, Clone)]
pub enum Event {
    Close,
    Resize(u32, u32),
}

/// Error types for window operations
#[derive(Debug)]
pub enum WindowError {
    RegistrationFailed,
    CreationFailed,
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowError::RegistrationFailed => write!(f, "Failed to register window class"),
            WindowError::CreationFailed => write!(f, "Failed to create window"),
        }
    }
}

impl std::error::Error for WindowError {}
