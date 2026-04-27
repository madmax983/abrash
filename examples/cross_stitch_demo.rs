#![cfg(feature = "backend-winit")]
//! Demonstration of the Nova Cross Stitch filter.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::experimental::cross_stitch::{CrossStitchConfig, apply_cross_stitch};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "🧵 Cross Stitch Filter Demo".bold().cyan());
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
            Cell::new("Digital cross-stitch pattern").fg(Color::Green),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-spinning visual")]);
    println!("{controls}\n");
}

struct CrossStitchApp {
    framebuffer: Framebuffer,
    original_fb: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    angle: f32,
}

impl CrossStitchApp {
    fn new() -> Result<Self, HostError> {
        let width = 800;
        let height = 600;

        let framebuffer = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create framebuffer: {e}")))?;

        let mut original_fb = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create original framebuffer: {e}")))?;

        // Checkerboard pattern
        for y in 0..height {
            for x in 0..width {
                let cx = x / 100;
                let cy = y / 100;
                let is_white = (cx + cy) % 2 == 0;
                let color = if is_white {
                    0xFF_DD_EE_FF
                } else {
                    0xFF_22_44_88
                };
                original_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        Ok(Self {
            framebuffer,
            original_fb,
            presenter: None,
            angle: 0.0,
        })
    }
}

impl WindowApp for CrossStitchApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Cross Stitch Filter Demo (Nova)".to_string(),
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
        self.angle += 0.05;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.original_fb.as_slice());

        // Draw a dynamic circle using the angle
        let cx = 400.0 + (self.angle.cos() * 150.0);
        let cy = 300.0 + (self.angle.sin() * 150.0);
        let radius = 100.0;

        let _width = self.framebuffer.width() as f32;
        let _height = self.framebuffer.height() as f32;

        for y in 0..self.framebuffer.height() {
            for x in 0..self.framebuffer.width() {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                if dx * dx + dy * dy < radius * radius {
                    self.framebuffer.set_pixel(x as i32, y as i32, 0xFF_FF_55_55);
                }
            }
        }

        let config = CrossStitchConfig {
            block_size: 10,
            stroke_thickness: 1,
            canvas_color: 0xFF_11_11_11,
        };

        apply_cross_stitch(&mut self.framebuffer, &config);

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
    run_windowed(CrossStitchApp::new().unwrap());
}
