//! TUI backend using ratatui + crossterm.
//!
//! Renders the framebuffer into a terminal using half-block characters.

use super::framebuffer_widget::FramebufferWidget;
use super::{Event, WindowError};
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
use std::io::{Stdout, stdout};
use std::thread;
use std::time::{Duration, Instant};

/// An application window rendered entirely within a terminal emulator.
///
/// `TuiWindow` leverages the `crossterm` backend and `ratatui` UI library to draw graphics
/// using Unicode half-blocks (▀) directly to standard output. This allows headless
/// or CLI-only environments to visualize framebuffers without needing an X11 or Wayland display server.
///
/// ## Examples
///
/// ```rust,no_run
/// use abrash_render::platform::tui::TuiWindow;
///
/// // Create a new terminal window 80 characters wide by 40 high
/// let mut window = TuiWindow::new("Retro Render", 80, 40)
///     .expect("Failed to initialize terminal interface");
///
/// // Poll for events like keyboard input or window close requests
/// let events = window.poll_events();
///
/// // Cleanly shutdown the terminal back to normal mode
/// drop(window);
/// ```
pub struct TuiWindow {
    width: u32,
    height: u32,
    title: String,
    is_open: bool,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    frame_count: u64,
    last_fps_update: Instant,
    frames_since_update: u64,
    fps: f64,
    frame_start: Instant,
    target_frame_time: Duration,
}

const DEFAULT_REFRESH_HZ: u32 = 60;

/// Query the monitor refresh rate via platform APIs.
///
/// On Windows, queries `EnumDisplaySettingsW` for the primary monitor's refresh rate.
/// Falls back to `DEFAULT_REFRESH_HZ` on non-Windows or if the query fails.
#[allow(clippy::missing_const_for_fn)]
fn query_refresh_rate() -> u32 {
    #[cfg(target_os = "windows")]
    {
        // SAFETY: EnumDisplaySettingsW with null device name queries the primary monitor.
        // DEVMODEW must be zero-initialized with dmSize set before the call.
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

impl TuiWindow {
    /// Create a terminal-backed window surface in the alternate screen.
    ///
    /// # Errors
    ///
    /// Returns [`WindowError::RegistrationFailed`] if raw mode cannot be enabled
    /// or [`WindowError::CreationFailed`] if the terminal backend cannot be
    /// initialized.
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
            is_open: true,
            terminal,
            frame_count: 0,
            last_fps_update: Instant::now(),
            frames_since_update: 0,
            fps: 0.0,
            frame_start: Instant::now(),
            target_frame_time,
        })
    }

    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.is_open
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn poll_events(&mut self) -> Vec<Event> {
        self.frame_start = Instant::now();
        let mut events = Vec::new();

        // Non-blocking poll
        if event::poll(Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(event::Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            self.is_open = false;
                            events.push(Event::Close);
                        }
                        _ => {}
                    }
                }
            } else if let Ok(event::Event::Resize(w, h)) = event::read() {
                events.push(Event::Resize(u32::from(w), u32::from(h)));
            }
        }
        events
    }

    pub fn blit_framebuffer(&mut self, framebuffer: &Framebuffer) {
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
        let title = &self.title;

        let _ = self.terminal.draw(|f| {
            draw_framebuffer(f, framebuffer, title, &metrics);
        });

        // Sleep to fill remaining frame budget (approximate vsync)
        let elapsed = self.frame_start.elapsed();
        if let Some(remaining) = self.target_frame_time.checked_sub(elapsed) {
            thread::sleep(remaining);
        }
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

    // Render Status Bar
    let status_text = format!(
        " FPS: {:.1} | Res: {}x{} | Frames: {} | [Q] Quit ",
        metrics.fps, metrics.fb_w, metrics.fb_h, metrics.frame_count
    );

    let status_bar = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Black).bg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(status_bar, chunks[1]);
}

impl Drop for TuiWindow {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

// --- Winit Backend Shim ---
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowHostConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
}

impl Default for WindowHostConfig {
    fn default() -> Self {
        Self {
            title: "Abrash TUI".to_string(),
            width: 80,
            height: 40,
            vsync: true,
        }
    }
}

#[derive(Debug)]
pub enum HostError {
    App(String),
    Window(String),
    EventLoop(String),
    Present(String),
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::App(s) => write!(f, "App error: {s}"),
            Self::Window(s) => write!(f, "Window error: {s}"),
            Self::EventLoop(s) => write!(f, "Event loop error: {s}"),
            Self::Present(s) => write!(f, "Present error: {s}"),
        }
    }
}

impl Error for HostError {}

#[derive(Clone)]
pub struct WindowContext<'a> {
    pub dt_seconds: f32,
    pub window: std::sync::Arc<()>,
    pub event_loop: &'a (),
}

pub trait WindowApp {
    type Error: Error + Send + Sync + 'static;

    fn config(&self) -> WindowHostConfig;

