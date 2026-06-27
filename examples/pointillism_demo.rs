use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::pointillism::{PointillismConfig, apply_pointillism};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Pointillism Demo".bold().cyan());
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
            Cell::new("Simulates pointillism painting style").fg(Color::Green),
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
const TITLE: &str = "🌟 Nova: Pointillism Demo";

struct PointillismDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    time: f32,
}

impl PointillismDemo {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            time: 0.0,
        })
    }
}

impl WindowApp for PointillismDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
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

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let width = self.framebuffer.width() as i32;
        let height = self.framebuffer.height() as i32;

        // Draw a simple background image (gradient and a moving circle)
        for y in 0..height {
            for x in 0..width {
                let r = ((x as f32 / width as f32) * 255.0) as u32;
                let g = ((y as f32 / height as f32) * 255.0) as u32;
                let b = ((self.time.sin() * 0.5 + 0.5) * 255.0) as u32;

                let cx = (width as f32 * 0.5 + self.time.cos() * 100.0) as i32;
                let cy = (height as f32 * 0.5 + self.time.sin() * 100.0) as i32;

                let dx = x - cx;
                let dy = y - cy;

                if dx * dx + dy * dy < 10000 {
                    self.framebuffer.set_pixel(x, y, 0xFF_FF_FF_FF);
                } else {
                    self.framebuffer
                        .set_pixel(x, y, 0xFF_00_00_00 | (r << 16) | (g << 8) | b);
                }
            }
        }

        let config = PointillismConfig {
            num_dots: 50_000,
            min_radius: 2.0,
            max_radius: 8.0,
            clear_background: true,
            background_color: 0xFF_F0_F0_F0,
        };

        apply_pointillism(&mut self.framebuffer, &config);

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    #[cfg(not(feature = "nova"))]
    println!("Pointillism Demo");

    run_windowed(PointillismDemo::new().unwrap());
}
