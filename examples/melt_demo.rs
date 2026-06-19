use abrash::framebuffer::Framebuffer;
use abrash::math::Vec2;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::melt::{MeltConfig, apply_melt};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Screen Melt Filter Demo";

struct MeltDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    config: MeltConfig,
    time: f32,
}

impl MeltDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Draw a simple scene to melt
        // Sky
        for y in 0..HEIGHT / 2 {
            for x in 0..WIDTH {
                let color = 0xFF_87_CE_EB; // Sky blue
                background_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        // Ground
        for y in HEIGHT / 2..HEIGHT {
            for x in 0..WIDTH {
                let color = 0xFF_8B_45_13; // Saddle brown
                background_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        // Sun
        let center = Vec2::new(WIDTH as f32 * 0.8, HEIGHT as f32 * 0.3);
        let radius = 50.0;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let p = Vec2::new(x as f32, y as f32);
                if (p - center).length() < radius {
                    background_fb.set_pixel(x as i32, y as i32, 0xFF_FF_D7_00); // Gold
                }
            }
        }

        // Title text approximation (just a block for effect)
        for y in 50..100 {
            for x in 50..400 {
                background_fb.set_pixel(x, y, 0xFF_DD_22_22); // Red block
            }
        }

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            background_fb,
            config: MeltConfig::new(150.0, 0xFF_11_11_11),
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

impl WindowApp for MeltDemoApp {
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
        // Wait a second before melting
        self.time += ctx.dt_seconds;

        if self.time > 1.0 {
            self.config.time = self.time - 1.0;
        }

        // Restart melt after a while
        if self.time > 6.0 {
            self.time = 0.0;
            self.config.reset();
        }

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        apply_melt(&mut self.framebuffer, &mut self.config);

        self.present()
    }
}

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🫠 Screen Melt Demo".bold().cyan());
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
            Cell::new("Classic DOOM-style screen melt effect").fg(Color::Green),
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
        match MeltDemoApp::new() {
            Ok(app) => run_windowed(app),
            Err(e) => {
                let mut error_table = comfy_table::Table::new();
                error_table
                    .load_preset(comfy_table::presets::UTF8_FULL)
                    .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        comfy_table::Cell::new("❌ Initialization Error")
                            .add_attribute(comfy_table::Attribute::Bold)
                            .fg(comfy_table::Color::Red),
                    ])
                    .add_row(vec![
                        comfy_table::Cell::new(format!("{e}")).fg(comfy_table::Color::Yellow),
                    ]);
                eprintln!("\n{error_table}");
                std::process::exit(1);
            }
        }
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
                Cell::new("Try running with:\ncargo run --example melt_demo --features nova")
                    .fg(Color::Green),
            ]);

        eprintln!("\n{error_table}");
        std::process::exit(1);
    }
}
