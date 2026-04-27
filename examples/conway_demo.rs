use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;

use comfy_table::{Cell, Color, Table, presets};

#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
use abrash_render::experimental::conway::{ConwayConfig, apply_conway};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Conway's Game of Life Filter Demo".bold().cyan());
    println!("{}", "===================================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Conway's Game of Life filter mapped onto the screen")
                .fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
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

struct ConwayDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    cube: Mesh,
    timestep: FixedTimestep,
    angle: f32,
}

impl ConwayDemoApp {
    fn new() -> Result<Self, HostError> {
        let cube = Mesh::cube(1.0);


        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            cube,
            timestep: FixedTimestep::new(60),
            angle: 0.0,
        })
    }
}

impl WindowApp for ConwayDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Conway's Game of Life Filter".to_string(),
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
            self.angle += 0.02;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let projection = Mat4::perspective(
            1.57, // PI/2
            self.framebuffer.width() as f32 / self.framebuffer.height() as f32,
            0.1,
            100.0,
        );
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        self.framebuffer.clear(0xFF_000000);
        self.zbuffer.clear();

        let model = Mat4::rotation_x(self.angle) * Mat4::rotation_y(self.angle * 0.7);
        let mvp = projection * (view * model);

        for tri_indices in &self.cube.indices {
            let [i0, i1, i2] = *tri_indices;
            let v0 = self.cube.vertices[i0];
            let v1 = self.cube.vertices[i1];
            let v2 = self.cube.vertices[i2];

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
                0xFF_FFFFFF,
            );
        }

        // Apply Conway filter
        #[cfg(feature = "nova")]
        {
            let config = ConwayConfig {
                cell_size: 6,
                seed_threshold: 128,
                live_color: 0xFF_00FF00,
                dead_color: 0xFF_000000,
                overlay: false,
                overlay_opacity: 1.0,
            };

            apply_conway(&mut self.framebuffer, &config);
        }

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

// Fallback for when "nova" feature is not enabled
#[cfg(not(feature = "nova"))]
fn main() {
    let mut error_table = Table::new();
    error_table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("⚠️  Missing Feature: Nova")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(Color::Red),
        ])
        .add_row(vec![
            Cell::new("This example requires the 'nova' feature to run.").fg(Color::White),
        ])
        .add_row(vec![
            Cell::new("Try running with:\ncargo run --example conway_demo --features nova")
                .fg(Color::Green),
        ]);

    eprintln!("\n{error_table}");
    std::process::exit(1);
}

#[cfg(feature = "nova")]
fn main() {
    print_banner();
    run_windowed(ConwayDemoApp::new().unwrap());
}
