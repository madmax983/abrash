import re

with open("examples/raytracer_demo.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), Box<dyn std::error::Error>> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")

with open("examples/raytracer_demo.rs", "w") as f:
    f.write(code)

with open("examples/edge_glow_demo.rs", "r") as f:
    code = f.read()

code = code.replace("return winit_demo::run(width, height);", "winit_demo::run(width, height);\n            return Ok(());")

with open("examples/edge_glow_demo.rs", "w") as f:
    f.write(code)
