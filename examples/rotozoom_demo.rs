use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::texture::Texture;
use abrash_render::experimental::rotozoom::render_rotozoom;

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Rotozoom Effect Demo".bold().cyan());
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
            Cell::new("Demonstrates continuous rotation and scaling of a 2D texture")
                .fg(Color::Green),
        ]);

    println!("{table}");
}

#[cfg(not(feature = "nova"))]
fn print_banner() {
    println!("Rotozoom Demo (Requires 'nova' feature)");
}

struct RotozoomDemo {
    texture: Texture,
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    time: f32,
}

impl RotozoomDemo {
    fn new() -> Self {
        Self {
            texture: Texture::checkered(256, 256, 0xFF_FF0000, 0xFF_0000FF).unwrap(),
            framebuffer: Framebuffer::new(800, 600).unwrap(),
            presenter: None,
            time: 0.0,
        }
    }
}

impl WindowApp for RotozoomDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash Rotozoom Demo".to_string(),
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
        if width > 0 && height > 0 {
            self.framebuffer =
                Framebuffer::new(width, height).map_err(|e| HostError::App(e.to_string()))?;
        }
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF_000000);

        #[cfg(feature = "nova")]
        {
            let angle = self.time * 0.5; // Rotate over time
            let scale = 1.0 + (self.time * 2.0).sin() * 0.5; // Pulsate scale

            render_rotozoom(
                &mut self.framebuffer,
                &self.texture,
                angle,
                scale,
                128.0, // Center of the 256x256 texture
                128.0,
            );
        }

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }

        Ok(())
    }
}

fn main() {
    print_banner();
    let app = RotozoomDemo::new();
    run_windowed(app);
}
