use abrash::experimental::glitch::apply_glitch;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Glitch / Datamosh Demo";

fn print_banner() {
    println!("\n{}", "🌟 Glitch Effect Demo".bold().cyan());
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
            Cell::new("Post-processing digital corruption and RGB separation").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Behavior"),
            Cell::new("Intensity pulses dynamically over time").fg(Color::Yellow),
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
            Cell::new("Auto-rotating object"),
        ]);
    println!("{controls}\n");
}

struct GlitchDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    mesh: Mesh,
    rotation_y: f32,
    rotation_x: f32,
    time_elapsed: f32,
}

impl GlitchDemo {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            mesh: Mesh::cube(1.0),
            rotation_y: 0.0,
            rotation_x: 0.0,
            time_elapsed: 0.0,
        })
    }
}

impl WindowApp for GlitchDemo {
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

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.rotation_y += ctx.dt_seconds * 1.5;
        self.rotation_x += ctx.dt_seconds * 1.0;
        self.time_elapsed += ctx.dt_seconds;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();

        // 1. Draw a background gradient
        let pixels = self.framebuffer.as_mut_slice();
        for y in 0..height {
            let t = y as f32 / height as f32;
            let r = (20.0 * (1.0 - t) + 10.0 * t) as u32;
            let g = (20.0 * (1.0 - t) + 30.0 * t) as u32;
            let b = (40.0 * (1.0 - t) + 80.0 * t) as u32;
            let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

            let offset = y as usize * width as usize;
            for x in 0..width as usize {
                pixels[offset + x] = color;
            }
        }

        self.zbuffer.clear();

        // 2. Setup Camera
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.047, width as f32 / height as f32, 0.1, 100.0);
        let view_proj = view * proj;

        // 3. Setup Model
        let model = Mat4::rotation_y(self.rotation_y) * Mat4::rotation_x(self.rotation_x);

        // Directional Light
        let light_dir = Vec3::new(1.0, 1.0, 1.0).normalize();

        // 4. Rasterize Mesh with flat shading
        for face in &self.mesh.indices {
            let i0 = face[0];
            let i1 = face[1];
            let i2 = face[2];

            let v0_local = self.mesh.vertices[i0];
            let v1_local = self.mesh.vertices[i1];
            let v2_local = self.mesh.vertices[i2];

            // Transform to World Space
            let (v0_world, _) = model.transform_point(v0_local);
            let (v1_world, _) = model.transform_point(v1_local);
            let (v2_world, _) = model.transform_point(v2_local);

            // Compute face normal
            let edge1: Vec3 = v1_world - v0_world;
            let edge2: Vec3 = v2_world - v0_world;
            let normal = edge1.cross(edge2).normalize();

            // Backface culling
            let view_dir = (Vec3::new(0.0, 0.0, 3.0) - v0_world).normalize();
            if normal.dot(view_dir) <= 0.0 {
                continue;
            }

            // Simple diffuse lighting
            let diffuse = normal.dot(light_dir).max(0.0);
            let ambient = 0.2;
            let intensity = (diffuse + ambient).min(1.0);

            let base_r = 255.0;
            let base_g = 100.0;
            let base_b = 255.0; // Magenta-ish

            let r = (base_r * intensity) as u32;
            let g = (base_g * intensity) as u32;
            let b = (base_b * intensity) as u32;
            let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

            // Transform to Clip Space
            let v0_clip = view_proj.transform_point(v0_world);
            let v1_clip = view_proj.transform_point(v1_world);
            let v2_clip = view_proj.transform_point(v2_world);

            fill_triangle_3d(
                &mut self.framebuffer,
                &mut self.zbuffer,
                v0_clip,
                v1_clip,
                v2_clip,
                color,
            );
        }

        // 5. Apply Glitch Post-Processing Filter
        // Pulse the glitch intensity over time
        let raw_intensity = (self.time_elapsed * 2.0).sin();

        // Only glitch 25% of the time, mapping sine wave peaks to a 0.0-1.0 intensity range
        let intensity = if raw_intensity > 0.5 {
            (raw_intensity - 0.5) * 2.0
        } else {
            0.0
        };

        apply_glitch(&mut self.framebuffer, intensity, self.time_elapsed);

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
    run_windowed(GlitchDemo::new().unwrap())
}
