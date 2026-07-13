import re

with open('crates/abrash-render/src/procedural.rs', 'r') as f:
    content = f.read()

content = content.replace(
    "pub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {",
    "/// # Errors\n/// Returns an error if dimensions are zero or if allocation fails.\npub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {"
)

with open('crates/abrash-render/src/procedural.rs', 'w') as f:
    f.write(content)
