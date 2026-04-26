import os
import re

with open("examples/gpu_deferred_showcase.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), DemoError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")

with open("examples/gpu_deferred_showcase.rs", "w") as f:
    f.write(code)
