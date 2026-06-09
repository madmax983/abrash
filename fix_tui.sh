cat << 'INNER_EOF' > src/platform/tui.rs
//! TUI backend using ratatui + crossterm.
//!
//! Renders the framebuffer into a terminal using half-block characters.

use super::framebuffer_widget::FramebufferWidget;
use crate::framebuffer::Framebuffer;
use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use std::error::Error;
use std::fmt;
use std::io::{Stdout, stdout};
use std::thread;
use std::time::{Duration, Instant};

/// Events that can be emitted by the windowing system.
#[derive(Debug, Clone)]
pub enum Event {
    /// The window has been requested to close.
    Close,
    /// The window has been resized. Contains the new width and height.
    Resize(u32, u32),
    /// A generic input event (currently a string representation of the crossterm event).
    Input(String),
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

impl Error for WindowError {}

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
            title: "Abrash Window".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }
}

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
    /// A placeholder since TUI does not use `winit::window::Window`.
    pub _dummy: &'a (),
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
    fn input(&mut self, _ctx: WindowContext<'_>, _event: &Event) -> Result<(), Self::Error> {
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

/// A combined error type covering both `WindowError` and app-specific errors.
#[derive(Debug)]
pub enum HostError {
    /// Platform event loop error.
    EventLoop(String),
    /// Platform window error.
    Window(String),
    /// Framebuffer presentation error.
    Present(String),
    /// App-defined error.
    App(String),
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventLoop(error) => write!(f, "Event loop error: {error}"),
            Self::Window(error) => write!(f, "Window creation error: {error}"),
            Self::Present(error) => write!(f, "Presentation error: {error}"),
            Self::App(error) => write!(f, "App error: {error}"),
        }
    }
}

impl Error for HostError {}

use comfy_table::{Cell, Color, Table, presets};

fn print_host_error_and_exit(err: &HostError) -> ! {
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("❌ Window Application Error")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(Color::Red),
        ])
        .add_row(vec![Cell::new(format!("{err}")).fg(Color::Yellow)]);

    eprintln!("\n{table}");
    std::process::exit(1);
}

const DEFAULT_REFRESH_HZ: u32 = 60;

#[allow(clippy::missing_const_for_fn)]
fn query_refresh_rate() -> u32 {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            use std::mem;
            use windows_sys::Win32::Graphics::Gdi::{
                DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplaySettingsW,
            };

            let mut devmode: DEVMODEW = mem::zeroed();
            devmode.dmSize = mem::size_of::<DEVMODEW>() as u16;

            if EnumDisplaySettingsW(std::ptr::null(), ENUM_CURRENT_SETTINGS, &raw mut devmode) != 0
                && devmode.dmDisplayFrequency > 0
            {
                devmode.dmDisplayFrequency
            } else {
                DEFAULT_REFRESH_HZ
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        DEFAULT_REFRESH_HZ
    }
}

pub struct TuiWindow {
    width: u32,
    height: u32,
    title: String,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    target_frame_time: Duration,
}

impl TuiWindow {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, WindowError> {
        if width > i32::MAX as u32 || height > i32::MAX as u32 || width == 0 || height == 0 {
            return Err(WindowError::CreationFailed);
        }

        enable_raw_mode().map_err(|_| WindowError::RegistrationFailed)?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|_| WindowError::CreationFailed)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|_| WindowError::CreationFailed)?;

        terminal.clear().ok();

        let hz = query_refresh_rate();
        let target_frame_time = Duration::from_secs_f64(1.0 / f64::from(hz));

        Ok(Self {
            width,
            height,
            title: title.to_string(),
            terminal,
            target_frame_time,
        })
    }
}

