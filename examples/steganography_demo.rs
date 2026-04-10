use abrash::experimental::plasma::apply_plasma;
use abrash::experimental::steganography::{decode_message, encode_message};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

use comfy_table::{presets, Cell, Color, Table};
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Steganography Demo".bold().cyan());
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
            Cell::new("Encodes and decodes a hidden message in a Plasma framebuffer")
                .fg(Color::Green),
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
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Close window to exit"),
        ]);
    println!("{controls}\n");
}

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "🌟 Nova: Steganography Demo";
const SECRET_MESSAGE: &str = "Nova was here! This is a secret message hidden in the pixels.";

struct SteganographyDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    time: f32,
    encoded: bool,
}

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

        if !self.encoded {
            // Encode the message
            if let Err(e) = encode_message(&mut self.framebuffer, SECRET_MESSAGE) {
                let mut error_table = Table::new();
                error_table
                    .load_preset(presets::UTF8_FULL)
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
                    .set_header(vec![
                        Cell::new("✅ Steganography Active")
                            .add_attribute(comfy_table::Attribute::Bold)
                            .fg(Color::Green),
                    ])
                    .add_row(vec![
                        Cell::new("Status").fg(Color::Cyan),
                        Cell::new("Message successfully encoded into framebuffer.").fg(Color::White),
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
                            Cell::new("Success! The decoded message matches the original.").fg(Color::Green),
                        ]);
                        println!("\n{success_table}");
                    } else {
                        let mut error_table = Table::new();
                        error_table
                            .load_preset(presets::UTF8_FULL)
                            .set_header(vec![
                                Cell::new("❌ Decoding Error")
                                    .add_attribute(comfy_table::Attribute::Bold)
                                    .fg(Color::Red),
                            ])
                            .add_row(vec![Cell::new("The decoded message does NOT match the original.").fg(Color::Yellow)]);
                        eprintln!("\n{success_table}");
                        eprintln!("\n{error_table}");
                    }
                } else {
                    let mut error_table = Table::new();
                    error_table
                        .load_preset(presets::UTF8_FULL)
                        .set_header(vec![
                            Cell::new("❌ Decoding Error")
                                .add_attribute(comfy_table::Attribute::Bold)
                                .fg(Color::Red),
                        ])
                        .add_row(vec![Cell::new("Failed to decode message.").fg(Color::Yellow)]);
                    eprintln!("\n{success_table}");
                    eprintln!("\n{error_table}");
                }
            }
        } else {
            // we re-encode it every frame because we redraw the plasma
            if let Err(e) = encode_message(&mut self.framebuffer, SECRET_MESSAGE) {
                let mut error_table = Table::new();
                error_table
                    .load_preset(presets::UTF8_FULL)
                    .set_header(vec![
                        Cell::new("❌ Encoding Error")
                            .add_attribute(comfy_table::Attribute::Bold)
                            .fg(Color::Red),
                    ])
                    .add_row(vec![Cell::new(e).fg(Color::Yellow)]);
                eprintln!("\n{error_table}");
            }
        }

        self.present()
    }
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(SteganographyDemoApp::new().unwrap())
}
