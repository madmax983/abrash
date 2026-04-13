use abrash::experimental::physarum::PhysarumSimulation;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_core::utils::XorShift32;
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
    println!("\n{}", "✨ Physarum Simulation Demo".bold().cyan());
    println!("{}", "==========================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Resolution"),
            Cell::new(format!("{WIDTH}x{HEIGHT}")).fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Effect"),
            Cell::new("Physarum / Slime Mold Network").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

struct PhysarumDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    sim: PhysarumSimulation,
    rng: XorShift32,
}

impl PhysarumDemoApp {
    fn new() -> Result<Self, AppError> {
        let mut rng = XorShift32::new(1337);
        let sim = PhysarumSimulation::new(WIDTH as usize, HEIGHT as usize, 10000, &mut rng);

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            sim,
            rng,
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

impl WindowApp for PhysarumDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Physarum Simulation".to_string(),
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
        self.sim.step(&mut self.rng);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.sim.render(&mut self.framebuffer);
        self.present()
    }
}

fn main() -> Result<(), AppError> {
    print_banner();
    run_windowed(PhysarumDemoApp::new().unwrap());
    Ok(())
}
