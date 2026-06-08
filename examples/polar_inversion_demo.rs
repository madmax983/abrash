use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::polar_inversion::{PolarInversionConfig, apply_polar_inversion};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!(
        "\n{}",
        "🌟 Polar Inversion (Tiny Planet) Demo".bold().cyan()
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
            Cell::new("Resolution"),
            Cell::new("800x600").fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Effect"),
            Cell::new("Tiny Planet").fg(Color::Green),
        ]);

    println!("{table}\n");
}

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Polar Inversion Demo";

struct PolarInversionApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    base_fb: Framebuffer,
    time: f32,
}

impl PolarInversionApp {
    fn new() -> Result<Self, HostError> {
        let mut base_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Generate a grid or interesting pattern into the base framebuffer
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let bx = (x / 20) % 2;
                let by = (y / 20) % 2;
                let color = if bx == by {
                    0xFF_FF_FF_FF // White
                } else {
                    0xFF_00_00_FF // Blue
                };
                base_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            base_fb,
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

impl WindowApp for PolarInversionApp {
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
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.base_fb.as_slice());

        let animated_radius = 200.0 + (self.time * 2.0).sin() * 50.0;

        let config = PolarInversionConfig {
            center_x: 0.5,
            center_y: 0.5,
            radius: animated_radius,
            zoom: 1.0,
        };

        apply_polar_inversion(&mut self.framebuffer, &config);

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(PolarInversionApp::new().unwrap());
}
