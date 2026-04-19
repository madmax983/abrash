//! Starfield Demo
//!
//! Renders a 3D flying starfield using the experimental Starfield effect.

use std::fmt;
use std::io::Error as IoError;

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::time::FixedTimestep;

#[cfg(feature = "nova")]
use abrash_render::experimental::starfield::Starfield;

#[cfg(feature = "nova")]
use crossterm::style::Stylize;

use comfy_table::{Cell, Color, Table, presets};

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

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Starfield Demo".bold().cyan());
    println!("{}", "=================".dark_grey());

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
            Cell::new("Renders a classic 3D retro starfield").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Feature"),
            Cell::new("nova").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
    println!();
}

#[cfg(feature = "nova")]
struct App {
    fb: Framebuffer,
    timestep: FixedTimestep,
    starfield: Starfield,
    presenter: Option<SoftwarePresenter>,
}

#[cfg(feature = "nova")]
impl WindowApp for App {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Starfield Demo".to_string(),
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
        let dt = self.timestep.dt();
        for _ in 0..steps {
            self.starfield.update(dt);
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.fb.clear(0xFF00_0000);

        self.starfield.render(&mut self.fb);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.fb)?;
        }
        Ok(())
    }
}

#[cfg(feature = "nova")]
#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), AppError> {
    print_banner();

    let app = App {
        fb: Framebuffer::new(WIDTH, HEIGHT).map_err(|e| AppError(e.to_string()))?,
        timestep: FixedTimestep::new(60),
        starfield: Starfield::new(1000, 20.0, 100.0),
        presenter: None,
    };

    run_windowed(app);
    Ok(())
}

#[cfg(not(feature = "nova"))]
fn main() {
    println!(
        "This demo requires the 'nova' feature. Run with `cargo run --example starfield_demo --features nova`"
    );
}
