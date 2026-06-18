use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::paper_cutout::{PaperCutoutConfig, apply_paper_cutout};
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::fmt;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFFF0_F0F0;

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

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<abrash::platform::HostError> for AppError {
    fn from(error: abrash::platform::HostError) -> Self {
        Self(error.to_string())
    }
}

fn print_banner() {
    println!("\n{}", "✂️  Paper Cutout Demo".bold().cyan());
    println!("{}", "===========================".dark_grey());

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
            Cell::new("2.5D Layered Paper Shader").fg(Color::Green),
        ]);

    println!("{table}\n");
}

struct PaperCutoutDemoApp {
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    presenter: Option<SoftwarePresenter>,
    config: PaperCutoutConfig,
    timestep: FixedTimestep,
    angle: f32,
    view_proj: Mat4,
}

impl PaperCutoutDemoApp {
    fn new() -> Result<Self, AppError> {
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, -10.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view_proj = view * proj;

        Ok(Self {
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
            presenter: None,
            config: PaperCutoutConfig {
                layers: 5,
                shadow_offset_x: 10,
                shadow_offset_y: 15,
                shadow_opacity: 0.6,
                outline_color: 0x0000_0000,
            },
            timestep: FixedTimestep::new(60),
            angle: 0.0,
            view_proj,
        })
    }

    fn present(&mut self) -> Result<(), AppError> {
        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }
        Ok(())
    }
}

impl WindowApp for PaperCutoutDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Paper Cutout Demo".to_string(),
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
        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        let draw_tri = |fb: &mut Framebuffer,
                        zb: &mut ZBuffer,
                        z: f32,
                        color: u32,
                        size: f32,
                        x_off: f32,
                        y_off: f32,
                        vp: &Mat4| {
            let (v0, w0) = vp.transform_point(Vec3::new(x_off, size + y_off, z));
            let (v1, w1) = vp.transform_point(Vec3::new(-size + x_off, -size + y_off, z));
            let (v2, w2) = vp.transform_point(Vec3::new(size + x_off, -size + y_off, z));
            fill_triangle_3d(fb, zb, (v0, w0), (v1, w1), (v2, w2), color);
        };

        let move_x = self.angle.sin() * 2.0;

        // Deepest layer
        draw_tri(
            &mut self.framebuffer,
            &mut self.zbuffer,
            8.0,
            0xFF44_88FF,
            5.0,
            -2.0 - move_x,
            2.0,
            &self.view_proj,
        );
        draw_tri(
            &mut self.framebuffer,
            &mut self.zbuffer,
            7.5,
            0xFF55_99FF,
            4.0,
            3.0 + move_x,
            1.0,
            &self.view_proj,
        );

        // Mid layer
        draw_tri(
            &mut self.framebuffer,
            &mut self.zbuffer,
            5.0,
            0xFF22_AA22,
            3.0,
            -3.0 + move_x,
            -1.0,
            &self.view_proj,
        );
        draw_tri(
            &mut self.framebuffer,
            &mut self.zbuffer,
            4.5,
            0xFF33_BB33,
            2.5,
            1.0 - move_x,
            -2.0,
            &self.view_proj,
        );

        // Near layer
        draw_tri(
            &mut self.framebuffer,
            &mut self.zbuffer,
            2.0,
            0xFFDD_4444,
            1.5,
            0.0 + move_x * 1.5,
            -3.0,
            &self.view_proj,
        );

        apply_paper_cutout(&mut self.framebuffer, &self.zbuffer, &self.config);

        self.present()
    }
}

fn main() -> Result<(), AppError> {
    print_banner();
    run_windowed(PaperCutoutDemoApp::new()?);
    Ok(())
}