impl Drop for TuiWindow {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

struct RenderMetrics {
    fps: f64,
    frame_count: u64,
    fb_w: u32,
    fb_h: u32,
}

fn draw_framebuffer(
    f: &mut ratatui::Frame,
    framebuffer: &Framebuffer,
    title: &str,
    metrics: &RenderMetrics,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    let fb_widget = FramebufferWidget { framebuffer };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(format!(" {title} "))
        .title_style(Style::default().fg(Color::Cyan));

    f.render_widget(fb_widget, block.inner(chunks[0]));
    f.render_widget(block, chunks[0]);

    let status_text = format!(
        " FPS: {:.1} | Res: {}x{} | Frames: {} | [Q] Quit ",
        metrics.fps, metrics.fb_w, metrics.fb_h, metrics.frame_count
    );

    let status_bar = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Black).bg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(status_bar, chunks[1]);
}

pub struct SoftwarePresenter {
    tui_window: TuiWindow,
    frame_count: u64,
    last_fps_update: Instant,
    frames_since_update: u64,
    fps: f64,
    frame_start: Instant,
}

impl SoftwarePresenter {
    pub fn new(ctx: WindowContext<'_>) -> Result<Self, HostError> {
        // TUI window logic will be managed internally. We could extract config somehow, but
        // it's easier to recreate the TUI window here with default size, or we need to pass
        // config down. Since SoftwarePresenter is created after TuiWindow starts inside
        // run_windowed, actually we should just create TuiWindow here or pass it.
        // Wait, the trait takes `ctx`. So we should probably store `TuiWindow` globally or inside presenter.
        // Creating TuiWindow here is the simplest way.
        let tui_window = TuiWindow::new("Abrash", 800, 600)
            .map_err(|e| HostError::Present(e.to_string()))?;
        Ok(Self {
            tui_window,
            frame_count: 0,
            last_fps_update: Instant::now(),
            frames_since_update: 0,
            fps: 0.0,
            frame_start: Instant::now(),
        })
    }

    pub fn present(&mut self, framebuffer: &Framebuffer) -> Result<(), HostError> {
        self.frame_count += 1;
        self.frames_since_update += 1;

        let now = Instant::now();
        let duration = now.duration_since(self.last_fps_update);
        if duration.as_secs_f64() >= 1.0 {
            self.fps = self.frames_since_update as f64 / duration.as_secs_f64();
            self.frames_since_update = 0;
            self.last_fps_update = now;
        }

        let metrics = RenderMetrics {
            fps: self.fps,
            frame_count: self.frame_count,
            fb_w: framebuffer.width(),
            fb_h: framebuffer.height(),
        };

        let title = &self.tui_window.title;

        let _ = self.tui_window.terminal.draw(|f| {
            draw_framebuffer(f, framebuffer, title, &metrics);
        });

        let elapsed = self.frame_start.elapsed();
        if let Some(remaining) = self.tui_window.target_frame_time.checked_sub(elapsed) {
            thread::sleep(remaining);
        }
        self.frame_start = Instant::now();
        Ok(())
    }

    // Hack to set title
    pub fn set_title(&mut self, title: String) {
        self.tui_window.title = title;
    }
}

pub fn run_windowed<A>(mut app: A)
where
    A: WindowApp,
{
    let config = app.config();
    let dummy = ();

    if let Err(error) = app.init(WindowContext {
        _dummy: &dummy,
        dt_seconds: 0.0,
    }) {
        print_host_error_and_exit(&HostError::App(error.to_string()));
    }

    let mut clock = FrameClock::new();
    let mut is_open = true;

    while is_open {
        if event::poll(Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(event::Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            is_open = false;
                            continue;
                        }
                        _ => {}
                    }
                }
            } else if let Ok(event::Event::Resize(w, h)) = event::read() {
                if let Err(error) = app.resize(
                    WindowContext {
                        _dummy: &dummy,
                        dt_seconds: 0.0,
                    },
                    w.into(),
                    h.into(),
                ) {
                    print_host_error_and_exit(&HostError::App(error.to_string()));
                }
            }
        }

        let dt_seconds = clock.tick();
        let redraw_context = WindowContext {
            _dummy: &dummy,
            dt_seconds,
        };

        if let Err(error) = app.update(redraw_context.clone()) {
            print_host_error_and_exit(&HostError::App(error.to_string()));
        } else if let Err(error) = app.render(redraw_context) {
            print_host_error_and_exit(&HostError::App(error.to_string()));
        }
    }
}
INNER_EOF
