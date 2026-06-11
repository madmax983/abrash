#![cfg(feature = "backend-winit")]

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::physarum::{PhysarumConfig, apply_physarum};
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Physarum Slime Mold Demo";

struct PhysarumDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    config: PhysarumConfig,
}

impl PhysarumDemoApp {
    pub fn new() -> Self {
        Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            config: PhysarumConfig::default(),
        }
    }
}

impl PhysarumDemoApp {
    fn present(&mut self) -> Result<(), HostError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for PhysarumDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window.clone())?);
        self.framebuffer.clear(0xFF_000000);
        Ok(())
    }

    fn update(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        // We only clear a small amount of the framebuffer to let the trail decay naturally
        // or let the simulation handle the decay entirely
        Ok(())
    }

    fn render(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF_000000); // the simulation keeps its own trail map
        apply_physarum(&mut self.framebuffer, &self.config);
        self.present()
    }
}

fn print_banner() {
    println!("\n{}", "🌟 Physarum Slime Mold Demo".bold().cyan());
    println!("{}", "==============================".dark_grey());

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
            Cell::new("Procedural slime mold agent simulation").fg(Color::Green),
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
        .add_row(vec![Cell::new("None"), Cell::new("Watch it evolve")]);
    println!("{controls}\n");
}

fn main() {
    print_banner();
    run_windowed(PhysarumDemoApp::new());
}
