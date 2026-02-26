//! Platform abstraction layer for windowing and input.
//!
//! This module provides a unified interface for creating windows, handling events, and displaying
//! the framebuffer, regardless of the underlying operating system or backend.
//!
//! # The `WindowBackend` Trait
//!
//! The core of this abstraction is the [`WindowBackend`] trait. It defines the contract that any
//! windowing system must fulfill to work with Abrash:
//!
//! *   **Creation**: `new(title, width, height)` to open a window.
//! *   **Looping**: `poll_events()` to retrieve keyboard/mouse input and window events.
//! *   **Presentation**: `blit_framebuffer()` to copy the CPU-rendered buffer to the screen.
//!
//! # Available Backends
//!
//! Abrash supports multiple backends, selected via Cargo features:
//!
//! 1.  **Win32 (`backend-win32`)**: Native Windows API implementation. High performance, zero dependencies.
//!     Uses a raw `HWND` and GDI/bitmap blitting.
//! 2.  **TUI (`backend-tui`)**: Terminal User Interface using `ratatui` and `crossterm`.
//!     Renders the framebuffer using half-block characters (▀/▄). Works over SSH and on non-graphical environments.
//! 3.  **WASM (`backend-wasm`)**: WebAssembly backend for running in browsers.
//!     Uses HTML5 Canvas for display.
//!
//! # Backend Selection
//!
//! The `Window` type alias is conditionally defined based on enabled features.
//! If multiple backends are enabled, priority is typically: Win32 > TUI.
//!
//! ```toml
//! # Cargo.toml
//! [features]
//! default = ["backend-win32"]
//! ```

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

#[cfg(all(feature = "backend-tui", not(feature = "backend-win32")))]
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
