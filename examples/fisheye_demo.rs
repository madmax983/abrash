use abrash::experimental::fisheye::apply_fisheye;
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
    println!("\n{}", "🌟 Fisheye Lens Demo".bold().cyan());
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
            Cell::new("Demonstrates a fisheye barrel distortion effect").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
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
const TITLE: &str = "Nova: Fisheye Lens Demo";

struct FisheyeDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    time: f32,
}

impl FisheyeDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Draw a grid pattern to clearly visualize the distortion
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                // A primary checkerboard pattern + grid lines
                let is_grid_line = x % 40 == 0 || y % 40 == 0;
                let color = if is_grid_line {
                    0xFF_FF_00_00 // Red lines
                } else if (x / 40 + y / 40) % 2 == 0 {
                    0xFF_44_44_44 // Dark gray
                } else {
                    0xFF_AA_AA_AA // Light gray
                };
                background_fb.set_pixel(x as i32, y as i32, color);
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

impl WindowApp for FisheyeDemoApp {
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
        // Copy the static grid background into the active framebuffer
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        // Animate the strength of the fisheye effect using a sine wave
        // Animate between slightly negative (pin-cushion) and heavily positive (fisheye barrel)
        let strength = (self.time * 2.0).sin() * 0.5 + 0.2;

        apply_fisheye(&mut self.framebuffer, strength);

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(FisheyeDemoApp::new().unwrap())
}
