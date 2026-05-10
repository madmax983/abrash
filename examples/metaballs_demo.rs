use abrash::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use abrash_render::experimental::metaballs::{Metaball, render_metaballs};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Metaballs Demo".bold().cyan());
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
            Cell::new("Simulates merging 2D liquid droplets (Metaballs)").fg(Color::Green),
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

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let time = 1.0f32; // Static time for headless demo
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    // Animate some metaballs using simple math
    let metaballs = vec![
        Metaball {
            position: Vec2::new(cx + time.cos() * 100.0, cy + time.sin() * 50.0),
            radius: 120.0,
            color: 0xFFFF_0000,
        },
        Metaball {
            position: Vec2::new(
                cx + (time * 1.5).sin() * 80.0,
                cy + (time * 1.2).cos() * 80.0,
            ),
            radius: 90.0,
            color: 0xFF00_FF00,
        },
        Metaball {
            position: Vec2::new(
                cx + (time * 0.8).cos() * 120.0,
                cy + (time * 0.5).sin() * 100.0,
            ),
            radius: 150.0,
            color: 0xFF00_00FF,
        },
    ];

    fb.clear(0xFF00_0000);
    render_metaballs(&mut fb, &metaballs, 1.0);

    // Normally this would be rendered to a window, but this is a minimal headless example
    // just to show it compiles and runs.
    println!("Metaballs demo ran successfully!");
}
