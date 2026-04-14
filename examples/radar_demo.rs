use abrash::experimental::radar::{RadarConfig, apply_radar};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Radar Sweep Demo".bold().cyan());
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
            Cell::new("Demonstrates a retro radar sweep effect").fg(Color::Green),
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
        .add_row(vec![Cell::new("ESC"), Cell::new("Exit")]);

    println!("{controls}\n");
}

struct RadarDemoApp {
    fb: Framebuffer,
    time: f32,
    config: RadarConfig,
    presenter: Option<SoftwarePresenter>,
}

impl RadarDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            fb: Framebuffer::new(800, 600).unwrap(),
            time: 0.0,
            config: RadarConfig::default(),
            presenter: None,
        })
    }
}

impl WindowApp for RadarDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            width: 800,
            height: 600,
            title: "Radar Sweep Post-Processing".into(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds.max(0.0);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.fb.clear(0xFF_00_00_00);

        // Target 1
        for y in 200..220 {
            for x in 500..520 {
                self.fb.set_pixel(x, y, 0xFF_FF_FF_FF);
            }
        }

        // Target 2
        for y in 400..410 {
            for x in 250..260 {
                self.fb.set_pixel(x, y, 0xFF_FF_FF_FF);
            }
        }

        apply_radar(&mut self.fb, self.time, &self.config);

        if let Some(p) = self.presenter.as_mut() {
            p.present(&self.fb)?;
        }
        Ok(())
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();
    run_windowed(RadarDemoApp::new().unwrap())
}
