//! Demonstration of the Speed Lines Filter.
//!
//! Creates an anime-style speed lines effect radiating from the center.
//!
//! Run with:
//! ```sh
//! cargo run --example speed_lines_demo --features "backend-winit parallel nova" --release
//! ```

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::speed_lines::{SpeedLinesConfig, apply_speed_lines};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "⚡ Speed Lines Filter Demo".bold().cyan());
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
            Cell::new("Description"),
            Cell::new("Anime-style radial speed lines").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Software Rasterizer + Post-Process").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-animated")]);
    println!("{controls}\n");
}

struct SpeedLinesApp {
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    time: f32,
}

impl SpeedLinesApp {
    fn new() -> Result<Self, HostError> {
        let width = 800;
        let height = 600;

        let framebuffer = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create framebuffer: {e:?}")))?;

        Ok(Self {
            framebuffer,
            presenter: None,
            time: 0.0,
        })
    }
}

impl WindowApp for SpeedLinesApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Speed Lines Demo".to_string(),
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
        self.time += 0.05;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Draw background
        self.framebuffer.clear(0xFF10_1030); // Dark blue

        // Render subject (a red circle)
        let cx = 400.0;
        let cy = 300.0;
        let r2 = 50.0 * 50.0;
        for y in 200..400 {
            for x in 300..500 {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                if dx * dx + dy * dy < r2 {
                    self.framebuffer.set_pixel(x, y, 0xFFFF_0000); // Red
                }
            }
        }

        let config = SpeedLinesConfig {
            center_x: 0.5,
            center_y: 0.5,
            inner_radius: 120.0,
            time: self.time,
            line_color: 0xFFFF_FFFF,
            density: 180,
            min_length: 50.0,
            max_length: 400.0,
        };

        apply_speed_lines(&mut self.framebuffer, &config);

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
    run_windowed(SpeedLinesApp::new().unwrap());
}
