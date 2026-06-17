//! Strange Attractor Demo
//!
//! Renders chaotic attractors like Lorenz and Roessler onto the screen.

#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
#[cfg(feature = "nova")]
use abrash_render::experimental::attractors::{AttractorType, StrangeAttractor};

#[cfg(feature = "nova")]
const WIDTH: u32 = 800;
#[cfg(feature = "nova")]
const HEIGHT: u32 = 600;

#[cfg(feature = "nova")]
struct AttractorDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    attractor: StrangeAttractor,
    time: f32,
}

#[cfg(feature = "nova")]
impl AttractorDemo {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            attractor: StrangeAttractor::default(),
            time: 0.0,
        })
    }
}

#[cfg(feature = "nova")]
impl WindowApp for AttractorDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Strange Attractor Demo".to_string(),
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
        // Slowly rotate the attractor through its chaotic variations over time
        // Just for visual effect in the demo.
        self.time += 0.01;
        self.attractor.attractor_type = AttractorType::Lorenz {
            sigma: 10.0 + self.time.sin() * 2.0,
            rho: 28.0 + (self.time * 0.5).cos() * 5.0,
            beta: 8.0 / 3.0,
        };
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF_00_00_00);
        self.attractor.render(&mut self.framebuffer);

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
cargo run --example attractor_demo --features nova",
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

    println!("\n{}", "🌟 Nova: Strange Attractor Demo".bold().cyan());
    println!("{}", "===============================".dark_grey());

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
            Cell::new("Renders a chaotic strange attractor").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}\n");
}

#[cfg(feature = "nova")]
fn main() {
    print_banner();
    run_windowed(AttractorDemo::new().unwrap());
}
