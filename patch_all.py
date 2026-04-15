import os
import re

# obj viewer
with open('examples/obj_viewer.rs', 'r') as f:
    content = f.read()

new_banner = """fn print_banner(source_name: &str, verts: usize, tris: usize) {
    println!("\\n{}", "🎨 Abrash OBJ Viewer".bold().cyan());
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
            Cell::new("Source"),
            Cell::new(source_name).fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Status"),
            Cell::new("✅ Loaded Successfully").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Vertices"),
            Cell::new(verts.to_string()),
        ])
        .add_row(vec![
            Cell::new("Triangles"),
            Cell::new(tris.to_string()),
        ]);

    println!("\\n{}", "📦 Asset Information".bold());
    println!("{table}");

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
            Cell::new("(Coming Soon)").fg(Color::DarkGrey),
        ])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);

    println!("\\n{}", "🎮 Controls".bold());
    println!("{controls}\\n");
}
"""

content = content.replace("fn main() -> Result<(), Box<dyn std::error::Error>> {", new_banner + "\nfn main() -> Result<(), Box<dyn std::error::Error>> {")

old_asset_info = """let mut table = Table::new();
            table
                .load_preset(presets::UTF8_FULL)
                .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                .set_header(vec![
                    Cell::new("Property").fg(Color::Cyan),
                    Cell::new("Value").fg(Color::Cyan),
                ])
                .add_row(vec![
                    Cell::new("Source"),
                    Cell::new(&source_name).fg(Color::Yellow),
                ])
                .add_row(vec![
                    Cell::new("Status"),
                    Cell::new("✅ Loaded Successfully").fg(Color::Green),
                ])
                .add_row(vec![
                    Cell::new("Vertices"),
                    Cell::new(m.vertices.len().to_string()),
                ])
                .add_row(vec![
                    Cell::new("Triangles"),
                    Cell::new(m.indices.len().to_string()),
                ]);

            println!("\\n{}", "📦 Asset Information".bold());
            println!("{table}");
            m"""
content = content.replace(old_asset_info, "print_banner(&source_name, m.vertices.len(), m.indices.len());\n            m")

old_controls = """let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Mouse"),
            Cell::new("(Coming Soon)").fg(Color::DarkGrey),
        ])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);

    println!("\\n{}", "🎮 Controls".bold());
    println!("{controls}\\n");"""
content = content.replace(old_controls, "")

content = content.replace('println!("\\n{}", "🎨 Abrash OBJ Viewer".bold().cyan());\n    println!("{}", "=====================".dark_grey());', '')

with open('examples/obj_viewer.rs', 'w') as f:
    f.write(content)

# mandelbrot
with open('examples/mandelbrot_demo.rs', 'r') as f:
    content = f.read()

banner_imports = "use comfy_table::{Cell, Color, Table, presets};\nuse crossterm::style::Stylize;\n"
if "use comfy_table" not in content:
    content = content.replace("use abrash::framebuffer::Framebuffer;", banner_imports + "use abrash::framebuffer::Framebuffer;")

banner_func = """
fn print_banner() {
    println!("\\n{}", "🌟 Mandelbrot Demo".bold().cyan());
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
            Cell::new("Renders the Mandelbrot set").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Behavior"),
            Cell::new("Auto-zooming").fg(Color::Yellow),
        ]);

    println!("\\n{}", "⚙️  Info".bold());
    println!("{table}");

    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Close window to exit")]);

    println!("\\n{}", "🎮 Controls".bold());
    println!("{controls}\\n");
}
"""

content = content.replace("fn main() {\n    println!(\"🌟 Nova: Mandelbrot Demo\");", banner_func + "\n#[cfg(feature = \"nova\")]\nfn main() {\n    print_banner();")

with open('examples/mandelbrot_demo.rs', 'w') as f:
    f.write(content)

# gpu_mvp
with open('examples/gpu_mvp_cube.rs', 'r') as f:
    content = f.read()

banner_func = """
fn print_banner() {
    use comfy_table::{Cell, Color, Table, presets};
    use crossterm::style::Stylize;

    println!("\\n{}", "🧊 GPU MVP Cube Demo".bold().cyan());
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
            Cell::new("Hardware-accelerated spinning cube").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("WGPU Backend").fg(Color::Yellow),
        ]);

    println!("\\n{}", "⚙️  Info".bold());
    println!("{table}");

    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![Cell::new("Mouse"), Cell::new("None")])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);

    println!("\\n{}", "🎮 Controls".bold());
    println!("{controls}\\n");
}
"""

content = content.replace("fn main() -> Result<(), DemoError> {", banner_func + "\nfn main() -> Result<(), DemoError> {\n    print_banner();")

