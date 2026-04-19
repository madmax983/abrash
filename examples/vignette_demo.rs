#![cfg(feature = "backend-winit")]
//! Demonstration of the Nova Vignette filter.

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::post_process::{VignetteConfig, apply_vignette};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "📷 Vignette Filter Demo".bold().cyan());
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
            Cell::new("Cinematic edge-darkening effect").fg(Color::Green),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-pulsating")]);
    println!("{controls}\n");
}

struct VignetteApp {
    framebuffer: Framebuffer,
    original_fb: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    angle: f32,
}

impl VignetteApp {
    fn new() -> Result<Self, HostError> {
        let width = 800;
        let height = 600;

        let framebuffer = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create framebuffer: {e}")))?;

        let mut original_fb = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create original framebuffer: {e}")))?;

        for y in 0..height {
            for x in 0..width {
                let cx = x / 50;
                let cy = y / 50;
                let is_white = (cx + cy) % 2 == 0;
                let color = if is_white {
                    0xFF_DD_EE_FF
                } else {
                    0xFF_22_44_88
                };
                original_fb.set_pixel(x as i32, y as i32, color);
                // Draw a center element
                if (x - 400) * (x - 400) + (y - 300) * (y - 300) < 10000 {
                    original_fb.set_pixel(x as i32, y as i32, 0xFF_FF_AA_00);
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

impl WindowApp for VignetteApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Vignette Filter Demo (Nova)".to_string(),
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
        // Reset framebuffer manually using slice copy
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.original_fb.as_slice());

        // Pulsate the intensity over time to make the demo dynamic
        let intensity = 0.7 + (self.angle * 0.5).sin() * 0.3; // Ranges from 0.4 to 1.0

        // Oscillate roundness
        let roundness = 0.5 + (self.angle * 0.3).cos() * 0.2; // 0.3 to 0.7

        let config = VignetteConfig {
            intensity,
            roundness,
        };

        // Apply the vignette filter
        apply_vignette(&mut self.framebuffer, &config);

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
    run_windowed(VignetteApp::new().unwrap());
}
