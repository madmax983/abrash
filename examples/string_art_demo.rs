#![cfg(feature = "backend-winit")]

use abrash_render::experimental::string_art::{StringArtConfig, apply_string_art};
use abrash::framebuffer::Framebuffer;
use abrash::math::Mat4;
use abrash::mesh::Mesh;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;

use std::fmt;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;
const BACKGROUND: u32 = 0xFF22_2222;

const COLORS: [u32; 6] = [
    0xFFFF_0000,
    0xFF00_FF00,
    0xFF00_00FF,
    0xFFFF_FF00,
    0xFFFF_00FF,
    0xFF00_FFFF,
];

#[derive(Debug)]
pub enum AppError {
    Host(HostError),
    Winit(winit::error::OsError),
    Io(std::io::Error),
}

impl std::error::Error for AppError {}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(err) => write!(f, "Host Error: {err:?}"),
            Self::Winit(err) => write!(f, "Winit Error: {err}"),
            Self::Io(err) => write!(f, "IO Error: {err}"),
        }
    }
}

impl From<HostError> for AppError {
    fn from(err: HostError) -> Self {
        Self::Host(err)
    }
}
impl From<winit::error::OsError> for AppError {
    fn from(err: winit::error::OsError) -> Self {
        Self::Winit(err)
    }
}
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

struct StringArtDemoApp {
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    presenter: Option<SoftwarePresenter>,
    config: StringArtConfig,
    cube: Mesh,
    angle_y: f32,
    view_proj: Mat4,
}

impl WindowApp for StringArtDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - String Art Demo".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window).map_err(AppError::Host)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.angle_y += 1.0 * ctx.dt_seconds;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        let model = Mat4::rotation_y(self.angle_y) * Mat4::rotation_x(self.angle_y * 0.5);
        let mvp = model * self.view_proj;

        // Note: Mesh indices are actually a Vec<[usize; 3]> or similar, so we use iter.

        for (face_idx, face) in self.cube.indices.iter().enumerate() {
            let color = COLORS[face_idx / 2 % COLORS.len()];

            let v0 = self.cube.vertices[face[0]];
            let v1 = self.cube.vertices[face[1]];
            let v2 = self.cube.vertices[face[2]];

            let (tv0, w0) = mvp.transform_point(v0);
            let (tv1, w1) = mvp.transform_point(v1);
            let (tv2, w2) = mvp.transform_point(v2);

            fill_triangle_3d(
                &mut self.framebuffer,
                &mut self.zbuffer,
                (tv0, w0),
                (tv1, w1),
                (tv2, w2),
                color,
            );
        }

        apply_string_art(&mut self.framebuffer, &self.config);

        if let Some(presenter) = &mut self.presenter {
            presenter
                .present(&self.framebuffer)
                .map_err(AppError::Host)?;
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let proj = Mat4::perspective(
        std::f32::consts::FRAC_PI_4,
        WIDTH as f32 / HEIGHT as f32,
        0.1,
        100.0,
    );
    let view = Mat4::translation(0.0, 0.0, -3.0);

    let app = StringArtDemoApp {
        framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
        zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
        presenter: None,
        config: StringArtConfig::default(),
        cube: Mesh::cube(1.5),
        angle_y: 0.0,
        view_proj: view * proj,
    };

    run_windowed(app);
    Ok(())
}
