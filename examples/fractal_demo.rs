#![cfg(feature = "backend-winit")]

#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
#[cfg(feature = "nova")]
use abrash_render::experimental::fractal::render_mandelbrot;

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
const WIDTH: u32 = 640;
#[cfg(feature = "nova")]
const HEIGHT: u32 = 480;
#[cfg(feature = "nova")]
const TITLE: &str = "Nova: Fractal Explorer Demo";

#[cfg(feature = "nova")]
struct FractalDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    time: f32,
}

#[cfg(feature = "nova")]
impl FractalDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
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

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Fractal Explorer Demo".bold().cyan());
    println!("{}", "========================".dark_grey());

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
            Cell::new("Interactive Mandelbrot set rendering!").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Software Rasterizer + Nova").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Feature"), Cell::new("Auto-Zoom (Time)")]);
    println!("{controls}\n");
}

#[cfg(feature = "nova")]
impl WindowApp for FractalDemoApp {
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
        // Simple auto-zoom logic towards an interesting point in the Mandelbrot set
        let target_x = -0.743_643_887_037_151;
        let target_y = 0.131_825_904_205_33;

        let elapsed = f64::from(self.time);

        // Starts at zoom 1.5, scales down over time
        let zoom = 1.5 * ((-elapsed * 0.1).exp());

        // Adjust max iterations based on zoom to maintain detail without blowing up performance
        let max_iter = 100 + (elapsed * 5.0) as u32;

        self.framebuffer.clear(0xFF_00_00_00);
        render_mandelbrot(&mut self.framebuffer, target_x, target_y, zoom, max_iter);
        self.present()
    }
}

#[cfg(feature = "nova")]
fn main() {
    print_banner();
    run_windowed(FractalDemoApp::new().unwrap());
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
                "Try running with:\ncargo run --example fractal_demo --features nova",
            )
            .fg(comfy_table::Color::Green),
        ]);
    eprintln!("\n{error_table}");
    std::process::exit(1);
}
