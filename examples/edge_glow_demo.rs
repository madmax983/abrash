#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash_render::experimental::edge_glow::{EdgeGlowConfig, apply_edge_glow};

#[cfg(all(feature = "nova", feature = "backend-tui"))]
use abrash::platform::tui::TuiWindow;
#[cfg(feature = "nova")]
use std::env;
#[cfg(all(feature = "nova", feature = "backend-tui"))]
use std::time::Duration;

use comfy_table::{Cell, Color, Table, presets};
#[cfg(all(feature = "nova", feature = "backend-tui"))]
use crossterm::event::{self, Event, KeyCode};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Edge Glow Demo".bold().magenta());
    println!("{}", "==========================".dark_grey());

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
            Cell::new("Edge Detection & Glow Post-Processing").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Effect"),
            Cell::new("Highlights edges with a neon glow").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Q / Esc"), Cell::new("Quit Demo")]);
    println!("{controls}\n");
}

#[cfg(all(feature = "nova", feature = "backend-winit"))]
mod winit_demo {
    use super::{EdgeGlowConfig, Framebuffer, apply_edge_glow};
    use abrash::platform::{
        SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed_app,
    };
    use std::io;

    #[allow(clippy::unnecessary_wraps)]
    pub fn run(width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
        run_windowed_app(
            EdgeGlowApp::new(width, height)
                .map_err(|e| abrash::platform::HostError::App(e.to_string())),
        );
        Ok(())
    }

    struct EdgeGlowApp {
        width: u32,
        height: u32,
        framebuffer: Framebuffer,
        presenter: Option<SoftwarePresenter>,
        config: EdgeGlowConfig,
    }

    impl EdgeGlowApp {
        fn new(width: u32, height: u32) -> Result<Self, io::Error> {
            let config = EdgeGlowConfig {
                edge_color: 0x00FF_00FF,
                intensity: 2.0,
                edge_threshold: 30,
                darken_factor: 0.1,
            };
            let framebuffer = Self::build_framebuffer(width, height, &config)?;
            Ok(Self {
                width,
                height,
                framebuffer,
                presenter: None,
                config,
            })
        }

        fn build_framebuffer(
            width: u32,
            height: u32,
            config: &EdgeGlowConfig,
        ) -> Result<Framebuffer, io::Error> {
            let mut framebuffer = Framebuffer::new(width, height).map_err(io::Error::other)?;
            framebuffer.clear(0xFF20_2020);

            for y in 100..200 {
                for x in 100..200 {
                    framebuffer.set_pixel(x, y, 0xFFFF_FFFF);
                }
            }

            for y in 300..400 {
                for x in 300..500 {
                    if (x + y) % 10 < 5 {
                        framebuffer.set_pixel(x, y, 0xFFAA_AAAA);
                    }
                }
            }

            apply_edge_glow(&mut framebuffer, config);
            Ok(framebuffer)
        }

        fn rebuild_framebuffer(&mut self, width: u32, height: u32) -> Result<(), io::Error> {
            self.width = width;
            self.height = height;
            self.framebuffer = Self::build_framebuffer(width, height, &self.config)?;
            Ok(())
        }
    }

    impl WindowApp for EdgeGlowApp {
        type Error = io::Error;

        fn config(&self) -> WindowHostConfig {
            WindowHostConfig {
                title: "Edge Glow Demo".to_string(),
                width: self.width,
                height: self.height,
                vsync: true,
            }
        }

        fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            self.presenter = Some(SoftwarePresenter::new(ctx.window).map_err(io::Error::other)?);
            Ok(())
        }

        fn resize(
            &mut self,
            _ctx: WindowContext<'_>,
            width: u32,
            height: u32,
        ) -> Result<(), Self::Error> {
            self.rebuild_framebuffer(width, height)
        }

        fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            Ok(())
        }

        fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
            let presenter = self
                .presenter
                .as_mut()
                .ok_or_else(|| io::Error::other("presenter not initialized"))?;
            presenter
                .present(&self.framebuffer)
                .map_err(io::Error::other)
        }
    }
}

#[cfg(feature = "nova")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();

    let width = 800;
    let height = 600;

    let use_tui = env::args().any(|arg| arg == "--tui");

    let mut fb = Framebuffer::new(width, height)?;

    // Draw something to show the effect
    fb.clear(0xFF_20_20_20); // Dark gray background

    // Draw some white squares
    for y in 100..200 {
        for x in 100..200 {
            fb.set_pixel(x, y, 0xFF_FF_FF_FF);
        }
    }

    // Draw some text-like pattern or random noise
    for y in 300..400 {
        for x in 300..500 {
            if (x + y) % 10 < 5 {
                fb.set_pixel(x, y, 0xFF_AA_AA_AA);
            }
        }
    }

    let config = EdgeGlowConfig {
        edge_color: 0x00_FF_00_FF, // Magenta
        intensity: 2.0,
        edge_threshold: 30,
        darken_factor: 0.1,
    };

    apply_edge_glow(&mut fb, &config);

    if use_tui {
        #[cfg(feature = "backend-tui")]
        {
            let mut window = TuiWindow::new("Edge Glow Demo", width, height)?;
            window.blit_framebuffer(&fb);

            loop {
                if event::poll(Duration::from_millis(100))?
                    && let Event::Key(key) = event::read()?
                {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }
        }
        #[cfg(not(feature = "backend-tui"))]
        {
            println!("TUI backend not enabled.");
        }
    } else {
        #[cfg(feature = "backend-winit")]
        {
            return winit_demo::run(width, height);
        }

        #[cfg(not(feature = "backend-winit"))]
        {
            println!(
                "Winit backend not enabled. Please use --tui or build with --features backend-winit"
            );
        }
    }

    Ok(())
}

#[cfg(not(feature = "nova"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            Cell::new("Try running with:\ncargo run --example edge_glow_demo --features nova")
                .fg(Color::Green),
        ]);

    eprintln!("\n{error_table}");
    std::process::exit(1);
}
