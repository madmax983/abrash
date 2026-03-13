
use std::fmt;

use crate::framebuffer::Framebuffer;

/// Represents an event from the windowing system.
#[derive(Debug, Clone)]
pub enum Event {
    /// Window close requested.
    Close,
    /// Window resized to the given width and height.
    Resize(u32, u32),
}

/// Error type for window creation failures.
#[derive(Debug)]
pub enum WindowError {
    /// Failed to register the window class with the OS.
    RegistrationFailed,
    /// Failed to create the window instance.
    CreationFailed,
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistrationFailed => write!(f, "Failed to register window class"),
            Self::CreationFailed => write!(f, "Failed to create window"),
        }
    }
}

impl std::error::Error for WindowError {}

/// Trait for native (synchronous loop) window backends.
pub trait WindowBackend {
    /// Creates a new window with the given title and dimensions.
    ///
    /// # Errors
    ///
    /// Returns a [`WindowError`] if window class registration or window creation fails.
    fn new(title: &str, width: u32, height: u32) -> Result<Self, WindowError>
    where
        Self: Sized;
    /// Returns whether the window is currently open and active.
    fn is_open(&self) -> bool;
    /// Returns the current client area width.
    fn width(&self) -> u32;
    /// Returns the current client area height.
    fn height(&self) -> u32;
    /// Polls for and returns a list of pending window events.
    fn poll_events(&mut self) -> Vec<Event>;
    /// Copies the framebuffer contents to the window display.
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

#[cfg(all(feature = "backend-tui", not(feature = "backend-win32")))]
/// The active Window backend type for the current build configuration.
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
