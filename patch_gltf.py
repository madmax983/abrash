import re

with open("examples/gltf_viewer.rs", "r") as f:
    content = f.read()

content = content.replace("fn main() {", "fn main() -> Result<(), Box<dyn std::error::Error>> {")
content = content.replace("    run_windowed(app);\n}", "    run_windowed(app);\n    Ok(())\n}")

with open("examples/gltf_viewer.rs", "w") as f:
    f.write(content)
