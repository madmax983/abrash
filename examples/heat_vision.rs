use abrash::framebuffer::Framebuffer;
use abrash::heat_vision::apply_heat_vision;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
// Use black background to make heat vision pop
const BACKGROUND: u32 = 0xFF00_0000;
const TITLE: &str = "Abrash - Heat Vision Demo";

fn print_banner() {
    println!("\n{}", "🔥 Heat Vision Demo".bold().red());
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
            Cell::new("Simulates thermal imaging effect").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Effect"),
            Cell::new("Post-process heat map shader").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);
    println!("{controls}\n");
}

struct HeatVisionApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    angle: f32,
}

impl HeatVisionApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            timestep: FixedTimestep::new(60),
            cube: Mesh::cube(1.0),
            angle: 0.0,
        })
    }
}

impl WindowApp for HeatVisionApp {
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

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.angle += 0.5 * self.timestep.dt();
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let projection = Mat4::perspective(
            PI / 3.0,
            self.framebuffer.width() as f32 / self.framebuffer.height() as f32,
            0.1,
            100.0,
        );
        let view = Mat4::look_at(
            Vec3::new(0.0, 3.0, 6.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        let models = [
            Mat4::rotation_y(self.angle) * Mat4::translation(-2.0, 0.0, 1.0),
            Mat4::rotation_x(self.angle * 0.5) * Mat4::rotation_z(self.angle * 0.3),
            Mat4::rotation_y(-self.angle * 0.5) * Mat4::translation(2.0, 0.0, -2.0),
        ];

        for model in models {
            let mvp = projection * (view * model);
            for tri_indices in &self.cube.indices {
                let v0 = self.cube.vertices[tri_indices[0]];
                let v1 = self.cube.vertices[tri_indices[1]];
                let v2 = self.cube.vertices[tri_indices[2]];

                let (clip0, w0) = mvp.transform_point(v0);
                let (clip1, w1) = mvp.transform_point(v1);
                let (clip2, w2) = mvp.transform_point(v2);

                if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                    continue;
                }

                fill_triangle_3d(
                    &mut self.framebuffer,
                    &mut self.zbuffer,
                    (clip0, w0),
                    (clip1, w1),
                    (clip2, w2),
                    0xFFFF_FFFF,
                );
            }
        }

        apply_heat_vision(&mut self.framebuffer, &self.zbuffer);
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
    run_windowed(HeatVisionApp::new().unwrap());
}
