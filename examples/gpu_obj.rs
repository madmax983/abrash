#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::unnecessary_wraps)]
use abrash::gpu_render::{GpuDemoConfig, GpuVertex, mesh_to_gpu, run_mesh_demo};
use abrash::obj_loader::load_obj;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const SPACESHIP_OBJ: &str = r"
# Simple Spacerocket
v 0.0 1.5 0.0
v 0.5 -0.5 0.5
v -0.5 -0.5 0.5
v -0.5 -0.5 -0.5
v 0.5 -0.5 -0.5
v 0.0 -0.8 0.0
# Top pyramid
f 1 2 3
f 1 3 4
f 1 4 5
f 1 5 2
# Bottom inverted pyramid (engine)
f 6 3 2
f 6 4 3
f 6 5 4
f 6 2 5
";

fn print_banner() {
    println!("\n{}", "🎨 Abrash GPU Loader".bold().cyan());
    println!("{}", "=====================".dark_grey());
}

fn main() {
    print_banner();

    // 1. Load Mesh (CPU)
    let mesh = match load_obj(SPACESHIP_OBJ) {
        Ok(m) => m,
        Err(e) => {
            let mut error_table = Table::new();
            error_table
                .load_preset(presets::UTF8_FULL)
                .set_header(vec![
                    Cell::new("❌ Mesh Error")
                        .add_attribute(comfy_table::Attribute::Bold)
                        .fg(Color::Red),
                ])
                .add_row(vec![Cell::new(e).fg(Color::Yellow)]);
            eprintln!("\n{error_table}");
            std::process::exit(1);
        }
    };

    let mut mesh_table = Table::new();
    mesh_table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Source"),
            Cell::new("Built-in Spacerocket").fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Vertices"),
            Cell::new(mesh.vertices.len().to_string()),
        ])
        .add_row(vec![
            Cell::new("Triangles"),
            Cell::new(mesh.indices.len().to_string()),
        ])
        .add_row(vec![
            Cell::new("Status"),
            Cell::new("✅ Loaded (CPU)").fg(Color::Green),
        ]);

    println!("\n{}", "📦 Mesh Information".bold());
    println!("{mesh_table}");

    // 2. Convert to GPU Format (Bridge)
    let (vertices, indices) = match mesh_to_gpu(&mesh) {
        Ok(res) => res,
        Err(e) => {
            let mut error_table = Table::new();
            error_table
                .load_preset(presets::UTF8_FULL)
                .set_header(vec![
                    Cell::new("❌ Mesh Error")
                        .add_attribute(comfy_table::Attribute::Bold)
                        .fg(Color::Red),
                ])
                .add_row(vec![Cell::new(e).fg(Color::Yellow)]);
            eprintln!("\n{error_table}");
            std::process::exit(1);
        }
    };

    let mut gpu_table = Table::new();
    gpu_table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Buffer").fg(Color::Cyan),
            Cell::new("Count").fg(Color::Cyan),
            Cell::new("Size (Bytes)").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Vertex Buffer"),
            Cell::new(vertices.len().to_string()),
            Cell::new((vertices.len() * std::mem::size_of::<GpuVertex>()).to_string()),
        ])
        .add_row(vec![
            Cell::new("Index Buffer"),
            Cell::new(indices.len().to_string()),
            Cell::new((indices.len() * std::mem::size_of::<u16>()).to_string()),
        ])
        .add_row(vec![
            Cell::new("Status"),
            Cell::new("✅ Ready").fg(Color::Green),
            Cell::new("-"),
        ]);

    println!("\n{}", "💾 GPU Upload".bold());
    println!("{gpu_table}");

    // 3. Configure and Run (GPU)
    let config = GpuDemoConfig {
        title: "Abrash GPU OBJ Viewer".to_string(),
        initial_distance: 3.0,
        ..GpuDemoConfig::default()
    };

    println!("\n{}", "🚀 Launching GPU Demo...".bold().green());
    if let Err(e) = run_mesh_demo(vertices, indices, config) {
        let mut error_table = Table::new();
        error_table
            .load_preset(presets::UTF8_FULL)
            .set_header(vec![
                Cell::new("❌ GPU Error")
                    .add_attribute(comfy_table::Attribute::Bold)
                    .fg(Color::Red),
            ])
            .add_row(vec![Cell::new(e).fg(Color::Yellow)]);
        eprintln!("\n{error_table}");
        std::process::exit(1);
    }
}
