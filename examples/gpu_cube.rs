use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "🧊 GPU Cube Demo".bold().cyan());
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
            Cell::new("Hardware-accelerated cube rendering").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Backend"),
            Cell::new("wgpu (Metal/Vulkan/DX12)").fg(Color::Yellow),
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
            Cell::new("Mouse"),
            Cell::new("Drag to orbit, Wheel to zoom"),
        ])
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Arrows/WASD (Orbit), Q/E (Zoom)"),
        ])
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Space (Toggle Rotate), R (Reset)"),
        ]);
    println!("{controls}\n");
}

fn main() {
    print_banner();
    if let Err(e) = abrash_gpu_render::run_gpu_cube() {
        let mut error_table = Table::new();
        error_table
            .load_preset(presets::UTF8_FULL)
            .set_header(vec![
                Cell::new("❌ Application Error")
                    .add_attribute(comfy_table::Attribute::Bold)
                    .fg(Color::Red),
            ])
            .add_row(vec![Cell::new(format!("{e}")).fg(Color::Yellow)]);

        eprintln!("\n{error_table}");
        std::process::exit(1);
    }
}
