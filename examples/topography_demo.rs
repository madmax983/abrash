use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
use abrash::experimental::topography::{TopographyConfig, apply_topography};

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Topography Filter Demo".bold().cyan());
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
            Cell::new("Simulates a Joy Division style topographical map").fg(Color::Green),
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

#[cfg(feature = "nova")]
struct TopographyDemoApp {
    framebuffer: Framebuffer,
    src_fb: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    time: f32,
}

#[cfg(feature = "nova")]
impl TopographyDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            framebuffer: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            src_fb: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            presenter: None,
            time: 0.0,
        })
    }
}

#[cfg(feature = "nova")]
impl WindowApp for TopographyDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Topography Filter Demo (Joy Division)".to_string(),
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
        let width = WIDTH;
        let height = HEIGHT;
        let t = self.time;

        // Generate a simple moving pattern
        for y in 0..height {
            for x in 0..width {
                let cx = width as f32 / 2.0;
                let cy = height as f32 / 2.0;
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;

                let dist = (dx * dx + dy * dy).sqrt();

                // Moving rings
                let mut val = (dist * 0.1 - t * 2.0).sin();
                // Map -1..1 to 0..255
                val = (val + 1.0) * 127.5;

                let c = val as u32;
                let color = 0xFF000000 | (c << 16) | (c << 8) | c;

                self.src_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        // Copy to output buffer
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.src_fb.as_slice());

        // Apply topography effect
        let config = TopographyConfig {
            line_spacing: 12,
            amplitude: 40.0,
            line_color: 0xFF_FF_FF_FF,
            background_color: 0xFF_11_11_22,
        };

        apply_topography(&mut self.framebuffer, &config);

        if let Some(ref mut presenter) = self.presenter {
            presenter.present(&self.framebuffer)?;
        }

        Ok(())
    }
}

fn main() {
    #[cfg(feature = "nova")]
    {
        print_banner();
        run_windowed(TopographyDemoApp::new().unwrap());
    }

    #[cfg(not(feature = "nova"))]
    {
        println!(
            "Please run this example with the `nova` feature enabled: cargo run --example topography_demo --features nova,backend-winit"
        );
    }
}
