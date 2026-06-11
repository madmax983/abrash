//! Abrash Graphics Demo - Normal Mapping
//!
//! Demonstrates normal mapping optimization.

use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3, Vec4};
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_normal_mapped;
use abrash::texture::Texture;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;
use std::fmt;
use std::io::Error as IoError;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

#[derive(Debug)]
struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AppError {}

impl From<&'static str> for AppError {
    fn from(error: &'static str) -> Self {
        Self(error.to_string())
    }
}

impl From<String> for AppError {
    fn from(error: String) -> Self {
        Self(error)
    }
}

impl From<IoError> for AppError {
    fn from(error: IoError) -> Self {
        Self(error.to_string())
    }
}

impl From<abrash::platform::HostError> for AppError {
    fn from(error: abrash::platform::HostError) -> Self {
        Self(error.to_string())
    }
}

fn print_banner() {
    println!("\n{}", "🧱 Normal Mapping Demo".bold().blue());
    println!("{}", "======================".dark_grey());

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
            Cell::new("Per-pixel lighting with normal maps").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Technique"),
            Cell::new("Tangent space calculation").fg(Color::Magenta),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    println!(" • Mouse: None");
    println!(" • Keyboard: Auto-rotating light source\n");
}

struct NormalMappingDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    diffuse_map: Texture,
    normal_map: Texture,
    projection: Mat4,
    view: Mat4,
    timestep: FixedTimestep,
    light_angle: f32,
}

impl NormalMappingDemoApp {
    fn new() -> Result<Self, AppError> {
        let diffuse_map = Texture::checkered(256, 256, 0xFF80_8080, 0xFF80_8080)?;
        let mut normal_map = Texture::new(256, 256)?;
        let normal_map_pixels = normal_map.pixels_mut();
        normal_map_pixels.fill(0xFFFF_8080);

        for y in 0..256 {
            for x in 0..256 {
                let dx = (x as f32 - 128.0) / 128.0;
                let dy = (y as f32 - 128.0) / 128.0;
                #[allow(clippy::imprecise_flops)]
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < 0.8 {
                    let z = (1.0 - dist * dist).sqrt();
                    let n = Vec3::new(dx, dy, z).normalize();

                    let r = ((n.x * 0.5 + 0.5) * 255.0) as u32;
                    let g = ((n.y * 0.5 + 0.5) * 255.0) as u32;
                    let b = ((n.z * 0.5 + 0.5) * 255.0) as u32;

                    normal_map_pixels[y * 256 + x] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                } else {
                    normal_map_pixels[y * 256 + x] = 0xFF80_80FF;
                }
            }
        }

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
            diffuse_map,
            normal_map,
            projection: Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0),
            view: Mat4::look_at(
                Vec3::new(0.0, 0.0, 4.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            timestep: FixedTimestep::new(60),
            light_angle: 0.0,
        })
    }

    fn present(&mut self) -> Result<(), AppError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| IoError::other("software presenter not initialized"))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for NormalMappingDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Normal Mapping".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window.clone())?);
        Ok(())
    }

    fn update(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.light_angle += 0.05;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF00_0000);
        self.zbuffer.clear();

        let p0 = Vec3::new(-2.0, 2.0, 0.0);
        let p1 = Vec3::new(-2.0, -2.0, 0.0);
        let p2 = Vec3::new(2.0, -2.0, 0.0);
        let p3 = Vec3::new(2.0, 2.0, 0.0);

        let uv0 = Vec2::new(0.0, 0.0);
        let uv1 = Vec2::new(0.0, 1.0);
        let uv2 = Vec2::new(1.0, 1.0);
        let uv3 = Vec2::new(1.0, 0.0);

        let n = Vec3::new(0.0, 0.0, 1.0);
        let t = Vec4::new(1.0, 0.0, 0.0, 1.0);

        let light_dir =
            Vec3::new(self.light_angle.cos(), self.light_angle.sin() * 0.5, -1.0).normalize();
        let light_color = Vec3::new(1.0, 1.0, 1.0);
        let ambient = Vec3::new(0.1, 0.1, 0.1);
        let mvp = self.projection * self.view;

        let v0 = mvp.transform_point(p0);
        let v1 = mvp.transform_point(p1);
        let v2 = mvp.transform_point(p2);
        let v3 = mvp.transform_point(p3);

        fill_triangle_normal_mapped(
            &mut self.framebuffer,
            &mut self.zbuffer,
            (v0, uv0, n, t),
            (v1, uv1, n, t),
            (v2, uv2, n, t),
            &self.diffuse_map,
            &self.normal_map,
            light_dir,
            light_color,
            ambient,
        );

        fill_triangle_normal_mapped(
            &mut self.framebuffer,
            &mut self.zbuffer,
            (v0, uv0, n, t),
            (v2, uv2, n, t),
            (v3, uv3, n, t),
            &self.diffuse_map,
            &self.normal_map,
            light_dir,
            light_color,
            ambient,
        );

        self.present()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), AppError> {
    print_banner();
    run_windowed(NormalMappingDemoApp::new().unwrap());
    Ok(())
}
