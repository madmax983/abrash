#![cfg(feature = "backend-winit")]
//! Demonstration of the Nova Kintsugi filter.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::experimental::kintsugi::{KintsugiConfig, apply_kintsugi};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "✨ Kintsugi Filter Demo".bold().yellow());
    println!("{}", "=======================".dark_grey());

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
            Cell::new("Repairs edges with gold while desaturating").fg(Color::Green),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating shapes")]);
    println!("{controls}\n");
}

struct KintsugiApp {
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    angle: f32,
}

impl KintsugiApp {
    fn new() -> Result<Self, HostError> {
        let width = 800;
        let height = 600;

        let framebuffer = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create framebuffer: {e}")))?;

        Ok(Self {
            framebuffer,
            presenter: None,
            angle: 0.0,
        })
    }
}

impl WindowApp for KintsugiApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Kintsugi Filter Demo (Nova)".to_string(),
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
        self.angle += 0.02;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();

        // Draw a base image: let's draw some colorful overlapping circles
        self.framebuffer.clear(0xFF_22_33_44); // Dark blueish background

        // Draw circles that move around
        let cx1 = (width / 2) as i32 + (self.angle.cos() * 150.0) as i32;
        let cy1 = (height / 2) as i32 + (self.angle.sin() * 100.0) as i32;

        let cx2 = (width / 2) as i32 + ((self.angle * 1.5).sin() * 200.0) as i32;
        let cy2 = (height / 2) as i32 + ((self.angle * 0.8).cos() * 150.0) as i32;

        let cx3 = (width / 2) as i32 + ((self.angle * 0.5).cos() * 100.0) as i32;
        let cy3 = (height / 2) as i32 + ((self.angle * 1.2).sin() * 200.0) as i32;

        for y in 0..height {
            for x in 0..width {
                let xi = x as i32;
                let yi = y as i32;

                // Background pattern
                if (x / 40 + y / 40) % 2 == 0 {
                    self.framebuffer.set_pixel(xi, yi, 0xFF_11_22_33);
                }

                // Circle 1
                if (xi - cx1) * (xi - cx1) + (yi - cy1) * (yi - cy1) < 10000 {
                    self.framebuffer.set_pixel(xi, yi, 0xFF_FF_44_44);
                }
                // Circle 2
                if (xi - cx2) * (xi - cx2) + (yi - cy2) * (yi - cy2) < 15000 {
                    self.framebuffer.set_pixel(xi, yi, 0xFF_44_FF_44);
                }
                // Circle 3
                if (xi - cx3) * (xi - cx3) + (yi - cy3) * (yi - cy3) < 8000 {
                    self.framebuffer.set_pixel(xi, yi, 0xFF_44_44_FF);
                }
            }
        }

        let config = KintsugiConfig::default();

        // Apply the kintsugi filter
        apply_kintsugi(&mut self.framebuffer, &config);

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), HostError> {
    print_banner();
    run_windowed(KintsugiApp::new()?);
    Ok(())
}
