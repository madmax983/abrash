use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::halftone::{HalftoneConfig, apply_halftone};
use std::f32::consts::PI;

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Halftone Filter Demo".bold().cyan());
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
            Cell::new("Applies a comic-book style halftone dot pattern").fg(Color::Green),
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
            Cell::new("Close window to exit"),
        ]);
    println!("{controls}\n");
}

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Halftone Filter Demo";

struct HalftoneDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    time: f32,
}

impl HalftoneDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Create a simple gradient background for testing the halftone effect
        for y in 0..HEIGHT {
            let v = y as f32 / HEIGHT as f32;
            for x in 0..WIDTH {
                let u = x as f32 / WIDTH as f32;

                let r = (u * 255.0) as u32;
                let g = (v * 255.0) as u32;
                let b = ((1.0 - u) * 255.0) as u32;

                let color = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
                background_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        // Add some shapes
        for y in 100..300 {
            for x in 100..300 {
                let dx = x as f32 - 200.0;
                let dy = y as f32 - 200.0;
                if dx * dx + dy * dy < 10000.0 {
                    background_fb.set_pixel(x, y, 0xFF_FF_FF_FF);
                }
            }
        }

        for y in 200..400 {
            for x in 300..500 {
                let dx = x as f32 - 400.0;
                let dy = y as f32 - 300.0;
                if dx * dx + dy * dy < 8000.0 {
                    background_fb.set_pixel(x, y, 0xFF_00_00_00);
                }
            }
        }

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            background_fb,
            time: 0.0,
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

impl WindowApp for HalftoneDemoApp {
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

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds.max(0.0);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        let dot_size = 4.0 + (self.time.sin() * 0.5 + 0.5) * 6.0;
        let angle = PI / 4.0 + self.time * 0.5;

        apply_halftone(
            &mut self.framebuffer,
            &HalftoneConfig {
                dot_size,
                angle_radians: angle,
            },
        );
        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(HalftoneDemoApp::new().unwrap());
}
