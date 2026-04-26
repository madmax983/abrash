import os
import re

with open("examples/raytracer_demo.rs", "r") as f:
    code = f.read()

code = code.replace("pub fn run() -> Result<(), Box<dyn std::error::Error>> {", "pub fn run() {")
code = code.replace("        Ok(())\n    }", "    }")

with open("examples/raytracer_demo.rs", "w") as f:
    f.write(code)
