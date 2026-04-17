use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::time::FixedTimestep;
use abrash_render::experimental::directional_blur::{
    DirectionalBlurConfig, apply_directional_blur,
};
use std::f32::consts::PI;
use std::fmt;
use std::io::Error as IoError;
use std::sync::Arc;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

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

fn print_banner() {
    println!("\n{}", "✨ Directional Blur Demo".bold().cyan());
    println!("{}", "==========================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Resolution"),
            Cell::new(format!("{WIDTH}x{HEIGHT}")).fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Effect"),
            Cell::new("Directional / Motion Blur").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

struct DirectionalBlurDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    scene: Scene,
    timestep: FixedTimestep,
    time: f32,
    last_cube_x: f32,
    last_cube_y: f32,
}

impl DirectionalBlurDemoApp {
    fn new() -> Result<Self, AppError> {
        let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let eye = Vec3::new(0.0, 2.0, 5.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);
        let camera = Camera::new(view, projection);

        let mut scene = Scene::new(camera);
        let cube_mesh = Arc::new(Mesh::cube(1.0));

        let floor_transform = Mat4::translation(0.0, -1.0, 0.0) * Mat4::scale(10.0, 0.1, 10.0);
        scene.add_object(SceneObject::new(
            cube_mesh.clone(),
            floor_transform,
            0xFF80_8080,
        ));

        scene.add_object(SceneObject::new(cube_mesh, Mat4::identity(), 0xFFFF_5555));

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            scene,
            timestep: FixedTimestep::new(60),
            time: 0.0,
            last_cube_x: 0.0,
            last_cube_y: 0.0,
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

impl WindowApp for DirectionalBlurDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Directional Blur".to_string(),
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
            self.time += 0.016;
        }

        let cube_x = (self.time * 3.0).sin() * 3.0;
        let cube_y = (self.time * 5.0).cos().abs() * 2.0;
        let translation = Mat4::translation(cube_x, cube_y, 0.0);
        let rotation = Mat4::rotation_y(self.time) * Mat4::rotation_z(self.time * 0.5);
        self.scene.objects[1].transform = translation * rotation;

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF11_1111);
        let mut zbuffer = abrash::zbuffer::ZBuffer::new(WIDTH, HEIGHT)?;

        let mut renderer = abrash::rasterizer::tile::TileRenderer::new(WIDTH, HEIGHT);
        self.scene
            .render(&mut renderer, &mut self.framebuffer, &mut zbuffer);

        let cube_x = (self.time * 3.0).sin() * 3.0;
        let cube_y = (self.time * 5.0).cos().abs() * 2.0;
        let dx = cube_x - self.last_cube_x;
        let dy = cube_y - self.last_cube_y;
        self.last_cube_x = cube_x;
        self.last_cube_y = cube_y;

        let config = DirectionalBlurConfig {
            dx: dx * 500.0,
            dy: -dy * 500.0,
            num_samples: 16,
        };
        apply_directional_blur(&mut self.framebuffer, &config);

        self.present()
    }
}

fn main() -> Result<(), AppError> {
    print_banner();
    run_windowed(DirectionalBlurDemoApp::new().unwrap());
    Ok(())
}
