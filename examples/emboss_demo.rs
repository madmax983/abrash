#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash_render::experimental::emboss::apply_emboss;
#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner(width: u32, height: u32) {
    println!("\n{}", "🗿 Emboss Filter Demo".bold().cyan());
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

#[cfg(feature = "nova")]
fn print_success(width: u32, height: u32) {
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("✅ Status")
                .fg(Color::Green)
                .add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Details").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Success"),
            Cell::new(format!(
                "Emboss filter applied to {width}x{height} framebuffer."
            )),
        ]);

    println!("\n{table}");
}

#[cfg(feature = "nova")]
fn main() {
    let width = 800;
    let height = 600;
    print_banner(width, height);

    let mut fb = Framebuffer::new(width, height).unwrap();

    // Draw some simple shapes
    for y in 0..height {
        for x in 0..width {
            let color = if (x / 50 + y / 50) % 2 == 0 {
                0xFFFF_FFFF // White
            } else {
                0xFF80_8080 // Gray
            };
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    // Apply the emboss filter
    apply_emboss(&mut fb);

    print_success(width, height);
}

#[cfg(not(feature = "nova"))]
fn main() {
    use comfy_table::{Cell, Color, Table, presets};
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
            Cell::new("Try running with:\ncargo run --example emboss_demo --features nova")
                .fg(Color::Cyan),
        ]);

    eprintln!("\n{error_table}");
    std::process::exit(1);
}
