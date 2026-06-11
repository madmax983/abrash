use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::vhs::{VhsConfig, apply_vhs};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 VHS Tracking Filter Demo".bold().cyan());
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
            Cell::new("Simulates analog VHS tape artifacts and tracking errors").fg(Color::Green),
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
const TITLE: &str = "🌟 Nova: VHS Tracking Filter Demo";

struct VhsDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    time: f32,
}

impl VhsDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // SMPTE-style Color Bars
        let colors = [
            0xFF_FF_FF_FF, // White
            0xFF_FF_FF_00, // Yellow
            0xFF_00_FF_FF, // Cyan
            0xFF_00_FF_00, // Green
            0xFF_FF_00_FF, // Magenta
            0xFF_FF_00_00, // Red
            0xFF_00_00_FF, // Blue
        ];

        let bar_width = WIDTH / colors.len() as u32;

        for y in 0..(HEIGHT * 3 / 4) {
            for x in 0..WIDTH {
                let color_idx = (x / bar_width).min((colors.len() - 1) as u32) as usize;
                background_fb.set_pixel(x as i32, y as i32, colors[color_idx]);
            }
        }

        // Bottom blocks
        let bottom_y_start = HEIGHT * 3 / 4;
        let bottom_colors = [
            0xFF_00_00_FF, // Blue
            0xFF_11_11_11, // Dark Gray
            0xFF_FF_00_FF, // Magenta
            0xFF_11_11_11, // Dark Gray
            0xFF_00_FF_FF, // Cyan
            0xFF_11_11_11, // Dark Gray
            0xFF_FF_FF_FF, // White
        ];

        for y in bottom_y_start..HEIGHT {
            for x in 0..WIDTH {
                let color_idx = (x / bar_width).min((colors.len() - 1) as u32) as usize;
                background_fb.set_pixel(x as i32, y as i32, bottom_colors[color_idx]);
            }
        }

        // Add a "PLAY" text block simulation in the top left
        for y in 30i32..60i32 {
            for x in 40i32..120i32 {
                // simple box
                if x > 40 && x < 120 && y > 30 && y < 60 {
                    background_fb.set_pixel(x, y, 0xFF_00_FF_00);
                }
            }
        }

        // Punch a hole in the box to make it look like text
        for y in 35i32..55i32 {
            for x in 45i32..115i32 {
                background_fb.set_pixel(x, y, 0xFF_00_00_00);
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

impl WindowApp for VhsDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window.clone())?);
        Ok(())
    }

    fn update(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds.max(0.0);
        Ok(())
    }

    fn render(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        // Copy original image
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        // Animate the tracking band moving down the screen
        // Wrap around 0.0 to 1.0, but let it go slightly out of bounds to simulate
        // the tracking bar wrapping around the screen.
        let raw_pos = (self.time * 0.2) % 1.2;
        let tracking_position = raw_pos - 0.1; // -0.1 to 1.1

        // Pulsing intensity for realism
        let intensity = 0.5 + (self.time * 2.0).sin() * 0.2;

        let config = VhsConfig {
            intensity,
            time: self.time,
            tracking_position,
            tracking_thickness: 0.15,
            noise_intensity: 0.25,
        };

        apply_vhs(&mut self.framebuffer, &config);

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(VhsDemoApp::new().unwrap());
}
