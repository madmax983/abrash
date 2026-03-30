use abrash::experimental::comic::{ComicConfig, apply_comic};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;
use std::fmt;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF_FFFFFF;

#[derive(Debug)]
struct AppError(String);
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for AppError {}
impl From<String> for AppError { fn from(s: String) -> Self { Self(s) } }
impl From<abrash::platform::HostError> for AppError { fn from(e: abrash::platform::HostError) -> Self { Self(e.to_string()) } }

struct ComicDemo {
    presenter: Option<SoftwarePresenter>,
    fb: Framebuffer,
    zb: ZBuffer,
    mesh: Mesh,
    angle: f32,
    timestep: FixedTimestep,
    config: ComicConfig,
}

impl ComicDemo {
    fn new() -> Self {
        let mesh = Mesh::cube(1.0);

        Self {
            presenter: None,
            fb: Framebuffer::new(WIDTH, HEIGHT).unwrap(),
            zb: ZBuffer::new(WIDTH, HEIGHT).unwrap(),
            mesh,
            angle: 0.0,
            timestep: FixedTimestep::new(60),
            config: ComicConfig {
                paint_radius: 2,
                edge_threshold: 15,
                use_halftone: false,
                halftone_dot_size: 5.0,
                halftone_angle: PI / 4.0,
            },
        }
    }
}

impl WindowApp for ComicDemo {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Comic Book Filter Demo (Nova)".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.angle += 0.02;
            // Omitted input handling for this simplistic demo
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.fb.clear(BACKGROUND);
        self.zb.clear();

        let model = Mat4::rotation_y(self.angle) * Mat4::rotation_x(self.angle * 0.5);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let mvp = proj * view * model;

        for tri_indices in &self.mesh.indices {
            let v0 = self.mesh.vertices[tri_indices[0]];
            let v1 = self.mesh.vertices[tri_indices[1]];
            let v2 = self.mesh.vertices[tri_indices[2]];

            let clip0 = mvp.transform_point(v0);
            let clip1 = mvp.transform_point(v1);
            let clip2 = mvp.transform_point(v2);

            // Assign colors based on face
            let color = match tri_indices[0] % 6 {
                0 => 0xFF_FF0000,
                1 => 0xFF_00FF00,
                2 => 0xFF_0000FF,
                3 => 0xFF_FFFF00,
                4 => 0xFF_FF00FF,
                _ => 0xFF_00FFFF,
            };

            fill_triangle_3d(
                &mut self.fb,
                &mut self.zb,
                clip0,
                clip1,
                clip2,
                color,
            );
        }

        // Apply comic filter
        apply_comic(&mut self.fb, &self.config);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.fb)?;
        }

        Ok(())
    }
}

fn main() {
    let app = ComicDemo::new();
    run_windowed(app).unwrap();
}
