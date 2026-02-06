//! TUI backend using ratatui + crossterm.
//!
//! Renders the framebuffer into a terminal using half-block characters.

use super::framebuffer_widget::FramebufferWidget;
use super::{Event, WindowBackend, WindowError};
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
use std::time::{Duration, Instant};

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
}

impl WindowBackend for TuiWindow {
    fn new(title: &str, width: u32, height: u32) -> Result<Self, WindowError> {
        enable_raw_mode().map_err(|_| WindowError::RegistrationFailed)?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|_| WindowError::CreationFailed)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|_| WindowError::CreationFailed)?;

        terminal.clear().ok();

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
        })
    }

    fn is_open(&self) -> bool {
        self.is_open
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn poll_events(&mut self) -> Vec<Event> {
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
                events.push(Event::Resize(w as u32, h as u32));
            }
        }
        events
    }

    fn blit_framebuffer(&mut self, framebuffer: &Framebuffer) {
        self.frame_count += 1;
        self.frames_since_update += 1;

        let now = Instant::now();
        let duration = now.duration_since(self.last_fps_update);
        if duration.as_secs_f64() >= 1.0 {
            self.fps = self.frames_since_update as f64 / duration.as_secs_f64();
            self.frames_since_update = 0;
            self.last_fps_update = now;
        }

        let fps = self.fps;
        let fb_w = framebuffer.width();
        let fb_h = framebuffer.height();
        let frame_count = self.frame_count;
        let title = &self.title;

        let _ = self.terminal.draw(|f| {
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
                .title(format!(" {} ", title))
                .title_style(Style::default().fg(Color::Cyan));

            f.render_widget(fb_widget, block.inner(chunks[0]));
            f.render_widget(block, chunks[0]);

            // Render Status Bar
            let status_text = format!(
                " FPS: {:.1} | Res: {}x{} | Frames: {} | [Q] Quit ",
                fps, fb_w, fb_h, frame_count
            );

            let status_bar = Paragraph::new(status_text)
                .style(Style::default().fg(Color::Black).bg(Color::Cyan))
                .alignment(ratatui::layout::Alignment::Center);

            f.render_widget(status_bar, chunks[1]);
        });
    }
}

impl Drop for TuiWindow {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
