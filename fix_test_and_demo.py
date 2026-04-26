import re

with open("tests/havoc_softbody.rs", "r") as f:
    code = f.read()

code = code.replace("0xFFFFFFFF", "0xFFFF_FFFF")

with open("tests/havoc_softbody.rs", "w") as f:
    f.write(code)

with open("examples/dither_demo.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), AppError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")

with open("examples/dither_demo.rs", "w") as f:
    f.write(code)
