use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::joy_division::{JoyDivisionConfig, apply_joy_division};
use std::f32::consts::PI;
use std::fmt;
use std::io::Error as IoError;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF_000000;

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
    println!("\n{}", "✨ Joy Division Filter Demo".bold().cyan());
    println!("{}", "===========================".dark_grey());

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
            Cell::new("Topographical Waveform Filter").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Features"),
            Cell::new("Masking, Luminance Displacement").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}\n");
}

fn render_scene(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    mesh: &Mesh,
    angle_y: f32,
    angle_x: f32,
    view_proj: Mat4,
) {
    let model =
        Mat4::translation(0.0, 0.0, 0.0) * Mat4::rotation_y(angle_y) * Mat4::rotation_x(angle_x);
    let mvp = view_proj * model;

    for tri_indices in &mesh.indices {
        let v0 = mesh.vertices[tri_indices[0]];
        let v1 = mesh.vertices[tri_indices[1]];
        let v2 = mesh.vertices[tri_indices[2]];

        let (clip0, w0) = mvp.transform_point(v0);
        let (clip1, w1) = mvp.transform_point(v1);
        let (clip2, w2) = mvp.transform_point(v2);

        if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
            continue;
        }

        // Draw the object in pure white so it provides maximum displacement
        fill_triangle_3d(fb, zb, (clip0, w0), (clip1, w1), (clip2, w2), 0xFF_FFFFFF);
    }
}

struct JoyDivisionDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    mesh: Mesh,
    view_proj: Mat4,
    angle_y: f32,
    angle_x: f32,
    total_time: f32,
}

impl JoyDivisionDemoApp {
    fn new() -> Result<Self, AppError> {
        let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 6.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
            timestep: FixedTimestep::new(60),
            // A torus provides a very nice wavy topography
            mesh: Mesh::torus(2.0, 0.8, 20, 20),
            view_proj: projection * view,
            angle_y: 0.0,
            angle_x: 0.0,
            total_time: 0.0,
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

impl WindowApp for JoyDivisionDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Joy Division Demo".to_string(),
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
            self.angle_y += 0.8 * self.timestep.dt();
            self.angle_x += 0.4 * self.timestep.dt();
            self.total_time += self.timestep.dt();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Clear background
        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        // 1. Render the base 3D scene (White shapes)
        render_scene(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.mesh,
            self.angle_y,
            self.angle_x,
            self.view_proj,
        );

        // 2. Apply the Joy Division Post-Processing Effect
        let config = JoyDivisionConfig {
            line_spacing: 12,
            height_scale: 80.0 + (self.total_time * 2.0).sin() * 20.0, // Pulsating height
            foreground_color: 0xFF_FFFFFF,
            background_color: 0xFF_000000,
            smoothing_window: 10,
        };
        apply_joy_division(&mut self.framebuffer, &config);

        self.present()
    }
}

fn main() -> Result<(), AppError> {
    print_banner();
    run_windowed(JoyDivisionDemoApp::new().unwrap());
    Ok(())
}
