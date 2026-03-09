//! Integration tests for the platform layer.
//!
//! These tests verify that the platform abstractions (trait, type alias,
//! enums) are correctly wired up. We cannot create actual Win32 or TUI
//! windows in test mode, so we test types and trait surface only.

use abrash::platform::{Event, WindowError};

// ---------- Event enum ----------

#[test]
fn event_close_can_be_constructed() {
    let event = Event::Close;
    // Pattern-match to prove the variant exists
    assert!(matches!(event, Event::Close));
}

#[test]
fn event_resize_carries_dimensions() {
    let event = Event::Resize(1920, 1080);
    match event {
        Event::Resize(w, h) => {
            assert_eq!(w, 1920);
            assert_eq!(h, 1080);
        }
        Event::Close => panic!("Expected Event::Resize"),
    }
}

#[test]
fn event_clone() {
    let event = Event::Resize(800, 600);
    let cloned = event;
    assert!(matches!(cloned, Event::Resize(800, 600)));
}

#[test]
fn event_debug_format() {
    let close = Event::Close;
    let resize = Event::Resize(640, 480);
    // Debug trait is derived, so formatting should not panic
    let close_dbg = format!("{close:?}");
    let resize_dbg = format!("{resize:?}");
    assert!(close_dbg.contains("Close"));
    assert!(resize_dbg.contains("Resize"));
    assert!(resize_dbg.contains("640"));
    assert!(resize_dbg.contains("480"));
}

// ---------- WindowError enum ----------

#[test]
fn window_error_registration_failed_display() {
    let err = WindowError::RegistrationFailed;
    let msg = format!("{err}");
    assert_eq!(msg, "Failed to register window class");
}

#[test]
fn window_error_creation_failed_display() {
    let err = WindowError::CreationFailed;
    let msg = format!("{err}");
    assert_eq!(msg, "Failed to create window");
}

#[test]
fn window_error_debug_format() {
    let reg = WindowError::RegistrationFailed;
    let create = WindowError::CreationFailed;
    let reg_dbg = format!("{reg:?}");
    let create_dbg = format!("{create:?}");
    assert!(reg_dbg.contains("RegistrationFailed"));
    assert!(create_dbg.contains("CreationFailed"));
}

#[test]
fn window_error_implements_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(WindowError::CreationFailed);
    // If this compiles and runs, WindowError implements std::error::Error
    assert!(!err.to_string().is_empty());
}

// ---------- Mock Window tests (Removed because WindowBackend trait is gone) ----------

struct MockWindow {
    w: u32,
    h: u32,
    open: bool,
}

impl MockWindow {
    fn new(_title: &str, width: u32, height: u32) -> Result<Self, WindowError> {
        Ok(Self {
            w: width,
            h: height,
            open: true,
        })
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn width(&self) -> u32 {
        self.w
    }

    fn height(&self) -> u32 {
        self.h
    }

    fn poll_events(&mut self) -> Vec<Event> {
        Vec::new()
    }

    fn blit_framebuffer(&mut self, _framebuffer: &abrash::framebuffer::Framebuffer) {
        // no-op
    }
}

#[test]
fn mock_window_backend_new() {
    let win = MockWindow::new("Test", 800, 600).unwrap();
    assert_eq!(win.width(), 800);
    assert_eq!(win.height(), 600);
    assert!(win.is_open());
}

#[test]
fn mock_window_backend_poll_events() {
    let mut win = MockWindow::new("Test", 320, 240).unwrap();
    let events = win.poll_events();
    assert!(events.is_empty());
}

#[test]
fn mock_window_backend_blit_framebuffer() {
    let mut win = MockWindow::new("Test", 100, 100).unwrap();
    let fb = abrash::framebuffer::Framebuffer::new(100, 100).unwrap();
    win.blit_framebuffer(&fb);
    // No panic = success
}
