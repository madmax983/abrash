file_path = "crates/abrash-render/src/experimental/tunnel.rs"
with open(file_path, "r") as f:
    content = f.read()

if "use abrash_core::math::fast_atan2;" not in content:
    content = content.replace("use abrash_core::framebuffer::Framebuffer;", "use abrash_core::framebuffer::Framebuffer;\nuse abrash_core::math::fast_atan2;")

with open(file_path, "w") as f:
    f.write(content)
