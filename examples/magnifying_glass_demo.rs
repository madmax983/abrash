use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::magnifying_glass::{
    MagnifyingGlassConfig, apply_magnifying_glass,
};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Magnifying Glass Filter Demo".bold().cyan());
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
            Cell::new("Simulates a magnifying glass moving across the screen").fg(Color::Green),
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
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Close window to exit"),
        ]);
    println!("{controls}\n");
}

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "🌟 Nova: Magnifying Glass Filter Demo";

struct MagnifyingGlassDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    time: f32,
}

impl MagnifyingGlassDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Draw a test pattern (checkerboard)
        let tile_size = 40;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let is_dark = ((x / tile_size) + (y / tile_size)) % 2 == 0;
                let color = if is_dark {
                    0xFF_22_22_22
                } else {
                    0xFF_DD_DD_DD
                };
                background_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        // Draw some text/lines
        for x in 0..WIDTH {
            background_fb.set_pixel(x as i32, (HEIGHT / 2) as i32, 0xFF_FF_00_00);
            background_fb.set_pixel(x as i32, (HEIGHT / 2 + 1) as i32, 0xFF_FF_00_00);
        }
        for y in 0..HEIGHT {
            background_fb.set_pixel((WIDTH / 2) as i32, y as i32, 0xFF_00_FF_00);
            background_fb.set_pixel((WIDTH / 2 + 1) as i32, y as i32, 0xFF_00_FF_00);
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

impl WindowApp for MagnifyingGlassDemoApp {
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
        // Copy the original background
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        // Calculate moving lens position using sine waves
        let cx = (WIDTH as f32 / 2.0 + (self.time * 2.0).cos() * (WIDTH as f32 / 3.0)) as i32;
        let cy = (HEIGHT as f32 / 2.0 + (self.time * 3.0).sin() * (HEIGHT as f32 / 3.0)) as i32;

        let config = MagnifyingGlassConfig {
            center_x: cx,
            center_y: cy,
            radius: 120,
            magnification: 2.5,
            border_thickness: 5,
            border_color: 0xFF_11_11_11,
        };

        apply_magnifying_glass(&mut self.framebuffer, &config);

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(MagnifyingGlassDemoApp::new().unwrap());
}
