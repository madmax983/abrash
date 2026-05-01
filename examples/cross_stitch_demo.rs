#![cfg(feature = "backend-winit")]
//! Demonstration of the Nova Cross-Stitch filter.

use abrash::experimental::cross_stitch::{CrossStitchConfig, apply_cross_stitch};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "🧶 Cross-Stitch Filter Demo".bold().cyan());
    println!("{}", "===========================".dark_grey());

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
            Cell::new("Converts the scene into an embroidery cross-stitch canvas").fg(Color::Green),
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
        .add_row(vec![Cell::new("Mouse"), Cell::new("None")])
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Auto-animated colors"),
        ]);
    println!("{controls}\n");
}

struct CrossStitchApp {
    framebuffer: Framebuffer,
    original_fb: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    time: f32,
}

impl CrossStitchApp {
    fn new() -> Result<Self, HostError> {
        let width = 800;
        let height = 600;

        let framebuffer = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create framebuffer: {e}")))?;

        let original_fb = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create original framebuffer: {e}")))?;

        Ok(Self {
            framebuffer,
            original_fb,
            presenter: None,
            time: 0.0,
        })
    }

    fn render_scene(&mut self) {
        self.original_fb.clear(0xFF_222222);

        let width = self.original_fb.width();
        let height = self.original_fb.height();
        let cx = width / 2;
        let cy = height / 2;

        // Draw a pulsating circle
        let radius = 150.0 + (self.time.sin() * 50.0);

        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - cx as f32;
                let dy = y as f32 - cy as f32;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < radius {
                    let r = ((self.time * 2.0).sin() * 127.0 + 128.0) as u32;
                    let g = ((self.time * 3.0).cos() * 127.0 + 128.0) as u32;
                    let b = ((self.time * 1.5).sin() * 127.0 + 128.0) as u32;
                    let color = 0xFF_000000 | (r << 16) | (g << 8) | b;
                    self.original_fb.set_pixel(x as i32, y as i32, color);
                } else if dist < radius + 20.0 {
                    self.original_fb.set_pixel(x as i32, y as i32, 0xFF_FFFFFF);
                }
            }
        }
    }
}

impl WindowApp for CrossStitchApp {
    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Cross-Stitch Filter".to_string(),
            width: 800,
            height: 600,
            ..Default::default()
        }
    }

    fn init(&mut self, context: WindowContext<'_>) -> Result<(), Box<dyn std::error::Error>> {
        self.presenter = Some(SoftwarePresenter::new(context.window.clone())?);
        print_banner();
        Ok(())
    }

    fn update(&mut self, context: WindowContext<'_>) -> Result<(), Box<dyn std::error::Error>> {
        let delta_time = context.dt_seconds;
        self.time += delta_time;

        self.render_scene();

        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.original_fb.as_slice());

        let config = CrossStitchConfig {
            cell_size: 10,
            canvas_color: 0xFF_EFEFEF,
        };

        apply_cross_stitch(&mut self.framebuffer, &config);

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }
        Ok(())
    }
}

#[cfg(feature = "backend-winit")]
fn main() -> Result<(), HostError> {
    let app = CrossStitchApp::new()?;
    let _config = WindowHostConfig {
        title: "Abrash - Cross-Stitch Filter".to_string(),
        width: 800,
        height: 600,
        ..Default::default()
    };

    run_windowed(app);
    Ok(())
}

#[cfg(not(feature = "backend-winit"))]
fn main() {
    use comfy_table::{Cell, Color, Table, presets};

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Error").fg(Color::Red),
            Cell::new("Missing Feature").fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Cannot run graphical demo"),
            Cell::new("The `backend-winit` feature is required."),
        ]);

    println!("{table}");
    println!("Run again with: cargo run --example cross_stitch_demo --features backend-winit,nova");
}
