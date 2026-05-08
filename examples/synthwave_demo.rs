#![cfg(feature = "backend-winit")]
//! Demonstration of the Nova Synthwave filter.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::synthwave::{SynthwaveConfig, apply_synthwave};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "🌆 Synthwave Filter Demo".bold().magenta());
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
            Cell::new("Retro-futuristic 3D wireframe and sun").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Procedural Post-Process").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .add_row(vec![
            Cell::new("Any Key"),
            Cell::new("No interaction, sit back and relax"),
        ]);
    println!("{controls}");
}

struct SynthwaveApp {
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    config: SynthwaveConfig,
}

impl SynthwaveApp {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            framebuffer: Framebuffer::new(800, 600)?,
            presenter: None,
            config: SynthwaveConfig::default(),
        })
    }
}

impl WindowApp for SynthwaveApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash Engine - Synthwave".to_string(),
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
        self.config.time += 0.016; // Roughly 60fps
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        apply_synthwave(&mut self.framebuffer, &self.config);

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

fn main() {
    print_banner();
    run_windowed(SynthwaveApp::new().unwrap());
}
