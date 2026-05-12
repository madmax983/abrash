//! Gameboy Camera Demo
//!
//! Applies the Gameboy Camera dither filter to a rotating 3D scene.

use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::phong::fill_triangle_phong;
use abrash::zbuffer::ZBuffer;
use abrash_core::mesh::Mesh;
use abrash_render::experimental::gameboy_camera::apply_gameboy_camera;

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Gameboy Camera Demo".bold().cyan());
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
            Cell::new("Simulates the classic 4-color Gameboy Camera aesthetic").fg(Color::Green),
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
            Cell::new("Close window to exit"),
        ]);
    println!("{controls}\n");
}

const WIDTH: u32 = 320; // Authentic low-res feel
const HEIGHT: u32 = 288;
const TITLE: &str = "🌟 Nova: Gameboy Camera Demo";

struct GameboyCameraDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    mesh: Mesh,
    angle_y: f32,
    angle_x: f32,
}

impl GameboyCameraDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut mesh = Mesh::new();
        // Create a simple cube mesh
        let s = 1.0;
        let v = [
            Vec3::new(-s, -s, -s), Vec3::new(s, -s, -s), Vec3::new(s, s, -s), Vec3::new(-s, s, -s),
            Vec3::new(-s, -s, s), Vec3::new(s, -s, s), Vec3::new(s, s, s), Vec3::new(-s, s, s),
        ];

        let n = [
            Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0),
        ];

        mesh.vertices.extend_from_slice(&v);
        // Duplicate normals per face for simple hard edges
        mesh.normals.extend_from_slice(&[
            n[0], n[0], n[0], n[0],
            n[1], n[1], n[1], n[1],
            n[2], n[2], n[2], n[2],
            n[3], n[3], n[3], n[3],
            n[4], n[4], n[4], n[4],
            n[5], n[5], n[5], n[5],
        ]);

        let indices = [
            [0, 1, 2], [0, 2, 3], // Front
            [5, 4, 7], [5, 7, 6], // Back
            [4, 0, 3], [4, 3, 7], // Left
            [1, 5, 6], [1, 6, 2], // Right
            [4, 5, 1], [4, 1, 0], // Bottom
            [3, 2, 6], [3, 6, 7], // Top
        ];
        mesh.indices.extend_from_slice(&indices);

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            mesh,
            angle_y: 0.0,
            angle_x: 0.0,
        })
    }

    fn present(&mut self) -> Result<(), HostError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for GameboyCameraDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            // Window is scaled up, buffer is small
            width: WIDTH * 2,
            height: HEIGHT * 2,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.angle_y += ctx.dt_seconds * 1.5;
        self.angle_x += ctx.dt_seconds * 0.8;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Clear buffers
        self.framebuffer.clear(0xFF_EEEEEE); // Light background
        self.zbuffer.clear();

        // View-Projection Matrix
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(
            std::f32::consts::FRAC_PI_4,
            WIDTH as f32 / HEIGHT as f32,
            0.1,
            100.0,
        );
        let vp = view * proj;

        // Model Matrix
        let model = Mat4::rotation_x(self.angle_x) * Mat4::rotation_y(self.angle_y);

        let light_dir = Vec3::new(1.0, 1.0, -1.0).fast_normalize();

        // Render Mesh
        for face in &self.mesh.indices {
            // Very hacky indexing mapping since our simple mesh test above didn't perfectly map normals to vertices
            let v0_idx = face[0];
            let v1_idx = face[1];
            let v2_idx = face[2];

            let mut n0 = self.mesh.normals[v0_idx.min(self.mesh.normals.len()-1)];
            let mut n1 = self.mesh.normals[v1_idx.min(self.mesh.normals.len()-1)];
            let mut n2 = self.mesh.normals[v2_idx.min(self.mesh.normals.len()-1)];

            // Transform normals (World space)
            let normal_mat = model.inverse().transpose();
            n0 = normal_mat.transform_vector(n0).fast_normalize();
            n1 = normal_mat.transform_vector(n1).fast_normalize();
            n2 = normal_mat.transform_vector(n2).fast_normalize();

            // Transform vertices to World
            let world_v0 = model.transform_point(self.mesh.vertices[v0_idx]).0;
            let world_v1 = model.transform_point(self.mesh.vertices[v1_idx]).0;
            let world_v2 = model.transform_point(self.mesh.vertices[v2_idx]).0;

            // Transform to Clip
            let (clip_v0, w0) = vp.transform_point(world_v0);
            let (clip_v1, w1) = vp.transform_point(world_v1);
            let (clip_v2, w2) = vp.transform_point(world_v2);

            // Simple backface culling
            let edge1 = clip_v1 - clip_v0;
            let edge2 = clip_v2 - clip_v0;
            if edge1.x * edge2.y - edge1.y * edge2.x > 0.0 {
                continue;
            }

            fill_triangle_phong(
                &mut self.framebuffer,
                &mut self.zbuffer,
                ((clip_v0, w0), n0),
                ((clip_v1, w1), n1),
                ((clip_v2, w2), n2),
                Vec3::new(0.6, 0.6, 0.6), // Color
                light_dir, // Light dir
                Vec3::new(1.0, 1.0, 1.0), // Light color
                Vec3::new(0.2, 0.2, 0.2), // Ambient
            );
        }

        // Apply post-processing filter
        apply_gameboy_camera(&mut self.framebuffer);

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(GameboyCameraDemoApp::new().unwrap());
}
