import re

with open("tests/verify_textured_gouraud_bilinear.rs", "r") as f:
    code = f.read()
code = code.replace("1568307511748357883", "1_568_307_511_748_357_883")
with open("tests/verify_textured_gouraud_bilinear.rs", "w") as f:
    f.write(code)

with open("examples/cel_shade_demo.rs", "r") as f:
    code = f.read()
code = code.replace("fn main() -> Result<(), AppError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")
with open("examples/cel_shade_demo.rs", "w") as f:
    f.write(code)

with open("examples/cloth_demo.rs", "r") as f:
    code = f.read()
code = code.replace("pub fn run() -> Result<(), Box<dyn std::error::Error>> {", "pub fn run() {")
code = code.replace("        Ok(())\n    }", "    }")
with open("examples/cloth_demo.rs", "w") as f:
    f.write(code)
