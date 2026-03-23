//! ASCII Display Demo
//!
//! Renders a rotating 3D lit cube and applies the ASCII display post-processing effect.

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
use abrash::experimental::ascii_display::{AsciiDisplayConfig, apply_ascii_display};

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

struct AsciiDisplayDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    cube: Mesh,
    face_normals: Vec<Vec3>,
    timestep: FixedTimestep,
    angle_y: f32,
    angle_x: f32,
}

impl AsciiDisplayDemo {
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
        })
    }
}

impl WindowApp for AsciiDisplayDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: ASCII Display Demo".to_string(),
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

        self.framebuffer.clear(0xFF_000000);
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

        // Apply ASCII filter
        #[cfg(feature = "nova")]
        {
            let config = AsciiDisplayConfig {
                cell_width: 6,
                cell_height: 10,
                colorize: true,
                monochrome_color: 0xFF_00FF00,
                background_color: 0xFF_000000,
            };

            apply_ascii_display(&mut self.framebuffer, &config);
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

// Fallback for when "nova" feature is not enabled
#[cfg(not(feature = "nova"))]
fn main() {
    use comfy_table::Table;
    let mut table = Table::new();
    table.set_header(vec!["Error"]);
    table.add_row(vec![
        "This example requires the `nova` feature.\nRun with: cargo run --example ascii_display_demo --features nova",
    ]);
    println!("{table}");
}

#[cfg(feature = "nova")]
fn main() -> Result<(), HostError> {
    println!("Starting ASCII Display Demo...");
    run_windowed(AsciiDisplayDemo::new()?)
}
