//! Mandelbrot Demo
//!
//! Renders the Mandelbrot set and allows panning and zooming.

#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed_app,
};

#[cfg(feature = "nova")]
use abrash_render::experimental::mandelbrot::{MandelbrotConfig, render_mandelbrot};

#[cfg(feature = "nova")]
const WIDTH: u32 = 800;
#[cfg(feature = "nova")]
const HEIGHT: u32 = 600;

#[cfg(feature = "nova")]
struct MandelbrotDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    #[cfg(feature = "nova")]
    config: MandelbrotConfig,
}

#[cfg(feature = "nova")]
impl MandelbrotDemo {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            #[cfg(feature = "nova")]
            config: MandelbrotConfig::default(),
        })
    }
}

#[cfg(feature = "nova")]
impl WindowApp for MandelbrotDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Mandelbrot Demo".to_string(),
            width: self.framebuffer.width(),
            height: self.framebuffer.height(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        self.framebuffer =
            Framebuffer::new(width, height).map_err(|error| HostError::App(error.to_string()))?;
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Here we would handle input for zooming/panning
        // To keep it simple for now, we'll auto-zoom
        #[cfg(feature = "nova")]
        {
            self.config.zoom *= 1.01;
            // self.config.center_x += 0.001 / self.config.zoom;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        #[cfg(feature = "nova")]
        render_mandelbrot(&mut self.framebuffer, &self.config);

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

// Fallback for when "nova" feature is not enabled
#[cfg(not(feature = "nova"))]
fn main() {
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
            Cell::new(
                "Try running with:
cargo run --example mandelbrot_demo --features nova",
            )
            .fg(Color::Green),
        ]);

    eprintln!(
        "
{error_table}"
    );
    std::process::exit(1);
}

#[cfg(feature = "nova")]
fn print_banner() {
    use comfy_table::{Cell, Color, Table, presets};
    use crossterm::style::Stylize;

    println!("\n{}", "🌟 Nova: Mandelbrot Demo".bold().cyan());
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
            Cell::new("Renders the Mandelbrot set with auto-zooming").fg(Color::Green),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-zooming")]);
    println!("{controls}\n");
}

#[cfg(feature = "nova")]
fn main() {
    print_banner();
    run_windowed_app(
        MandelbrotDemo::new().map_err(|e| abrash::platform::HostError::App(e.to_string())),
    );
}
