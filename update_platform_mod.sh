cat << 'INNER_EOF' > src/platform/mod.rs
//! Platform abstraction layer.
//!
//! Provides unified windowing and event handling across different operating systems.

use std::fmt;

// Shared half-block framebuffer widget (used by TUI and WASM backends)
#[cfg(any(feature = "backend-tui", feature = "backend-wasm"))]
pub mod framebuffer_widget;

// --- Backend modules ---

#[cfg(feature = "backend-winit")]
pub mod winit;

#[cfg(feature = "backend-winit")]
pub use self::winit::{
    Event, FrameClock, HostError, SoftwarePresenter, WindowApp, WindowContext, WindowError,
    WindowHostConfig, run_windowed,
};

#[cfg(feature = "backend-tui")]
pub mod tui;

#[cfg(feature = "backend-tui")]
pub use self::tui::{
    Event, FrameClock, HostError, SoftwarePresenter, WindowApp, WindowContext, WindowError,
    WindowHostConfig, run_windowed,
};

#[cfg(feature = "backend-wasm")]
pub mod wasm;
INNER_EOF
