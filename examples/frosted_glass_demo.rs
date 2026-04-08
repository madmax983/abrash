use abrash::experimental::frosted_glass::apply_frosted_glass;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Frosted Glass Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Simulates looking through frosted or structured glass").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
    println!("\n{}", "🎮 Controls:".bold());
    println!("  • Press {} to exit", "ESC".bold().red());
    println!("  • Watch the radius modulate over time\n");
}

#[cfg(not(feature = "nova"))]
fn print_banner() {}

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "Nova: Frosted Glass Filter Demo";

struct FrostedGlassApp {
    presenter: Option<SoftwarePresenter>,
    fb: Framebuffer,
    time: f32,
}

impl FrostedGlassApp {
    fn new() -> Self {
        Self {
            presenter: None,
            fb: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            time: 0.0,
        }
    }

    fn draw_pattern(&mut self) {
        let width = self.fb.width() as i32;
        let height = self.fb.height() as i32;

        self.fb.clear(0xFF_111111);

        // Draw some sharp rectangles to show the blurring effect
        for y in (height / 4)..(height * 3 / 4) {
            for x in (width / 4)..(width * 3 / 4) {
                // Checkerboard pattern
                if ((x / 30) + (y / 30)) % 2 == 0 {
                    self.fb.set_pixel(x, y, 0xFF_EE4444); // Red
                } else {
                    self.fb.set_pixel(x, y, 0xFF_4444EE); // Blue
                }
            }
        }
    }
}

impl WindowApp for FrostedGlassApp {
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
        // Animate the radius between 1.0 and 15.0
        let radius = 8.0 + (self.time.sin() * 7.0);

        self.draw_pattern();

        apply_frosted_glass(&mut self.fb, radius, (self.time * 100.0) as u32);

        let presenter = self.presenter.as_mut().unwrap();
        presenter.present(&self.fb)?;
        Ok(())
    }
}

fn main() {
    print_banner();
    run_windowed(FrostedGlassApp::new())
}
