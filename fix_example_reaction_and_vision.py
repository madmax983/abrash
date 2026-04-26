import re

with open("examples/reaction_diffusion_demo.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), Box<dyn std::error::Error>> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")

with open("examples/reaction_diffusion_demo.rs", "w") as f:
    f.write(code)

with open("examples/vision_demo.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), AppError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")

with open("examples/vision_demo.rs", "w") as f:
    f.write(code)
