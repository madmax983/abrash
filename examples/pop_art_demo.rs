#![cfg(feature = "backend-winit")]
//! Demonstration of the Nova Pop Art filter.

use abrash::experimental::pop_art::{PopArtConfig, apply_pop_art};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "🎨 Pop Art Filter Demo".bold().magenta());
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
            Cell::new("Andy Warhol-style quadrant effect").fg(Color::Green),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-animating")]);
    println!("{controls}\n");
}

struct PopArtApp {
    framebuffer: Framebuffer,
    source_fb: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    angle: f32,
}

impl PopArtApp {
    fn new() -> Result<Self, HostError> {
        let width = 800;
        let height = 600;

        let framebuffer = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create framebuffer: {e:?}")))?;

        let source_fb = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create source framebuffer: {e:?}")))?;

        Ok(Self {
            framebuffer,
            source_fb,
            presenter: None,
            angle: 0.0,
        })
    }
}

impl WindowApp for PopArtApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Pop Art Filter Demo (Nova)".to_string(),
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
        // Draw a simple moving pattern to the source framebuffer
        let cx = 400.0 + (self.angle * 0.5).cos() * 200.0;
        let cy = 300.0 + (self.angle * 0.3).sin() * 150.0;

        let width = self.source_fb.width();
        let height = self.source_fb.height();
        let pixels = self.source_fb.as_mut_slice();

        for y in 0..height {
            for x in 0..width {
                // Checkerboard background
                let is_white = ((x / 40) + (y / 40)) % 2 == 0;
                let mut color = if is_white { 0xFF_DDDDDD } else { 0xFF_222222 };

                // Moving circle
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                if dx * dx + dy * dy < 10000.0 {
                    color = 0xFF_EEEEEE; // Bright circle
                }

                pixels[(y * width + x) as usize] = color;
            }
        }

        // Apply the pop art filter
        let config = PopArtConfig::default();
        apply_pop_art(&mut self.framebuffer, &self.source_fb, &config);

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
    run_windowed(PopArtApp::new().unwrap());
}
