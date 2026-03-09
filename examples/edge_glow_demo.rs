use abrash::experimental::edge_glow::{EdgeGlowConfig, apply_edge_glow};
use abrash::framebuffer::Framebuffer;

#[cfg(feature = "backend-tui")]
use abrash::platform::tui::TuiWindow;
#[cfg(feature = "backend-win32")]
use abrash::platform::win32::Win32Window;
use std::env;
use comfy_table::{Table, Cell, Color as TableColor, Attribute};

fn print_banner(width: u32, height: u32) {
    println!("\n\x1b[1;36m🌟 Edge Glow Demo\x1b[0m");
    println!("\x1b[90m=====================\x1b[0m");

    let mut table = Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL_CONDENSED)
        .set_header(vec![
            Cell::new("Property").add_attribute(Attribute::Bold),
            Cell::new("Value").add_attribute(Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Resolution").fg(TableColor::Cyan),
            Cell::new(format!("{}x{}", width, height)),
        ])
        .add_row(vec![
            Cell::new("Algorithm").fg(TableColor::Cyan),
            Cell::new("Sobel Operator Edge Detection"),
        ])
        .add_row(vec![
            Cell::new("Feature").fg(TableColor::Cyan),
            Cell::new("Nova (Experimental)"),
        ]);

    println!("\n\x1b[1m⚙️  Info\x1b[0m");
    println!("{table}");

    println!("\n\x1b[1m🎮 Controls\x1b[0m");
    let mut controls = Table::new();
    controls
        .load_preset(comfy_table::presets::NOTHING)
        .add_row(vec![
            Cell::new("[ESC] / [Q]").fg(TableColor::Yellow),
            Cell::new("Exit Demo"),
        ]);
    println!("{controls}\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 800;
    let height = 600;

    let use_tui = env::args().any(|arg| arg == "--tui");

    print_banner(width, height);

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
            std::thread::sleep(std::time::Duration::from_secs(3));
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
