//! Cross-platform native window host built on `winit`.
//!
//! The host is intentionally thin: it owns the event loop and window lifecycle,
//! then delegates behavior to a caller-provided [`WindowApp`]. CPU examples can
//! use [`SoftwarePresenter`] to blit a [`crate::framebuffer::Framebuffer`], while
//! GPU examples can keep their own `wgpu` state and render directly in `render()`.

use crate::framebuffer::Framebuffer;
use softbuffer::{Context as SoftbufferContext, Surface as SoftbufferSurface};
use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::window::{Window, WindowBuilder};

/// Static configuration for the host window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowHostConfig {
    /// Window title.
    pub title: String,
    /// Initial width in pixels.
    pub width: u32,
    /// Initial height in pixels.
    pub height: u32,
    /// Whether the caller prefers vsync.
    pub vsync: bool,
}

impl Default for WindowHostConfig {
    fn default() -> Self {
        Self {
            title: "Abrash".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }
}

/// Host-level platform errors.
#[derive(Debug)]
pub enum HostError {
    /// Failed to create or drive the event loop.
    EventLoop(String),
    /// Failed to create the native window.
    Window(String),
    /// An application callback returned an error.
    App(String),
    /// Failed to present pixels to the window.
    Present(String),
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventLoop(error) => write!(f, "event loop error: {error}"),
            Self::Window(error) => write!(f, "window error: {error}"),
            Self::App(error) => write!(f, "application error: {error}"),
            Self::Present(error) => write!(f, "present error: {error}"),
        }
    }
}

impl Error for HostError {}

/// Frame clock for deterministic `dt` updates.
#[derive(Debug)]
pub struct FrameClock {
    last_tick: Instant,
}

impl FrameClock {
    /// Create a new frame clock starting now.
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_tick: Instant::now(),
        }
    }

    /// Advance the clock and return elapsed seconds since the previous tick.
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let dt_seconds = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;
        dt_seconds
    }
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}

/// The contextual data passed to [`WindowApp`] callbacks.
#[derive(Clone)]
pub struct WindowContext<'a> {
    /// Event loop target for window lifecycle control.
    pub event_loop: &'a EventLoopWindowTarget<()>,
    /// The active window handle.
    pub window: Arc<Window>,
    /// Seconds since the previous redraw tick.
    pub dt_seconds: f32,
}

/// Trait implemented by callers that want to run inside the native host.
pub trait WindowApp {
    /// Concrete application error type.
    type Error: Error + Send + Sync + 'static;

    /// Static window configuration.
    fn config(&self) -> WindowHostConfig;

    /// One-time initialization after window creation.
    ///
    /// # Errors
    ///
    /// Returns an application-defined error if initialization fails.
    fn init(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Resize notification for non-zero size changes.
    ///
    /// # Errors
    ///
    /// Returns an application-defined error if resize handling fails.
    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        _width: u32,
        _height: u32,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Raw window input events.
    ///
    /// # Errors
    ///
    /// Returns an application-defined error if input handling fails.
    fn input(&mut self, _ctx: WindowContext<'_>, _event: &WindowEvent) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Update app state once per redraw.
    ///
    /// # Errors
    ///
    /// Returns an application-defined error if the update step fails.
    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error>;

    /// Render the current frame.
    ///
    /// # Errors
    ///
    /// Returns an application-defined error if rendering fails.
    fn render(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error>;
}

use comfy_table::{Cell, Color, Table, presets};

fn print_host_error_and_exit(err: HostError) -> ! {
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("❌ Window Application Error")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(Color::Red),
        ])
        .add_row(vec![
            Cell::new(format!("{err}")).fg(Color::Yellow),
        ]);

    eprintln!("\n{table}");
    std::process::exit(1);
}

/// Run an app inside a native desktop `winit` event loop.
///
/// Handles initialization and main event loop. Prints a formatted
/// error table and exits cleanly if any window or application error occurs.
pub fn run_windowed<A>(mut app: A)
where
    A: WindowApp,
{
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(error) => print_host_error_and_exit(HostError::EventLoop(error.to_string())),
    };
    let config = app.config();
    let window = Arc::new(match WindowBuilder::new()
        .with_title(config.title)
        .with_inner_size(winit::dpi::PhysicalSize::new(config.width, config.height))
        .build(&event_loop)
    {
        Ok(w) => w,
        Err(error) => print_host_error_and_exit(HostError::Window(error.to_string())),
    });

    if let Err(error) = app.init(WindowContext {
        event_loop: &event_loop,
        window: window.clone(),
        dt_seconds: 0.0,
    }) {
        print_host_error_and_exit(HostError::App(error.to_string()));
    }

