import os
import re

with open("examples/obj_viewer.rs", "r") as f:
    code = f.read()
code = code.replace(") -> Result<(), Box<dyn std::error::Error>> {", ") {")
code = code.replace("        Ok(())\n    }", "    }")
with open("examples/obj_viewer.rs", "w") as f:
    f.write(code)

with open("examples/night_vision_demo.rs", "r") as f:
    code = f.read()
code = code.replace("fn main() -> Result<(), AppError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")
with open("examples/night_vision_demo.rs", "w") as f:
    f.write(code)
