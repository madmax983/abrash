use abrash::gpu_render::{GpuDemoConfig, GpuVertex, run_mesh_demo};
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn pyramid_mesh() -> (Vec<GpuVertex>, Vec<u16>) {
    let vertices = vec![
        GpuVertex {
            position: [-1.1, -1.0, -1.1],
            color: [0.9, 0.2, 0.2],
        },
        GpuVertex {
            position: [1.1, -1.0, -1.1],
            color: [0.2, 0.9, 0.2],
        },
        GpuVertex {
            position: [1.1, -1.0, 1.1],
            color: [0.2, 0.2, 0.9],
        },
        GpuVertex {
            position: [-1.1, -1.0, 1.1],
            color: [0.9, 0.9, 0.2],
        },
        GpuVertex {
            position: [0.0, 1.3, 0.0],
            color: [1.0, 0.7, 0.2],
        },
    ];

    let indices = vec![
        // Side faces
        0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4, // Base (two triangles)
        0, 3, 2, 2, 1, 0,
    ];

    (vertices, indices)
}

fn print_banner() {
    println!("\n{}", "📐 GPU Pyramid Demo".bold().cyan());
    println!("{}", "========================".dark_grey());

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
            Cell::new("Generic mesh rendering test").fg(Color::Green),
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
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
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
    let (vertices, indices) = pyramid_mesh();

    let config = GpuDemoConfig {
        title: "Abrash GPU Pyramid (wgpu)".to_string(),
        rotation_speed: 0.55,
        initial_pitch: 0.45,
        initial_distance: 5.2,
        ..GpuDemoConfig::default()
    };

    if let Err(e) = run_mesh_demo(vertices, indices, config) {
        let mut error_table = Table::new();
        error_table
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
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
