import os

file_path = "crates/abrash-render/src/experimental/tunnel.rs"
with open(file_path, "r") as f:
    content = f.read()

# Add import if missing
if "use abrash_core::math::funcs::fast_atan2;" not in content:
    content = content.replace("use crate::framebuffer::Framebuffer;", "use crate::framebuffer::Framebuffer;\nuse abrash_core::math::funcs::fast_atan2;")

# Replace f32::atan2 with fast_atan2
content = content.replace("let angle = f32::atan2(dy, dx);", "let angle = fast_atan2(dy, dx);")

with open(file_path, "w") as f:
    f.write(content)
