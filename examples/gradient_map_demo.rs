use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_core::gradient::Gradient;
use abrash_render::experimental::gradient_map::apply_gradient_map;

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Gradient Map Demo".bold().cyan());
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
            Cell::new("Maps grayscale luminance into arbitrary color gradients").fg(Color::Green),
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
const TITLE: &str = "Nova: Gradient Map Demo";

struct GradientMapDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    time: f32,
    gradient_idx: usize,
    gradients: Vec<Gradient>,
}

impl GradientMapDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Draw a procedural radial gradient / checkerboard pattern to visualize luminance mapping
        for y in 0..HEIGHT {
            let dy = (y as f32 - HEIGHT as f32 / 2.0) / (HEIGHT as f32 / 2.0);
            for x in 0..WIDTH {
                let dx = (x as f32 - WIDTH as f32 / 2.0) / (WIDTH as f32 / 2.0);

                let dist = (dx * dx + dy * dy).sqrt();
                // A smooth radial luminance with a grid overlay
                let mut lum = ((1.0 - dist.min(1.0)) * 255.0) as u32;

                let is_grid = (x % 40 == 0) || (y % 40 == 0);
                if is_grid {
                    lum = 255 - lum; // Invert grid lines
                }

                let color = 0xFF_00_00_00 | (lum << 16) | (lum << 8) | lum;
                background_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        let gradients = vec![
            Gradient::heat(),
            Gradient::plasma(),
            Gradient::terrain(),
            Gradient::health(),
        ];

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            background_fb,
            time: 0.0,
            gradient_idx: 0,
            gradients,
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

impl WindowApp for GradientMapDemoApp {
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
        // Cycle gradients every 3 seconds
        self.gradient_idx = ((self.time / 3.0) as usize) % self.gradients.len();
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Copy the static luminance background into the active framebuffer
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        apply_gradient_map(&mut self.framebuffer, &self.gradients[self.gradient_idx]);

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(GradientMapDemoApp::new().unwrap());
}
