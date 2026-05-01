//! Starfield Demo
//!
//! Renders a 3D flying starfield using the experimental Starfield effect.

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
    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Starfield Demo".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Box<dyn std::error::Error>> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Box<dyn std::error::Error>> {
        let steps = self.timestep.update();
        let dt = self.timestep.dt();
        for _ in 0..steps {
            self.starfield.update(dt);
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Box<dyn std::error::Error>> {
        self.fb.clear(0xFF00_0000);

        self.starfield.render(&mut self.fb);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.fb)?;
        }
        Ok(())
    }
}

#[cfg(feature = "nova")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();

    let app = App {
        fb: Framebuffer::new(WIDTH, HEIGHT).map_err(|e| e.to_string())?,
        timestep: FixedTimestep::new(60),
        starfield: Starfield::new(1000, 20.0, 100.0),
        presenter: None,
    };

    run_windowed(app);
    Ok(())
}

#[cfg(not(feature = "nova"))]
fn main() {
    use comfy_table::{Cell, Color, Table, presets};
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
            Cell::new("Try running with:\ncargo run --example starfield_demo --features nova")
                .fg(Color::Green),
        ]);

    eprintln!("\n{error_table}");
    std::process::exit(1);
}
