import re

with open("examples/pencil_sketch_demo.rs", "r") as f:
    code = f.read()

code = code.replace("0xFF000000", "0xFF00_0000")

with open("examples/pencil_sketch_demo.rs", "w") as f:
    f.write(code)

with open("examples/jelly_demo.rs", "r") as f:
    code = f.read()

code = code.replace("pub fn run() -> Result<(), Box<dyn std::error::Error>> {", "pub fn run() {")
code = code.replace("        Ok(())\n    }", "    }")

with open("examples/jelly_demo.rs", "w") as f:
    f.write(code)
