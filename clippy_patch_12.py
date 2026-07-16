import re

with open('crates/abrash-render/src/procedural.rs', 'r') as f:
    content = f.read()

replacement = """///
/// # Errors
/// Returns an error if dimensions are zero or if texture allocation fails."""

content = content.replace("static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();", f"{replacement}\nstatic PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();")

with open('crates/abrash-render/src/procedural.rs', 'w') as f:
    f.write(content)
