import re

with open("crates/abrash-render/src/procedural.rs", "r") as f:
    content = f.read()

content = content.replace("static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();\n\npub fn plasma", "static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();\n\n#[allow(clippy::missing_errors_doc)]\npub fn plasma")

with open("crates/abrash-render/src/procedural.rs", "w") as f:
    f.write(content)
