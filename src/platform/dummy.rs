use super::{Event, WindowError};
use crate::framebuffer::Framebuffer;
use crossterm::{
    cursor,
    event::{self, KeyCode},
    execute,
    terminal,
};
use std::io::{self, Write};
use std::time::Instant;

pub struct Window {
    width: u32,
    height: u32,
    is_open: bool,
    last_frame_time: Instant,
}

impl Window {
    pub fn new(_title: &str, width: u32, height: u32) -> Result<Self, WindowError> {
        terminal::enable_raw_mode().map_err(|_| WindowError::CreationFailed)?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            cursor::Hide,
            terminal::Clear(terminal::ClearType::All)
        )
        .map_err(|_| WindowError::CreationFailed)?;

        Ok(Self {
            width,
            height,
            is_open: true,
            last_frame_time: Instant::now(),
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
        // Poll for events (non-blocking)
        if event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(event::Event::Key(key)) = event::read() {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    self.is_open = false;
                    return vec![Event::Close];
                }
            }
        }

        Vec::new()
    }

    pub fn blit_framebuffer(&mut self, framebuffer: &Framebuffer) {
        let (term_w_u16, term_h_u16) = terminal::size().unwrap_or((80, 40));
        let term_w = term_w_u16 as usize;
        let term_h = term_h_u16 as usize;

        // Calculate FPS
        let now = Instant::now();
        let duration = now.duration_since(self.last_frame_time);
        let seconds = duration.as_secs_f32();
        self.last_frame_time = now;
        let fps = if seconds > 0.0 { 1.0 / seconds } else { 60.0 };

        let mut output = String::with_capacity(term_w * term_h * 30);

        // Move cursor to top-left
        use std::fmt::Write;
        let _ = write!(output, "{}", crossterm::cursor::MoveTo(0, 0));

        // Reserve one line for status bar
        let render_h_chars = if term_h > 1 { term_h - 1 } else { 1 };

        for y in 0..render_h_chars {
            for x in 0..term_w {
                let src_x = (x * framebuffer.width() as usize) / term_w;

                // Top pixel (foreground)
                let src_y_top = (y * 2 * framebuffer.height() as usize) / (render_h_chars * 2);
                let p_top = framebuffer
                    .get_pixel(src_x as i32, src_y_top as i32)
                    .unwrap_or(0);
                let r1 = (p_top >> 16) & 0xFF;
                let g1 = (p_top >> 8) & 0xFF;
                let b1 = p_top & 0xFF;

                // Bottom pixel (background)
                let src_y_bot =
                    ((y * 2 + 1) * framebuffer.height() as usize) / (render_h_chars * 2);
                let p_bot = framebuffer
                    .get_pixel(src_x as i32, src_y_bot as i32)
                    .unwrap_or(0);
                let r2 = (p_bot >> 16) & 0xFF;
                let g2 = (p_bot >> 8) & 0xFF;
                let b2 = p_bot & 0xFF;

                // FG color (38;2) for top, BG color (48;2) for bottom, then Upper Half Block
                let _ = write!(
                    output,
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                    r1, g1, b1, r2, g2, b2
                );
            }
            output.push_str("\x1b[0m\r\n");
        }

        // Status Bar
        let status_text = format!(
            " FPS: {:>3.0} | Size: {:>3}x{:<3} | [Q]uit ",
            fps,
            framebuffer.width(),
            framebuffer.height()
        );

        let padding = if term_w > status_text.len() {
            term_w - status_text.len()
        } else {
            0
        };

        // Blue background for status bar
        let _ = write!(
            output,
            "\x1b[38;2;255;255;255m\x1b[48;2;50;50;200m{}{}\x1b[0m",
            status_text,
            " ".repeat(padding)
        );

        let mut stdout = io::stdout();
        let _ = stdout.write_all(output.as_bytes());
        let _ = stdout.flush();
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        execute!(stdout, cursor::Show).ok();
        terminal::disable_raw_mode().ok();
    }
}
