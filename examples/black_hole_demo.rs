use abrash::experimental::black_hole::apply_black_hole;
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
    println!(
        "\n{}",
        "🌟 Black Hole Gravitational Lensing Filter Demo"
            .bold()
            .cyan()
    );
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
            Cell::new("Demonstrates gravitational lensing effect simulating a black hole")
                .fg(Color::Green),
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
const TITLE: &str = "Nova: Black Hole Gravitational Lensing Filter Demo";

struct BlackHoleDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    time: f32,
}

impl BlackHoleDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Draw a simple sci-fi grid and starfield to visualize lensing
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let is_grid = (x % 40 == 0) || (y % 40 == 0);

                // Simple pseudo-random hash for stars
                let hash = x
                    .wrapping_mul(374_761_393)
                    .wrapping_add(y.wrapping_mul(668_265_263));
                let is_star = hash % 1000 < 5; // 0.5% chance of being a star

                let color = if is_grid {
                    0xFF_22_22_44 // Faint blue/purple grid
                } else if is_star {
                    0xFF_FF_FF_FF // White star
                } else {
                    0xFF_0A_0A_1A // Deep space background
                };

                background_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        // Draw a bright central object that the black hole will pass over
        let center_x = WIDTH as f32 * 0.5;
        let center_y = HEIGHT as f32 * 0.5;
        let star_radius = 80.0;

        for y in 0..HEIGHT {
            let dy = y as f32 - center_y;
            for x in 0..WIDTH {
                let dx = x as f32 - center_x;
                let dist = dx.hypot(dy);

                if dist < star_radius {
                    let intensity = 1.0 - (dist / star_radius);
                    let r = (255.0 * intensity) as u32;
                    let g = (200.0 * intensity) as u32;
                    let b = (100.0 * intensity) as u32;
                    let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                    background_fb.set_pixel(x as i32, y as i32, color);
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

impl WindowApp for BlackHoleDemoApp {
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
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Copy the static background
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        // Animate black hole position in a figure-8 or circle
        let speed = 1.5;
        let radius_x = 250.0;
        let radius_y = 150.0;
        let bh_x = (WIDTH as f32 / 2.0) + (self.time * speed).sin() * radius_x;
        let bh_y = (HEIGHT as f32 / 2.0) + (self.time * speed * 0.7).cos() * radius_y;

        let mass = 40.0; // Size of event horizon

        apply_black_hole(&mut self.framebuffer, bh_x, bh_y, mass);

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    {
        print_banner();
        run_windowed(BlackHoleDemoApp::new().unwrap());
    }
    #[cfg(not(feature = "nova"))]
    {
        println!("This example requires the 'nova' feature to be enabled.");
        println!("Run with: cargo run --example black_hole_demo --features nova");
        Ok(())
    }
}
