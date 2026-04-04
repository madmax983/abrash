use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    run_windowed, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig,
};
use abrash_render::experimental::fractal::render_mandelbrot;
use std::fmt;
use std::io::Error as IoError;

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

struct FractalDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
}

impl FractalDemo {
    fn new() -> Result<Self, AppError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(800, 600)?,
        })
    }
}

impl WindowApp for FractalDemo {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Fractal Demo".to_string(),
            width: 800,
            height: 600,
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
        let width = self.framebuffer.width() as usize;
        let height = self.framebuffer.height() as usize;

        render_mandelbrot(self.framebuffer.as_mut_slice(), width, height);

        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| IoError::other("software presenter not initialized"))?;
        presenter.present(&self.framebuffer)?;

        Ok(())
    }
}

fn main() -> Result<(), AppError> {
    run_windowed(FractalDemo::new()?);
    Ok(())
}
