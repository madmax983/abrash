import re

with open("crates/abrash-render/src/render_api/cpu_renderer.rs", "r") as f:
    content = f.read()

content = content.replace("    \n\n\n    /// Execute", "    /// Execute")

with open("crates/abrash-render/src/render_api/cpu_renderer.rs", "w") as f:
    f.write(content)
