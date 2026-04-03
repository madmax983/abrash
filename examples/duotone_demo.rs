#![cfg(feature = "backend-winit")]
use abrash::experimental::duotone::Duotone;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::time::FixedTimestep;
use std::fmt;
use std::io::Error as IoError;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

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

fn print_banner() {
    println!("\n{}", "🌀  Duotone Filter Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Maps luminance to a two-color gradient").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}\n");
}

struct DuotoneDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    timestep: FixedTimestep,
    time: f32,
    filter: Duotone,
}

impl DuotoneDemoApp {
    fn new() -> Result<Self, AppError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            timestep: FixedTimestep::new(60),
            time: 0.0,
            filter: Duotone::new(0xFF00_0080, 0xFFFF_A500), // Dark Blue to Orange
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

impl WindowApp for DuotoneDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Duotone Filter Demo".to_string(),
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
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.time += self.timestep.dt();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF00_0000);

        // Background stripes
        for y in 0..HEIGHT {
            let lum = ((y as f32 / 20.0).sin() * 127.0 + 128.0) as u32;
            for x in 0..WIDTH {
                self.framebuffer.set_pixel(
                    x as i32,
                    y as i32,
                    0xFF00_0000 | (lum << 16) | (lum << 8) | lum,
                );
            }
        }

        // Draw a glowing circle in the center
        let cx = WIDTH as i32 / 2;
        let cy = HEIGHT as i32 / 2;
        let radius = 150.0 + (self.time * 2.0).sin() * 50.0;

        for y in -200..200 {
            for x in -200..200 {
                let d = ((x * x + y * y) as f32).sqrt();
                if d < radius {
                    let intensity = (1.0 - d / radius).powf(2.0);
                    let lum = (intensity * 255.0) as u32;
                    let px = cx + x;
                    let py = cy + y;
                    if px >= 0 && px < WIDTH as i32 && py >= 0 && py < HEIGHT as i32 {
                        self.framebuffer.set_pixel(
                            px,
                            py,
                            0xFF00_0000 | (lum << 16) | (lum << 8) | lum,
                        );
                    }
                }
            }
        }

        // Apply post-processing filter
        self.filter.apply(&mut self.framebuffer);

        self.present()
    }
}

fn main() -> Result<(), AppError> {
    print_banner();
    run_windowed(DuotoneDemoApp::new()?)?;
    Ok(())
}
