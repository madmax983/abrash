use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::anaglyph::{AnaglyphConfig, apply_anaglyph};
use std::f32::consts::PI;
use std::fmt;
use std::io::Error as IoError;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF00_0000;

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
    println!("\n{}", "👓 Anaglyph 3D Demo".bold().cyan());
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
            Cell::new("Stereoscopic 3D rendering").fg(Color::Green),
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

struct AnaglyphDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    projection: Mat4,
    view: Mat4,
    angle_y: f32,
    angle_x: f32,
    anaglyph_config: AnaglyphConfig,
}

impl AnaglyphDemoApp {
    fn new() -> Result<Self, AppError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
            timestep: FixedTimestep::new(60),
            cube: Mesh::cube(1.0),
            projection: Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0),
            view: Mat4::look_at(
                Vec3::new(0.0, 1.5, 3.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            angle_y: 0.0,
            angle_x: 0.0,
            anaglyph_config: AnaglyphConfig {
                max_offset: 20,
                focal_depth: 3.5,
            },
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

impl WindowApp for AnaglyphDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Anaglyph 3D".to_string(),
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
            self.angle_y += 1.0 * self.timestep.dt();
            self.angle_x += 0.5 * self.timestep.dt();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        let model = Mat4::rotation_y(self.angle_y) * Mat4::rotation_x(self.angle_x);
        let mvp = self.projection * (self.view * model);

        for (face_idx, tri_indices) in self.cube.indices.iter().enumerate() {
            let v0 = self.cube.vertices[tri_indices[0]];
            let v1 = self.cube.vertices[tri_indices[1]];
            let v2 = self.cube.vertices[tri_indices[2]];

            let (clip0, w0) = mvp.transform_point(v0);
            let (clip1, w1) = mvp.transform_point(v1);
            let (clip2, w2) = mvp.transform_point(v2);

            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                continue;
            }

            let intensity = (face_idx as u32 * 30 + 100).min(255);
            let color = 0xFF00_0000 | (intensity << 16) | (intensity << 8) | intensity;

            fill_triangle_3d(
                &mut self.framebuffer,
                &mut self.zbuffer,
                (clip0, w0),
                (clip1, w1),
                (clip2, w2),
                color,
            );
        }

        let floor_model = Mat4::translation(0.0, -1.0, 0.0) * Mat4::scale(5.0, 0.1, 5.0);
        let floor_mvp = self.projection * (self.view * floor_model);

        for tri_indices in &self.cube.indices {
            let v0 = self.cube.vertices[tri_indices[0]];
            let v1 = self.cube.vertices[tri_indices[1]];
            let v2 = self.cube.vertices[tri_indices[2]];

            let (clip0, w0) = floor_mvp.transform_point(v0);
            let (clip1, w1) = floor_mvp.transform_point(v1);
            let (clip2, w2) = floor_mvp.transform_point(v2);

            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                continue;
            }

            fill_triangle_3d(
                &mut self.framebuffer,
                &mut self.zbuffer,
                (clip0, w0),
                (clip1, w1),
                (clip2, w2),
                0xFF_444444,
            );
        }

        apply_anaglyph(&mut self.framebuffer, &self.zbuffer, self.anaglyph_config);
        self.present()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), AppError> {
    print_banner();
    run_windowed(AnaglyphDemoApp::new().unwrap());
    Ok(())
}
