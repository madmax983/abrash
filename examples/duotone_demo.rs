use abrash::experimental::duotone::apply_duotone;
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

struct DuotoneDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    view_proj: Mat4,
    angle: f32,
}

impl DuotoneDemo {
    fn new() -> Result<Self, AppError> {
        let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 2.0, 5.0),
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
            timestep: FixedTimestep::new(60),
            cube: Mesh::cube(1.0),
            view_proj: projection * view,
            angle: 0.0,
        })
    }

    fn present(&mut self) -> Result<(), AppError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| IoError::other("software presenter not initialized"))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for DuotoneDemo {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Duotone Filter Demo".to_string(),
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
            self.angle += 1.0 * self.timestep.dt();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF10_1010);
        self.zbuffer.clear();

        // Draw a background pattern (simple gradient) before drawing the cube
        let width = self.framebuffer.width() as usize;
        let height = self.framebuffer.height() as usize;
        for y in 0..height {
            let row_offset = y * width;
            for x in 0..width {
                let r = (x as f32 / width as f32 * 255.0) as u32;
                let g = (y as f32 / height as f32 * 255.0) as u32;
                self.framebuffer.as_mut_slice()[row_offset + x] =
                    0xFF00_0000 | (r << 16) | (g << 8) | 128;
            }
        }

        // --- Core 3D Rendering (Cube) ---
        let model = Mat4::rotation_y(self.angle) * Mat4::rotation_x(self.angle * 0.5);
        let mvp = self.view_proj * model;

        for (_face_idx, tri_indices) in self.cube.indices.iter().enumerate() {
            let v0 = self.cube.vertices[tri_indices[0]];
            let v1 = self.cube.vertices[tri_indices[1]];
            let v2 = self.cube.vertices[tri_indices[2]];

            let (clip0, w0) = mvp.transform_point(v0);
            let (clip1, w1) = mvp.transform_point(v1);
            let (clip2, w2) = mvp.transform_point(v2);

            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                continue;
            }

            // Let's use a solid color for the cube to let the duotone do the coloring
            fill_triangle_3d(
                &mut self.framebuffer,
                &mut self.zbuffer,
                (clip0, w0),
                (clip1, w1),
                (clip2, w2),
                0xFFDD_DDDD,
            );
        }

        // --- Apply Post-Processing Filter ---
        // Apply the Duotone filter: Dark Blue and Light Pink/Orange
        apply_duotone(&mut self.framebuffer, 0xFF001133, 0xFFFF6699);

        self.present()
    }
}

fn main() -> Result<(), AppError> {
    run_windowed(DuotoneDemo::new().unwrap());
    Ok(())
}
