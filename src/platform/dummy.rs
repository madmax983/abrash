use crate::framebuffer::Framebuffer;
use super::{Event, WindowError};

pub struct Window {
    width: u32,
    height: u32,
    is_open: bool,
}

impl Window {
    pub fn new(_title: &str, width: u32, height: u32) -> Result<Self, WindowError> {
        Ok(Self {
            width,
            height,
            is_open: true,
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
        // Close immediately on first poll to prevent infinite loops in examples
        if self.is_open {
            self.is_open = false;
            vec![Event::Close]
        } else {
            Vec::new()
        }
    }

    pub fn blit_framebuffer(&self, _framebuffer: &Framebuffer) {}
}
