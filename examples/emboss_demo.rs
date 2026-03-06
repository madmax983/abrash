use abrash::experimental::emboss::apply_emboss;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{Window, WindowBackend};
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "🏔️  Emboss Filter Demo".bold().cyan());
    println!("{}", "=======================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Post-processing emboss effect").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Input"),
            Cell::new("Procedural Checkerboard").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Window"), Cell::new("Close window to exit")]);
    println!("{controls}\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    let width = 800;
    let height = 600;

    let mut window = Window::new("Abrash - Emboss Demo", width, height)?;
    let mut fb = Framebuffer::new(width, height)?;

    let mut offset_x = 0;
    let mut offset_y = 0;

    while window.is_open() {
        window.poll_events();

        // Animate the checkerboard
        offset_x += 1;
        offset_y += 1;

        // Draw some simple shapes
        for y in 0..height {
            for x in 0..width {
                let color = if (((x + offset_x) / 50) + ((y + offset_y) / 50)) % 2 == 0 {
                    0xFFFFFFFF // White
                } else {
                    0xFF808080 // Gray
                };
                fb.set_pixel(x as i32, y as i32, color);
            }
        }

        // Apply the emboss filter
        apply_emboss(&mut fb);

        window.blit_framebuffer(&fb);
    }

    Ok(())
}
