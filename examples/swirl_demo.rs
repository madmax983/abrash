#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::unnecessary_wraps)]
//! Demonstration of the Swirl Filter.
//!
//! Creates a colorful grid pattern and applies a continuously twisting Swirl filter.
//!
//! Run with:
//! ```sh
//! cargo run --example swirl_demo --no-default-features --features "backend-tui parallel nova" --release
//! ```

use abrash::experimental::swirl::{SwirlConfig, apply_swirl};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use std::f32::consts::PI;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "🌀 Swirl Filter Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Twisting swirl screen distortion").fg(Color::Green),
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
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![Cell::new("Mouse"), Cell::new("None")])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-twisting")]);
    println!("{controls}\n");
}

struct SwirlApp {
    framebuffer: Framebuffer,
    original_fb: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    angle: f32,
}

impl SwirlApp {
    fn new() -> Result<Self, HostError> {
        let width = 800;
        let height = 600;

        let framebuffer = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create framebuffer: {e:?}")))?;

        let mut original_fb = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create original framebuffer: {e:?}")))?;

        for y in 0..height {
            for x in 0..width {
                let cx = x / 50;
                let cy = y / 50;
                let is_white = (cx + cy) % 2 == 0;
                let color = if is_white {
                    0xFF_FF_FF_FF
                } else {
                    0xFF_00_00_AA
                };
                original_fb.set_pixel(x as i32, y as i32, color);
                // Draw a red center line to make the swirl obvious
                if (x == 400 && y > 100 && y < 500) || (y == 300 && x > 100 && x < 700) {
                    original_fb.set_pixel(x as i32, y as i32, 0xFF_FF_00_00);
                }
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

impl WindowApp for SwirlApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Swirl Demo".to_string(),
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
        let twist = self.angle.sin() * PI; // oscillate between -PI and +PI

        // Reset framebuffer manually using slice copy
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.original_fb.as_slice());

        let config = SwirlConfig {
            center_x: 0.5,
            center_y: 0.5,
            radius: 250.0,
            angle: twist,
        };

        apply_swirl(&mut self.framebuffer, &config);

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
    run_windowed(SwirlApp::new().unwrap())
}
