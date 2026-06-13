#![cfg(feature = "backend-winit")]

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::scanline_jitter::{ScanlineJitterConfig, apply_scanline_jitter};
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Scanline Jitter Demo";

struct JitterDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    frame_count: u32,
}

impl JitterDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            frame_count: 0,
        })
    }

    fn draw_pattern(&mut self) {
        self.framebuffer.clear(0xFF_000000);
        let fb_w = self.framebuffer.width();
        let fb_h = self.framebuffer.height();

        // draw a grid
        for y in 0..fb_h {
            for x in 0..fb_w {
                if x % 40 == 0 || y % 40 == 0 {
                    self.framebuffer.set_pixel(x as i32, y as i32, 0xFF_00FF00); // Green grid
                }

                // Draw a circle
                let dx = x as i32 - (fb_w / 2) as i32;
                let dy = y as i32 - (fb_h / 2) as i32;
                if dx * dx + dy * dy < 10000 && dx * dx + dy * dy > 9000 {
                    self.framebuffer.set_pixel(x as i32, y as i32, 0xFF_FF0000); // Red ring
                }
            }
        }
    }
}

impl WindowApp for JitterDemoApp {
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
        self.frame_count = self.frame_count.wrapping_add(1);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.draw_pattern();

        let config = ScanlineJitterConfig {
            max_shift: 15,
            probability: 0.15,
            seed: self.frame_count,
        };
        apply_scanline_jitter(&mut self.framebuffer, &config);

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
    println!("\n{}", "🌟 Scanline Jitter Demo".bold().cyan());
    println!("{}", "=======================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Analog video signal degradation effect").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}\n");
}

fn main() {
    print_banner();
    run_windowed(JitterDemoApp::new().unwrap());
}