    let last_error: Rc<RefCell<Option<HostError>>> = Rc::new(RefCell::new(None));
    let error_slot = last_error.clone();
    let window_for_loop = window;
    let mut clock = FrameClock::new();

    let event_loop_result = event_loop.run(move |event, event_loop_target| {
        event_loop_target.set_control_flow(ControlFlow::Poll);

        let context = WindowContext {
            event_loop: event_loop_target,
            window: window_for_loop.clone(),
            dt_seconds: 0.0,
        };

        match event {
            Event::WindowEvent { event, window_id } if window_id == window_for_loop.id() => {
                match event {
                    WindowEvent::CloseRequested => event_loop_target.exit(),
                    WindowEvent::Resized(size) => {
                        if size.width != 0 && size.height != 0 {
                            if let Err(error) = app.resize(
                                WindowContext {
                                    dt_seconds: 0.0,
                                    ..context.clone()
                                },
                                size.width,
                                size.height,
                            ) {
                                *error_slot.borrow_mut() =
                                    Some(HostError::App(error.to_string()));
                                event_loop_target.exit();
                            }
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        let dt_seconds = clock.tick();
                        let redraw_context = WindowContext {
                            dt_seconds,
                            ..context.clone()
                        };
                        if let Err(error) = app.update(redraw_context.clone()) {
                            *error_slot.borrow_mut() = Some(HostError::App(error.to_string()));
                            event_loop_target.exit();
                        } else if let Err(error) = app.render(redraw_context) {
                            *error_slot.borrow_mut() = Some(HostError::App(error.to_string()));
                            event_loop_target.exit();
                        }
                    }
                    other => {
                        if let Err(error) = app.input(context, &other) {
                            *error_slot.borrow_mut() = Some(HostError::App(error.to_string()));
                            event_loop_target.exit();
                        }
                    }
                }
            }
            Event::AboutToWait => {
                window_for_loop.request_redraw();
            }
            _ => {}
        }
    });

    if let Err(error) = event_loop_result {
        print_host_error_and_exit(HostError::EventLoop(error.to_string()));
    }

    if let Some(err) = last_error.take() {
        print_host_error_and_exit(err);
    }
}

/// Cross-platform presenter for software-rendered framebuffers.
pub struct SoftwarePresenter {
    surface: SoftbufferSurface<Arc<Window>, Arc<Window>>,
}

impl SoftwarePresenter {
    /// Create a presenter for the given window.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform surface cannot be created.
    pub fn new(window: Arc<Window>) -> Result<Self, HostError> {
        let context = SoftbufferContext::new(window.clone())
            .map_err(|error| HostError::Present(error.to_string()))?;
        let surface = SoftbufferSurface::new(&context, window)
            .map_err(|error| HostError::Present(error.to_string()))?;
        Ok(Self { surface })
    }

    /// Present a framebuffer into the window.
    ///
    /// # Errors
    ///
    /// Returns an error if the surface cannot be resized or presented.
    pub fn present(&mut self, framebuffer: &Framebuffer) -> Result<(), HostError> {
        let width = NonZeroU32::new(framebuffer.width())
            .ok_or_else(|| HostError::Present("framebuffer width must be non-zero".to_string()))?;
        let height = NonZeroU32::new(framebuffer.height())
            .ok_or_else(|| HostError::Present("framebuffer height must be non-zero".to_string()))?;

        self.surface
            .resize(width, height)
            .map_err(|error| HostError::Present(error.to_string()))?;

        let mut buffer = self
            .surface
            .buffer_mut()
            .map_err(|error| HostError::Present(error.to_string()))?;

        for (dst, src) in buffer.iter_mut().zip(framebuffer.as_slice()) {
            *dst = argb_to_softbuffer(*src);
        }

        buffer
            .present()
            .map_err(|error| HostError::Present(error.to_string()))
    }
}

const fn argb_to_softbuffer(argb: u32) -> u32 {
    argb & 0x00FF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_host_config_default() {
        let config = WindowHostConfig::default();
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
        assert!(config.vsync);
    }

    #[test]
    fn test_frame_clock_dt_is_non_negative() {
        let mut clock = FrameClock::new();
        let dt_seconds = clock.tick();
        assert!(dt_seconds >= 0.0);
    }

    #[test]
    fn test_argb_to_softbuffer_pixel() {
        assert_eq!(argb_to_softbuffer(0xFF11_2233), 0x0011_2233);
    }

    #[test]
    fn test_argb_to_softbuffer_ignores_alpha() {
        assert_eq!(argb_to_softbuffer(0x8011_2233), 0x0011_2233);
    }
}
