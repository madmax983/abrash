use abrash::experimental::matrix_rain::{MatrixRainConfig, apply_matrix_rain};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;
use std::fmt;
use std::io::Error as IoError;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF00_0000;

#[derive(Debug)]
struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for AppError {}
impl From<&'static str> for AppError {
    fn from(error: &'static str) -> Self {
        Self(error.to_string())
    }
}
impl From<String> for AppError {
    fn from(error: String) -> Self {
        Self(error)
    }
}
impl From<IoError> for AppError {
    fn from(error: IoError) -> Self {
        Self(error.to_string())
    }
}
impl From<abrash::platform::HostError> for AppError {
    fn from(error: abrash::platform::HostError) -> Self {
        Self(error.to_string())
    }
}

struct MatrixRainDemoApp {
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    projection: Mat4,
    view: Mat4,
    angle_y: f32,
    angle_x: f32,
    matrix_config: MatrixRainConfig,
}

impl MatrixRainDemoApp {
    fn new() -> Result<Self, AppError> {
        Ok(Self {
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
            timestep: FixedTimestep::new(60),
            cube: Mesh::cube(1.0),
            projection: Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0),
            view: Mat4::look_at(
                Vec3::new(0.0, 1.5, 3.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            angle_y: 0.0,
            angle_x: 0.0,
            matrix_config: MatrixRainConfig::default(),
        })
    }
}

impl WindowApp for MatrixRainDemoApp {
    type Error = AppError;
    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Matrix Rain Demo".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }
    fn init(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.angle_y += 1.0 * self.timestep.dt();
            self.angle_x += 0.5 * self.timestep.dt();
            self.matrix_config.time += self.timestep.dt();
        }
        Ok(())
    }
    fn render(
        &mut self,
        _ctx: WindowContext<'_>,
        presenter: &mut SoftwarePresenter,
    ) -> Result<(), Self::Error> {
        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();
        let model = Mat4::rotation_y(self.angle_y) * Mat4::rotation_x(self.angle_x);
        let mvp = self.projection * (self.view * model);
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
            let intensity = (face_idx as u32 * 30 + 100).min(255);
            let color = 0xFF00_0000 | (intensity << 16) | (intensity << 8) | intensity;
            fill_triangle_3d(
                &mut self.framebuffer,
                &mut self.zbuffer,
                (clip0, w0),
                (clip1, w1),
                (clip2, w2),
                color,
            );
        }
        apply_matrix_rain(&mut self.framebuffer, &mut self.matrix_config);
        presenter.present(&self.framebuffer)?;
        Ok(())
    }
}

fn main() -> Result<(), AppError> {
    println!("Matrix Rain Demo Started");
    run_windowed(MatrixRainDemoApp::new()?)?;
    Ok(())
}
