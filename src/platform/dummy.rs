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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::io::{Stdout, stdout};
use std::time::{Duration, Instant};

pub struct Window {
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

impl Window {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, WindowError> {
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

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn poll_events(&mut self) -> Vec<Event> {
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

struct FramebufferWidget<'a> {
    framebuffer: &'a Framebuffer,
}

impl<'a> Widget for FramebufferWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let term_w = area.width as usize;
        let term_h = area.height as usize;
        let fb_w = self.framebuffer.width() as usize;
        let fb_h = self.framebuffer.height() as usize;

        for y in 0..term_h {
            for x in 0..term_w {
                // Map terminal cell (x,y) to framebuffer coordinates
                // Nearest neighbor scaling
                let fb_x = (x * fb_w) / term_w;

                // Top sub-pixel
                let fb_y_top = (y * 2 * fb_h) / (term_h * 2);
                // Bottom sub-pixel
                let fb_y_bot = ((y * 2 + 1) * fb_h) / (term_h * 2);

                if fb_x >= fb_w || fb_y_top >= fb_h {
                    continue;
                }

                // Get colors
                let p_top = self
                    .framebuffer
                    .get_pixel(fb_x as i32, fb_y_top as i32)
                    .unwrap_or(0);
                let p_bot = self
                    .framebuffer
                    .get_pixel(fb_x as i32, fb_y_bot as i32)
                    .unwrap_or(0);

                // unpack (r, g, b) from u32 0xRRGGBB
                let (r1, g1, b1) = ((p_top >> 16) as u8, (p_top >> 8) as u8, p_top as u8);
                let (r2, g2, b2) = ((p_bot >> 16) as u8, (p_bot >> 8) as u8, p_bot as u8);

                if let Some(cell) = buf.cell_mut((area.x + x as u16, area.y + y as u16)) {
                    cell.set_char('▀')
                        .set_fg(Color::Rgb(r1, g1, b1))
                        .set_bg(Color::Rgb(r2, g2, b2));
                }
            }
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
