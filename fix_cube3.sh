cat << 'INNER_EOF' > examples/edge_glow_demo.rs
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::post_process::filters::{EdgeGlowConfig, apply_edge_glow};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;
use std::time::Duration;

#[cfg(feature = "backend-winit")]
use winit::event::WindowEvent;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF00_0000;
const TITLE: &str = "Abrash - Edge Glow Demo";

// Bright neon colors for edge detection
const COLORS: [u32; 6] = [
    0xFFFF_0000, // Red
    0xFF00_FF00, // Green
    0xFF00_00FF, // Blue
    0xFFFF_FF00, // Yellow
    0xFFFF_00FF, // Magenta
    0xFF00_FFFF, // Cyan
];

fn print_banner() {
    println!("\n{}", "🌟 Edge Glow Demo".bold().cyan());
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
            Cell::new("Effect"),
            Cell::new("Sobel Edge Detection + Glow").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Highlights geometric edges with a neon glow").fg(Color::White),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);
    println!("{controls}\n");
}

struct EdgeGlowApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    rotation: f32,
}

impl EdgeGlowApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            timestep: FixedTimestep::new(60),
            cube: Mesh::cube(1.0),
            rotation: 0.0,
        })
    }
}

impl WindowApp for EdgeGlowApp {
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
            self.rotation += dt * 0.5;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let fb_w = self.framebuffer.width();
        let fb_h = self.framebuffer.height();

        let projection = Mat4::perspective(PI / 3.0, fb_w as f32 / fb_h as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 1.5, 3.5),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        // Draw multiple cubes
        for i in 0..3 {
            let offset_x = (i as f32 - 1.0) * 1.5;
            let current_rot = self.rotation + (i as f32 * PI / 4.0);

            let model = Mat4::translation(Vec3::new(offset_x, 0.0, 0.0)) *
                       Mat4::rotation_y(current_rot) *
                       Mat4::rotation_x(current_rot * 0.7);

            let mvp = projection * (view * model);

            for (face_idx, tri_indices) in self.cube.indices.iter().enumerate() {
                let v0 = self.cube.vertices[tri_indices[0]];
                let v1 = self.cube.vertices[tri_indices[1]];
                let v2 = self.cube.vertices[tri_indices[2]];

                let (clip0, w0) = mvp.transform_point(v0);
                let (clip1, w1) = mvp.transform_point(v1);
                let (clip2, w2) = mvp.transform_point(v2);

                if w0 > 0.0 || w1 > 0.0 || w2 > 0.0 {
                    let color = COLORS[(face_idx / 2 + i * 2) % COLORS.length];
                    fill_triangle_3d(
                        &mut self.framebuffer,
                        &mut self.zbuffer,
                        (clip0, w0),
                        (clip1, w1),
                        (clip2, w2),
                        color,
                    );
                }
            }
        }

        // Apply Edge Glow Effect
        let config = EdgeGlowConfig {
            threshold: 0.1,
            intensity: 2.0,
            glow_radius: 2,
        };
        apply_edge_glow(&mut self.framebuffer, &config);

        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(&self.framebuffer)?;
        Ok(())
    }
}

trait LengthExt {
    fn length(&self) -> usize;
}

impl<T, const N: usize> LengthExt for [T; N] {
    fn length(&self) -> usize {
        N
    }
}

fn main() {
    print_banner();
    run_windowed(EdgeGlowApp::new().unwrap());
}
INNER_EOF
