//! Abrash Graphics Demo - Depth Visualizer
//!
//! Demonstrates rendering a 3D scene and using an experimental post-processing
//! effect to visualize the Z-Buffer directly as a grayscale image.

use abrash::experimental::depth_visualizer::depth_visualize;
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

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "Abrash - Depth Visualizer";

fn print_banner() {
    println!("\n{}", "🎥 Depth Visualizer Demo".bold().cyan());
    println!("{}", "==========================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Effect"),
            Cell::new("Z-Buffer visualization mapping depth to luminance").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Mapping"),
            Cell::new("White (Near Plane) -> Black (Far Plane)").fg(Color::Yellow),
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
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Auto-rotating objects"),
        ]);
    println!("{controls}\n");
}

struct DepthVisualizerApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    cube: Mesh,
    face_normals: Vec<Vec3>,
    timestep: FixedTimestep,
    angle_y: f32,
    angle_x: f32,
}

impl DepthVisualizerApp {
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

impl WindowApp for DepthVisualizerApp {
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
        let near_plane = 0.1;
        let far_plane = 10.0;
        let projection = Mat4::perspective(
            PI / 3.0,
            self.framebuffer.width() as f32 / self.framebuffer.height() as f32,
            near_plane,
            far_plane,
        );
        let view = Mat4::look_at(
            Vec3::new(0.0, 1.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let ambient_color = Vec3::new(1.0, 1.0, 1.0);
        let sun_dir = Vec3::new(-0.5, -1.0, -0.3).normalize();
        let sun_color = Vec3::new(0.0, 0.0, 0.0);

        self.framebuffer.clear(0xFF1A_1A2E);
        self.zbuffer.clear();

        // Render first cube
        let model1 = Mat4::translation(-1.0, 0.0, 0.0)
            * Mat4::rotation_y(self.angle_y)
            * Mat4::rotation_x(self.angle_x);
        let mvp1 = projection * (view * model1);

        for (face_idx, tri_indices) in self.cube.indices.iter().enumerate() {
            let [i0, i1, i2] = *tri_indices;
            let v0 = mvp1.transform_point(self.cube.vertices[i0]);
            let v1 = mvp1.transform_point(self.cube.vertices[i1]);
            let v2 = mvp1.transform_point(self.cube.vertices[i2]);
            let world_normal = model1.transform_normal(self.face_normals[face_idx]);

            fill_triangle_lit(
                &mut self.framebuffer,
                &mut self.zbuffer,
                v0,
                v1,
                v2,
                world_normal,
                Vec3::new(1.0, 1.0, 1.0),
                ambient_color,
                sun_dir,
                sun_color,
            );
        }

        // Render second cube further back
        let model2 = Mat4::translation(1.0, 0.0, -2.0)
            * Mat4::rotation_y(-self.angle_y * 1.5)
            * Mat4::rotation_x(-self.angle_x * 0.5);
        let mvp2 = projection * (view * model2);

        for (face_idx, tri_indices) in self.cube.indices.iter().enumerate() {
            let [i0, i1, i2] = *tri_indices;
            let v0 = mvp2.transform_point(self.cube.vertices[i0]);
            let v1 = mvp2.transform_point(self.cube.vertices[i1]);
            let v2 = mvp2.transform_point(self.cube.vertices[i2]);
            let world_normal = model2.transform_normal(self.face_normals[face_idx]);

            fill_triangle_lit(
                &mut self.framebuffer,
                &mut self.zbuffer,
                v0,
                v1,
                v2,
                world_normal,
                Vec3::new(1.0, 1.0, 1.0),
                ambient_color,
                sun_dir,
                sun_color,
            );
        }

        // Apply depth visualizer post-processing
        depth_visualize(&mut self.framebuffer, &self.zbuffer, near_plane, far_plane);

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

fn main() -> Result<(), HostError> {
    print_banner();
    run_windowed(DepthVisualizerApp::new()?)
}