to_remove_start = content.find('use comfy_table::{Cell, Color, Table, presets};')
to_remove_end = content.find('println!("{controls}\\n");', to_remove_start) + len('println!("{controls}\\n");')

if to_remove_start != -1 and to_remove_end != -1:
    content = content[:to_remove_start] + content[to_remove_end:]

with open('examples/gpu_mvp_cube.rs', 'w') as f:
    f.write(content)

# color_splash
with open('examples/color_splash_demo.rs', 'r') as f:
    content = f.read()

if "use comfy_table" not in content:
    content = content.replace("use abrash::framebuffer::Framebuffer;", banner_imports + "use abrash::framebuffer::Framebuffer;")

banner_func = """
fn print_banner() {
    println!("\\n{}", "🌟 Color Splash Demo".bold().cyan());
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
            Cell::new("Selective color post-processing filter").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Behavior"),
            Cell::new("Moving colored triangles in a circle").fg(Color::Yellow),
        ]);

    println!("\\n{}", "⚙️  Info".bold());
    println!("{table}");

    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Close window to exit")]);

    println!("\\n{}", "🎮 Controls".bold());
    println!("{controls}\\n");
}
"""

content = content.replace("fn main() {", banner_func + "\nfn main() {\n    print_banner();")

with open('examples/color_splash_demo.rs', 'w') as f:
    f.write(content)


# reaction
with open('examples/reaction_diffusion_demo.rs', 'r') as f:
    content = f.read()

if "use comfy_table" not in content:
    content = content.replace("use abrash::framebuffer::Framebuffer;", banner_imports + "use abrash::framebuffer::Framebuffer;")

banner_func = """
fn print_banner() {
    println!("\\n{}", "🌟 Reaction-Diffusion (Gray-Scott) Demo".bold().cyan());
    println!("{}", "=======================================".dark_grey());

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
            Cell::new("Simulates the Turing patterns formed by two interacting chemicals.").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Behavior"),
            Cell::new("Double-buffered grid and 3x3 Laplacian convolutions").fg(Color::Yellow),
        ]);

    println!("\\n{}", "⚙️  Info".bold());
    println!("{table}");

    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Close window to exit")]);

    println!("\\n{}", "🎮 Controls".bold());
    println!("{controls}\\n");
}
"""

content = content.replace("fn main() -> Result<(), Box<dyn std::error::Error>> {", banner_func + "\nfn main() -> Result<(), Box<dyn std::error::Error>> {\n    print_banner();")

with open('examples/reaction_diffusion_demo.rs', 'w') as f:
    f.write(content)


# gpu_deferred
with open('examples/gpu_deferred_showcase.rs', 'r') as f:
    content = f.read()

banner_func = """
fn print_banner() {
    use comfy_table::{Cell, Color, Table, presets};
    use crossterm::style::Stylize;

    println!(
        "\\n{}",
        "✨ Abrash Deferred Rendering Showcase".bold().cyan()
    );
    println!("{}", "=====================================".dark_grey());

    let mut info_table = Table::new();
    info_table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Pipeline"),
            Cell::new("Shadow > G-Buffer > Deferred Lighting > Tone Map").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Lights"),
            Cell::new("1 directional (shadows) + 3 point (orbiting)").fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Meshes"),
            Cell::new("sphere, cube, cylinder, torus, plane"),
        ])
        .add_row(vec![
            Cell::new("Objects"),
            Cell::new("15 objects, 6 materials"),
        ]);

    println!("\\n{}", "⚙️  Info".bold());
    println!("{info_table}");

    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("D"),
            Cell::new("Cycle debug modes (position/normal/albedo/roughness/metallic/depth)"),
        ])
        .add_row(vec![
            Cell::new("T"),
            Cell::new("Toggle TAA (temporal anti-aliasing)"),
        ])
        .add_row(vec![
            Cell::new("Space"),
            Cell::new("Pause/resume animation"),
        ])
        .add_row(vec![Cell::new("Esc"), Cell::new("Quit")]);

    println!("\\n{}", "🎮 Controls".bold());
    println!("{controls}\\n");
}
"""

content = content.replace("fn main() -> Result<(), DemoError> {", banner_func + "\nfn main() -> Result<(), DemoError> {\n    print_banner();")

to_remove_start = content.find('println!(\n            "\\n{}",\n            "✨ Abrash Deferred Rendering Showcase".bold().cyan()\n        );')
to_remove_end = content.find('println!("{controls}\\n");', to_remove_start) + len('println!("{controls}\\n");')

if to_remove_start != -1 and to_remove_end != -1:
    content = content[:to_remove_start] + content[to_remove_end:]

with open('examples/gpu_deferred_showcase.rs', 'w') as f:
    f.write(content)
