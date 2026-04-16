//! Selective Color Demo
//!
//! Renders rotating 3D lit cubes with different colors and applies the Selective Color
//! post-processing effect to emphasize only the target hue.

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

#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
use abrash::experimental::selective_color::{SelectiveColorConfig, apply_selective_color};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Selective Color Demo".bold().cyan());
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
            Cell::new("Renders rotating 3D lit cubes with different colors and applies the Selective Color effect")
                .fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Effect"),
            Cell::new("Converts image to grayscale except for a specific target hue").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

#[cfg(not(feature = "nova"))]
fn main() {
    println!("This example requires the 'nova' feature.");
    println!("Run with: cargo run --release --example selective_color_demo --features nova");
}

#[cfg(feature = "nova")]
fn main() -> Result<(), HostError> {
    print_banner();

    let config = WindowHostConfig {
        width: WIDTH,
        height: HEIGHT,
        title: "Abrash - Selective Color Demo".to_string(),
        ..Default::default()
    };
    run_windowed::<SelectiveColorApp>(config)
}

#[cfg(feature = "nova")]
struct SelectiveColorApp {
    fb: Framebuffer,
    zb: ZBuffer,
    mesh: Mesh,
    rotation_y: f32,
    rotation_x: f32,
    timer: FixedTimestep,
    light_dir: Vec3,
    filter_config: SelectiveColorConfig,
}

#[cfg(feature = "nova")]
impl WindowApp for SelectiveColorApp {
    fn new(_ctx: &WindowContext) -> Result<Self, HostError> {
        let mut mesh = Mesh::cube();
        mesh.compute_normals();

        let target_fps = 60.0;
        let timer = FixedTimestep::new(target_fps);

        Ok(Self {
            fb: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            zb: ZBuffer::new(WIDTH, HEIGHT).unwrap(),
            mesh,
            rotation_y: 0.0,
            rotation_x: 0.0,
            timer,
            light_dir: Vec3::new(-1.0, 1.0, -1.0).normalize(),
            filter_config: SelectiveColorConfig {
                target_hue: 0.0, // Red
                tolerance: 45.0,
                desaturation: 1.0,
            },
        })
    }

    fn update(&mut self, _ctx: &WindowContext) {
        if self.timer.update() {
            self.rotation_y += 0.02;
            self.rotation_x += 0.015;

            // Animate target hue over time
            self.filter_config.target_hue = (self.rotation_y * 50.0) % 360.0;
        }
    }

    fn draw(&mut self, presenter: &mut SoftwarePresenter) {
        self.fb.clear(0xFF111111); // Dark grey background
        self.zb.clear();

        let aspect = WIDTH as f32 / HEIGHT as f32;
        let proj = Mat4::perspective(PI / 3.0, aspect, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        let view_proj = view * proj;

        // Render multiple cubes with different colors
        let cube_configs = [
            (Vec3::new(-1.5, 0.0, 0.0), 0xFFFF0000), // Red
            (Vec3::new(1.5, 0.0, 0.0), 0xFF00FF00),  // Green
            (Vec3::new(0.0, 1.5, 0.0), 0xFF0000FF),  // Blue
            (Vec3::new(0.0, -1.5, 0.0), 0xFFFFFF00), // Yellow
        ];

        for (pos, color) in &cube_configs {
            let model = Mat4::translation(*pos)
                * Mat4::rotation_x(self.rotation_x)
                * Mat4::rotation_y(self.rotation_y);
            let mvp = model * view_proj;

            // Transform light direction to local space for shading
            let inv_model = model.invert().unwrap_or_else(Mat4::identity);
            let local_light_dir = inv_model.transform_vector(self.light_dir).normalize();

            for tri in self.mesh.triangles() {
                let v0_local = self.mesh.vertices[tri.v0];
                let v1_local = self.mesh.vertices[tri.v1];
                let v2_local = self.mesh.vertices[tri.v2];

                let v0_clip = mvp.transform_point(v0_local);
                let v1_clip = mvp.transform_point(v1_local);
                let v2_clip = mvp.transform_point(v2_local);

                let n0_local = self.mesh.normals[tri.n0];
                let n1_local = self.mesh.normals[tri.n1];
                let n2_local = self.mesh.normals[tri.n2];

                // Backface culling in local space
                let edge1 = v1_local - v0_local;
                let edge2 = v2_local - v0_local;
                let face_normal = edge1.cross(edge2).normalize();

                let view_dir_local =
                    inv_model.transform_point(Vec3::new(0.0, 0.0, 5.0)).0 - v0_local;

                if face_normal.dot(view_dir_local) > 0.0 {
                    let calc_intensity = |normal: Vec3| -> f32 {
                        let ambient = 0.2;
                        let diffuse = normal.dot(local_light_dir).max(0.0) * 0.8;
                        ambient + diffuse
                    };

                    let i0 = calc_intensity(n0_local);
                    let i1 = calc_intensity(n1_local);
                    let i2 = calc_intensity(n2_local);

                    fill_triangle_lit(
                        &mut self.fb,
                        &mut self.zb,
                        v0_clip,
                        v1_clip,
                        v2_clip,
                        i0,
                        i1,
                        i2,
                        *color,
                    );
                }
            }
        }

        // Apply selective color post-processing effect
        apply_selective_color(&mut self.fb, &self.filter_config);

        presenter.present(&self.fb);
    }
}
