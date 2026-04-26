import re

with open("examples/normal_mapping_demo.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), AppError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")

with open("examples/normal_mapping_demo.rs", "w") as f:
    f.write(code)
