#[cfg(feature = "nova")]
use abrash::experimental::edge_glow::{EdgeGlowConfig, apply_edge_glow};
#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;

#[cfg(all(feature = "nova", feature = "backend-tui"))]
use abrash::platform::tui::TuiWindow;
#[cfg(all(feature = "nova", feature = "backend-win32"))]
use abrash::platform::win32::Win32Window;
#[cfg(feature = "nova")]
use std::env;

use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Edge Glow Demo".bold().magenta());
    println!("{}", "==========================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
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
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![Cell::new("Q / Esc"), Cell::new("Quit Demo")]);
    println!("{controls}\n");
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

            use crossterm::event::{self, Event, KeyCode};
            use std::time::Duration;

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
        #[cfg(feature = "backend-win32")]
        {
            let mut window = Win32Window::new("Edge Glow Demo", width, height)?;
            window.blit_framebuffer(&fb);

            while window.is_open() {
                window.poll_events();
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        }
        #[cfg(not(feature = "backend-win32"))]
        {
            println!(
                "Win32 backend not enabled. Please use --tui or build with --features backend-win32"
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
