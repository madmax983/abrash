#![cfg(feature = "backend-winit")]
use abrash_render::experimental::radar::{apply_radar, RadarConfig};
use abrash::platform::{run_windowed, WindowApp, WindowContext, WindowHostConfig, SoftwarePresenter, HostError};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash::time::FixedTimestep;
use std::f32::consts::TAU;
use crossterm::style::Stylize;
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

impl From<HostError> for AppError {
    fn from(error: HostError) -> Self {
        Self(error.to_string())
    }
}

struct RadarDemoApp {
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    angle: f32,
    timestep: FixedTimestep,
    presenter: Option<SoftwarePresenter>,
}

impl RadarDemoApp {
    fn new() -> Result<Self, AppError> {
        let width = 800;
        let height = 600;
        let mut zbuffer = ZBuffer::new(width, height)?;
        // Add some fake geometry to the zbuffer for testing
        for y in 200..250 {
            for x in 200..250 {
                zbuffer.test_and_set(x as i32, y as i32, 10.0);
            }
        }
        for y in 400..420 {
            for x in 500..550 {
                zbuffer.test_and_set(x as i32, y as i32, 5.0);
            }
        }

        Ok(Self {
            framebuffer: Framebuffer::new(width, height)?,
            zbuffer,
            angle: 0.0,
            timestep: FixedTimestep::new(60),
            presenter: None,
        })
    }
}

impl WindowApp for RadarDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Radar Sweep Filter".to_string(),
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
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.angle = (self.angle + self.timestep.dt() * 2.0) % TAU;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let config = RadarConfig {
            center_x: self.framebuffer.width() as f32 / 2.0,
            center_y: self.framebuffer.height() as f32 / 2.0,
            angle: self.angle,
            sweep_width: 0.5,
            grid_color: 0xFF004400,
            sweep_color: 0xFF00AA00,
            blip_color: 0xFFFFFFFF,
            bg_color: 0xFF001100,
            ring_spacing: 50.0,
        };

        apply_radar(&mut self.framebuffer, &self.zbuffer, &config);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }

        Ok(())
    }
}

fn main() {
    println!("\n{}", "📡 Radar Sweep Demo".bold().green());
    let _ = run_windowed(RadarDemoApp::new().unwrap());
}
