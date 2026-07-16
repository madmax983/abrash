import re

with open('crates/abrash-render/src/procedural.rs', 'r') as f:
    content = f.read()

content = content.replace("""///
/// # Errors
/// Returns an error if dimensions are zero or if texture allocation fails.
static PLASMA_LUT""", "static PLASMA_LUT")

content = content.replace("static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();\n\npub fn plasma", """static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();

/// # Errors
/// Returns an error if dimensions are zero or if texture allocation fails.
pub fn plasma""")

with open('crates/abrash-render/src/procedural.rs', 'w') as f:
    f.write(content)
