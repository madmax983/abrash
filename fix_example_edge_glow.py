import re

with open("examples/edge_glow_demo.rs", "r") as f:
    code = f.read()

code = code.replace("pub fn run(width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {", "pub fn run(width: u32, height: u32) {")
code = code.replace("        Ok(())\n    }", "    }")

with open("examples/edge_glow_demo.rs", "w") as f:
    f.write(code)
