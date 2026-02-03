use super::{Event, WindowError};
use crate::framebuffer::Framebuffer;

pub struct Window {
    width: u32,
    height: u32,
    is_open: bool,
    frame_count: usize,
}

impl Window {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, WindowError> {
        println!("\n🎨 Abrash Graphics Engine - Headless Mode");
        println!("   Running: {}", title);
        println!("   Resolution: {}x{}", width, height);
        println!("   Auto-exit after 120 frames.\n");

        Ok(Self {
            width,
            height,
            is_open: true,
            frame_count: 0,
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
        self.frame_count += 1;

        if self.frame_count >= 120 {
            self.is_open = false;
            vec![Event::Close]
        } else {
            Vec::new()
        }
    }

    pub fn blit_framebuffer(&self, framebuffer: &Framebuffer) {
        const TERM_W: usize = 80;
        const TERM_H: usize = 40;

        let mut output = String::with_capacity(TERM_W * TERM_H * 30);

        // Move cursor up to overwrite previous frame
        if self.frame_count > 1 {
            use std::fmt::Write;
            // +1 for the status bar
            let _ = write!(output, "\x1b[{}A", TERM_H + 1);
        }

        for y in 0..TERM_H {
            for x in 0..TERM_W {
                let src_x = (x * framebuffer.width() as usize) / TERM_W;

                // Top pixel (foreground)
                let src_y_top = (y * 2 * framebuffer.height() as usize) / (TERM_H * 2);
                let p_top = framebuffer
                    .get_pixel(src_x as i32, src_y_top as i32)
                    .unwrap_or(0);
                let r1 = (p_top >> 16) & 0xFF;
                let g1 = (p_top >> 8) & 0xFF;
                let b1 = p_top & 0xFF;

                // Bottom pixel (background)
                let src_y_bot = ((y * 2 + 1) * framebuffer.height() as usize) / (TERM_H * 2);
                let p_bot = framebuffer
                    .get_pixel(src_x as i32, src_y_bot as i32)
                    .unwrap_or(0);
                let r2 = (p_bot >> 16) & 0xFF;
                let g2 = (p_bot >> 8) & 0xFF;
                let b2 = p_bot & 0xFF;

                use std::fmt::Write;
                // FG color (38;2) for top, BG color (48;2) for bottom, then Upper Half Block
                let _ = write!(
                    output,
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                    r1, g1, b1, r2, g2, b2
                );
            }
            output.push_str("\x1b[0m\n");
        }

        // Status Bar
        let progress = (self.frame_count as f32 / 120.0).clamp(0.0, 1.0);
        let bars = (progress * 20.0) as usize;
        let spaces = 20 - bars;
        let bar_str = format!("[{}{}]", "=".repeat(bars), " ".repeat(spaces));

        use std::fmt::Write;
        // Inverse video for status bar
        let _ = writeln!(
            output,
            "\x1b[7m Frame: {:>3}/120 | Res: 80x80 | {} {:>3.0}% \x1b[0m",
            self.frame_count,
            bar_str,
            progress * 100.0
        );

        print!("{}", output);
    }
}
