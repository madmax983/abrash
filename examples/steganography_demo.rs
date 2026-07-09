#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
#[cfg(feature = "nova")]
use abrash_render::experimental::plasma::apply_plasma;
#[cfg(feature = "nova")]
use abrash_render::experimental::steganography::{decode_message, encode_message};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Steganography Demo".bold().cyan());
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
            Cell::new("Encodes and decodes a hidden message in a Plasma framebuffer")
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
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Close window to exit"),
        ]);
    println!("{controls}\n");
}

#[cfg(feature = "nova")]
const WIDTH: u32 = 640;
#[cfg(feature = "nova")]
const HEIGHT: u32 = 480;
#[cfg(feature = "nova")]
const TITLE: &str = "🌟 Nova: Steganography Demo";
#[cfg(feature = "nova")]
const SECRET_MESSAGE: &str = "Nova was here! This is a secret message hidden in the pixels.";

#[cfg(feature = "nova")]
struct SteganographyDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    time: f32,
    encoded: bool,
}

#[cfg(feature = "nova")]
impl SteganographyDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            time: 0.0,
            encoded: false,
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

#[cfg(feature = "nova")]
impl WindowApp for SteganographyDemoApp {
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
        // Generate a plasma background
        apply_plasma(&mut self.framebuffer, self.time, 1.0);

        if self.encoded {
            // we re-encode it every frame because we redraw the plasma
            if let Err(e) = encode_message(&mut self.framebuffer, SECRET_MESSAGE) {
                let mut error_table = Table::new();
                error_table
                    .load_preset(presets::UTF8_FULL)
                    .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        Cell::new("❌ Encoding Error")
                            .add_attribute(comfy_table::Attribute::Bold)
                            .fg(Color::Red),
                    ])
                    .add_row(vec![Cell::new(e).fg(Color::Yellow)]);
                eprintln!("\n{error_table}");
            }
        } else {
            // Encode the message
            if let Err(e) = encode_message(&mut self.framebuffer, SECRET_MESSAGE) {
                let mut error_table = Table::new();
                error_table
                    .load_preset(presets::UTF8_FULL)
                    .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        Cell::new("❌ Encoding Error")
                            .add_attribute(comfy_table::Attribute::Bold)
                            .fg(Color::Red),
                    ])
                    .add_row(vec![Cell::new(e).fg(Color::Yellow)]);
                eprintln!("\n{error_table}");
            } else {
                let mut success_table = Table::new();
                success_table
                    .load_preset(presets::UTF8_FULL)
                    .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        Cell::new("✅ Steganography Active")
                            .add_attribute(comfy_table::Attribute::Bold)
                            .fg(Color::Green),
                    ])
                    .add_row(vec![
                        Cell::new("Status").fg(Color::Cyan),
                        Cell::new("Message successfully encoded into framebuffer.")
                            .fg(Color::White),
                    ]);

                self.encoded = true;

                // Decode it immediately to prove it works
                if let Some(decoded) = decode_message(&self.framebuffer) {
                    success_table.add_row(vec![
                        Cell::new("Decoded").fg(Color::Cyan),
                        Cell::new(&decoded).fg(Color::Yellow),
                    ]);
                    if decoded == SECRET_MESSAGE {
                        success_table.add_row(vec![
                            Cell::new("Verification").fg(Color::Cyan),
                            Cell::new("Success! The decoded message matches the original.")
                                .fg(Color::Green),
                        ]);
                        println!("\n{success_table}");
                    } else {
                        let mut error_table = Table::new();
                        error_table
                            .load_preset(presets::UTF8_FULL)
                            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                            .set_header(vec![
                                Cell::new("❌ Decoding Error")
                                    .add_attribute(comfy_table::Attribute::Bold)
                                    .fg(Color::Red),
                            ])
                            .add_row(vec![
                                Cell::new("The decoded message does NOT match the original.")
                                    .fg(Color::Yellow),
                            ]);
                        eprintln!("\n{success_table}");
                        eprintln!("\n{error_table}");
                    }
                } else {
                    let mut error_table = Table::new();
                    error_table
                        .load_preset(presets::UTF8_FULL)
                        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                        .set_header(vec![
                            Cell::new("❌ Decoding Error")
                                .add_attribute(comfy_table::Attribute::Bold)
                                .fg(Color::Red),
                        ])
                        .add_row(vec![
                            Cell::new("Failed to decode message.").fg(Color::Yellow),
                        ]);
                    eprintln!("\n{success_table}");
                    eprintln!("\n{error_table}");
                }
            }
        }

        self.present()
    }
}

#[cfg(feature = "nova")]
fn main() {
    print_banner();

    run_windowed(SteganographyDemoApp::new().unwrap());
}

#[cfg(not(feature = "nova"))]
fn main() {
    let mut error_table = comfy_table::Table::new();
    error_table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            comfy_table::Cell::new("⚠️  Missing Feature: Nova")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Red),
        ])
        .add_row(vec![
            comfy_table::Cell::new("This example requires the 'nova' feature to run.")
                .fg(comfy_table::Color::White),
        ])
        .add_row(vec![
            comfy_table::Cell::new(
                "Try running with:\ncargo run --example steganography_demo --features nova",
            )
            .fg(comfy_table::Color::Green),
        ]);
    eprintln!("\n{error_table}");
    std::process::exit(1);
}
