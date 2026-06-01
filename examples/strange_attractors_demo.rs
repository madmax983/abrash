//! Strange Attractors Demo
//!
//! Renders an animated Clifford Strange Attractor that breathes over time.

#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

#[cfg(feature = "nova")]
use abrash_render::experimental::strange_attractors::{AttractorConfig, render_attractor};

#[cfg(feature = "nova")]
const WIDTH: u32 = 800;
#[cfg(feature = "nova")]
const HEIGHT: u32 = 800;

#[cfg(feature = "nova")]
struct StrangeAttractorsDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    #[cfg(feature = "nova")]
    config: AttractorConfig,
    time: f64,
}

#[cfg(feature = "nova")]
impl StrangeAttractorsDemo {
    fn new() -> Result<Self, HostError> {
        let config = AttractorConfig {
            iterations: 2_000_000,
            intensity: 1.2,
            ..Default::default()
        };

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            #[cfg(feature = "nova")]
            config,
            time: 0.0,
        })
    }
}

#[cfg(feature = "nova")]
impl WindowApp for StrangeAttractorsDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Strange Attractors Demo".to_string(),
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
        #[cfg(feature = "nova")]
        {
            self.time += 0.01;
            // Morph the attractor parameters slightly over time
            self.config.a = -1.24458 + (self.time * 0.5).sin() * 0.1;
            self.config.b = -1.25191 + (self.time * 0.7).cos() * 0.1;
            self.config.c = -1.815_908 + (self.time * 0.3).sin() * 0.1;
            self.config.d = -1.90866 + (self.time * 0.4).cos() * 0.1;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        #[cfg(feature = "nova")]
        {
            // We must clear because we blend additively
            self.framebuffer.clear(0xFF_00_00_00);
            render_attractor(&mut self.framebuffer, &self.config);
        }

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
cargo run --example strange_attractors_demo --features nova",
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

    println!("\n{}", "🌟 Nova: Strange Attractors Demo".bold().cyan());
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
            Cell::new("Animated Clifford Attractor").fg(Color::Green),
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
        .add_row(vec![Cell::new("None"), Cell::new("Observe the animation")]);
    println!("{controls}\n");
}

#[cfg(feature = "nova")]
fn main() {
    print_banner();
    run_windowed(StrangeAttractorsDemo::new().unwrap());
}
