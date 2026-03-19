use abrash::experimental::wobble::{WobbleConfig, apply_wobble};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{Event, Window};
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "🌟 Wobble Filter Demo".bold().cyan());
    println!("{}", "=======================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Resolution"),
            Cell::new("640x480").fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Features"),
            Cell::new("Dynamic Wobble Effect").fg(Color::Green),
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
        .add_row(vec![Cell::new("Mouse"), Cell::new("None")])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("None")]);
    println!("{controls}\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();

    let width = 640;
    let height = 480;

    let mut window = Window::new("🌟 Nova: Wobble Filter Demo", width, height)?;
    let mut fb = Framebuffer::new(width, height)?;
    let mut background_fb = Framebuffer::new(width, height)?;

    // Generate a simple procedural background (e.g. checkerboard)
    for y in 0..height {
        for x in 0..width {
            let color = if (x / 32 + y / 32) % 2 == 0 {
                0xFF_222222 // Dark Gray
            } else {
                0xFF_DDDDDD // Light Gray
            };
            background_fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let mut config = WobbleConfig {
        amplitude: 20.0,
        frequency: 4.0,
        time: 0.0,
    };

    let mut time = 0.0;
    let time_step = 0.05;

    'main: loop {
        for event in window.poll_events() {
            if matches!(event, Event::Close) {
                break 'main;
            }
        }

        // Copy the pristine background into our working framebuffer
        fb.as_mut_slice().copy_from_slice(background_fb.as_slice());

        // Update time
        time += time_step;
        config.time = time;

        // Apply wobble effect
        apply_wobble(&mut fb, &config);

        // Blit to window
        window.blit_framebuffer(&fb);
    }

    Ok(())
}
