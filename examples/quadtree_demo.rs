#![cfg(feature = "backend-winit")]

use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::circle::fill_circle;
use abrash_render::experimental::quadtree::apply_quadtree;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::time::Instant;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Quadtree Stylization Demo";

struct QuadtreeDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    angle: f32,
    last_time: Instant,
}

impl QuadtreeDemoApp {
    pub fn new() -> Self {
        Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            angle: 0.0,
            last_time: Instant::now(),
        }
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

impl WindowApp for QuadtreeDemoApp {
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
        self.last_time = Instant::now();
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;
        self.angle += dt * 2.0;

        self.framebuffer.clear(0xFF_22_22_22);

        let cx = WIDTH as f32 * 0.5;
        let cy = HEIGHT as f32 * 0.5;

        let r_x = cx + self.angle.cos() * 150.0;
        let r_y = cy + self.angle.sin() * 150.0;
        fill_circle(
            &mut self.framebuffer,
            r_x as i32,
            r_y as i32,
            80,
            0xFF_FF_00_00,
        );

        let g_x = cx + (self.angle + 2.094).cos() * 150.0;
        let g_y = cy + (self.angle + 2.094).sin() * 150.0;
        fill_circle(
            &mut self.framebuffer,
            g_x as i32,
            g_y as i32,
            80,
            0xFF_00_FF_00,
        );

        let b_x = cx + (self.angle + 4.188).cos() * 150.0;
        let b_y = cy + (self.angle + 4.188).sin() * 150.0;
        fill_circle(
            &mut self.framebuffer,
            b_x as i32,
            b_y as i32,
            80,
            0xFF_00_00_FF,
        );

        let pulse = ((self.angle * 0.5).sin() + 1.0) * 0.5;
        let threshold = (pulse * 100.0) as u32;

        apply_quadtree(&mut self.framebuffer, threshold, 4, true, 0xFF_00_00_00);

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.present()
    }
}

fn print_banner() {
    println!("\n{}", "🌟 Quadtree Stylization Demo".bold().cyan());
    println!("{}", "====================".dark_grey());

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
            Cell::new("Quadtree image stylization").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Software Rasterizer + Post-Process").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

fn main() {
    print_banner();
    run_windowed(QuadtreeDemoApp::new());
}
