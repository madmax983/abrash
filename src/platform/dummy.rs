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

        let mut output = String::with_capacity(TERM_W * TERM_H * 20);

        // Move cursor up to overwrite previous frame
        if self.frame_count > 1 {
            use std::fmt::Write;
            let _ = write!(output, "\x1b[{}A", TERM_H);
        }

        for y in 0..TERM_H {
            for x in 0..TERM_W {
                // Nearest neighbor sampling
                let src_x = (x * framebuffer.width() as usize) / TERM_W;
                let src_y = (y * framebuffer.height() as usize) / TERM_H;

                let pixel = framebuffer
                    .get_pixel(src_x as i32, src_y as i32)
                    .unwrap_or(0);

                let r = (pixel >> 16) & 0xFF;
                let g = (pixel >> 8) & 0xFF;
                let b = pixel & 0xFF;

                // ANSI truecolor background + 2 spaces
                use std::fmt::Write;
                let _ = write!(output, "\x1b[48;2;{};{};{}m  ", r, g, b);
            }
            output.push_str("\x1b[0m\n");
        }

        print!("{}", output);
    }
}
