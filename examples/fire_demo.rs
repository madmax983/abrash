use abrash::experimental::fire::apply_fire;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_core::utils::XorShift32;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Fire Effect Demo".bold().cyan());
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
            Cell::new("Classic demoscene fire effect using cellular automata.").fg(Color::Green),
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
const TITLE: &str = "🌟 Nova: Fire Effect Demo";

struct FireDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    cooling_map: Vec<u8>,
    rng: XorShift32,
}

impl FireDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            cooling_map: vec![0; (WIDTH * HEIGHT) as usize],
            rng: XorShift32::new(1337),
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

impl WindowApp for FireDemoApp {
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

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let width = WIDTH as usize;
        let height = HEIGHT as usize;

        // Update cooling map with some noise
        for y in 0..height {
            for x in 0..width {
                let r = self.rng.next_u32() % 2;
                self.cooling_map[y * width + x] = r as u8;
            }
        }

        // Feed the fire at the bottom
        let fb_slice = self.framebuffer.as_mut_slice();
        for x in 0..width {
            let r = self.rng.next_u32() % 256;
            let val = if r < 128 { 0xFF } else { 0x00 };
            fb_slice[(height - 1) * width + x] = (val << 16) | (val << 8) | val;
        }

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        apply_fire(&mut self.framebuffer, &self.cooling_map);

        // Apply a fire palette (convert grayscale heat to fire colors)
        let pixels = self.framebuffer.as_mut_slice();
        for p in pixels.iter_mut() {
            let heat = (*p >> 16) & 0xFF;
            let (r, g, b) = if heat > 160 {
                (255, 255, heat) // White/Yellow
            } else if heat > 80 {
                (255, heat * 2, 0) // Orange/Red
            } else {
                (heat * 3, 0, 0) // Dark Red
            };
            *p = (r << 16) | (g << 8) | b;
        }

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(FireDemoApp::new().unwrap());
}
