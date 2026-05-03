use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::light_leak::{LightLeakConfig, apply_light_leak};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Light Leak Filter Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

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
            Cell::new("Simulates analog film light leaks and flares").fg(Color::Green),
        ]);

    println!("{table}\n");
}

struct LightLeakDemo {
    time: f32,
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
}

impl LightLeakDemo {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            time: 0.0,
            presenter: None,
            framebuffer: Framebuffer::new(640, 480)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(640, 480).map_err(|error| HostError::App(error.to_string()))?,
        })
    }
}

impl WindowApp for LightLeakDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Light Leak Filter (Nova)".to_string(),
            width: 640,
            height: 480,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let fb = &mut self.framebuffer;
        let zb = &mut self.zbuffer;

        // Clear to dark blue
        fb.clear(0xFF_001030);
        zb.clear();

        let width = fb.width();
        let height = fb.height();

        // Standard camera setup
        let eye = Vec3::new(0.0, 0.0, 5.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);
        let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);

        // Rotating cube for background context
        let rotation = Mat4::rotation_y(self.time) * Mat4::rotation_x(self.time * 0.7);
        let mvp = rotation * view * proj;

        let vertices = [
            // Front face
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            // Back face
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
        ];

        let faces = [
            // Front
            (0, 1, 2, 0xFF_5555FF),
            (0, 2, 3, 0xFF_5555FF),
            // Back
            (5, 4, 7, 0xFF_55FF55),
            (5, 7, 6, 0xFF_55FF55),
            // Left
            (4, 0, 3, 0xFF_FF5555),
            (4, 3, 7, 0xFF_FF5555),
            // Right
            (1, 5, 6, 0xFF_FFFF55),
            (1, 6, 2, 0xFF_FFFF55),
            // Top
            (3, 2, 6, 0xFF_55FFFF),
            (3, 6, 7, 0xFF_55FFFF),
            // Bottom
            (4, 5, 1, 0xFF_FF55FF),
            (4, 1, 0, 0xFF_FF55FF),
        ];

        for (i0, i1, i2, color) in faces.iter() {
            let v0 = mvp.transform_point(vertices[*i0]);
            let v1 = mvp.transform_point(vertices[*i1]);
            let v2 = mvp.transform_point(vertices[*i2]);

            fill_triangle_3d(fb, zb, v0, v1, v2, *color);
        }

        // Apply Light Leak Filter
        let config = LightLeakConfig {
            intensity: 0.9,
            time: self.time,
            color_primary: 0xFF_FF4400,
            color_secondary: 0xFF_FF0055,
        };
        apply_light_leak(fb, &config);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(fb)?;
        }

        Ok(())
    }
}

fn main() -> Result<(), HostError> {
    #[cfg(feature = "nova")]
    print_banner();

    let app = LightLeakDemo::new()?;
    run_windowed(app);
    Ok(())
}
