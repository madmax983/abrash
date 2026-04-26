import os
import re

for filename in ["examples/pixel_sort_demo.rs", "examples/voronoi_demo.rs", "examples/anaglyph_demo.rs"]:
    with open(filename, "r") as f:
        code = f.read()

    code = code.replace("fn main() -> Result<(), AppError> {", "fn main() {")
    code = code.replace("    Ok(())\n}", "}")

    with open(filename, "w") as f:
        f.write(code)
