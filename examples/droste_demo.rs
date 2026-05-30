//! Droste Effect Demo
//!
//! Demonstrates the recursive picture-in-picture Droste effect
//! applied to a 3D scene (a rotating cube).

use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::droste::{DrosteConfig, apply_droste};
use abrash_render::rasterizer::fill_triangle_3d;
use std::f32::consts::PI;

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Droste Effect Filter Demo".bold().cyan());
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
            Cell::new("Recursive picture-in-picture post-processing effect").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Droste Effect Filter Demo";

struct DrosteDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    cube: Mesh,
    time: f32,
}

impl DrosteDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            cube: Mesh::cube(1.0),
            time: 0.0,
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

impl WindowApp for DrosteDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds.max(0.0);
        Ok(())
    }

    fn render(&mut self, _ctx: &WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF_10_20_30);
        self.zbuffer.clear();

        let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 2.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        let colors = [0xFF_FF5555, 0xFF_55FF55, 0xFF_5555FF];

        let elapsed = self.time;

        let models = [
            Mat4::rotation_y(elapsed * 0.5) * Mat4::rotation_x(elapsed * 0.3),
            Mat4::translation(2.0, 0.0, 0.0)
                * Mat4::rotation_y(-elapsed)
                * Mat4::scale(0.5, 0.5, 0.5),
            Mat4::translation(-2.0, 0.0, 0.0)
                * Mat4::rotation_z(elapsed)
                * Mat4::scale(0.5, 0.5, 0.5),
        ];

        for (i, model) in models.iter().enumerate() {
            let mvp = projection * (view * *model);

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

                // Simple flat coloring for each face
                // Derive face color from the object's base color
                let base_color = colors[i];
                let shade = 255 - (face_idx as u32 * 20);
                let r = ((base_color >> 16) & 0xFF) * shade / 255;
                let g = ((base_color >> 8) & 0xFF) * shade / 255;
                let b = (base_color & 0xFF) * shade / 255;
                let color = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;

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

        let config = DrosteConfig {
            iterations: 5,
            scale: 0.5 + (elapsed.sin() * 0.2),
            offset_x: (elapsed * 0.5).cos() * 0.2,
            offset_y: (elapsed * 0.5).sin() * 0.2,
        };

        apply_droste(&mut self.framebuffer, &config);

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(DrosteDemoApp::new().unwrap());
}
