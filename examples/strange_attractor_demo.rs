//! Strange Attractor Demo
//!
//! Demonstrates rendering a mathematical strange attractor (Clifford)
//! directly to a framebuffer.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

#[cfg(feature = "nova")]
use abrash_render::experimental::strange_attractor::{CliffordParams, render_clifford};

use std::fmt;
use std::io::Error as IoError;

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
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
    fn from(s: &'static str) -> Self {
        Self(s.to_string())
    }
}

impl From<IoError> for AppError {
    fn from(e: IoError) -> Self {
        Self(e.to_string())
    }
}

impl From<HostError> for AppError {
    fn from(e: HostError) -> Self {
        Self(e.to_string())
    }
}

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Strange Attractor Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Generates a Clifford Attractor via procedural mathematical chaos")
                .fg(Color::Green),
        ]);

    println!("{table}\n");
}

#[cfg(feature = "nova")]
struct StrangeAttractorDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    params: CliffordParams,
    time: f64,
}

#[cfg(feature = "nova")]
impl StrangeAttractorDemo {
    fn new() -> Result<Self, AppError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            params: CliffordParams {
                a: -1.4,
                b: 1.6,
                c: 1.0,
                d: 0.7,
            },
            time: 0.0,
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

#[cfg(feature = "nova")]
impl WindowApp for StrangeAttractorDemo {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Strange Attractor Demo".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        // Initialize with black
        self.framebuffer.clear(0xFF_00_00_00);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let dt = 1.0 / 60.0;
        self.time += dt * 0.2;
        // Slowly animate the parameters
        self.params.a = -1.4 + (self.time).sin() * 0.5;
        self.params.b = 1.6 + (self.time * 0.8).cos() * 0.3;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Fade out the previous frame slightly instead of clearing, for motion blur/ghosting
        for pixel in self.framebuffer.as_mut_slice() {
            let r = ((*pixel >> 16) & 0xFF).saturating_sub(5);
            let g = ((*pixel >> 8) & 0xFF).saturating_sub(5);
            let b = (*pixel & 0xFF).saturating_sub(5);
            *pixel = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
        }
        let w = f64::from(self.framebuffer.width());
        render_clifford(
            &mut self.framebuffer,
            &self.params,
            50_000,
            w * 0.2,
            0xFF_55_FF_AA,
        );

        self.present()
    }
}

#[cfg(not(feature = "nova"))]
fn print_missing_feature() {
    use comfy_table::{Cell, Color, Table, presets};
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Error").fg(Color::Red),
            Cell::new("Missing Feature").fg(Color::Red),
        ])
        .add_row(vec![
            Cell::new("This demo requires an experimental feature to run."),
            Cell::new("nova").fg(Color::Yellow),
        ]);

    println!("\nFailed to Start Demo");
    println!("{table}");
    println!("\nFix: Run with `--features nova`");
}

#[cfg(feature = "nova")]
#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), HostError> {
    print_banner();
    run_windowed(StrangeAttractorDemo::new().unwrap());
    Ok(())
}

#[cfg(not(feature = "nova"))]
fn main() -> Result<(), HostError> {
    print_missing_feature();
    Ok(())
}
