use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::glass_refraction::{
    GlassRefractionConfig, apply_glass_refraction,
};
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Glass Refraction Filter Demo";

struct GlassRefractionApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    mesh: Mesh,
    time: f32,
}

impl GlassRefractionApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|e| HostError::App(e.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT).map_err(|e| HostError::App(e.to_string()))?,
            mesh: Mesh::sphere(1.5, 20, 20),
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

impl WindowApp for GlassRefractionApp {
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

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds.max(0.0);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF_22_22_22);
        self.zbuffer.clear();

        // Draw background checkerboard
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let is_dark = ((x / 40) + (y / 40)) % 2 == 0;
                let color = if is_dark {
                    0xFF_33_33_33
                } else {
                    0xFF_CC_CC_CC
                };
                self.framebuffer.set_pixel(x as i32, y as i32, color);
            }
        }

        let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        let mut model = Mat4::identity();
        model = Mat4::rotation_y(self.time * 0.5) * model;
        model = Mat4::rotation_x(self.time * 0.3) * model;

        let transform = model * view * projection;

        for triangle in &self.mesh.indices {
            let v0 = self.mesh.vertices[triangle[0]];
            let v1 = self.mesh.vertices[triangle[1]];
            let v2 = self.mesh.vertices[triangle[2]];

            let v0_proj = transform.transform_point(v0);
            let v1_proj = transform.transform_point(v1);
            let v2_proj = transform.transform_point(v2);

            fill_triangle_3d(
                &mut self.framebuffer,
                &mut self.zbuffer,
                v0_proj,
                v1_proj,
                v2_proj,
                0xFF_AA_BB_CC,
            );
        }

        let config = GlassRefractionConfig {
            ior: 1.0 + (self.time.sin() * 0.5 + 0.5) * 1.5,
            ..Default::default()
        };
        apply_glass_refraction(&mut self.framebuffer, &self.zbuffer, &config);

        self.present()?;
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = GlassRefractionApp::new()?;
    run_windowed(app);
    Ok(())
}
