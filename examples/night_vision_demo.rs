use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::night_vision::{NightVisionConfig, apply_night_vision};
use std::f32::consts::PI;
use std::fmt;
use std::io::Error as IoError;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF10_1010;

const COLORS: [u32; 6] = [
    0xFFFF_0000,
    0xFF00_FF00,
    0xFF00_00FF,
    0xFFFF_FF00,
    0xFFFF_00FF,
    0xFF00_FFFF,
];

fn print_banner() {
    println!("\n{}", "🌙 Night Vision Demo".bold().cyan());
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
            Cell::new("Simulates military-grade night vision goggles").fg(Color::Green),
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
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Auto-rotating scene"),
        ]);
    println!("{controls}\n");
}

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

        let color = COLORS[(face_idx / 2) % 6];
        fill_triangle_3d(fb, zb, (clip0, w0), (clip1, w1), (clip2, w2), color);
    }
}

struct NightVisionDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    view_proj: Mat4,
    angle_y: f32,
    config: NightVisionConfig,
}

impl NightVisionDemoApp {
    fn new() -> Result<Self, AppError> {
        let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 2.0, 5.0),
            Vec3::new(0.0, 0.0, -5.0),
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
            config: NightVisionConfig::default(),
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

impl WindowApp for NightVisionDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Night Vision Filter Demo".to_string(),
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
            self.config.time += self.timestep.dt();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        let model1 = Mat4::translation(-2.0, 0.0, -2.0) * Mat4::rotation_y(self.angle_y);
        render_cube(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.cube,
            model1,
            self.view_proj,
        );

        let model2 = Mat4::translation(0.0, 0.0, -7.0) * Mat4::rotation_x(self.angle_y * 0.5);
        render_cube(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.cube,
            model2,
            self.view_proj,
        );

        let model3 = Mat4::translation(2.0, 0.0, -12.0) * Mat4::rotation_z(self.angle_y * 0.3);
        render_cube(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.cube,
            model3,
            self.view_proj,
        );

        apply_night_vision(&mut self.framebuffer, &self.config);
        self.present()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), AppError> {
    print_banner();
    match NightVisionDemoApp::new() {
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
