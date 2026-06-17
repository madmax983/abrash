//! Seam Carving Demo
//!
//! Demonstrates the experimental Seam Carving (Content-Aware Scaling)
//! algorithm by progressively shrinking the width of a generated image.

use abrash_core::{
    framebuffer::Framebuffer,
    texture::Texture,
};
use abrash::platform::{SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed};
use abrash_render::experimental::seam_carving::{apply_seam_carving, SeamCarvingConfig};
use std::error::Error;

struct SeamCarvingDemo {
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    original_texture: Texture,
    carved_buffer: Vec<u32>,
    current_width: u32,
    frames_since_carve: u32,
    original_width: u32,
    height: u32,
}

impl SeamCarvingDemo {
    pub fn new(width: u32, height: u32) -> Self {
        let mut original_texture = Texture::new(width, height).unwrap();

        // Generate an interesting pattern (sky with some "clouds" and "buildings")
        for y in 0..height {
            for x in 0..width {
                // Sky gradient
                let mut r = 100;
                let mut g = 150 + (y * 50 / height);
                let mut b = 255;

                // A "sun" feature (should have high energy)
                let cx = width / 4;
                let cy = height / 4;
                let dist_sq = ((x as i32 - cx as i32).pow(2) + (y as i32 - cy as i32).pow(2)) as u32;
                if dist_sq < 2000 {
                    r = 255;
                    g = 255;
                    b = 100;
                }

                // A "building" feature (should have high energy)
                if x > width * 3 / 4 && x < width * 3 / 4 + 50 && y > height / 2 {
                    r = 50;
                    g = 50;
                    b = 50;
                }

                let color = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32); original_texture.set_pixel(x, y, color);
            }
        }

        Self {
            framebuffer: Framebuffer::new(width, height).unwrap(),
            presenter: None,
            carved_buffer: original_texture.pixels().to_vec(),
            current_width: width,
            frames_since_carve: 0,
            original_width: width,
            height,
            original_texture,
        }
    }
}

impl WindowApp for SeamCarvingDemo {
    type Error = abrash::platform::HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Seam Carving Demo".to_string(),
            width: self.original_width,
            height: self.height,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window).map_err(|e| abrash::platform::HostError::App(e.to_string()))?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.frames_since_carve += 1;

        // Carve one seam every few frames
        if self.frames_since_carve > 2 && self.current_width > self.original_width / 2 {
            let config = SeamCarvingConfig {
                seams_to_remove: 2,
                original_width: self.current_width,
                stride: self.original_width,
                height: self.height,
            };

            apply_seam_carving(&mut self.carved_buffer, config);
            self.current_width -= 2;
            self.frames_since_carve = 0;
        } else if self.current_width <= self.original_width / 2 && self.frames_since_carve > 60 {
            // Reset after waiting
            self.carved_buffer = self.original_texture.pixels().to_vec();
            self.current_width = self.original_width;
            self.frames_since_carve = 0;
        }

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF222222);

        // Copy the carved buffer to the center of the framebuffer
        let offset_x = (self.original_width - self.current_width) / 2;

        for y in 0..self.height {
            for x in 0..self.current_width {
                let color = self.carved_buffer[(y * self.original_width + x) as usize];
                self.framebuffer.set_pixel((offset_x + x) as i32, y as i32, color);
            }
        }

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer).map_err(|e| abrash::platform::HostError::Present(e.to_string()))?;
        }

        Ok(())
    }
}

fn main() {
    let app = SeamCarvingDemo::new(800, 600);
    run_windowed(app);
}
