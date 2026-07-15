file_path = "crates/abrash-render/src/experimental/tunnel.rs"
with open(file_path, "r") as f:
    content = f.read()

content = content.replace(
    "                // Angle\n                let angle = fast_atan2(dy, dx);",
    "                // Angle\n                // ⚡ Bolt: Fast mathematical approximation for atan2 to reduce overhead\n                // and avoid the heavy CPU bottleneck in this per-pixel hot loop.\n                let angle = fast_atan2(dy, dx);"
)

with open(file_path, "w") as f:
    f.write(content)
