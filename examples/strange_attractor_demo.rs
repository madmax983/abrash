#![cfg(feature = "backend-winit")]

use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::strange_attractor::generate_lorenz_attractor;
use abrash::zbuffer::ZBuffer;
use abrash::rasterizer::draw_line_3d;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "Abrash - Strange Attractor";

struct StrangeAttractorApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    mesh: Mesh,
    angle_y: f32,
}

impl StrangeAttractorApp {
    fn new() -> Result<Self, HostError> {
        let framebuffer = Framebuffer::new(WIDTH, HEIGHT).map_err(|e| HostError::App(e.to_string()))?;
        let zbuffer = ZBuffer::new(WIDTH, HEIGHT).map_err(|e| HostError::App(e.to_string()))?;

        // Generate the Lorenz Attractor mesh
        let iterations = 10000;
        let dt = 0.005;
        let sigma = 10.0;
        let rho = 28.0;
        let beta = 8.0 / 3.0;
        let mesh = generate_lorenz_attractor(iterations, dt, sigma, rho, beta);

        Ok(Self {
            presenter: None,
            framebuffer,
            zbuffer,
            mesh,
            angle_y: 0.0,
        })
    }
}

impl WindowApp for StrangeAttractorApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            width: WIDTH,
            height: HEIGHT,
            title: TITLE.to_string(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window.clone())?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.angle_y += 0.01;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF10_1020);
        self.zbuffer.clear();

        let center_offset = Vec3::new(0.0, 0.0, -27.0);

        let model = Mat4::translation(center_offset.x, center_offset.y, center_offset.z) * Mat4::rotation_y(self.angle_y);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 80.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let projection = Mat4::perspective(PI / 4.0, WIDTH as f32 / HEIGHT as f32, 0.1, 200.0);

        let mvp = projection * (view * model);
        let color = 0xFF00_FFFF;

        for (face_idx, tri_indices) in self.mesh.indices.iter().enumerate() {
            let v0 = self.mesh.vertices[tri_indices[0]];
            let v1 = self.mesh.vertices[tri_indices[1]];
            let v2 = self.mesh.vertices[tri_indices[2]];

            let (clip0, w0) = mvp.transform_point(v0);
            let (clip1, w1) = mvp.transform_point(v1);
            let (clip2, w2) = mvp.transform_point(v2);

            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                continue;
            }

            draw_line_3d(&mut self.framebuffer, &mut self.zbuffer, (clip0, w0), (clip1, w1), color);
            draw_line_3d(&mut self.framebuffer, &mut self.zbuffer, (clip1, w1), (clip2, w2), color);
            draw_line_3d(&mut self.framebuffer, &mut self.zbuffer, (clip2, w2), (clip0, w0), color);
        }

        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(&self.framebuffer)?;

        Ok(())
    }
}

fn main() -> Result<(), HostError> {
    let app = StrangeAttractorApp::new()?;
    run_windowed(app);
    Ok(())
}
