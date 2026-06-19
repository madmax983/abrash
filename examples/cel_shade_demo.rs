use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::cel_shade::{CelShadeConfig, apply_cel_shade};
use std::f32::consts::PI;
use std::fmt;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF55_77AA; // Comic blue sky

const COLORS: [u32; 6] = [
    0xFFFF_5555, // Red
    0xFF55_FF55, // Green
    0xFF55_55FF, // Blue
    0xFFFF_FF55, // Yellow
    0xFFFF_55FF, // Magenta
    0xFF55_FFFF, // Cyan
];

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

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<abrash::platform::HostError> for AppError {
    fn from(error: abrash::platform::HostError) -> Self {
        Self(error.to_string())
    }
}

fn print_banner() {
    println!("\n{}", "🖋️  Cel Shading (Comic Book) Demo".bold().magenta());
    println!("{}", "===============================".dark_grey());

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
            Cell::new("Toon Shader post-processing effect").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Features"),
            Cell::new("Color quantization, depth/lum edge detection").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

fn render_cube(fb: &mut Framebuffer, zb: &mut ZBuffer, cube: &Mesh, model: Mat4, view_proj: Mat4) {
    let mvp = view_proj * model;

    for (face_idx, tri_indices) in cube.indices.iter().enumerate() {
        let v0 = cube.vertices[tri_indices[0]];
        let v1 = cube.vertices[tri_indices[1]];
        let v2 = cube.vertices[tri_indices[2]];

        let (clip0, w0) = mvp.transform_point(v0);
        let (clip1, w1) = mvp.transform_point(v1);
        let (clip2, w2) = mvp.transform_point(v2);

        if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
            continue;
        }

        let base_color = COLORS[(face_idx / 2) % 6];

        // Simple faux directional lighting for some interior creases
        let normal = (v1 - v0).cross(v2 - v0).normalize();
        let light_dir = Vec3::new(0.5, 0.5, -0.5).normalize();
        let diff = normal.dot(light_dir).max(0.2); // ambient 0.2

        let r = (((base_color >> 16) & 0xFF) as f32 * diff) as u32;
        let g = (((base_color >> 8) & 0xFF) as f32 * diff) as u32;
        let b = ((base_color & 0xFF) as f32 * diff) as u32;

        let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

        fill_triangle_3d(fb, zb, (clip0, w0), (clip1, w1), (clip2, w2), color);
    }
}

struct CelShadeDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    view_proj: Mat4,
    angle: f32,
    config: CelShadeConfig,
}

impl CelShadeDemoApp {
    fn new() -> Result<Self, AppError> {
        let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 2.0, 6.0),
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
            angle: 0.0,
            config: CelShadeConfig {
                levels: 4,
                edge_threshold: 0.2,
                edge_color: 0xFF_000000,
            },
        })
    }

    fn present(&mut self) -> Result<(), AppError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| std::io::Error::other("software presenter not initialized"))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for CelShadeDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Cel Shading Demo".to_string(),
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
            self.angle += 1.0 * self.timestep.dt();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        // Draw multiple cubes
        let model1 = Mat4::translation(-2.0, 0.0, -2.0) * Mat4::rotation_y(self.angle);
        render_cube(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.cube,
            model1,
            self.view_proj,
        );

        let model2 = Mat4::translation(0.0, 0.0, -1.0)
            * Mat4::rotation_x(self.angle * 0.5)
            * Mat4::rotation_z(self.angle * 0.3);
        render_cube(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.cube,
            model2,
            self.view_proj,
        );

        let model3 = Mat4::translation(2.0, 0.5, -4.0) * Mat4::rotation_z(self.angle * 0.7);
        render_cube(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.cube,
            model3,
            self.view_proj,
        );

        let floor = Mat4::translation(0.0, -1.5, -2.0) * Mat4::scale(10.0, 0.1, 10.0);
        render_cube(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.cube,
            floor,
            self.view_proj,
        );

        apply_cel_shade(&mut self.framebuffer, &self.zbuffer, &self.config);

        self.present()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), AppError> {
    print_banner();
    match CelShadeDemoApp::new() {
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
    Ok(())
}
