//! Platform abstraction layer.
//!
//! Provides unified windowing and event handling across different operating systems.

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
    run_windowed,
};

#[cfg(feature = "backend-tui")]
pub mod tui;

#[cfg(feature = "backend-wasm")]
pub mod wasm;
