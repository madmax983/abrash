cat << 'INNER_EOF' > examples/cube_3d.rs
use abrash::anim::Timeline;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;
use std::time::Duration;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

#[cfg(feature = "backend-winit")]
use winit::event::WindowEvent;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF00_0000;
const TITLE: &str = "Abrash - 3D Cube";

// Face colors for the cube
const COLORS: [u32; 6] = [
    0xFFFF_0000, // Red - front
    0xFF00_FF00, // Green - back
    0xFF00_00FF, // Blue - top
    0xFFFF_FF00, // Yellow - bottom
    0xFFFF_00FF, // Magenta - right
    0xFF00_FFFF, // Cyan - left
];

fn print_banner() {
    println!("\n{}", "🧊 Cube 3D Demo".bold().cyan());
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
            Cell::new("Basic 3D cube rendering").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Software Rasterizer").fg(Color::Yellow),
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

struct Cube3dApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    rotation_y: Timeline<f32>,
    rotation_x: Timeline<f32>,
}

impl Cube3dApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            timestep: FixedTimestep::new(60),
            cube: Mesh::cube(1.0),
            rotation_y: Timeline::tween(0.0, std::f32::consts::TAU, Duration::from_secs(6))
                .loop_forever(),
            rotation_x: Timeline::tween(0.0, std::f32::consts::TAU, Duration::from_secs(12))
                .loop_forever(),
        })
    }
}

impl WindowApp for Cube3dApp {
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
        self.presenter = Some(SoftwarePresenter::new(ctx)?);
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
        let steps = self.timestep.update();
        for _ in 0..steps {
            let dt = self.timestep.dt();
            self.rotation_y.tick(dt);
            self.rotation_x.tick(dt);
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let projection = Mat4::perspective(
            PI / 3.0,
            self.framebuffer.width() as f32 / self.framebuffer.height() as f32,
            0.1,
            100.0,
        );
        let view = Mat4::look_at(
            Vec3::new(0.0, 1.5, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        let model = Mat4::rotation_y(self.rotation_y.current_value())
            * Mat4::rotation_x(self.rotation_x.current_value());
        let mvp = projection * (view * model);

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

            fill_triangle_3d(
                &mut self.framebuffer,
                &mut self.zbuffer,
                (clip0, w0),
                (clip1, w1),
                (clip2, w2),
                COLORS[face_idx / 2],
            );
        }

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
    run_windowed(Cube3dApp::new().unwrap());
}
INNER_EOF
