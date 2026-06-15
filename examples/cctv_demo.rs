//! CCTV Security Camera Filter Demo

use abrash::framebuffer::Framebuffer;

use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::cctv::{CctvConfig, apply_cctv};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: CCTV Filter Demo";

struct CctvDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    config: CctvConfig,
    time: f32,
}

impl CctvDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Draw a simple scene to apply CCTV effect to
        background_fb.clear(0xFF_44_44_44); // Dark grey background

        let width = WIDTH as usize;
        let height = HEIGHT as usize;

        // Draw a "room" structure
        for y in 0..height {
            for x in 0..width {
                // Checkered floor
                if y > height / 2 {
                    let is_white = ((x / 40) + (y / 40)) % 2 == 0;
                    if is_white {
                        background_fb.set_pixel(x as i32, y as i32, 0xFF_88_88_88);
                    } else {
                        background_fb.set_pixel(x as i32, y as i32, 0xFF_66_66_66);
                    }
                }

                // Wall
                if y <= height / 2 && (x % 100 == 0 || y % 100 == 0) {
                    background_fb.set_pixel(x as i32, y as i32, 0xFF_55_55_55);
                }
            }
        }

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            background_fb,
            config: CctvConfig::default(),
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

impl WindowApp for CctvDemoApp {
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
        self.time += ctx.dt_seconds;
        self.config.time = self.time;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        let width = WIDTH as usize;
        let height = HEIGHT as usize;

        // Draw a moving "intruder" block
        let cx = (width as f32 / 2.0 + (self.config.time * 2.0).sin() * 200.0) as usize;
        let cy = height / 2 - 50;

        for dy in 0..100 {
            for dx in 0..50 {
                let x = cx + dx;
                let y = cy + dy;
                if x < width && y < height {
                    self.framebuffer
                        .set_pixel(x as i32, y as i32, 0xFF_AA_33_33);
                }
            }
        }

        apply_cctv(&mut self.framebuffer, &self.config);

        self.present()
    }
}

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "📹 CCTV Security Camera Filter Demo".bold().cyan());
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
            Cell::new("Security camera effect with distortion, scanlines, and timestamp")
                .fg(Color::Green),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("None")]);
    println!("{controls}\n");
}

fn main() {
    #[cfg(feature = "nova")]
    {
        print_banner();
        run_windowed(CctvDemoApp::new().unwrap());
    }
    #[cfg(not(feature = "nova"))]
    {
        use comfy_table::{Cell, Color, Table, presets};
        let mut error_table = Table::new();
        error_table
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("⚠️  Missing Feature: Nova")
                    .add_attribute(comfy_table::Attribute::Bold)
                    .fg(Color::Red),
            ])
            .add_row(vec![
                Cell::new("This example requires the 'nova' feature to run.").fg(Color::White),
            ])
            .add_row(vec![
                Cell::new("Try running with:\ncargo run --example cctv_demo --features nova")
                    .fg(Color::Green),
            ]);

        eprintln!("\n{error_table}");
        std::process::exit(1);
    }
}
