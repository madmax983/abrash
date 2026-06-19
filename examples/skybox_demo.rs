use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::skybox::{Cubemap, draw_skybox};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use std::time::Instant;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "Abrash - Skybox Demo";

fn print_banner() {
    println!("\n{}", "🌌 Skybox Demo".bold().cyan());
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
            Cell::new("Renders a cubemap skybox").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Texture"),
            Cell::new("6x Checkerboard faces").fg(Color::Yellow),
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
            Cell::new("Auto-rotating camera"),
        ]);
    println!("{controls}\n");
}

struct SkyboxDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    cubemap: Cubemap,
    start_time: Instant,
}

impl SkyboxDemoApp {
    fn new() -> Result<Self, HostError> {
        let size = 64;
        let faces = [
            Texture::checkered(size, size, 0xFFFF_0000, 0xFFFF_FFFF)
                .map_err(|error| HostError::App(error.to_string()))?,
            Texture::checkered(size, size, 0xFF00_FFFF, 0xFFFF_FFFF)
                .map_err(|error| HostError::App(error.to_string()))?,
            Texture::checkered(size, size, 0xFF00_00FF, 0xFFFF_FFFF)
                .map_err(|error| HostError::App(error.to_string()))?,
            Texture::checkered(size, size, 0xFFFF_FF00, 0xFFFF_FFFF)
                .map_err(|error| HostError::App(error.to_string()))?,
            Texture::checkered(size, size, 0xFF00_FF00, 0xFFFF_FFFF)
                .map_err(|error| HostError::App(error.to_string()))?,
            Texture::checkered(size, size, 0xFFFF_00FF, 0xFFFF_FFFF)
                .map_err(|error| HostError::App(error.to_string()))?,
        ];

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            cubemap: Cubemap::new(faces),
            start_time: Instant::now(),
        })
    }
}

impl WindowApp for SkyboxDemoApp {
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
        self.zbuffer =
            ZBuffer::new(width, height).map_err(|error| HostError::App(error.to_string()))?;
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let t = self.start_time.elapsed().as_secs_f32();
        let proj = Mat4::perspective(
            1.57,
            self.framebuffer.width() as f32 / self.framebuffer.height() as f32,
            0.1,
            100.0,
        );

        let eye = Vec3::new(0.0, 0.0, 0.0);
        let target = Vec3::new(t.sin(), (t * 0.3).cos() * 0.5, t.cos());
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);

        self.framebuffer.clear(0xFF00_0000);
        self.zbuffer.clear();
        draw_skybox(
            &mut self.framebuffer,
            &mut self.zbuffer,
            view,
            proj,
            &self.cubemap,
        );
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
    print_banner();
    match SkyboxDemoApp::new() {
        Ok(app) => run_windowed(app),
        Err(e) => {
            let mut error_table = comfy_table::Table::new();
            error_table
                .load_preset(comfy_table::presets::UTF8_FULL)
                .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                .set_header(vec![
                    comfy_table::Cell::new("❌ Initialization Error")
                        .add_attribute(comfy_table::Attribute::Bold)
                        .fg(comfy_table::Color::Red),
                ])
                .add_row(vec![
                    comfy_table::Cell::new(format!("{e}")).fg(comfy_table::Color::Yellow),
                ]);
            eprintln!("\n{error_table}");
            std::process::exit(1);
        }
    }
}
