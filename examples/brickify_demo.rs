#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::unnecessary_wraps)]
//! Abrash Graphics Demo - Lit 3D Cube with Brickify Filter
//!
//! Demonstrates the retro "plastic interlocking bricks" post-processing filter.

use abrash::experimental::brickify::apply_brickify;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_lit;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Brickify Demo".bold().cyan());
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
            Cell::new("Demonstrates the retro plastic interlocking bricks post-processing filter")
                .fg(Color::Green),
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
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Close window to exit"),
        ]);
    println!("{controls}\n");
}

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// Face colors for the cube
const FACE_COLORS: [Vec3; 6] = [
    Vec3 {
        x: 0.90,
        y: 0.30,
        z: 0.24,
    }, // Red
    Vec3 {
        x: 0.18,
        y: 0.80,
        z: 0.44,
    }, // Green
    Vec3 {
        x: 0.20,
        y: 0.60,
        z: 0.86,
    }, // Blue
    Vec3 {
        x: 0.95,
        y: 0.61,
        z: 0.07,
    }, // Orange
    Vec3 {
        x: 0.61,
        y: 0.35,
        z: 0.71,
    }, // Purple
    Vec3 {
        x: 0.10,
        y: 0.74,
        z: 0.61,
    }, // Teal
];

struct BrickifyDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    cube: Mesh,
    face_normals: Vec<Vec3>,
    timestep: FixedTimestep,
    angle_y: f32,
    angle_x: f32,
    block_size: u32,
}

impl BrickifyDemoApp {
    fn new() -> Result<Self, HostError> {
        let cube = Mesh::cube(1.5);
        let face_normals = cube.compute_face_normals();

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            cube,
            face_normals,
            timestep: FixedTimestep::new(60),
            angle_y: 0.0,
            angle_x: 0.0,
            block_size: 10,
        })
    }
}

impl WindowApp for BrickifyDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Brickify Demo".to_string(),
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
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.angle_y += 0.02;
            self.angle_x += 0.008;
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
            Vec3::new(0.0, 2.0, 4.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let ambient_color = Vec3::new(0.15, 0.15, 0.15);
        let sun_dir = Vec3::new(-0.5, -1.0, -0.3).normalize();
        let sun_color = Vec3::new(1.0, 0.95, 0.9);

        self.framebuffer.clear(0xFF1A_1A2E);
        self.zbuffer.clear();

        let model = Mat4::rotation_y(self.angle_y) * Mat4::rotation_x(self.angle_x);
        let mvp = projection * (view * model);

        for (face_idx, tri_indices) in self.cube.indices.iter().enumerate() {
            let [i0, i1, i2] = *tri_indices;
            let v0 = mvp.transform_point(self.cube.vertices[i0]);
            let v1 = mvp.transform_point(self.cube.vertices[i1]);
            let v2 = mvp.transform_point(self.cube.vertices[i2]);
            let world_normal = model.transform_normal(self.face_normals[face_idx]);

            fill_triangle_lit(
                &mut self.framebuffer,
                &mut self.zbuffer,
                v0,
                v1,
                v2,
                world_normal,
                FACE_COLORS[face_idx / 2],
                ambient_color,
                sun_dir,
                sun_color,
            );
        }

        // Apply the Brickify effect
        apply_brickify(&mut self.framebuffer, self.block_size);

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
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(BrickifyDemoApp::new().unwrap())
}
