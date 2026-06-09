cat << 'INNER_EOF' > examples/god_rays_demo.rs
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed};
use abrash::post_process::filters::{GodRaysConfig, apply_god_rays};
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
const TITLE: &str = "Abrash - God Rays Demo";

fn print_banner() {
    println!("\n{}", "✨ God Rays Demo".bold().cyan());
    println!("{}", "=======================".dark_grey());

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
            Cell::new("Volumetric Light Scattering").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("A bright cube casting god rays across the screen").fg(Color::White),
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

struct GodRaysApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    rotation: f32,
    light_pos: Vec3,
}

impl GodRaysApp {
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
            light_pos: Vec3::new(0.0, 0.0, 0.0), // Center of rotation
        })
    }
}

impl WindowApp for GodRaysApp {
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
            self.rotation += dt * 0.5; // Rotate slowly
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let fb_w = self.framebuffer.width();
        let fb_h = self.framebuffer.height();

        let projection = Mat4::perspective(PI / 3.0, fb_w as f32 / fb_h as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 2.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        // 1. Draw the "Sun" (a bright white cube)
        let sun_model = Mat4::rotation_y(self.rotation) * Mat4::rotation_x(self.rotation * 0.5);
        let sun_mvp = projection * (view * sun_model);

        for tri_indices in self.cube.indices.chunks(3) {
            let v0 = self.cube.vertices[tri_indices[0]];
            let v1 = self.cube.vertices[tri_indices[1]];
            let v2 = self.cube.vertices[tri_indices[2]];

            let (clip0, w0) = sun_mvp.transform_point(v0);
            let (clip1, w1) = sun_mvp.transform_point(v1);
            let (clip2, w2) = sun_mvp.transform_point(v2);

            if w0 > 0.0 || w1 > 0.0 || w2 > 0.0 {
                fill_triangle_3d(
                    &mut self.framebuffer,
                    &mut self.zbuffer,
                    (clip0, w0),
                    (clip1, w1),
                    (clip2, w2),
                    0xFFFF_FFFF, // Bright white for the sun
                );
            }
        }

        // Project the light source position to screen space to find the origin of the rays
        let (light_clip, light_w) = sun_mvp.transform_point(self.light_pos);

        let light_screen_x = ((light_clip.x + 1.0) * 0.5 * fb_w as f32) as i32;
        let light_screen_y = ((1.0 - light_clip.y) * 0.5 * fb_h as f32) as i32;

        // Only apply rays if the light source is in front of the camera
        if light_w > 0.0 {
            let config = GodRaysConfig {
                light_x: light_screen_x,
                light_y: light_screen_y,
                density: 0.8,
                weight: 0.1,
                decay: 0.95,
                exposure: 0.8,
                num_samples: 50,
            };
            apply_god_rays(&mut self.framebuffer, &config);
        }

        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(&self.framebuffer)?;
        Ok(())
    }
}

fn main() {
    print_banner();
    run_windowed(GodRaysApp::new().unwrap());
}
INNER_EOF
