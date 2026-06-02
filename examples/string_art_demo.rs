use abrash::framebuffer::Framebuffer;
use abrash::platform::{SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed};
use abrash_render::experimental::string_art::apply_string_art;

use std::fmt;
use std::io::Error as IoError;

const WIDTH: u32 = 600;
const HEIGHT: u32 = 600;

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

impl From<IoError> for AppError {
    fn from(error: IoError) -> Self {
        Self(error.to_string())
    }
}

impl From<abrash::platform::HostError> for AppError {
    fn from(error: abrash::platform::HostError) -> Self {
        Self(error.to_string())
    }
}

struct StringArtDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
}

impl StringArtDemoApp {
    fn new() -> Result<Self, AppError> {
        let mut source_fb = Framebuffer::new(WIDTH, HEIGHT)?;

        let cx = WIDTH as i32 / 2;
        let cy = HEIGHT as i32 / 2;
        let max_dist = (cx as f32).hypot(cy as f32);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let dx = x as i32 - cx;
                let dy = y as i32 - cy;
                let dist = (dx as f32).hypot(dy as f32);

                let intensity = (dist / max_dist).min(1.0);
                let v = (intensity * 255.0) as u32;
                let color = 0xFF00_0000 | (v << 16) | (v << 8) | v;
                source_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        apply_string_art(&mut source_fb, 200, 3000);

        let mut fb = Framebuffer::new(WIDTH, HEIGHT)?;
        fb.as_mut_slice().copy_from_slice(source_fb.as_slice());

        Ok(Self {
            presenter: None,
            framebuffer: fb,
        })
    }

    fn present(&mut self) -> Result<(), AppError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| IoError::other("software presenter not initialized"))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for StringArtDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: String Art Generator".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.present()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), AppError> {
    run_windowed(StringArtDemoApp::new().unwrap());
    Ok(())
}