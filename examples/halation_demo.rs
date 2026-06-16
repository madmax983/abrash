use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::halation::{HalationConfig, apply_halation};
use std::f32::consts::PI;
use std::fmt;
use std::io::Error as IoError;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF10_1010;

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
    println!("\n{}", "🎥 Halation Demo".bold().cyan());
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
            Cell::new("Simulates analog film light bleed (halation)").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
    println!();
}

fn render_cube(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    cube: &Mesh,
    model: Mat4,
    view_proj: Mat4,
    color: u32,
) {
    let mvp = view_proj * model;

    for tri_indices in &cube.indices {
        let v0 = cube.vertices[tri_indices[0]];
        let v1 = cube.vertices[tri_indices[1]];
        let v2 = cube.vertices[tri_indices[2]];

        let (clip0, w0) = mvp.transform_point(v0);
        let (clip1, w1) = mvp.transform_point(v1);
        let (clip2, w2) = mvp.transform_point(v2);

        if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
            continue;
        }

        fill_triangle_3d(fb, zb, (clip0, w0), (clip1, w1), (clip2, w2), color);
    }
}

struct HalationDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    view_proj: Mat4,
    angle_y: f32,
}

impl HalationDemoApp {
    fn new() -> Result<Self, AppError> {
        let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 1.0, 5.0),
            Vec3::new(0.0, 0.0, -2.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
            timestep: FixedTimestep::new(60),
            cube: Mesh::cube(1.0),
            view_proj: projection * view,
            angle_y: 0.0,
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

impl WindowApp for HalationDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Halation Demo".to_string(),
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
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.angle_y += 1.0 * self.timestep.dt();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        // 1. Render dark background cube
        let model1 = Mat4::translation(0.0, 0.0, -3.0)
            * Mat4::rotation_y(-self.angle_y * 0.5)
            * Mat4::scale(2.0, 2.0, 2.0);
        render_cube(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.cube,
            model1,
            self.view_proj,
            0xFF20_2020, // Dark grey
        );

        // 2. Render extremely bright white cube intersecting it
        let model2 = Mat4::translation(1.5 * f32::sin(self.angle_y), 0.0, -2.5)
            * Mat4::rotation_x(self.angle_y)
            * Mat4::scale(0.5, 0.5, 0.5);
        render_cube(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.cube,
            model2,
            self.view_proj,
            0xFFFF_FFFF, // Pure white
        );

        // Apply halation to cause the bright white cube to bleed red onto the dark background
        let config = HalationConfig {
            threshold: 0.8,
            radius: 8,
            intensity: 1.5,
            tint: (1.0, 0.2, 0.0), // Red-orange bleed
        };
        apply_halation(&mut self.framebuffer, config);

        self.present()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), AppError> {
    print_banner();
    run_windowed(HalationDemoApp::new().unwrap());
    Ok(())
}
