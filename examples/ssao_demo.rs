//! Abrash Graphics Demo - SSAO
//!
//! Demonstrates Screen-Space Ambient Occlusion.
//! Renders a scene with multiple objects and a floor to show contact shadows.
//!
//! Use the --ssao flag to control initial state (default: on).
//! SSAO toggles every 3 seconds for comparison.

use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::post_process::{apply_ssao, ssao::SsaoConfig};
use abrash::rasterizer::fill_triangle_lit;
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
    println!("\n{}", "⚫ SSAO Demo".bold().cyan());
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
            Cell::new("Screen-Space Ambient Occlusion").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Status"),
            Cell::new("Toggles every 3 seconds").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("None (Auto-toggle)")]);
    println!("{controls}\n");
}

fn create_floor() -> Mesh {
    let mut mesh = Mesh::new();
    let size = 10.0;
    let h = size / 2.0;

    mesh.vertices.push(Vec3::new(-h, 0.0, h));
    mesh.vertices.push(Vec3::new(h, 0.0, h));
    mesh.vertices.push(Vec3::new(h, 0.0, -h));
    mesh.vertices.push(Vec3::new(-h, 0.0, -h));

    mesh.indices.push([2, 1, 0]);
    mesh.indices.push([3, 2, 0]);

    mesh
}

struct SsaoDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    cube: Mesh,
    floor: Mesh,
    cube_normals: Vec<Vec3>,
    floor_normals: Vec<Vec3>,
    projection: Mat4,
    view: Mat4,
    ambient_color: Vec3,
    sun_dir: Vec3,
    sun_color: Vec3,
    floor_color: Vec3,
    cube_color: Vec3,
    timestep: FixedTimestep,
    time: f32,
}

impl SsaoDemoApp {
    fn new() -> Result<Self, AppError> {
        let cube = Mesh::cube(1.0);
        let floor = create_floor();
        let cube_normals = cube.compute_face_normals();
        let floor_normals = floor.compute_face_normals();

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
            cube,
            floor,
            cube_normals,
            floor_normals,
            projection: Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0),
            view: Mat4::look_at(
                Vec3::new(0.0, 3.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            ambient_color: Vec3::new(0.2, 0.2, 0.2),
            sun_dir: Vec3::new(-0.5, -1.0, -0.3).normalize(),
            sun_color: Vec3::new(0.8, 0.8, 0.8),
            floor_color: Vec3::new(0.6, 0.6, 0.6),
            cube_color: Vec3::new(0.9, 0.4, 0.3),
            timestep: FixedTimestep::new(60),
            time: 0.0,
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

impl WindowApp for SsaoDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - SSAO Demo".to_string(),
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
            self.time += 0.016;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let ssao_enabled = (self.time % 6.0) < 3.0;

        self.framebuffer.clear(0xFF1A_1A2E);
        self.zbuffer.clear();

        let model_floor = Mat4::translation(0.0, -0.5, 0.0);
        let mvp_floor = self.projection * (self.view * model_floor);

        for (face_idx, tri) in self.floor.indices.iter().enumerate() {
            let [i0, i1, i2] = *tri;
            let v0 = mvp_floor.transform_point(self.floor.vertices[i0]);
            let v1 = mvp_floor.transform_point(self.floor.vertices[i1]);
            let v2 = mvp_floor.transform_point(self.floor.vertices[i2]);
            let normal = model_floor.transform_normal(self.floor_normals[face_idx]);

            fill_triangle_lit(
                &mut self.framebuffer,
                &mut self.zbuffer,
                v0,
                v1,
                v2,
                normal,
                self.floor_color,
                self.ambient_color,
                self.sun_dir,
                self.sun_color,
            );
        }

        let cube_positions = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(-1.2, 0.0, 0.5),
            Vec3::new(1.2, 0.0, -0.5),
            Vec3::new(0.0, 1.0, 0.0),
        ];

        for pos in &cube_positions {
            let model = Mat4::translation(pos.x, pos.y, pos.z);
            let mvp = self.projection * (self.view * model);

            for (face_idx, tri) in self.cube.indices.iter().enumerate() {
                let [i0, i1, i2] = *tri;
                let v0 = mvp.transform_point(self.cube.vertices[i0]);
                let v1 = mvp.transform_point(self.cube.vertices[i1]);
                let v2 = mvp.transform_point(self.cube.vertices[i2]);
                let normal = model.transform_normal(self.cube_normals[face_idx]);

                fill_triangle_lit(
                    &mut self.framebuffer,
                    &mut self.zbuffer,
                    v0,
                    v1,
                    v2,
                    normal,
                    self.cube_color,
                    self.ambient_color,
                    self.sun_dir,
                    self.sun_color,
                );
            }
        }

        if ssao_enabled {
            let ssao_config = SsaoConfig {
                radius: 0.5,
                bias: 0.025,
                intensity: 2.0,
            };
            apply_ssao(
                &mut self.framebuffer,
                &self.zbuffer,
                &self.projection,
                &ssao_config,
            );

            for y in 10..20 {
                for x in 10..20 {
                    self.framebuffer.set_pixel(x, y, 0xFF00_FF00);
                }
            }
        } else {
            for y in 10..20 {
                for x in 10..20 {
                    self.framebuffer.set_pixel(x, y, 0xFFFF_0000);
                }
            }
        }

        self.present()?;
        std::thread::sleep(std::time::Duration::from_millis(16));
        Ok(())
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), AppError> {
    print_banner();
    run_windowed(SsaoDemoApp::new().unwrap());
    Ok(())
}
