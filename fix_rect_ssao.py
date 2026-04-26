import re

with open("benches/rect_bench.rs", "r") as f:
    code = f.read()

code = code.replace("0xFFFFFFFF", "0xFFFF_FFFF")

with open("benches/rect_bench.rs", "w") as f:
    f.write(code)

with open("examples/ssao_demo.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), AppError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")

with open("examples/ssao_demo.rs", "w") as f:
    f.write(code)
