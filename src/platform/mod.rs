use std::fmt;

#[derive(Debug, Clone)]
pub enum Event {
    Close,
    Resize(u32, u32),
}

#[derive(Debug)]
pub enum WindowError {
    RegistrationFailed,
    CreationFailed,
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowError::RegistrationFailed => write!(f, "Failed to register window class"),
            WindowError::CreationFailed => write!(f, "Failed to create window"),
        }
    }
}

impl std::error::Error for WindowError {}

#[cfg(target_os = "windows")]
pub mod win32;

#[cfg(target_os = "windows")]
pub use win32::Window;

#[cfg(not(target_os = "windows"))]
pub mod tui;

#[cfg(not(target_os = "windows"))]
pub use tui::Window;
