//! Demonstration of the Frosted Glass Filter.
//!
//! Applies a frosted privacy glass displacement filter to the screen.
//!
//! Run with:
//! ```sh
//! cargo run --example frosted_glass_demo --no-default-features --features "backend-tui parallel nova" --release
//! ```

use abrash::experimental::frosted_glass::apply_frosted_glass;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "❄️ Frosted Glass Demo".bold().cyan());
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
            Cell::new("Privacy frosted glass spatial displacement").fg(Color::Green),
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
            Cell::new("Auto-updating noise"),
        ]);
    println!("{controls}\n");
}

struct FrostedGlassApp {
    framebuffer: Framebuffer,
    original_fb: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    frame_count: u64,
}

impl FrostedGlassApp {
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
                // Draw a red center line to make the displacement obvious
                if (x == 400 && y > 100 && y < 500) || (y == 300 && x > 100 && x < 700) {
                    original_fb.set_pixel(x as i32, y as i32, 0xFF_FF_00_00);
                }
            }
        }

        Ok(Self {
            framebuffer,
            original_fb,
            presenter: None,
            frame_count: 0,
        })
    }
}

impl WindowApp for FrostedGlassApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Frosted Glass Demo".to_string(),
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
        self.frame_count = self.frame_count.wrapping_add(1);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Reset framebuffer manually using slice copy
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.original_fb.as_slice());

        // Animate the seed so the noise changes every frame, simulating moving glass/light
        let seed = 1337 + self.frame_count;
        let intensity = 10.0; // 10 pixels displacement

        apply_frosted_glass(&mut self.framebuffer, intensity, seed);

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
    run_windowed(FrostedGlassApp::new().unwrap())
}
