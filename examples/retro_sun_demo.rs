use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::infinite_grid::{apply_infinite_grid, InfiniteGridConfig};
use abrash_render::experimental::retro_sun::render_retro_sun;
use std::fmt;

const WIDTH: u32 = 800;
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

impl From<HostError> for AppError {
    fn from(error: HostError) -> Self {
        Self(error.to_string())
    }
}

struct RetroSunDemoApp {
    presenter: Option<SoftwarePresenter>,
    fb: Framebuffer,
    time: f32,
    last_time: std::time::Instant,
}

impl RetroSunDemoApp {
    fn new() -> Result<Self, AppError> {
        Ok(Self {
            presenter: None,
            fb: Framebuffer::new(WIDTH, HEIGHT)?,
            time: 0.0,
            last_time: std::time::Instant::now(),
        })
    }

    fn present(&mut self) -> Result<(), AppError> {
        let framebuffer = &self.fb;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| AppError::from("software presenter not initialized"))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for RetroSunDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Outrun Demo".to_string(),
            width: WIDTH,
            height: HEIGHT,
            ..Default::default()
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;
        self.time += dt;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let width = self.fb.width() as usize;

        // Deep sunset background
        self.fb.clear(0xFF_0a001a);

        // Render Retro Sun
        let sun_radius = 180;
        let sun_center_y = 250;
        render_retro_sun(
            &mut self.fb,
            (width / 2) as i32,
            sun_center_y,
            sun_radius,
            0xFF_ff0055, // Bright pink top
            0xFF_ffaa00, // Golden bottom
        );

        // Render Infinite Grid (Synthwave style)
        let config = InfiniteGridConfig {
            line_color: 0xFF_00ffff,
            background_color: 0xFF_0a001a,
            time: self.time,
            ..Default::default()
        };
        apply_infinite_grid(&mut self.fb, &config);

        self.present()
    }
}

fn main() {
    let app = RetroSunDemoApp::new().unwrap();
    run_windowed(app);
}
