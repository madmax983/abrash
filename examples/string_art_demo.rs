use abrash::platform::WindowApp;
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::string_art::apply_string_art;

struct StringArtApp {
    fb: Framebuffer,
    width: u32,
    height: u32,
    original_fb: Framebuffer,
}

impl StringArtApp {
    fn new(width: u32, height: u32) -> Self {
        let mut original_fb = Framebuffer::new(width, height).unwrap();
        original_fb.clear(0xFFFF_FFFF);

        // Draw a basic pattern to recreate (a circle and a cross)
        let cx = (width / 2) as i32;
        let cy = (height / 2) as i32;
        let r = (width.min(height) / 4) as i32;

        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let dx = x - cx;
                let dy = y - cy;
                if dx * dx + dy * dy < r * r {
                    original_fb.set_pixel(x, y, 0xFF00_0000); // Black circle
                }

                if dx.abs() < 20 || dy.abs() < 20 {
                    original_fb.set_pixel(x, y, 0xFF44_4444); // Dark grey cross
                }
            }
        }

        Self {
            fb: Framebuffer::new(width, height).unwrap(),
            width,
            height,
            original_fb,
        }
    }
}

use abrash::platform::{SoftwarePresenter, WindowContext, WindowHostConfig, run_windowed};
use std::fmt;

#[derive(Debug)]
struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AppError {}

impl From<&'static str> for AppError {
    fn from(error: &'static str) -> Self {
        Self(error.to_string())
    }
}

impl From<String> for AppError {
    fn from(error: String) -> Self {
        Self(error)
    }
}

impl From<abrash::platform::HostError> for AppError {
    fn from(error: abrash::platform::HostError) -> Self {
        Self(error.to_string())
    }
}

impl WindowApp for StringArtApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "String Art Demo".to_string(),
            width: self.width,
            height: self.height,
            vsync: true,
        }
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Reset to the target image
        self.fb
            .as_mut_slice()
            .copy_from_slice(self.original_fb.as_slice());

        // Apply string art effect
        // 200 pegs around the circle, 3000 lines drawn
        apply_string_art(&mut self.fb, 200, 3000, 0.1);

        let mut presenter = SoftwarePresenter::new(ctx.window)?;
        presenter.present(&self.fb)?;
        Ok(())
    }
}

fn main() {
    let app = StringArtApp::new(600, 600);
    run_windowed(app);
}
