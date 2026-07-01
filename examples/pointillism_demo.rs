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

use abrash_render::experimental::pointillism::{PointillismConfig, apply_pointillism};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// Face colors for the cube
const COLORS: [u32; 6] = [
    0xFFFF_0000, // Red
    0xFF00_FF00, // Green
    0xFF00_00FF, // Blue
    0xFFFF_FF00, // Yellow
    0xFFFF_00FF, // Magenta
    0xFF00_FFFF, // Cyan
];

struct PointillismDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    mesh: Mesh,
    angle: f32,
    timestep: FixedTimestep,
    config: PointillismConfig,
}

impl PointillismDemo {
    pub fn new() -> Result<Self, HostError> {
        let framebuffer =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|e| HostError::App(e.to_string()))?;
        let zbuffer = ZBuffer::new(WIDTH, HEIGHT).map_err(|e| HostError::App(e.to_string()))?;
        let mesh = Mesh::cube(1.0);

        Ok(Self {
            presenter: None,
            framebuffer,
            zbuffer,
            mesh,
            angle: 0.0,
            timestep: FixedTimestep::new(60),
            config: PointillismConfig::default(),
        })
    }
}

impl WindowApp for PointillismDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            width: WIDTH,
            height: HEIGHT,
            title: "Abrash - Pointillism Filter".to_string(),
            ..Default::default()
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), HostError> {
        for _ in 0..self.timestep.update() {
            self.angle += 0.02;
            if self.angle > PI * 2.0 {
                self.angle -= PI * 2.0;
            }
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), HostError> {
        self.framebuffer.clear(self.config.canvas_color);
        self.zbuffer.clear();

        let aspect_ratio = WIDTH as f32 / HEIGHT as f32;
        let projection = Mat4::perspective(PI / 3.0, aspect_ratio, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        let model = Mat4::rotation_y(self.angle)
            * Mat4::rotation_x(self.angle * 0.5)
            * Mat4::scale(1.5, 1.5, 1.5);

        let mvp = model * view * projection;

        for (i, face) in self.mesh.indices.iter().enumerate() {
            let color = COLORS[(i / 2) % COLORS.len()];

            let v0 = self.mesh.vertices[face[0]];
            let v1 = self.mesh.vertices[face[1]];
            let v2 = self.mesh.vertices[face[2]];

            let mvp_v0 = mvp.transform_point(v0);
            let mvp_v1 = mvp.transform_point(v1);
            let mvp_v2 = mvp.transform_point(v2);

            fill_triangle_3d(
                &mut self.framebuffer,
                &mut self.zbuffer,
                mvp_v0,
                mvp_v1,
                mvp_v2,
                color,
            );
        }

        apply_pointillism(&mut self.framebuffer, &self.config);

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
    let app = PointillismDemo::new().unwrap();

    println!("Controls:");
    println!("  None.");
    println!("Rendering...");

    run_windowed(app);
}