    #[allow(clippy::missing_errors_doc)]
    fn init(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    fn resize(&mut self, _ctx: WindowContext<'_>, _width: u32, _height: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    fn input(&mut self, _ctx: WindowContext<'_>, _event: &WindowEvent) -> Result<(), Self::Error> {
        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error>;

    #[allow(clippy::missing_errors_doc)]
    fn render(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub struct FrameClock {
    last_tick: Instant,
}

impl FrameClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_tick: Instant::now(),
        }
    }

    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;
        dt
    }
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub logical_key: Key,
    pub state: ElementState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowEvent {
    CloseRequested,
    Resized(PhysicalSize),
    RedrawRequested,
    KeyboardInput { event: KeyEvent },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Named(NamedKey),
    Character(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedKey {
    Escape,
    Enter,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}

// Global channel for software presenter -> loop
use std::sync::mpsc::{sync_channel, SyncSender};

std::thread_local! {
    static FB_SENDER: std::cell::RefCell<Option<SyncSender<(u32, u32, Vec<u32>)>>> = std::cell::RefCell::new(None);
}

pub struct SoftwarePresenter {
    // Empty, since we use the global sender
}

impl SoftwarePresenter {
    #[allow(clippy::missing_errors_doc)]
    pub fn new(_window: std::sync::Arc<()>) -> Result<Self, HostError> {
        Ok(Self {})
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn present(&mut self, framebuffer: &Framebuffer) -> Result<(), HostError> {
        let fb = (framebuffer.width(), framebuffer.height(), framebuffer.as_slice().to_vec());
        FB_SENDER.with(|sender| {
            if let Some(tx) = sender.borrow().as_ref() {
                let _ = tx.send(fb);
            }
        });
        Ok(())
    }
}

use comfy_table::{Cell, Color as ComfyColor, Table, presets};

fn print_host_error_and_exit(err: &HostError) -> ! {
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("❌ Window Application Error")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(ComfyColor::Red),
        ])
        .add_row(vec![Cell::new(format!("{err}")).fg(ComfyColor::Yellow)]);

    eprintln!("\n{table}");
    std::process::exit(1);
}

#[allow(clippy::missing_panics_doc)]
pub fn run_windowed<A: WindowApp>(mut app: A) {
    let config = app.config();
    let mut window = match TuiWindow::new(&config.title, config.width, config.height) {
        Ok(w) => w,
        Err(e) => {
            print_host_error_and_exit(&HostError::Window(e.to_string()));
        }
    };

    let (tx, rx) = sync_channel::<(u32, u32, Vec<u32>)>(1);
    FB_SENDER.with(|sender| {
        *sender.borrow_mut() = Some(tx);
    });

    let window_handle = std::sync::Arc::new(());
    let mut clock = FrameClock::new();

    if let Err(e) = app.init(WindowContext {
        dt_seconds: 0.0,
        window: window_handle.clone(),
        event_loop: &(),
    }) {
        print_host_error_and_exit(&HostError::App(e.to_string()));
    }

    while window.is_open() {
        // Poll input events
        let mut events = Vec::new();
        if event::poll(Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(event::Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    if let Some(logical_key) = crossterm_key_to_winit(key.code) {
                        events.push(WindowEvent::KeyboardInput {
                            event: KeyEvent {
                                logical_key,
                                state: ElementState::Pressed,
                            },
                        });
                    }
                    if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                        events.push(WindowEvent::CloseRequested);
                    }
                }
            } else if let Ok(event::Event::Resize(w, h)) = event::read() {
                events.push(WindowEvent::Resized(PhysicalSize {
                    width: u32::from(w),
                    height: u32::from(h),
                }));
            }
        }

        for wev in events {
            match &wev {
                WindowEvent::CloseRequested => {
                    return;
                }
                WindowEvent::Resized(size) => {
                    if let Err(e) = app.resize(
                        WindowContext {
                            dt_seconds: 0.0,
                            window: window_handle.clone(),
                            event_loop: &(),
                        },
                        size.width,
                        size.height,
                    ) {
                        print_host_error_and_exit(&HostError::App(e.to_string()));
                    }
                }
                _ => {}
            }

            if let Err(e) = app.input(
                WindowContext {
                    dt_seconds: 0.0,
                    window: window_handle.clone(),
                    event_loop: &(),
                },
                &wev,
            ) {
                print_host_error_and_exit(&HostError::App(e.to_string()));
            }
        }

        let dt = clock.tick();
        let ctx = WindowContext {
            dt_seconds: dt,
            window: window_handle.clone(),
            event_loop: &(),
        };

        if let Err(e) = app.update(ctx.clone()) {
            print_host_error_and_exit(&HostError::App(e.to_string()));
        }
        if let Err(e) = app.render(ctx.clone()) {
            print_host_error_and_exit(&HostError::App(e.to_string()));
        }

        if let Ok((width, height, pixels)) = rx.try_recv() {
            let mut fb = Framebuffer::new(width, height).unwrap();
            fb.as_mut_slice().copy_from_slice(&pixels);
            window.blit_framebuffer(&fb);
        }
    }
}

fn crossterm_key_to_winit(key: KeyCode) -> Option<Key> {
    match key {
        KeyCode::Char(' ') => Some(Key::Named(NamedKey::Space)),
        KeyCode::Char(c) => Some(Key::Character(c.to_string())),
        KeyCode::Esc => Some(Key::Named(NamedKey::Escape)),
        KeyCode::Enter => Some(Key::Named(NamedKey::Enter)),
        KeyCode::Up => Some(Key::Named(NamedKey::ArrowUp)),
        KeyCode::Down => Some(Key::Named(NamedKey::ArrowDown)),
        KeyCode::Left => Some(Key::Named(NamedKey::ArrowLeft)),
        KeyCode::Right => Some(Key::Named(NamedKey::ArrowRight)),
        _ => None,
    }
}
