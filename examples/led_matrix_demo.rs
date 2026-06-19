use abrash::experimental::led_matrix::{LedMatrixConfig, apply_led_matrix};
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
    println!("\n{}", "🌟 LED Matrix Filter Demo".bold().cyan());
    println!("{}", "=======================".dark_grey());

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
            Cell::new("Applies a pixelated LED matrix / jumbotron effect").fg(Color::Green),
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

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "Nova: LED Matrix Filter Demo";

struct LedMatrixDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    background_fb: Framebuffer,
    time: f32,
}

impl LedMatrixDemoApp {
    fn new() -> Result<Self, HostError> {
        let mut background_fb =
            Framebuffer::new(WIDTH, HEIGHT).map_err(|error| HostError::App(error.to_string()))?;

        // Create a basic animated plasma-like gradient background
        for y in 0..HEIGHT {
            let v = y as f32 / HEIGHT as f32;
            for x in 0..WIDTH {
                let u = x as f32 / WIDTH as f32;

                let r = (u * 255.0) as u32;
                let g = (v * 255.0) as u32;
                let b = ((1.0 - u) * 255.0) as u32;

                let color = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
                background_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            background_fb,
            time: 0.0,
        })
    }

    fn present(&mut self) -> Result<(), HostError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for LedMatrixDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: WIDTH,
            height: HEIGHT,
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
        // Redraw dynamic shapes into background
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let mut u = x as f32 / WIDTH as f32;
                let mut v = y as f32 / HEIGHT as f32;

                u += (self.time * 0.5).sin() * 0.2;
                v += (self.time * 0.3).cos() * 0.2;

                let r = ((u * std::f32::consts::PI * 4.0).sin() * 127.0 + 128.0) as u32;
                let g = ((v * std::f32::consts::PI * 4.0).cos() * 127.0 + 128.0) as u32;
                let b = (((u + v) * std::f32::consts::PI * 2.0).sin() * 127.0 + 128.0) as u32;

                let color = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
                self.background_fb.set_pixel(x as i32, y as i32, color);
            }
        }

        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.background_fb.as_slice());

        // We want the LED effect to pulse slightly
        let pulse = (self.time.sin() * 0.5 + 0.5) * 1.5;
        let config = LedMatrixConfig {
            cell_size: 10,
            led_radius: 3.5 + pulse,
            background_darken: 0.1,
            edge_softness: 1.0,
        };

        apply_led_matrix(&mut self.framebuffer, &config);
        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    match LedMatrixDemoApp::new() {
        Ok(app) => run_windowed(app),
        Err(e) => {
            let mut error_table = comfy_table::Table::new();
            error_table
                .load_preset(comfy_table::presets::UTF8_FULL)
                .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                .set_header(vec![
                    comfy_table::Cell::new("❌ Initialization Error")
                        .add_attribute(comfy_table::Attribute::Bold)
                        .fg(comfy_table::Color::Red),
                ])
                .add_row(vec![
                    comfy_table::Cell::new(format!("{e}")).fg(comfy_table::Color::Yellow),
                ]);
            eprintln!("\n{error_table}");
            std::process::exit(1);
        }
    }
}
