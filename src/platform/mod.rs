use std::fmt;

use crate::framebuffer::Framebuffer;

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

/// Trait for native (synchronous loop) window backends.
pub trait WindowBackend {
    fn new(title: &str, width: u32, height: u32) -> Result<Self, WindowError>
    where
        Self: Sized;
    fn is_open(&self) -> bool;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn poll_events(&mut self) -> Vec<Event>;
    fn blit_framebuffer(&mut self, framebuffer: &Framebuffer);
}

// Shared half-block framebuffer widget (used by TUI and WASM backends)
#[cfg(any(feature = "backend-tui", feature = "backend-wasm"))]
pub mod framebuffer_widget;

// --- Backend modules ---

#[cfg(feature = "backend-win32")]
pub mod win32;

#[cfg(feature = "backend-win32")]
pub type Window = win32::Win32Window;

#[cfg(feature = "backend-tui")]
pub mod tui;

#[cfg(feature = "backend-tui")]
pub type Window = tui::TuiWindow;

#[cfg(feature = "backend-wasm")]
pub mod wasm;

// Compile-time check: at least one native backend must be selected (WASM has its own entry point)
#[cfg(not(any(
    feature = "backend-win32",
    feature = "backend-tui",
    feature = "backend-wasm"
)))]
compile_error!("No backend selected. Enable one of: backend-win32, backend-tui, backend-wasm");
