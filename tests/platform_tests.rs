//! Integration tests for the platform layer.
//!
//! These tests verify that the platform abstractions are correctly wired up.
//! We cannot create actual native or TUI windows in test mode, so we test
//! enums and the host trait surface only.

use abrash::platform::{Event, WindowError};
#[cfg(feature = "backend-winit")]
use abrash::platform::{WindowApp, WindowContext, WindowHostConfig};

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

// ---------- Winit host surface ----------

#[cfg(feature = "backend-winit")]
#[derive(Default)]
struct MockApp;

#[cfg(feature = "backend-winit")]
impl WindowApp for MockApp {
    type Error = std::convert::Infallible;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig::default()
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(feature = "backend-winit")]
#[test]
fn window_host_config_default_values() {
    let config = WindowHostConfig::default();
    assert_eq!(config.title, "Abrash");
    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 720);
    assert!(config.vsync);
}

#[cfg(feature = "backend-winit")]
#[test]
fn window_app_trait_surface_compiles() {
    let app = MockApp;
    let config = app.config();
    assert_eq!(config, WindowHostConfig::default());
}
