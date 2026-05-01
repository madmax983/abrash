#![cfg(feature = "backend-winit")]

use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_gouraud;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::color_splash::{ColorSplashConfig, apply_color_splash};
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::time::Instant;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: Color Splash Demo";

struct ColorSplashDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    config: ColorSplashConfig,
    angle: f32,
    last_time: Instant,
}

impl ColorSplashDemoApp {
    pub fn new() -> Self {
        Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            zbuffer: ZBuffer::new(WIDTH, HEIGHT).unwrap(),
            config: ColorSplashConfig::default(),
            angle: 0.0,
            last_time: Instant::now(),
        }
    }
}

impl ColorSplashDemoApp {
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

impl WindowApp for ColorSplashDemoApp {
    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Box<dyn std::error::Error>> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        self.last_time = Instant::now();
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;
        self.angle += dt * 2.0;

        self.framebuffer.clear(0xFF_22_22_22);
        self.zbuffer.clear();

        let cx = WIDTH as f32 * 0.5;
        let cy = HEIGHT as f32 * 0.5;
        let radius = 80.0;

        let mut draw_tri = |x: f32, y: f32, color: u32| {
            let c = Vec3::new(
                ((color >> 16) & 0xFF) as f32 / 255.0,
                ((color >> 8) & 0xFF) as f32 / 255.0,
                (color & 0xFF) as f32 / 255.0,
            );

            let v0 = ((Vec3::new(x, y - radius, 0.5), 1.0), c);
            let v1 = ((Vec3::new(x - radius, y + radius, 0.5), 1.0), c);
            let v2 = ((Vec3::new(x + radius, y + radius, 0.5), 1.0), c);

            fill_triangle_gouraud(&mut self.framebuffer, &mut self.zbuffer, v0, v1, v2);
        };

        // Draw multiple colored triangles moving in a circle
        let r_x = cx + self.angle.cos() * 150.0;
        let r_y = cy + self.angle.sin() * 150.0;
        draw_tri(r_x, r_y, 0xFF_FF_00_00); // Red

        let g_x = cx + (self.angle + 2.094).cos() * 150.0;
        let g_y = cy + (self.angle + 2.094).sin() * 150.0;
        draw_tri(g_x, g_y, 0xFF_00_FF_00); // Green

        let b_x = cx + (self.angle + 4.188).cos() * 150.0;
        let b_y = cy + (self.angle + 4.188).sin() * 150.0;
        draw_tri(b_x, b_y, 0xFF_00_00_FF); // Blue

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Box<dyn std::error::Error>> {
        apply_color_splash(&mut self.framebuffer, &self.config);
        self.present()
    }
}

fn print_banner() {
    println!("\n{}", "🌟 Color Splash Demo".bold().cyan());
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
            Cell::new("Selective color post-processing filter").fg(Color::Green),
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
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![Cell::new("Mouse"), Cell::new("None")])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);
    println!("{controls}\n");
}

fn main() {
    print_banner();
    run_windowed(ColorSplashDemoApp::new());
}
