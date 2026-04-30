import re

with open("crates/abrash-render/src/experimental/vision.rs", "r") as f:
    content = f.read()

content = content.replace("let max_radius = center_x.hypot(center_y);", "#[allow(clippy::imprecise_flops)]\n        let max_radius = (center_x * center_x + center_y * center_y).sqrt();")
content = content.replace("let dist = dx.hypot(dy);", "#[allow(clippy::imprecise_flops)]\n        let dist = (dx * dx + dy * dy).sqrt();")

with open("crates/abrash-render/src/experimental/vision.rs", "w") as f:
    f.write(content)
