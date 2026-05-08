use abrash::experimental::crt::apply_crt;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::time::FixedTimestep;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn print_banner() {
    println!("\n{}", "📺 CRT Filter Demo".bold().cyan());
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
            Cell::new("Simulates a retro CRT monitor with barrel distortion").fg(Color::Green),
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
            Cell::new("Esc / Close"),
            Cell::new("Exit"),
        ]);
    println!("{controls}\n");
}

struct CrtDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    timestep: FixedTimestep,
    time: f32,
}

#[cfg(feature = "nova")]
impl CrtDemoApp {
    #[allow(clippy::unnecessary_wraps)]
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            timestep: FixedTimestep::new(60),
            time: 0.0,
        })
    }
}

#[cfg(feature = "nova")]
impl WindowApp for CrtDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - CRT Filter Demo".to_string(),
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
        self.framebuffer = Framebuffer::new(width, height).unwrap();
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.time += self.timestep.dt();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Draw a test pattern
        let w = self.framebuffer.width() as i32;
        let h = self.framebuffer.height() as i32;

        self.framebuffer.clear(0xFF00_0000);

        let offset_x = (self.time * 50.0) as i32 % 40;
        let offset_y = (self.time * 30.0) as i32 % 40;

        for y in 0..h {
            for x in 0..w {
                let mut color = 0xFF00_0000;

                // Grid lines
                if (x + offset_x) % 40 == 0 || (y + offset_y) % 40 == 0 {
                    color = 0xFF00_FF00;
                } else if (x + offset_x) % 10 == 0 || (y + offset_y) % 10 == 0 {
                    color = 0xFF00_4000;
                }

                // Color blocks
                let bx = x / 100;
                let by = y / 100;
                if (bx + by) % 2 == 0 {
                    let r = ((x as f32 / w as f32) * 255.0) as u32;
                    let b = ((y as f32 / h as f32) * 255.0) as u32;
                    color |= (r << 16) | b;
                }

                self.framebuffer.set_pixel(x, y, color);
            }
        }

        let distortion = 0.15 + (self.time * 2.0).sin() * 0.05;
        apply_crt(&mut self.framebuffer, distortion);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }
        Ok(())
    }
}

#[cfg(not(feature = "nova"))]
fn main() {
    println!("This example requires the 'nova' feature. Run with --features nova");
}

#[cfg(feature = "nova")]
#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), HostError> {
    print_banner();
    run_windowed(CrtDemoApp::new().unwrap());
    Ok(())
}
