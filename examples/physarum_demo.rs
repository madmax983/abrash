//! Physarum (Slime Mold) Demo
//!
//! Visualizes the experimental Physarum (Slime Mold) simulation using agents and trail diffusion.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

use comfy_table::{Cell, Color, Table, presets};

#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
use abrash_render::experimental::physarum::{Physarum, PhysarumConfig};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Physarum (Slime Mold) Simulation".bold().cyan());
    println!("{}", "===================================".dark_grey());

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
            Cell::new("Simulates thousands of agents following and depositing trails.")
                .fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Feature"),
            Cell::new("Physarum (Slime Mold) Algorithm").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![Cell::new("None"), Cell::new("Watch it grow")]);
    println!("{controls}\n");
}

#[cfg(feature = "nova")]
struct PhysarumDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    sim: Physarum,
}

#[cfg(feature = "nova")]
impl PhysarumDemo {
    fn new() -> Result<Self, HostError> {
        let config = PhysarumConfig::default();
        let sim = Physarum::new(WIDTH as usize, HEIGHT as usize, config);

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            sim,
        })
    }
}

#[cfg(feature = "nova")]
impl WindowApp for PhysarumDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Physarum Simulation".to_string(),
            width: self.framebuffer.width(),
            height: self.framebuffer.height(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        self.framebuffer =
            Framebuffer::new(width, height).map_err(|error| HostError::App(error.to_string()))?;
        self.sim = Physarum::new(width as usize, height as usize, PhysarumConfig::default());
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.sim.step();
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.sim
            .render(&mut self.framebuffer, 0xFF_000000, 0xFF_00FF88);

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

// Fallback for when "nova" feature is not enabled
#[cfg(not(feature = "nova"))]
fn main() {
    let mut error_table = Table::new();
    error_table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("⚠️  Missing Feature: Nova")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(Color::Red),
        ])
        .add_row(vec![
            Cell::new("This example requires the 'nova' feature to run.").fg(Color::White),
        ])
        .add_row(vec![
            Cell::new("Try running with:\ncargo run --example physarum_demo --features nova")
                .fg(Color::Green),
        ]);

    eprintln!("\n{error_table}");
    std::process::exit(1);
}

#[cfg(feature = "nova")]
fn main() {
    print_banner();
    run_windowed(PhysarumDemo::new().unwrap());
}
