import sys

filepath = "crates/abrash-render/src/experimental/mosaic.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace(
    "/// # Arguments",
    "/// # Panics\n///\n/// Panics if the internal framebuffer allocation fails due to memory exhaustion or extreme dimensions.\n///\n/// # Arguments"
)

with open(filepath, "w") as f:
    f.write(content)
