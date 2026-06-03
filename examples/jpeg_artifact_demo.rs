use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::jpeg_artifact::{JpegArtifactConfig, apply_jpeg_artifact};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 JPEG Artifact Demo".bold().cyan());
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
            Cell::new("Simulates blocky compression artifacts and color ringing from low-quality JPEG compression").fg(Color::Green),
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

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "Nova: JPEG Artifact Demo";

struct JpegArtifactDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    config: JpegArtifactConfig,
}

impl JpegArtifactDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Draw a simple, high-contrast gradient background that will show artifacts well
        for y in 0..HEIGHT {
            let t = y as f32 / HEIGHT as f32;
            for x in 0..WIDTH {
                let u = x as f32 / WIDTH as f32;

                // Red/Blue diagonal gradient
                let r = (t * 255.0) as u32;
                let b = (u * 255.0) as u32;
                let color = 0xFF00_0000 | (r << 16) | b;

                // Mix in some sharp high-frequency shapes (rectangles)
                if x > 200 && x < 400 && y > 200 && y < 400 {
                    background_fb.set_pixel(x as i32, y as i32, 0xFFFF_FFFF);
                } else if x > 500 && x < 600 && y > 100 && y < 500 {
                    background_fb.set_pixel(x as i32, y as i32, 0xFF00_FF00);
                } else {
                    background_fb.set_pixel(x as i32, y as i32, color);
                }
            }
        }

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            background_fb,
            config: JpegArtifactConfig {
                block_size: 16, // Extra large blocks for obvious demonstration
                color_levels: 8,
                noise_intensity: 30,
            },
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

impl WindowApp for JpegArtifactDemoApp {
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
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        apply_jpeg_artifact(&mut self.framebuffer, &self.config);

        self.present()
    }
}

#[cfg(not(feature = "nova"))]
fn main() {
    let mut error_table = comfy_table::Table::new();
    error_table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            comfy_table::Cell::new("⚠️  Missing Feature: Nova")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Red),
        ])
        .add_row(vec![
            comfy_table::Cell::new("This example requires the 'nova' feature to run.")
                .fg(comfy_table::Color::White),
        ])
        .add_row(vec![
            comfy_table::Cell::new(
                "Try running with:\ncargo run --example jpeg_artifact_demo --features nova",
            )
            .fg(comfy_table::Color::Green),
        ]);

    eprintln!("\n{error_table}");
    std::process::exit(1);
}

#[cfg(feature = "nova")]
fn main() {
    print_banner();
    run_windowed(JpegArtifactDemoApp::new().unwrap());
}
