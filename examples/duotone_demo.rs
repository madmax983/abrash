#![cfg(feature = "backend-winit")]

use abrash::experimental::duotone::{apply_duotone, DuotoneConfig};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Duotone Filter Demo";

struct DuotoneDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    time: f32,
}

impl DuotoneDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
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

fn print_banner() {
    println!("\n{}", "🌟 Duotone Filter Demo".bold().cyan());
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
            Cell::new("A Spotify-style duotone filter!").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Software Rasterizer + Post-Process").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("None")]);
    println!("{controls}\n");
}

impl WindowApp for DuotoneDemoApp {
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
        self.time += ctx.dt_seconds.max(0.0) * 2.0;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let width = self.framebuffer.width() as usize;
        let height = self.framebuffer.height() as usize;

        // Draw a simple animated gradient background to demonstrate the effect
        let pixels = self.framebuffer.as_mut_slice();
        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / width as f32;
                let v = y as f32 / height as f32;

                let r = ((u + self.time.sin() * 0.5) * 255.0).clamp(0.0, 255.0) as u32;
                let g = ((v + self.time.cos() * 0.5) * 255.0).clamp(0.0, 255.0) as u32;
                let b = 128;

                pixels[y * width + x] = 0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }

        let config = DuotoneConfig::default();
        apply_duotone(&mut self.framebuffer, &config);

        self.present()
    }
}

fn main() {
    print_banner();

    run_windowed(DuotoneDemoApp::new().unwrap())
}
