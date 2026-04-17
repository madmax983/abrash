use abrash_render::experimental::mode7::{Mode7Config, render_mode7};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::texture::Texture;

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Mode 7 Pseudo-3D Demo".bold().cyan());
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
            Cell::new("Demonstrates classic SNES-style Mode 7 floor rendering").fg(Color::Green),
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
            Cell::new("Close window to exit"),
        ]);
    println!("{controls}\n");
}

struct Mode7Demo {
    texture: Texture,
    config: Mode7Config,
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
}

impl Mode7Demo {
    fn new() -> Self {
        // Generate a simple checkerboard texture with some color
        let tex_size = 256;
        let mut tex_data = vec![0; tex_size * tex_size];
        for y in 0..tex_size {
            for x in 0..tex_size {
                let check = ((x / 32) + (y / 32)) % 2 == 0;
                let color = if check { 0xFF44_AA44 } else { 0xFF22_6622 };

                // Add some grid lines
                let border = (x % 32 == 0) || (y % 32 == 0);
                tex_data[y * tex_size + x] = if border { 0xFF88_FF88 } else { color };
            }
        }

        let mut texture = Texture::new(tex_size as u32, tex_size as u32).unwrap();
        for y in 0..tex_size {
            for x in 0..tex_size {
                texture.set_pixel(x as u32, y as u32, tex_data[y * tex_size + x]);
            }
        }

        Self {
            texture,
            config: Mode7Config {
                cy: 30.0,
                fov: 300.0,
                horizon: 200.0,
                fog_start: 300.0,
                fog_end: 1500.0,
                fog_color: 0xFF88_CCFF, // Sky blue fog
                ..Default::default()
            },
            framebuffer: Framebuffer::new(800, 600).unwrap(),
            presenter: None,
        }
    }
}

impl WindowApp for Mode7Demo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Mode 7 Pseudo-3D Demo".to_string(),
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
        if width > 0
            && height > 0
            && (width != self.framebuffer.width() || height != self.framebuffer.height())
        {
            self.framebuffer = Framebuffer::new(width, height).unwrap();
        }
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let dt = ctx.dt_seconds;

        // Auto-pilot
        self.config.cx += self.config.angle.sin() * 50.0 * dt;
        self.config.cz -= self.config.angle.cos() * 50.0 * dt;
        self.config.angle += 0.2 * dt;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        let sky_color = 0xFF88_CCFF;
        self.framebuffer.clear(sky_color);

        // Draw gradient sky
        let horizon_i = self.config.horizon as i32;
        if horizon_i > 0 {
            for y in 0..horizon_i.min(self.framebuffer.height() as i32) as usize {
                let t = y as f32 / self.config.horizon;
                let r = (136.0 * t + 64.0 * (1.0 - t)) as u32;
                let g = (204.0 * t + 128.0 * (1.0 - t)) as u32;
                let b = (255.0 * t + 255.0 * (1.0 - t)) as u32;
                let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

                for x in 0..self.framebuffer.width() as usize {
                    self.framebuffer.set_pixel(x as i32, y as i32, color);
                }
            }
        }

        // Render the Mode 7 floor
        render_mode7(&mut self.framebuffer, &self.texture, &self.config);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }

        Ok(())
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    let app = Mode7Demo::new();
    run_windowed(app);
}
