use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::droste::{DrosteConfig, apply_droste};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Droste Effect Demo".bold().cyan());
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
            Cell::new("Demonstrates the recursive Droste (picture-in-picture) effect").fg(Color::Green),
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
const TITLE: &str = "Nova: Droste Effect Demo";

struct DrosteDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    time: f32,
}

impl DrosteDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Draw a pattern to clearly visualize the recursive effect
        let cx = WIDTH as f32 / 2.0;
        let cy = HEIGHT as f32 / 2.0;

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;

                let dist = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx);

                // Color stripes radiating outwards
                let spiral = ((dist * 0.1) - (angle * 3.0)).sin();

                let r = ((spiral + 1.0) * 127.5) as u32;
                let g = (((angle + std::f32::consts::PI) / (2.0 * std::f32::consts::PI)) * 255.0) as u32;
                let b = 255 - r;

                let color = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;

                // Draw a distinct border to show the picture frame
                if x < 10 || x > WIDTH - 10 || y < 10 || y > HEIGHT - 10 {
                    background_fb.set_pixel(x as i32, y as i32, 0xFF_FF_FF_FF);
                } else {
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

impl WindowApp for DrosteDemoApp {
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
        // Copy the static background into the active framebuffer
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        // Animate the scale factor to zoom in/out
        let scale = (self.time * 0.5).sin() * 0.2 + 0.5; // oscillate between 0.3 and 0.7

        let config = DrosteConfig {
            scale_factor: scale,
            iterations: 4,
        };

        apply_droste(&mut self.framebuffer, &config);

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(DrosteDemoApp::new().unwrap());
}
