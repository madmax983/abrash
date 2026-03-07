use abrash::experimental::emboss::apply_emboss;
use abrash::framebuffer::Framebuffer;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner(width: u32, height: u32) {
    println!("\n{}", "🗿 Emboss Filter Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Resolution"),
            Cell::new(format!("{width}x{height}")).fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Effect"),
            Cell::new("Directional Emboss").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

fn print_success(width: u32, height: u32) {
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("✅ Status").fg(Color::Green).add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Details").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Success"),
            Cell::new(format!("Emboss filter applied to {width}x{height} framebuffer.")),
        ]);

    println!("\n{}", table);
}

fn main() {
    let width = 800;
    let height = 600;
    print_banner(width, height);

    let mut fb = Framebuffer::new(width, height).unwrap();

    // Draw some simple shapes
    for y in 0..height {
        for x in 0..width {
            let color = if (x / 50 + y / 50) % 2 == 0 {
                0xFFFFFFFF // White
            } else {
                0xFF808080 // Gray
            };
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    // Apply the emboss filter
    apply_emboss(&mut fb);

    print_success(width, height);
}
