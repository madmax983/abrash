//! # Pop Art Demo
//!
//! Demonstrates the experimental Pop Art post-processing filter.
//! Creates an Andy Warhol inspired 4-quadrant recoloring effect.

use abrash::experimental::pop_art::{PopArtConfig, apply_pop_art};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Pop Art Filter Demo";

struct PopArtApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    config: PopArtConfig,
}

impl PopArtApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Generate a simple gradient test pattern for the base image
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                // Diagonal gradient from black to white
                let val =
                    ((x as f32 / WIDTH as f32 + y as f32 / HEIGHT as f32) / 2.0 * 255.0) as u32;
                // Add some shapes
                let dx = x as f32 - WIDTH as f32 / 2.0;
                let dy = y as f32 - HEIGHT as f32 / 2.0;
                let dist = (dx * dx + dy * dy).sqrt();

                let mut final_val = val;
                if dist < 100.0 {
                    final_val = 255 - val; // Invert inside circle
                }

                let color = 0xFF_000000 | (final_val << 16) | (final_val << 8) | final_val;
                background_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            background_fb,
            config: PopArtConfig::default(),
        })
    }

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

impl WindowApp for PopArtApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Copy base image to framebuffer
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        // Apply effect
        apply_pop_art(&mut self.framebuffer, &self.config);

        self.present()
    }
}

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🎨 Pop Art Filter Demo".bold().cyan());
    println!("{}", "=======================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Andy Warhol inspired 4-quadrant recoloring").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Software Rasterizer + Post-Process").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

fn main() {
    #[cfg(feature = "nova")]
    {
        print_banner();
        run_windowed(PopArtApp::new().unwrap())
    }
    #[cfg(not(feature = "nova"))]
    {
        println!("This example requires the 'nova' feature to be enabled.");
        println!("Run with: cargo run --example pop_art_demo --features nova");
    }
}
