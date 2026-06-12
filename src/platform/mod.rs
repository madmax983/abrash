//! Platform abstraction layer.
//!
//! Provides unified windowing and event handling across different operating systems.

use std::fmt;

/// Events that can be emitted by the windowing system.
#[derive(Debug, Clone)]
pub enum Event {
    /// The window has been requested to close.
    Close,
    /// The window has been resized. Contains the new width and height.
    Resize(u32, u32),
}

/// Errors that can occur during window creation and registration.
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

// Shared half-block framebuffer widget (used by TUI and WASM backends)
#[cfg(any(feature = "backend-tui", feature = "backend-wasm"))]
pub mod framebuffer_widget;

// --- Backend modules ---

#[cfg(feature = "backend-winit")]
pub mod winit;

#[cfg(feature = "backend-winit")]
pub use winit::{
    FrameClock, HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig,
    print_error_and_exit, run_windowed,
};

#[cfg(feature = "backend-tui")]
pub mod tui;

#[cfg(feature = "backend-wasm")]
pub mod wasm;
