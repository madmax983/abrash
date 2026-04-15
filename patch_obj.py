import re

with open('examples/obj_viewer.rs', 'r') as f:
    content = f.read()

# Fix unused args and imports
content = content.replace("fn print_banner(source_name: &str, verts: usize, tris: usize) {", "fn print_banner(_source_name: &str, _verts: usize, _tris: usize) {")
content = content.replace("Cell::new(source_name).fg(Color::Yellow),", "Cell::new(_source_name).fg(Color::Yellow),")
content = content.replace("Cell::new(verts.to_string()),", "Cell::new(_verts.to_string()),")
content = content.replace("Cell::new(tris.to_string()),", "Cell::new(_tris.to_string()),")

with open('examples/obj_viewer.rs', 'w') as f:
    f.write(content)

with open('examples/gpu_deferred_showcase.rs', 'r') as f:
    content = f.read()

content = content.replace("use comfy_table::{Cell, Color, Table, presets};\nuse crossterm::style::Stylize;", "")

with open('examples/gpu_deferred_showcase.rs', 'w') as f:
    f.write(content)
